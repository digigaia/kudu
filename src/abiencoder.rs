use std::collections::HashMap;

use serde_json::{Map, Value};
// use anyhow::Result;
use color_eyre::eyre::Result;
use strum::VariantNames;
use log::debug;

use crate::abi::*;
use super::*;

pub struct ABIEncoder {
    // ABI-related fields
    typedefs: HashMap<TypeName, TypeName>,
    structs: HashMap<TypeName, Struct>,
    actions: HashMap<Name, TypeName>,
    tables: HashMap<Name, TypeName>,

    // FIXME: not implemented for now
    variants: HashMap<TypeName, TypeName>, // FIXME: check this is the correct type

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
            // builtin_types: get_packing_functions(),

            // FIXME: not implemented for now
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
        Self::from_bin_abi(&hex_to_bin(abi)?) //.unwrap())
    }

    pub fn from_bin_abi(abi: &[u8]) -> Result<Self, InvalidValue> {
        let mut data = ByteStream::from(abi.to_owned());
        let abi_def = ABIDefinition::from_bin(&mut data)?;
        Ok(Self::from_abi(&abi_def))
    }

    pub fn set_abi(&mut self, abi: &ABIDefinition) {
        self.typedefs.clear();
        self.structs.clear();
        self.actions.clear();
        self.tables.clear();

        // FIXME: check if we have to clone objects here or if it is ok to keep refs only
        //        maybe we want to move the whole ABIDefinition inside the encoder so it
        //        owns it and then we're fine just using refs everywhere

        for s in &abi.structs { self.structs.insert(s.name.to_owned(), s.clone()); }
        for td in &abi.types {
            assert!(!self.is_type(&td.new_type_name),
                    "Type already exists: {}", td.new_type_name);
            self.typedefs.insert(td.new_type_name.clone(), td.type_.clone());
        }

        for a in &abi.actions { self.actions.insert(a.name.clone(), a.type_.clone()); }
        for t in &abi.tables { self.tables.insert(t.name.clone(), t.type_.clone()); }

        // The ABI vector may contain duplicates which would make it an invalid ABI
        assert_eq!(self.typedefs.len(), abi.types.len(), "duplicate type definition detected");
        assert_eq!(self.structs.len(), abi.structs.len(), "duplicate struct definition detected");
        assert_eq!(self.actions.len(), abi.actions.len(), "duplicate action definition detected");
        assert_eq!(self.tables.len(), abi.tables.len(), "duplicate table definition detected");

        self.validate();
    }

    pub fn is_type(&self, t: &str) -> bool {
        let t = fundamental_type(t);
        AntelopeType::VARIANTS.contains(&t) ||
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

    pub fn json_to_bin(&self, typename: &str, obj: &Value) -> Result<Vec<u8>, InvalidValue> {
        let mut ds = ByteStream::new();
        self.encode_variant(&mut ds, typename, obj)?;
        Ok(ds.pop())
    }

    pub fn encode(&self, stream: &mut ByteStream, obj: &AntelopeType) {
        obj.to_bin(stream)
    }

    pub fn encode_variant(&self, ds: &mut ByteStream, typename: &str, object: &Value) -> Result<(), InvalidValue> {
        // see C++ implementation here: https://github.com/AntelopeIO/leap/blob/main/libraries/chain/abi_serializer.cpp#L491
        let rtype = self.resolve_type(typename);
        let ftype = fundamental_type(rtype); //.to_owned();  // FIXME: remove this .to_owned()

        let incompatible_types = || {
            InvalidValue::IncompatibleVariantTypes(rtype.to_owned(), object.clone())
        };

        if AntelopeType::VARIANTS.contains(&ftype) {
            // if our fundamental type is a builtin type, we can serialize it directly
            // to the stream
            if is_array(rtype) {
                let a = object.as_array().ok_or_else(incompatible_types)?;
                AntelopeType::VarUint32(a.len() as u32).to_bin(ds);
                for v in a {
                    AntelopeType::from_variant(ftype, v)?.to_bin(ds);
                }
            }
            else if is_optional(rtype) {
                match !object.is_null() {
                    true => {
                        AntelopeType::Bool(true).to_bin(ds);
                        AntelopeType::from_variant(ftype, object)?.to_bin(ds);
                    },
                    false => AntelopeType::Bool(false).to_bin(ds),
                }
            }
            else {
                AntelopeType::from_variant(ftype, object)?.to_bin(ds);
            }
        }
        else {
            // not a builtin type, we have to recurse down

            if is_array(rtype) {
                let a: &Vec<Value> = object.as_array().unwrap();
                ds.write_var_u32(a.len() as u32);
                for v in a {
                    self.encode_variant(ds, ftype, v)?;
                }
            }
            else if is_optional(rtype) {
                match !object.is_null() {
                    true => {
                        AntelopeType::Bool(true).to_bin(ds);
                        self.encode_variant(ds, ftype, object)?;
                    },
                    false => AntelopeType::Bool(false).to_bin(ds),
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
                }
                else {
                    // error
                }

            }
            else {
                assert!(false, "Do not know how to serialize type: {}", rtype);
            }
        }

        Ok(())
    }

    pub fn decode_variant(&self, ds: &mut ByteStream, typename: &str) -> Result<Value, InvalidValue> {
        let rtype = self.resolve_type(typename);
        let ftype = fundamental_type(rtype);

        Ok(if AntelopeType::VARIANTS.contains(&ftype) {
            // if our fundamental type is a builtin type, we can deserialize it directly
            // from the stream
            if is_array(rtype) {
                let item_count: usize = AntelopeType::from_bin("varuint32", ds)?.try_into()?;
                debug!(r#"reading array of {item_count} elements of type "{ftype}""#);
                let mut a = Vec::with_capacity(item_count);
                for _ in 0..item_count {
                    a.push(AntelopeType::from_bin(ftype, ds)?.to_variant());
                }
                Value::Array(a)
            }
            else if is_optional(rtype) {
                let non_null: bool = AntelopeType::from_bin("bool", ds)?.into();
                match non_null {
                    true => AntelopeType::from_bin(ftype, ds)?.to_variant(),
                    false => Value::Null,
                }
            }
            else {
                AntelopeType::from_bin(ftype, ds)?.to_variant()
            }
        }
        else {
            if is_array(rtype) {
                // not a builtin type, we have to recurse down
                let item_count: usize = AntelopeType::from_bin("varuint32", ds)?.try_into()?;
                debug!(r#"reading array of {item_count} elements of type "{ftype}""#);
                let mut a = Vec::with_capacity(item_count);
                for _ in 0..item_count {
                    a.push(self.decode_variant(ds, ftype)?);
                }
                Value::Array(a)
            }
            else if is_optional(rtype) {
                let non_null = AntelopeType::from_bin("bool", ds)?.into();
                match non_null {
                    true => self.decode_variant(ds, ftype)?,
                    false => Value::Null,
                }
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

    fn decode_struct(&self, ds: &mut ByteStream, struct_def: &Struct) -> Result<Value, InvalidValue> {
        // let mut result: Map<String, Value> = Map::new();
        // result.insert("name".to_owned(), json!(struct_def.name));
        // result.insert("base".to_owned(), json!(struct_def.base));
        // result.insert("fields".to_owned(), Value::Array(vec![]));

        debug!(r#"reading struct with name "{}" and base "{}""#, struct_def.name, struct_def.base);

        // let mut result = json!({
        //     "name": struct_def.name,
        //     "base": struct_def.base,
        //     "fields": []
        // });
        // let mut result = json!({});
        let mut result: Map<String, Value> = Map::new();

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
        Ok(Value::Object(result))
    }

}
