use std::sync::OnceLock;

use hex::FromHexError;
use serde::{Deserialize, Serialize};
use serde_json::{json, Error as JsonError};
use snafu::{ensure, Snafu, IntoError, ResultExt};

use antelope_core::{
    JsonValue, ActionName, TableName, impl_auto_error_conversion,
};
use antelope_macros::with_location;

use crate::binaryserializable::BinarySerializable;
use crate::{ABI, ByteStream, SerializeError};

pub use crate::typenameref::TypeNameRef;

// see doc at: https://docs.eosnetwork.com/manuals/cdt/latest/best-practices/abi/understanding-abi-files/
//             https://docs.eosnetwork.com/docs/latest/advanced-topics/understanding-ABI-files/

// C++ reference implementation is at:
// https://github.com/AntelopeIO/spring/blob/main/libraries/chain/include/eosio/chain/abi_def.hpp
// see also builtin types:
// https://github.com/AntelopeIO/spring/blob/main/libraries/chain/abi_serializer.cpp#L90-L131



// from https://github.com/AntelopeIO/leap/blob/main/libraries/chain/include/eosio/chain/abi_def.hpp#L7
pub type TypeName = String;
pub type FieldName = String;


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Type {
    pub new_type_name: TypeName,

    #[serde(rename = "type")]
    pub type_: TypeName,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Field {
    pub name: FieldName,
    #[serde(rename = "type")]
    pub type_: TypeName,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Struct {
    pub name: TypeName,
    #[serde(default)]
    pub base: TypeName,
    pub fields: Vec<Field>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Action {
    pub name: ActionName,
    #[serde(rename = "type")]
    pub type_: TypeName,
    #[serde(default)]
    pub ricardian_contract: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClausePair {
    pub id: String,
    pub body: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ErrorMessage {
    pub error_code: u64,
    pub error_msg: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Variant {
    pub name: TypeName,
    #[serde(default)]
    pub types: Vec<TypeName>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActionResult {
    pub name: ActionName,
    pub result_type: TypeName,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ABIDefinition {
    pub version: String,
    #[serde(default)]
    pub types: Vec<Type>,
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
    pub fn from_str(s: &str) -> Result<Self, ABIError> {
        serde_json::from_str(s).context(JsonSnafu)
    }

    pub fn from_variant(v: &JsonValue) -> Result<Self, ABIError> {
        serde_json::from_str(&v.to_string()).context(JsonSnafu)
    }

    pub fn from_bin(data: &mut ByteStream) -> Result<Self, ABIError> {
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
        assert_eq!(data.leftover(), [0u8; 2]);

        Self::from_str(&abi.to_string())
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

fn abi_schema() -> &'static ABIDefinition {
    static ABI_SCHEMA: OnceLock<ABIDefinition> = OnceLock::new();
    ABI_SCHEMA.get_or_init(|| { ABIDefinition {
        structs: vec![
            Struct {
                name: "typedef".to_owned(),
                base: "".to_owned(),
                fields: vec![
                    Field { name: "new_type_name".to_owned(), type_: "string".to_owned() },
                    Field { name: "type".to_owned(), type_: "string".to_owned() },
                ],
            },
            Struct {
                name: "field".to_owned(),
                base: "".to_owned(),
                fields: vec![
                    Field { name: "name".to_owned(), type_: "string".to_owned() },
                    Field { name: "type".to_owned(), type_: "string".to_owned() },
                ],
            },
            Struct {
                name: "struct".to_owned(),
                base: "".to_owned(),
                fields: vec![
                    Field { name: "name".to_owned(), type_: "string".to_owned() },
                    Field { name: "base".to_owned(), type_: "string".to_owned() },
                    Field { name: "fields".to_owned(), type_: "field[]".to_owned() },
                ],
            },
            Struct {
                name: "action".to_owned(),
                base: "".to_owned(),
                fields: vec![
                    Field { name: "name".to_owned(), type_: "name".to_owned() },
                    Field { name: "type".to_owned(), type_: "string".to_owned() },
                    // FIXME: should this be made optional? `signing_request_abi.json` defines an action without it
                    Field { name: "ricardian_contract".to_owned(), type_: "string".to_owned() },
                ],
            },
            Struct {
                name: "table".to_owned(),
                base: "".to_owned(),
                fields: vec![
                    Field { name: "name".to_owned(), type_: "name".to_owned() },
                    Field { name: "index_type".to_owned(), type_: "string".to_owned() },
                    Field { name: "key_names".to_owned(), type_: "string[]".to_owned() },
                    Field { name: "key_types".to_owned(), type_: "string[]".to_owned() },
                    Field { name: "type".to_owned(), type_: "string".to_owned() },
                ],
            },
            Struct {
                name: "variant".to_owned(),
                base: "".to_owned(),
                fields: vec![
                    Field { name: "name".to_owned(), type_: "name".to_owned() },
                    Field { name: "types".to_owned(), type_: "string[]".to_owned() }, // FIXME: is it String[] or Name[] here?
                ],
            },

        ],
        ..ABIDefinition::default()
    }})
}

fn bin_abi_parser() -> &'static ABI {
    static BIN_ABI_PARSER: OnceLock<ABI> = OnceLock::new();
    BIN_ABI_PARSER.get_or_init(|| {
        ABI::with_abi(abi_schema())
    })
}


#[with_location]
#[derive(Debug, Snafu)]
pub enum ABIError {
    #[snafu(display("cannot deserialize {what} from stream"), visibility(pub))]
    DeserializeError { what: String, source: SerializeError },

    #[snafu(display(r#"unsupported ABI version: "{version}""#))]
    VersionError { version: String },

    #[snafu(display("cannot deserialize ABIDefinition from JSON"))]
    JsonError { source: JsonError },

    #[snafu(display("cannot decode hex representation for hex ABI"))]
    HexABIError { source: FromHexError },
}

impl_auto_error_conversion!(FromHexError, ABIError, HexABISnafu);


#[cfg(test)]
mod tests {
    use crate::data::ABI_EXAMPLE;
    use super::*;

    #[test]
    fn parse_abi_def() {
        let abi: ABIDefinition = serde_json::from_str(ABI_EXAMPLE).unwrap();

        assert_eq!(abi.version, "eosio::abi/1.1");

        println!("{:#?}", &abi);

        // assert!(false);
    }
}
