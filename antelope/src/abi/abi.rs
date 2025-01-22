use std::collections::HashMap;

use hex::FromHexError;
use serde_json::{
    json,
    Error as JsonError,
    Map as JsonMap,
    Value as JsonValue,
};
use snafu::{ensure, Snafu, ResultExt};
use strum::VariantNames;
use tracing::{debug, warn, instrument};

use antelope_macros::with_location;

use crate::{
    AntelopeType, AntelopeValue, Name, VarUint32, InvalidValue, TypeNameRef, impl_auto_error_conversion,
    ABIDefinition, ByteStream, BinarySerializable, SerializeError,
    abidefinition::{
        TypeName, Struct, Variant
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
    action_results: HashMap<Name, TypeName>,
}


impl ABI {
    pub fn new() -> Self {
        Self {
            typedefs: HashMap::new(),
            structs: HashMap::new(),
            actions: HashMap::new(),
            tables: HashMap::new(),
            variants: HashMap::new(),
            action_results: HashMap::new(),
        }
    }

    // -----------------------------------------------------------------------------
    //     Constructors and validation of ABI
    // -----------------------------------------------------------------------------


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
        self.action_results.clear();

        self.structs.extend(abi.structs.iter().map(|s| (s.name.to_string(), s.clone())));

        for td in &abi.types {
            // note: this check seems redundant with the circular reference detection
            //       in `validate()` but this also checks that we have no duplicates
            //       between the previously defined structs and the typedefs
            ensure!(!self.is_type(TypeNameRef(&td.new_type_name)),
                    IntegritySnafu { message: format!("type already exists: {}",
                                                      td.new_type_name) });
            self.typedefs.insert(td.new_type_name.clone(), td.type_.clone());
        }

        self.actions.extend(abi.actions.iter()
                            .map(|a| (a.name, a.type_.clone())));
        self.tables.extend(abi.tables.iter()
                           .map(|t| (t.name, t.type_.clone())));
        self.variants.extend(abi.variants.iter()
                             .map(|v| (v.name.clone(), v.clone())));
        self.action_results.extend(abi.action_results.iter()
                                   .map(|a| (a.name, a.result_type.clone())));

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
        ensure!(self.action_results.len() == abi.action_results.len(),
                IntegritySnafu { message: "" });

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

    pub fn validate(&self) -> Result<(), ABIError> {
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
                    ensure!(self.structs.contains_key(&current.base),
                            IntegritySnafu { message: format!("invalid type used in '{}::base': `{}`", &s.name, &current.base) });
                    let base = self.structs.get(&current.base).unwrap();  // safe unwrap
                    ensure!(!types_seen.contains(&&base.name),
                            IntegritySnafu { message: format!("circular reference in struct '{}'", &s.name) });
                    types_seen.push(&base.name);
                    current = base;
                }
            }

            // check all field types are valid types
            for field in &s.fields {
                ensure!(self.is_type(TypeNameRef(&field.type_[..]).remove_bin_extension()),
                        IntegritySnafu { message: format!("invalid type used in field '{}::{}': `{}`",
                                                          &s.name, &field.name, &field.type_) });
            }
        }

        // check all types from a variant are valid types
        for v in self.variants.values() {
            for t in &v.types {
                ensure!(self.is_type(t.into()),
                        IntegritySnafu { message: format!("invalid type `{}` used in variant '{}'",
                                                          t, v.name) });
            }
        }

        // check all actions are valid types
        for (name, type_) in &self.actions {
            ensure!(self.is_type(type_.into()),
                    IntegritySnafu { message: format!("invalid type `{}` used in action '{}'",
                                                      type_, name) });
        }

        // check all tables are valid types
        for (name, type_) in &self.tables {
            ensure!(self.is_type(type_.into()),
                    IntegritySnafu { message: format!("invalid type `{}` used in table '{}'",
                                                      type_, name) });
        }

        // check all action results are valid types
        for (name, type_) in &self.action_results {
            ensure!(self.is_type(type_.into()),
                    IntegritySnafu { message: format!("invalid type `{}` used in action result '{}'",
                                                      type_, name) });
        }

        Ok(())
    }

    // -----------------------------------------------------------------------------
    //     Encoding of variant -> binary
    // -----------------------------------------------------------------------------


    pub fn variant_to_binary<'a, T>(&self, typename: T, obj: &JsonValue)
                                    -> Result<Vec<u8>>
    where
        T: Into<TypeNameRef<'a>>
    {
        let mut ds = ByteStream::new();
        self.encode_variant(&mut ds, typename.into(), obj)?;
        Ok(ds.into_bytes())
    }

    #[inline]
    pub fn encode<T: BinarySerializable>(&self, stream: &mut ByteStream, obj: &T) {
        obj.encode(stream)
    }

    #[inline]
    pub fn encode_variant<'a, T>(&self, ds: &mut ByteStream, typename: T, object: &JsonValue)
                                 -> Result<(), ABIError>
    where
        T: Into<TypeNameRef<'a>>
    {
        self.encode_variant_(&mut VariantToBinaryContext::new(), ds, typename.into(), object)
    }

    #[instrument(skip(self, ctx, ds))]
    fn encode_variant_(&self, ctx: &mut VariantToBinaryContext, ds: &mut ByteStream,
                       typename: TypeNameRef, object: &JsonValue)
                       -> Result<(), ABIError> {
        // see C++ implementation here: https://github.com/AntelopeIO/spring/blob/main/libraries/chain/abi_serializer.cpp#L493
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
                    self.encode_variant_(ctx, ds, ftype, v)?;
                }
            }
            else if rtype.is_optional() {
                match !object.is_null() {
                    true => {
                        true.encode(ds);
                        self.encode_variant_(ctx, ds, ftype, object)?;
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
                    self.encode_variant_(ctx, ds, variant_type, &object[1])?;
                }
                else {
                    EncodeSnafu {
                        message: format!("specified type `{}` is not valid within the variant '{}'",
                                         variant_type, rtype)
                    }.fail()?;
                }
            }
            else if let Some(struct_def) = self.structs.get(rtype.0) {
                warn!(t=rtype.0, obj=object.to_string());
                self.encode_struct(ctx, ds, struct_def, object)?;
            }
            else {
                EncodeSnafu { message: format!("do not know how to serialize type: `{}`", rtype) }.fail()?;
            }
        }

        Ok(())
    }

    fn encode_struct(&self, ctx: &mut VariantToBinaryContext, ds: &mut ByteStream,
                     struct_def: &Struct, object: &JsonValue)
                     -> Result<(), ABIError> {
        // we want to serialize a struct...
        if let Some(obj) = object.as_object() {
            // ...and we are given an object -> serialize fields using their name
            if !struct_def.base.is_empty() {
                let _ = ctx.disallow_extensions_unless(false);
                self.encode_variant(ds, TypeNameRef(&struct_def.base), object)?;
            }

            let mut allow_additional_fields = true;
            for (i, field) in struct_def.fields.iter().enumerate() {
                let ftype = TypeNameRef(&field.type_);
                let nfields = struct_def.fields.len();
                let present: bool = obj.contains_key(&field.name);
                if present || ftype.is_optional() {
                    ensure!(allow_additional_fields,
                            EncodeSnafu { message: format!(
                                "Unexpected field '{}' found in input object while processing struct '{}'",
                                &field.name, &struct_def.name) });
                    let value = if present { obj.get(&field.name).unwrap() }  // safe unwrap
                    else                   { &JsonValue::Null };
                    // TODO: ctx.push_to_path
                    ctx.disallow_extensions_unless(i == nfields-1); // disallow except for the last field
                    self.encode_variant(ds, ftype.remove_bin_extension(), value)?;
                }
                else if ftype.has_bin_extension() && ctx.allow_extensions {
                    allow_additional_fields = false;
                }
                else if !allow_additional_fields {
                    EncodeSnafu { message: format!(
                        "Encountered field '{}' without binary extension designation while processing struct '{}'",
                        &field.name, &struct_def.name) }.fail()?;
                }
                else {
                    EncodeSnafu { message: format!(
                        "missing field '{}' in input object while processing struct '{}'",
                        &field.name, &struct_def.name) }.fail()?;
                }
            }
        }
        else if let Some(arr) = object.as_array() {
            // ..and we are given an array -> serialize fields using their position
            ensure!(struct_def.base.is_empty(),
                    EncodeSnafu { message: format!(concat!(
                        "using input array to specify the fields of the derived struct '{}'; ",
                        "input arrays are currently only allowed for structs without a base"
                    ), struct_def.name) });

            for (i, field) in struct_def.fields.iter().enumerate() {
                // let field = &struct_def.fields[i];
                let ftype = TypeNameRef(&field.type_);
                let nfields = struct_def.fields.len();
                if i < arr.len() {
                    // TODO: ctx.push_to_path
                    ctx.disallow_extensions_unless(i == nfields-1);
                    self.encode_variant(ds, ftype.remove_bin_extension(), &arr[i])?;
                }
                else if ftype.has_bin_extension() && ctx.allow_extensions {
                    break;
                }
                else {
                    EncodeSnafu { message: format!(concat!(
                        "early end to input array specifying the fields of struct '{}'; ",
                        "require input for field '{}'"
                    ), struct_def.name, field.name) }.fail()?;
                }
            }
        }
        else {
            EncodeSnafu { message: format!(
                "unexpected input while encoding struct '{}': {}",
                struct_def.name, object) }.fail()?;
        }

        Ok(())
    }


    // -----------------------------------------------------------------------------
    //     Decoding of binary data -> variant
    // -----------------------------------------------------------------------------

    pub fn binary_to_variant<'a, T>(&self, typename: T, bytes: Vec<u8>)
                                    -> Result<JsonValue>
    where
        T: Into<TypeNameRef<'a>>
    {
        let mut ds = ByteStream::from(bytes);
        self.decode_variant_(&mut ds, typename.into())
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
                // limit the maximum size that can be reserved before data is read
                let initial_capacity = item_count.min(1024);
                let mut a = Vec::with_capacity(initial_capacity);
                // loop {}
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
                read_value(ds, type_, "single `AntelopeValue`")?
            }
        }
        else {
            if rtype.is_array() {
                // not a builtin type, we have to recurse down
                let item_count = decode_usize(ds, "item_count (as varuint32)")?;
                debug!(r#"reading array of {item_count} elements22 of type "{ftype}""#);
                // limit the maximum size that can be reserved before data is read
                let initial_capacity = item_count.min(1024);
                let mut a = Vec::with_capacity(initial_capacity);
                // loop {}
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

        let mut encountered_extension = false;
        let nfields = struct_def.fields.len();
        debug!("reading {nfields} fields");
        for field in &struct_def.fields {
            let fname = &field.name;
            let ftype = TypeNameRef(&field.type_);
            encountered_extension |= ftype.has_bin_extension();
            if ds.leftover().is_empty() {
                if ftype.has_bin_extension() {
                    continue;
                }
                ensure!(!encountered_extension,
                        DecodeSnafu { message: format!(
                            "encountered field '{}' without binary extension designation while processing struct '{}'",
                            fname, &struct_def.name) });

                DecodeSnafu { message: format!(
                    "stream ended unexpectedly; unable to unpack field '{}' of struct '{}'",
                    fname, struct_def.name) }.fail()?
            }

            let rtype = self.resolve_type(ftype.remove_bin_extension());
            let value = self.decode_variant(ds, rtype)?;
            debug!(r#"decoded field '{fname}' with type `{ftype}`: {value}"#);
            result.insert(fname.to_string(), value);
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


#[with_location]
#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum ABIError {
    #[snafu(display("cannot deserialize {what} from stream"))]
    DeserializeError { what: String, source: SerializeError },

    #[snafu(display(r#"unsupported ABI version: "{version}""#))]
    VersionError { version: String },

    #[snafu(display(r#"incompatible versions: "{a}" vs. "{b}""#))]
    IncompatibleVersionError { a: String, b: String },

    #[snafu(display("integrity error: {message}"))]
    IntegrityError { message: String },

    #[snafu(display("encode error: {message}"))]
    EncodeError { message: String },

    #[snafu(display("decode error: {message}"))]
    DecodeError { message: String },

    #[snafu(display("cannot deserialize ABIDefinition from JSON"))]
    JsonError { source: JsonError },

    #[snafu(display("cannot decode hex representation for hex ABI"))]
    HexABIError { source: FromHexError },

    #[snafu(display("cannot convert variant to AntelopeValue: {v}"))]
    VariantConversionError { v: Box<JsonValue>, source: InvalidValue },

    #[snafu(display(r#"cannot convert given variant {value} to Antelope type "{typename}""#))]
    IncompatibleVariantTypes {
        typename: String,
        value: Box<JsonValue>,
    },
}

impl_auto_error_conversion!(FromHexError, ABIError, HexABISnafu);

// TODO: rename this ScopeGuard?
struct ScopeExit<T>
where
    T: FnMut()
{
    callback: T,
}

impl<T: FnMut()> ScopeExit<T> {
    pub fn new(f: T) -> ScopeExit<T> {
        ScopeExit { callback: f }
    }
}

impl<T: FnMut()> Drop for ScopeExit<T> {
    fn drop(&mut self) {
        (self.callback)()
    }
}

struct VariantToBinaryContext {
    allow_extensions: bool,
}

impl VariantToBinaryContext {
    pub fn new() -> VariantToBinaryContext {
        VariantToBinaryContext { allow_extensions: true }
    }

    pub fn disallow_extensions_unless(&mut self, cond: bool) -> ScopeExit<impl FnMut() + '_> {
        let old_allow_extensions = self.allow_extensions;

        if !cond { self.allow_extensions = false }

        let callback = move || { self.allow_extensions = old_allow_extensions; };
        ScopeExit::new(callback)
    }
}
