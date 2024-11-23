use std::sync::OnceLock;

use hex::FromHexError;
use serde::{Deserialize, Serialize};
use serde_json::{json, Error as JsonError};
use snafu::{ensure, Snafu, IntoError, ResultExt};

use antelope_core::{
    JsonValue, InvalidValue, ActionName, TableName, impl_auto_error_conversion,
};
use antelope_macros::with_location;

use crate::binaryserializable::BinarySerializable;
use crate::{ABI, ByteStream, SerializeError, data::{ABI_SCHEMA, CONTRACT_ABI}};

pub use crate::typenameref::TypeNameRef;

// see doc at: https://docs.eosnetwork.com/manuals/cdt/latest/best-practices/abi/understanding-abi-files/
//             https://docs.eosnetwork.com/docs/latest/advanced-topics/understanding-ABI-files/

// C++ reference implementation is at:
// https://github.com/AntelopeIO/spring/blob/main/libraries/chain/include/eosio/chain/abi_def.hpp
// see also builtin types:
// https://github.com/AntelopeIO/spring/blob/main/libraries/chain/abi_serializer.cpp#L90-L131

type Result<T, E = ABIError> = core::result::Result<T, E>;

// from https://github.com/AntelopeIO/leap/blob/main/libraries/chain/include/eosio/chain/abi_def.hpp#L7
pub type TypeName = String;
pub type FieldName = String;


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct TypeDef {
    pub new_type_name: TypeName,

    #[serde(rename = "type")]
    pub type_: TypeName,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: FieldName,
    #[serde(rename = "type")]
    pub type_: TypeName,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Struct {
    pub name: TypeName,
    #[serde(default)]
    pub base: TypeName,
    pub fields: Vec<Field>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Action {
    pub name: ActionName,
    #[serde(rename = "type")]
    pub type_: TypeName,
    #[serde(default)]
    pub ricardian_contract: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Table {
    pub name: TableName,
    #[serde(rename = "type")]
    pub type_: TypeName, // TODO: should map into a struct defined within the ABI
    #[serde(default)]
    pub index_type: TypeName,
    pub key_names: Vec<FieldName>,
    #[serde(default)]
    pub key_types: Vec<TypeName>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ClausePair {
    pub id: String,
    pub body: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ErrorMessage {
    pub error_code: u64,
    pub error_msg: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    pub name: TypeName,
    #[serde(default)]
    pub types: Vec<TypeName>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ActionResult {
    pub name: ActionName,
    pub result_type: TypeName,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ABIDefinition {
    pub version: String,
    #[serde(default)]
    pub types: Vec<TypeDef>,
    #[serde(default)]
    pub structs: Vec<Struct>,
    #[serde(default)]
    pub actions: Vec<Action>,
    #[serde(default)]
    pub tables: Vec<Table>,
    #[serde(default)]
    pub ricardian_clauses: Vec<ClausePair>,
    #[serde(default)]
    pub error_messages: Vec<ErrorMessage>,
    #[serde(default)]
    pub variants: Vec<Variant>,

    // TODO: implement ricardian_clauses and abi_extensions
}


impl ABIDefinition {
    pub fn from_str(s: &str) -> Result<Self> {
        serde_json::from_str(s).context(JsonSnafu)
    }

    pub fn from_variant(v: &JsonValue) -> Result<Self> {
        serde_json::from_str(&v.to_string()).context(JsonSnafu)
    }

    pub fn from_bin(data: &mut ByteStream) -> Result<Self> {
        let version = String::decode(data).context(DeserializeSnafu { what: "version" })?;

        ensure!(version.starts_with("eosio::abi/1."), VersionSnafu { version });

        let parser = bin_abi_parser();
        let abi = json!({
            "version": version,
            "types":    parser.decode_variant(data, "typedef[]")?,
            "structs":  parser.decode_variant(data, "struct[]")?,
            "actions":  parser.decode_variant(data, "action[]")?,
            "tables":   parser.decode_variant(data, "table[]")?,
            "variants": parser.decode_variant(data, "variants[]")?,
        });

        // FIXME: we should deserialize everything here, we have some fields missing...
        //        also, probably "variants" doesn't come first... we need to check this...
        // check here: https://github.com/wharfkit/antelope/blob/master/src/chain/abi.ts#L109
        // see ref order here: https://github.com/AntelopeIO/spring/blob/main/libraries/chain/include/eosio/chain/abi_def.hpp#L179
        assert_eq!(data.leftover(), [0u8; 2]);

        Self::from_str(&abi.to_string())
    }

    pub fn update(&mut self, other: &ABIDefinition) -> Result<()> {
        ensure!(self.version.is_empty() || other.version.is_empty() ||
                self.version == other.version,
                IncompatibleVersionSnafu { a: self.version.clone(), b: other.version.clone() });

        self.types.extend(other.types.iter().map(Clone::clone));
        self.structs.extend(other.structs.iter().map(Clone::clone));
        self.actions.extend(other.actions.iter().map(Clone::clone));
        self.tables.extend(other.tables.iter().map(Clone::clone));
        self.ricardian_clauses.extend(other.ricardian_clauses.iter().map(Clone::clone));
        self.error_messages.extend(other.error_messages.iter().map(Clone::clone));
        self.variants.extend(other.variants.iter().map(Clone::clone));

        Ok(())
    }

    // FIXME: do we really need this? we should remove it
    pub fn with_contract_abi(mut self) -> Result<Self> {
        // ref impl: `spring/libraries/chain/eosio_contract_abi.cpp`
        self.update(&ABIDefinition::from_str(CONTRACT_ABI)?)?;
        Ok(self)
    }
}

impl Default for ABIDefinition {
    fn default() -> ABIDefinition {
        ABIDefinition {
            version: "eosio::abi/1.2".to_owned(),
            types: vec![],
            structs: vec![],
            actions: vec![],
            tables: vec![],
            ricardian_clauses: vec![],
            error_messages: vec![],
            variants: vec![],
        }
    }
}

pub fn abi_schema() -> &'static ABIDefinition {
    static ABI_SCHEMA_ONCE: OnceLock<ABIDefinition> = OnceLock::new();
    ABI_SCHEMA_ONCE.get_or_init(|| { ABIDefinition::from_str(ABI_SCHEMA).unwrap() })
}

fn bin_abi_parser() -> &'static ABI {
    static BIN_ABI_PARSER: OnceLock<ABI> = OnceLock::new();
    BIN_ABI_PARSER.get_or_init(|| {
        ABI::from_definition(abi_schema()).unwrap()  // safe unwrap
    })
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


#[cfg(test)]
mod tests {
    use crate::data::ABI_EXAMPLE;
    use super::*;

    #[test]
    fn parse_abi_def() -> Result<(), JsonError> {
        let abi: ABIDefinition = serde_json::from_str(ABI_EXAMPLE)?;

        assert_eq!(abi.version, "eosio::abi/1.1");

        println!("{:#?}", &abi);

        Ok(())
    }
}
