use serde::{Serialize, Deserialize};

use crate::{AntelopeType, Name, ByteStream};

// see doc at: https://docs.eosnetwork.com/manuals/cdt/latest/best-practices/abi/understanding-abi-files/
//             https://docs.eosnetwork.com/docs/latest/advanced-topics/understanding-ABI-files/

// see also builtin types: https://github.com/AntelopeIO/leap/blob/6817911900a088c60f91563995cf482d6b380b2d/libraries/chain/abi_serializer.cpp#L88-L129


// from https://github.com/AntelopeIO/leap/blob/6817911900a088c60f91563995cf482d6b380b2d/libraries/chain/include/eosio/chain/types.hpp#L128C1-L133C1
pub type ActionName = Name;
pub type ScopeName = Name;
pub type AccountName = Name;
pub type PermissionName = Name;
pub type TableName = Name;

// from https://github.com/AntelopeIO/leap/blob/6817911900a088c60f91563995cf482d6b380b2d/libraries/chain/include/eosio/chain/abi_def.hpp#L7
pub type TypeName = String;
pub type FieldName = String;

pub fn is_array(t: &str) -> bool {
    t.ends_with("[]")
}

pub fn is_sized_array(t: &str) -> bool {
    match (t.rfind('['), t.rfind(']')) {
        (Some(pos1), Some(pos2)) => {
            if pos1 + 1 == pos2 { false }
            else { t[pos1+1..pos2].chars().all(|c| c.is_digit(10)) }
        },
        _ => false,
    }
}

pub fn is_optional(t: &str) -> bool {
    t.ends_with('?')
}

// FIXME: should this be recursive? ie: what is `fundamental_type("int[]?")` ?
pub fn fundamental_type<'a>(t: &'a str) -> &'a str {
    if is_array(t) { &t[..t.len()-2] }
    else if is_sized_array(t) { &t[..t.rfind('[').unwrap()] }
    else if is_optional(t) { &t[..t.len()-1] }
    else { t }
}



#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Type {
    pub new_type_name: TypeName,

    #[serde(rename = "type")]
    pub type_: TypeName, // TODO: should map into a struct defined within the ABI? or base types?
}

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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Action {
    pub name: ActionName,
    #[serde(rename = "type")]
    pub type_: TypeName,  // TODO: should map into a struct defined within the ABI
    pub ricardian_contract: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Table {
    pub name: TableName,
    #[serde(rename = "type")]
    pub type_: TypeName, // TODO: should map into a struct defined within the ABI
    pub index_type: TypeName,
    pub key_names: Vec<FieldName>,
    pub key_types: Vec<TypeName>,
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
}


impl ABIDefinition {
    pub fn from_str(s: &str) -> Self {
        serde_json::from_str(s).unwrap()
    }

    pub fn from_bin(stream: &mut ByteStream) -> Self {
        let _version = AntelopeType::from_bin("string", stream);
        todo!();
    }
}

impl Default for ABIDefinition {
    fn default() -> ABIDefinition {
        ABIDefinition {
            version: "eosio::abi/1.1".to_owned(),
            types: vec![],
            structs: vec![],
            actions: vec![],
            tables: vec![],
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::read_to_string;

    #[test]
    fn parse_abi_def() {
        let abi_str = read_to_string("tests/abi_example.json").unwrap();
        let abi: ABIDefinition = serde_json::from_str(&abi_str).unwrap();

        assert_eq!(abi.version, "eosio::abi/1.1");

        println!("{:#?}", &abi);

        // assert!(false);
    }
}
