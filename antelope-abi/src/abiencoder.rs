use std::collections::HashMap;

// use anyhow::Result;
use color_eyre::eyre::Result;
use strum::VariantNames;
use tracing::{debug, instrument};

use crate::abi::*;
use crate::{
    binaryserializable::write_var_u32,
    abiserializable::ABISerializable,
};

use serde_json::{
    Map as JsonMap,
    Value as JsonValue,
    // Error as JsonError,
    json
};

use super::*;

use antelope_core::{Name, AntelopeType, AntelopeValue, InvalidValue};

#[derive(Default)]
pub struct ABIEncoder {
    // ABI-related fields
    typedefs: HashMap<TypeName, TypeName>,
    structs: HashMap<TypeName, Struct>,
    actions: HashMap<Name, TypeName>,
    tables: HashMap<Name, TypeName>,
    variants: HashMap<TypeName, Variant>,

    // TODO: missing https://github.com/AntelopeIO/leap/blob/6817911900a088c60f91563995cf482d6b380b2d/libraries/chain/abi_serializer.cpp#L140-L142

}


impl ABIEncoder {
    pub fn new() -> Self {
        Self {
            // data: ByteStream::new(),
            typedefs: HashMap::new(),
            structs: HashMap::new(),
            actions: HashMap::new(),
            tables: HashMap::new(),
            variants: HashMap::new(),
        }
    }

    pub fn from_abi(abi: &ABIDefinition) -> Self {
        let mut result = Self::new();
        result.set_abi(abi);
        result
    }

    pub fn with_abi(abi: &ABIDefinition) -> Self { Self::from_abi(abi) }

    pub fn from_hex_abi(abi: &str) -> Result<Self, InvalidValue> {
        Self::from_bin_abi(&hex::decode(abi)?)
    }

    pub fn from_bin_abi(abi: &[u8]) -> Result<Self, InvalidValue> {
        let mut data = ByteStream::from(abi.to_owned());
        let abi_def = ABIDefinition::from_bin(&mut data)?;
        Ok(Self::from_abi(&abi_def))
    }

    // FIXME: we should probably move abi in there instead of passing a ref
    pub fn set_abi(&mut self, abi: &ABIDefinition) {
        self.typedefs.clear();
        self.structs.clear();
        self.actions.clear();
        self.tables.clear();
        self.variants.clear();

        // FIXME: check if we have to clone objects here or if it is ok to keep refs only
        //        maybe we want to move the whole ABIDefinition inside the encoder so it
        //        owns it and then we're fine just using refs everywhere

        for s in &abi.structs { self.structs.insert(s.name.to_owned(), s.clone()); }
        for td in &abi.types {
            assert!(!self.is_type(&td.new_type_name),
                    "Type already exists: {}", td.new_type_name);
            self.typedefs.insert(td.new_type_name.clone(), td.type_.clone());
        }

        for a in &abi.actions { self.actions.insert(a.name, a.type_.clone()); }
        for t in &abi.tables { self.tables.insert(t.name, t.type_.clone()); }
        for v in &abi.variants { self.variants.insert(v.name.clone(), v.clone()); }

        // The ABI vector may contain duplicates which would make it an invalid ABI
        assert_eq!(self.typedefs.len(), abi.types.len(), "duplicate type definition detected");
        assert_eq!(self.structs.len(), abi.structs.len(), "duplicate struct definition detected");
        assert_eq!(self.actions.len(), abi.actions.len(), "duplicate action definition detected");
        assert_eq!(self.tables.len(), abi.tables.len(), "duplicate table definition detected");
        assert_eq!(self.variants.len(), abi.variants.len(), "duplicate variants definition detected");

        self.validate();
    }

    pub fn is_type(&self, t: &str) -> bool {
        let t = fundamental_type(t);
        AntelopeValue::VARIANTS.contains(&t) ||
            (self.typedefs.contains_key(t) && self.is_type(self.typedefs.get(t).unwrap())) ||
            self.structs.contains_key(t) ||
            self.variants.contains_key(t)
    }

    pub fn resolve_type<'a>(&'a self, t: &'a str) -> &'a str {
        let mut rtype = t;
        loop {
            match self.typedefs.get(rtype) {
                Some(t) => rtype = t,
                None => return rtype,
            }
        }

    }

    pub fn json_to_bin(&self, typename: &str, obj: &JsonValue) -> Result<Vec<u8>, InvalidValue> {
        let mut ds = ByteStream::new();
        self.encode_variant(&mut ds, typename, obj)?;
        Ok(ds.pop())
    }

    pub fn encode(&self, stream: &mut ByteStream, obj: &AntelopeValue) {
        obj.to_bin(stream)
    }

    #[instrument(skip(self, ds))]
    pub fn encode_variant(&self, ds: &mut ByteStream, typename: &str, object: &JsonValue) -> Result<(), InvalidValue> {
        // see C++ implementation here: https://github.com/AntelopeIO/leap/blob/main/libraries/chain/abi_serializer.cpp#L491
        let rtype = self.resolve_type(typename);
        let ftype = fundamental_type(rtype); //.to_owned();  // FIXME: remove this .to_owned()

        debug!("rtype: {} - ftype: {}", rtype, ftype);

        let incompatible_types = || {
            InvalidValue::IncompatibleVariantTypes(rtype.to_owned(), object.clone())
        };

        if AntelopeValue::VARIANTS.contains(&ftype) {
            // if our fundamental type is a builtin type, we can serialize it directly
            // to the stream
            if is_array(rtype) {
                let a = object.as_array().ok_or_else(incompatible_types)?;
                AntelopeValue::VarUint32(a.len() as u32).to_bin(ds);
                for v in a {
                    AntelopeValue::from_variant(ftype.try_into()?, v)?.to_bin(ds);
                }
            }
            else if is_optional(rtype) {
                match !object.is_null() {
                    true => {
                        AntelopeValue::Bool(true).to_bin(ds);
                        AntelopeValue::from_variant(ftype.try_into()?, object)?.to_bin(ds);
                    },
                    false => AntelopeValue::Bool(false).to_bin(ds),
                }
            }
            else {
                AntelopeValue::from_variant(ftype.try_into()?, object)?.to_bin(ds);
            }
        }
        else {
            // not a builtin type, we have to recurse down

            if is_array(rtype) {
                let Some(a) = object.as_array() else {
                    return Err(InvalidValue::InvalidData("JSON object cannot be converted to array".to_owned())); };
                write_var_u32(ds, a.len() as u32);
                for v in a {
                    self.encode_variant(ds, ftype, v)?;
                }
            }
            else if is_optional(rtype) {
                match !object.is_null() {
                    true => {
                        AntelopeValue::Bool(true).to_bin(ds);
                        self.encode_variant(ds, ftype, object)?;
                    },
                    false => AntelopeValue::Bool(false).to_bin(ds),
                }
            }
            else if let Some(variant_def) = self.variants.get(rtype) {
                debug!("serializing type {:?} with variant: {:?}", rtype, object);
                assert!(object.is_array() && object.as_array().unwrap().len() == 2,
                        "expected input to be an array of 2 elements while processing variant: {}",
                        &object);
                assert!(object[0].is_string(), "expected variant typename to be a string: {}",
                        object[0]);
                let variant_type = object[0].as_str().unwrap();
                if let Some(vpos) = variant_def.types.iter().position(|v| v == variant_type) {
                    write_var_u32(ds, vpos as u32);
                    self.encode_variant(ds, variant_type, &object[1])?;
                }
                else {
                    panic!("specified type {} is not valid within the variant {}",
                           variant_type, rtype);
                }
            }
            else if let Some(struct_def) = self.structs.get(rtype) {
                if object.is_object() {
                    if !struct_def.base.is_empty() {
                        self.encode_variant(ds, &struct_def.base, object)?;
                    }
                    let obj = object.as_object().unwrap();

                    for field in &struct_def.fields {
                        let present: bool = obj.contains_key(&field.name);
                        assert!(present, r#"Missing field "{}" in input object while processing struct "{}""#,
                                &field.name, &struct_def.name);
                        self.encode_variant(ds, &field.type_, obj.get(&field.name).unwrap())?;
                    }
                }
                else if object.is_array() {
                    unimplemented!();
                }
                else {
                    // error
                    unimplemented!();
                }
            }
            else {
                panic!("Do not know how to serialize type: {}", rtype);
            }
        }

        Ok(())
    }

    #[allow(clippy::collapsible_else_if)]
    pub fn decode_variant(&self, ds: &mut ByteStream, typename: &str) -> Result<JsonValue, InvalidValue> {
        let rtype = self.resolve_type(typename);
        let ftype = fundamental_type(rtype);

        Ok(if AntelopeValue::VARIANTS.contains(&ftype) {
            // if our fundamental type is a builtin type, we can deserialize it directly
            // from the stream
            if is_array(rtype) {
                let item_count: usize = AntelopeValue::from_bin(AntelopeType::VarUint32, ds)?.try_into()?;
                debug!(r#"reading array of {item_count} elements of type "{ftype}""#);
                let mut a = Vec::with_capacity(item_count);
                for _ in 0..item_count {
                    a.push(AntelopeValue::from_bin(ftype.try_into()?, ds)?.to_variant());
                }
                JsonValue::Array(a)
            }
            else if is_optional(rtype) {
                let non_null: bool = AntelopeValue::from_bin(AntelopeType::Bool, ds)?.into();
                match non_null {
                    true => AntelopeValue::from_bin(ftype.try_into()?, ds)?.to_variant(),
                    false => JsonValue::Null,
                }
            }
            else {
                AntelopeValue::from_bin(ftype.try_into()?, ds)?.to_variant()
            }
        }
        else {
            if is_array(rtype) {
                // not a builtin type, we have to recurse down
                let item_count: usize = AntelopeValue::from_bin(AntelopeType::VarUint32, ds)?.try_into()?;
                debug!(r#"reading array of {item_count} elements of type "{ftype}""#);
                let mut a = Vec::with_capacity(item_count);
                for _ in 0..item_count {
                    a.push(self.decode_variant(ds, ftype)?);
                }
                JsonValue::Array(a)
            }
            else if is_optional(rtype) {
                let non_null = AntelopeValue::from_bin(AntelopeType::Bool, ds)?.into();
                match non_null {
                    true => self.decode_variant(ds, ftype)?,
                    false => JsonValue::Null,
                }
            }
            else if let Some(variant_def) = self.variants.get(rtype) {
                let variant_tag: usize = AntelopeValue::from_bin(AntelopeType::VarUint32, ds)?.try_into()?;
                assert!(variant_tag < variant_def.types.len(),
                        "deserialized invalid tag {} for variant {}", variant_tag, rtype);
                let variant_type = &variant_def.types[variant_tag];
                json!([variant_type, self.decode_variant(ds, variant_type)?])
            }
            else if let Some(struct_def) = self.structs.get(rtype) {
                self.decode_struct(ds, struct_def)?
            }
            else {
                panic!("Do not know how to deserialize type: {}", rtype);
            }
        })
    }

    pub fn validate(&self) {
        // FIXME: implement me!
        // see: https://github.com/AntelopeIO/leap/blob/6817911900a088c60f91563995cf482d6b380b2d/libraries/chain/abi_serializer.cpp#L273
    }


    /*
    #[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Field {
    pub name: FieldName,
    #[serde(rename = "type")]
    pub type_: TypeName, // TODO: should map into a struct defined within the ABI?
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Struct {
    pub name: TypeName,
    #[serde(default)]
    pub base: TypeName, // TODO: should map into a struct defined within the ABI
    pub fields: Vec<Field>,
}
     */

    fn decode_struct(&self, ds: &mut ByteStream, struct_def: &Struct) -> Result<JsonValue, InvalidValue> {
        // let mut result: Map<String, JsonValue> = Map::new();
        // result.insert("name".to_owned(), json!(struct_def.name));
        // result.insert("base".to_owned(), json!(struct_def.base));
        // result.insert("fields".to_owned(), JsonValue::Array(vec![]));

        debug!(r#"reading struct with name "{}" and base "{}""#, struct_def.name, struct_def.base);

        // let mut result = json!({
        //     "name": struct_def.name,
        //     "base": struct_def.base,
        //     "fields": []
        // });
        // let mut result = json!({});
        let mut result: JsonMap<String, JsonValue> = JsonMap::new();

        if !struct_def.base.is_empty() {
            // result.insert("base".to_owned(), json!(struct_def.base));
            let base_def = self.structs.get(&struct_def.base).unwrap();
            let mut base = self.decode_struct(ds, base_def)?;
            debug!("base {base:?}");
            // array(&mut result, "fields").append(array(&mut base, "fields"));
            result.append(base.as_object_mut().unwrap());
        }

        let nfields = struct_def.fields.len();
        debug!("reading {nfields} fields");
        for field in &struct_def.fields {
            // let present: bool = obj.contains_key(&field.name);
            // assert!(present, "Missing field {} in input object while processing struct {}", &field.name, &struct_def.name);

            let name = &field.name;
            let value = self.decode_variant(ds, &field.type_)?;
            debug!(r#"decoded field "{name}": {value} "#);
            result.insert(name.to_string(), value);
            // array(&mut result, "fields").push(json!({
            //     "name": name,
            //     "type": value
            // }));
        }

        debug!("fully decoded struct: {:#?}", result);
        Ok(JsonValue::Object(result))
    }

}
