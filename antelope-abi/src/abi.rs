use std::collections::HashMap;

use antelope_core::{
    AntelopeType, AntelopeValue, Name, VarUint32,
};
use serde_json::{
    json,
    Map as JsonMap,
    Value as JsonValue,
};
use snafu::{ensure, ResultExt};
use strum::VariantNames;
use tracing::{debug, warn, instrument};

use crate::{
    ABIDefinition, ABISerializable, ByteStream, BinarySerializable,
    abidefinition::{
        ABIError, IntegritySnafu, DeserializeSnafu, EncodeSnafu, DecodeSnafu,
        IncompatibleVariantTypesSnafu, VariantConversionSnafu, VersionSnafu,
        TypeName, TypeNameRef, Struct, Variant
    },
};

type Result<T, E = ABIError> = core::result::Result<T, E>;

// TODO: make sure that we can (de)serialize an ABI (ABIDefinition?) itself (eg, see: https://github.com/wharfkit/antelope/blob/master/src/chain/abi.ts, which implements ABISerializableObject)

// FIXME: remove all `.0` lying in this file

#[derive(Default, Clone, Debug)]
pub struct ABI {
    // ABI-related fields
    typedefs: HashMap<TypeName, TypeName>,
    structs: HashMap<TypeName, Struct>,
    actions: HashMap<Name, TypeName>,
    tables: HashMap<Name, TypeName>,
    variants: HashMap<TypeName, Variant>,

    // TODO: missing https://github.com/AntelopeIO/leap/blob/6817911900a088c60f91563995cf482d6b380b2d/libraries/chain/abi_serializer.cpp#L149-L151

}


impl ABI {
    pub fn new() -> Self {
        Self {
            typedefs: HashMap::new(),
            structs: HashMap::new(),
            actions: HashMap::new(),
            tables: HashMap::new(),
            variants: HashMap::new(),
        }
    }

    pub fn from_definition(abi: &ABIDefinition) -> Result<Self> {
        let mut result = Self::new();
        result.set_abi(abi)?;
        Ok(result)
    }

    pub fn from_str(abi: &str) -> Result<Self> {
        Self::from_definition(&ABIDefinition::from_str(abi)?)
    }

    pub fn from_hex_abi(abi: &str) -> Result<Self> {
        Self::from_bin_abi(&hex::decode(abi)?)
    }

    pub fn from_bin_abi(abi: &[u8]) -> Result<Self> {
        let mut data = ByteStream::from(abi.to_owned());
        let abi_def = ABIDefinition::from_bin(&mut data)?;
        Self::from_definition(&abi_def)
    }

    fn set_abi(&mut self, abi: &ABIDefinition) -> Result<()> {
        ensure!(abi.version.starts_with("eosio::abi/1."), VersionSnafu { version: &abi.version });

        self.typedefs.clear();
        self.structs.clear();
        self.actions.clear();
        self.tables.clear();
        self.variants.clear();

        // for s in &abi.structs { self.structs.insert(s.name.to_string(), s.clone()); }
        self.structs.extend(abi.structs.iter().map(|s| (s.name.to_string(), s.clone())));

        for td in &abi.types {
            // TODO: this check is redundant with the circular reference detection
            //       in `validate()` (right?), so we should remove it
            //       BUT! we also check this way the we have no duplicates between
            //       the previously defined structs and the typedefs
            ensure!(!self.is_type(TypeNameRef(&td.new_type_name)),
                    IntegritySnafu { message: format!("type already exists: {}",
                                                      td.new_type_name) });
            self.typedefs.insert(td.new_type_name.clone(), td.type_.clone());
        }

        for a in &abi.actions {
            self.actions.insert(a.name, a.type_.clone());
        }
        for t in &abi.tables {
            self.tables.insert(t.name, t.type_.clone());
        }
        for v in &abi.variants {
            self.variants.insert(v.name.clone(), v.clone());
        }

        // The ABIDefinition vectors may contain duplicates which would make it an invalid ABI
        ensure!(self.typedefs.len() == abi.types.len(),
                IntegritySnafu { message: "duplicate type definition detected" });
        ensure!(self.structs.len() == abi.structs.len(),
                IntegritySnafu { message: "duplicate struct definition detected" });
        ensure!(self.actions.len() == abi.actions.len(),
                IntegritySnafu { message: "duplicate action definition detected" });
        ensure!(self.tables.len() == abi.tables.len(),
                IntegritySnafu { message: "duplicate table definition detected" });
        ensure!(self.variants.len() == abi.variants.len(),
                IntegritySnafu { message: "duplicate variants definition detected" });

        self.validate()
    }

    pub fn is_type(&self, t: TypeNameRef) -> bool {
        // NOTE: this would be a better behavior IMO but it doesn't match the C++ code
        //       for Antelope Spring; keep the latter for better compatibility

        let mut t = t;
        let mut ft = t.fundamental_type();
        while ft != t {
            t = ft;
            ft = t.fundamental_type();
        }

        // NOTE: this is the C++ Antelope Spring behavior
        // let t = t.fundamental_type();
        AntelopeValue::VARIANTS.contains(&t.0)
            || (self.typedefs.contains_key(t.0) &&
               self.is_type(TypeNameRef(self.typedefs.get(t.0).unwrap())))  // safe unwrap
            || self.structs.contains_key(t.0)
            || self.variants.contains_key(t.0)
    }

    pub fn resolve_type<'a>(&'a self, t: TypeNameRef<'a>) -> TypeNameRef<'a> {
        let mut rtype = t;
        loop {
            match self.typedefs.get(rtype.0) {
                Some(t) => rtype = TypeNameRef(t),
                None => return rtype,
            }
        }
    }

    pub fn variant_to_binary<'a, T>(&self, typename: T, obj: &JsonValue)
                                    -> Result<Vec<u8>>
    where
        T: Into<TypeNameRef<'a>>
    {
        let mut ds = ByteStream::new();
        self.encode_variant(&mut ds, typename.into(), obj)?;
        Ok(ds.pop())
    }

    pub fn binary_to_variant<'a, T>(&self, typename: T, bytes: Vec<u8>)
                                    -> Result<JsonValue>
    where
        T: Into<TypeNameRef<'a>>
    {
        let mut ds = ByteStream::from(bytes);
        self.decode_variant_(&mut ds, typename.into())
    }

    #[inline]
    pub fn encode<T: ABISerializable>(&self, stream: &mut ByteStream, obj: &T) {
        obj.to_bin(stream)
    }

    #[inline]
    pub fn encode_variant<'a, T>(&self, ds: &mut ByteStream, typename: T, object: &JsonValue)
                                 -> Result<(), ABIError>
    where
        T: Into<TypeNameRef<'a>>
    {
        self.encode_variant_(ds, typename.into(), object)
    }

    #[instrument(skip(self, ds))]
    fn encode_variant_(&self, ds: &mut ByteStream, typename: TypeNameRef, object: &JsonValue) -> Result<(), ABIError> {
        // see C++ implementation here: https://github.com/AntelopeIO/leap/blob/main/libraries/chain/abi_serializer.cpp#L491
        let rtype = self.resolve_type(typename);
        let ftype = rtype.fundamental_type();

        debug!(rtype=rtype.0, ftype=ftype.0);

        // use a closure to avoid cloning and copying if no error occurs
        let incompatible_types = || { IncompatibleVariantTypesSnafu {
            typename: rtype.0.to_owned(),
            value: Box::new(object.clone())
        }.build() };

        if AntelopeValue::VARIANTS.contains(&ftype.0) {
            // if our fundamental type is a builtin type, we can serialize it directly
            // to the stream
            let inner_type: AntelopeType = ftype.try_into().unwrap();  // safe unwrap
            if rtype.is_array() {
                let a = object.as_array().ok_or_else(incompatible_types)?;
                VarUint32::from(a.len()).encode(ds);
                for v in a {
                    AntelopeValue::from_variant(inner_type, v)
                        .with_context(|_| VariantConversionSnafu { v: v.clone() })?
                        .to_bin(ds);
                }
            }
            else if rtype.is_optional() {
                match !object.is_null() {
                    true => {
                        true.encode(ds);
                        AntelopeValue::from_variant(inner_type, object)
                            .with_context(|_| VariantConversionSnafu { v: object.clone() })?
                            .to_bin(ds);
                    },
                    false => false.encode(ds),
                }
            }
            else {
                AntelopeValue::from_variant(inner_type, object)
                    .with_context(|_| VariantConversionSnafu { v: object.clone() })?
                    .to_bin(ds);
            }
        }
        else {
            // not a builtin type, we have to recurse down

            if rtype.is_array() {
                let a = object.as_array().ok_or_else(incompatible_types)?;
                VarUint32::from(a.len()).encode(ds);
                for v in a {
                    self.encode_variant(ds, ftype, v)?;
                }
            }
            else if rtype.is_optional() {
                match !object.is_null() {
                    true => {
                        true.encode(ds);
                        self.encode_variant(ds, ftype, object)?;
                    },
                    false => false.encode(ds),
                }
            }
            else if let Some(variant_def) = self.variants.get(rtype.0) {
                debug!("serializing type {:?} with variant: {:?}", rtype.0, object);
                ensure!(object.is_array() && object.as_array().unwrap().len() == 2,
                        EncodeSnafu {
                            message: format!("expected input to be an array of 2 elements while processing variant: {}",
                                             &object)
                        });
                ensure!(object[0].is_string(),
                        EncodeSnafu {
                            message: format!("expected variant typename to be a string: {}",
                                             object[0])
                        });
                let variant_type = TypeNameRef(object[0].as_str().unwrap());
                if let Some(vpos) = variant_def.types.iter().position(|v| v == variant_type.0) {
                    VarUint32::from(vpos).encode(ds);
                    self.encode_variant(ds, variant_type, &object[1])?;
                }
                else {
                    EncodeSnafu {
                        message: format!("specified type {} is not valid within the variant {}",
                                         variant_type, rtype)
                    }.fail()?;
                }
            }
            else if let Some(struct_def) = self.structs.get(rtype.0) {
                if object.is_object() {
                    if !struct_def.base.is_empty() {
                        self.encode_variant(ds, TypeNameRef(&struct_def.base), object)?;
                    }
                    let obj = object.as_object().unwrap();

                    for field in &struct_def.fields {
                        let present: bool = obj.contains_key(&field.name);
                        ensure!(present,
                                EncodeSnafu {
                                    message: format!(r#"missing field "{}" in input object while processing struct "{}""#,
                                                     &field.name, &struct_def.name)
                                });
                        self.encode_variant(ds, TypeNameRef(&field.type_), obj.get(&field.name).unwrap())?;
                    }
                }
                else if object.is_array() {
                    warn!(t=rtype.0, obj=object.to_string());
                    unimplemented!();
                }
                else {
                    // error
                    unimplemented!();
                }
            }
            else {
                EncodeSnafu { message: format!("do not know how to serialize type: {}", rtype) }.fail()?;
            }
        }

        Ok(())
    }

    #[inline]
    pub fn decode_variant<'a, T>(&self, ds: &mut ByteStream, typename: T) -> Result<JsonValue, ABIError>
    where
        T: Into<TypeNameRef<'a>>
    {
        self.decode_variant_(ds, typename.into())
    }

    #[allow(clippy::collapsible_else_if)]
    fn decode_variant_(&self, ds: &mut ByteStream, typename: TypeNameRef) -> Result<JsonValue, ABIError> {
        let rtype = self.resolve_type(typename);
        let ftype = rtype.fundamental_type();

        Ok(if AntelopeValue::VARIANTS.contains(&ftype.0) {
            let type_ = ftype.try_into().unwrap();  // safe unwrap

            // if our fundamental type is a builtin type, we can deserialize it directly
            // from the stream
            if rtype.is_array() {
                let item_count = decode_usize(ds, "item_count (as varuint32)")?;
                debug!(r#"reading array of {item_count} elements of type "{ftype}""#);
                let mut a = Vec::with_capacity(item_count);
                for _ in 0..item_count {
                    a.push(read_value(ds, type_, "array item")?);
                }
                JsonValue::Array(a)
            }
            else if rtype.is_optional() {
                let non_null = bool::decode(ds)
                    .context(DeserializeSnafu { what: "optional discriminant" })?;
                match non_null {
                    true => read_value(ds, type_, "optional value")?,
                    false => JsonValue::Null,
                }
            }
            else {
                read_value(ds, type_, "single AntelopeValue")?
            }
        }
        else {
            if rtype.is_array() {
                // not a builtin type, we have to recurse down
                let item_count = decode_usize(ds, "item_count (as varuint32)")?;
                debug!(r#"reading array of {item_count} elements of type "{ftype}""#);
                let mut a = Vec::with_capacity(item_count);
                for _ in 0..item_count {
                    a.push(self.decode_variant(ds, ftype)?);
                }
                JsonValue::Array(a)
            }
            else if rtype.is_optional() {
                let non_null = bool::decode(ds)
                    .context(DeserializeSnafu { what: "optional discriminant" })?;
                match non_null {
                    true => self.decode_variant(ds, ftype)?,
                    false => JsonValue::Null,
                }
            }
            else if let Some(variant_def) = self.variants.get(rtype.0) {
                let variant_tag: usize = decode_usize(ds, "variant tag (as varuint32)")?;
                ensure!(variant_tag < variant_def.types.len(),
                        DecodeSnafu { message: format!("deserialized invalid tag {} for variant {}",
                                                       variant_tag, rtype)
                        });
                let variant_type = TypeNameRef(&variant_def.types[variant_tag]);
                json!([variant_type.0, self.decode_variant(ds, variant_type)?])
            }
            else if let Some(struct_def) = self.structs.get(rtype.0) {
                self.decode_struct(ds, struct_def)?
            }
            else {
                DecodeSnafu { message: format!("do not know how to deserialize type: {}", rtype) }.fail()?
            }
        })
    }

    pub fn validate(&self) -> Result<(), ABIError> {
        // FIXME: implement me!
        // see: https://github.com/AntelopeIO/leap/blob/6817911900a088c60f91563995cf482d6b380b2d/libraries/chain/abi_serializer.cpp#L273
        // https://github.com/AntelopeIO/leap/blob/main/libraries/chain/abi_serializer.cpp#L282

        // check there are no circular references in the typedefs definition
        for t in &self.typedefs {
            let mut types_seen = vec![t.0, t.1];
            let mut itr = self.typedefs.get(&t.1[..]);
            while itr.is_some() {
                let it = itr.unwrap();
                ensure!(!types_seen.contains(&it),
                        IntegritySnafu { message: format!("circular reference in type `{}`", t.0) });
                types_seen.push(it);
                itr = self.typedefs.get(it);
            }
        }

        // check all types used in typedefs are valid types
        for t in &self.typedefs {
            ensure!(self.is_type(t.1.into()),
                    IntegritySnafu { message: format!("invalid type used in typedef `{}`", t.1) });
        }

        // check there are no circular references in the structs definition
        for s in self.structs.values() {
            if !s.base.is_empty() {
                let mut current = s;
                let mut types_seen = vec![&current.name];
                while !current.base.is_empty() {
                    let base = self.structs.get(&current.base).unwrap();  // safe unwrap
                    ensure!(!types_seen.contains(&&base.name),
                            IntegritySnafu { message: format!("circular reference in struct `{}`", &s.name) });
                    types_seen.push(&base.name);
                    current = base;
                }
            }

            // check all field types are valid types
            for field in &s.fields {
                ensure!(self.is_type(TypeNameRef(&field.type_[..]).remove_bin_extension()),
                        IntegritySnafu { message: format!("invalid type used in field `{}::{}`: `{}`",
                                                          &s.name, &field.name, &field.type_) });
            }
        }

        // check all types from a variant are valid types
        for v in self.variants.values() {
            for t in &v.types {
                ensure!(self.is_type(t.into()),
                        IntegritySnafu { message: format!("invalid type `{}` used in variant `{}`",
                                                          t, v.name) });
            }
        }

        // check all actions are valid types
        for (name, type_) in &self.actions {
            ensure!(self.is_type(type_.into()),
                    IntegritySnafu { message: format!("invalid type `{}` used in action `{}`",
                                                      type_, name) });
        }

        // check all tables are valid types
        for (name, type_) in &self.tables {
            ensure!(self.is_type(type_.into()),
                    IntegritySnafu { message: format!("invalid type `{}` used in table `{}`",
                                                      type_, name) });
        }

        // check all action results are valid types
        // FIXME: implement me once we have a field for it
        // for (name, type_) in &self.action_results {
        //     ensure!(self.is_type(type_.into()),
        //             IntegritySnafu { message: format!("invalid type `{}` used in action `{}`",
        //                                               type_, name) });
        // }


        Ok(())
    }


    fn decode_struct(&self, ds: &mut ByteStream, struct_def: &Struct) -> Result<JsonValue, ABIError> {
        debug!(r#"reading struct with name "{}" and base "{}""#, struct_def.name, struct_def.base);

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
            let type_ = TypeNameRef(&field.type_);
            let value = self.decode_variant(ds, type_)?;
            debug!(r#"decoded field "{name}" with type "{type_}": {value}"#);
            result.insert(name.to_string(), value);
        }

        debug!("fully decoded `{}` struct: {:#?}", struct_def.name, result);
        Ok(JsonValue::Object(result))
    }
}

fn read_value(stream: &mut ByteStream, type_: AntelopeType, what: &str) ->  Result<JsonValue, ABIError> {
    Ok(AntelopeValue::from_bin(type_, stream)
       .context(DeserializeSnafu { what })?.to_variant())
}

fn decode_usize(stream: &mut ByteStream, what: &str) -> Result<usize, ABIError> {
    let n = VarUint32::decode(stream).context(DeserializeSnafu { what })?;
    Ok(n.into())
}
