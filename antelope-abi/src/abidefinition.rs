use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use serde_json::json;
use snafu::{ensure, ResultExt};

use antelope_core::{
    JsonValue, ActionName, TableName
};

use crate::binaryserializable::{BinarySerializable, ABISnafu};
use crate::{
    ByteStream, SerializeError,
    abi::{ABI, ABIError, JsonSnafu, DeserializeSnafu, VersionSnafu, IncompatibleVersionSnafu},
    data::{ABI_SCHEMA, CONTRACT_ABI}};

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
    #[serde(default)]
    pub action_results: Vec<ActionResult>,

    // TODO: implement ricardian_clauses and abi_extensions
}


impl ABIDefinition {
    pub fn from_str(s: &str) -> Result<Self> {
        serde_json::from_str(s).context(JsonSnafu)
    }

    pub fn from_variant(v: &JsonValue) -> Result<Self> {
        ABIDefinition::from_str(&v.to_string())
    }

    pub fn from_bin(data: &mut ByteStream) -> Result<Self> {
        // FIXME: check how to deserialize properly the different versions: 1.0, 1.1, 1.2, ...
        let version = String::decode(data).context(DeserializeSnafu { what: "version" })?;

        ensure!(version.starts_with("eosio::abi/1."), VersionSnafu { version });

        let parser = bin_abi_parser();
        let abi = json!({
            "version": version,
            "types":    parser.decode_variant(data, "typedef[]")?,
            "structs":  parser.decode_variant(data, "struct[]")?,
            "actions":  parser.decode_variant(data, "action[]")?,
            "tables":   parser.decode_variant(data, "table[]")?,
            "variants": parser.decode_variant(data, "variant[]")?,
        });

        // FIXME: we should deserialize everything here, we have some fields missing...
        //        also, probably "variants" doesn't come first... we need to check this...
        // check here: https://github.com/wharfkit/antelope/blob/master/src/chain/abi.ts#L109
        // see ref order here: https://github.com/AntelopeIO/spring/blob/main/libraries/chain/include/eosio/chain/abi_def.hpp#L179
        assert_eq!(data.leftover(), [0u8; 2]);

        Self::from_variant(&abi)
    }

    pub fn to_bin(&self, stream: &mut ByteStream) -> Result<()> {
        let parser = bin_abi_parser();
        parser.encode(stream, &self.version);
        parser.encode_variant(stream, "typedef[]", &json!(self.types))?;
        parser.encode_variant(stream, "struct[]", &json!(self.structs))?;
        parser.encode_variant(stream, "action[]", &json!(self.actions))?;
        parser.encode_variant(stream, "table[]", &json!(self.tables))?;
        parser.encode_variant(stream, "variant[]", &json!(self.variants))?;

        stream.write_byte(0);
        stream.write_byte(0);

        Ok(())
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
            action_results: vec![],
        }
    }
}

impl BinarySerializable for ABIDefinition {
    fn encode(&self, stream: &mut ByteStream) {
        self.to_bin(stream).unwrap()  // safe unwrap
    }
    fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError> {
        ABIDefinition::from_bin(stream).context(ABISnafu)
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



#[cfg(test)]
mod tests {
    use serde_json::Error as JsonError;
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
