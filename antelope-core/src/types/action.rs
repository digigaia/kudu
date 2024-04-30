use std::fmt;

use anyhow::Result;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

use crate::{
    json, JsonValue,
};
use crate::types::{AccountName, ActionName, PermissionName};

// #[derive(Error, Debug)]
// pub enum InvalidName {
//     #[error("Name is longer than 13 characters: \"{0}\"")]
//     TooLong(String),

//     #[error(r#"Name not properly normalized (given name: "{0}", normalized: "{1}")"#)]
//     InvalidNormalization(String, String),
// }


#[derive(Eq, Hash, PartialEq, Debug, Copy, Clone)]
pub struct PermissionLevel {
    actor: AccountName,
    permission: PermissionName,
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct Action {
    account: AccountName,
    name: ActionName,
    auth: Vec<PermissionLevel>,
    data: Vec<u8>,
}
