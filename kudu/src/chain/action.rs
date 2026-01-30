use std::fmt;

use hex::FromHexError;
use serde::{Deserialize, Serialize};
use serde_json::json;
use snafu::{Snafu, OptionExt};

use crate::{
    AccountName, ActionName, Contract,
    PermissionName, Name, ABISerializable,
    abiserializable::to_bin, Bytes, JsonValue,
    ByteStream, ABI, ABIError, InvalidName,
    abi, with_location, impl_auto_error_conversion,
};

// this is needed to be able to call the `ABISerializable` derive macro, which needs
// access to the `kudu` crate
extern crate self as kudu;


// from: https://github.com/AntelopeIO/spring/blob/main/libraries/chain/include/eosio/chain/action.hpp


#[derive(Eq, Hash, PartialEq, Debug, Copy, Clone, Default, Deserialize, Serialize, ABISerializable)]
pub struct PermissionLevel {
    pub actor: AccountName,
    pub permission: PermissionName,
}

pub trait IntoPermissionVec {
    fn into_permission_vec(self) -> Vec<PermissionLevel>;
}

impl IntoPermissionVec for Vec<PermissionLevel> {
    fn into_permission_vec(self) -> Vec<PermissionLevel> {
        self
    }
}

impl IntoPermissionVec for PermissionLevel {
    fn into_permission_vec(self) -> Vec<PermissionLevel> {
        vec![self]
    }
}

// NOTE: this panics if the `&str` are not valid `Name`s
impl IntoPermissionVec for (&str, &str) {
    fn into_permission_vec(self) -> Vec<PermissionLevel> {
        vec![PermissionLevel {
            actor: AccountName::constant(self.0),
            permission: PermissionName::constant(self.1)
        }]
    }
}

impl fmt::Display for PermissionLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.actor, self.permission)
    }
}



#[with_location]
#[derive(Debug, Snafu)]
// TODO: rename to `InvalidAction` for consistency?
pub enum ActionError {
    #[snafu(display("Cannot convert action['{field_name}'] to str, actual type: {value:?}"))]
    FieldType {
        field_name: String,
        value: JsonValue,
    },

    #[snafu(display("Invalid name"))]
    Name { source: InvalidName },

    #[snafu(display("invalid hex representation"))]
    FromHex { source: FromHexError },

    #[snafu(display("could not match JSON object to transaction"))]
    FromJson { source: serde_json::Error },

    #[snafu(display("ABI error"))]
    ABI { source: ABIError },
}

impl_auto_error_conversion!(InvalidName, ActionError, NameSnafu);
impl_auto_error_conversion!(FromHexError, ActionError, FromHexSnafu);
impl_auto_error_conversion!(ABIError, ActionError, ABISnafu);
impl_auto_error_conversion!(serde_json::Error, ActionError, FromJsonSnafu);


/// An action is performed by an actor, aka an account. It may
/// be created explicitly and authorized by signatures or might be
/// generated implicitly by executing application code.
///
/// This follows the design pattern of React Flux where actions are
/// named and then dispatched to one or more action handlers (aka stores).
/// In the context of eosio, every action is dispatched to the handler defined
/// by account 'scope' and function 'name', but the default handler may also
/// forward the action to any number of additional handlers. Any application
/// can write a handler for "scope::name" that will get executed if and only if
/// this action is forwarded to that application.
///
/// Each action may require the permission of specific actors. Actors can define
/// any number of permission levels. The actors and their respective permission
/// levels are declared on the action and validated independently of the executing
/// application code. An application code will check to see if the required
/// authorization were properly declared when it executes.
#[derive(Eq, Hash, PartialEq, Debug, Clone, Default, Deserialize, Serialize, ABISerializable)]
pub struct Action {
    pub account: AccountName,
    pub name: ActionName,
    pub authorization: Vec<PermissionLevel>,
    pub data: Bytes,
}

impl Action {
    pub fn new<T: Contract>(authorization: impl IntoPermissionVec, contract: &T) -> Action {
        Action {
            account: T::account(),
            name: T::name(),
            authorization: authorization.into_permission_vec(),
            data: to_bin(contract)
        }
    }

    pub fn conv_action_field_str<'a>(
        action: &'a JsonValue,
        field: &str
    ) -> Result<&'a str, ActionError> {
        action[field].as_str().with_context(|| FieldTypeSnafu {
            field_name: field,
            value: action[field].clone(),
        })
    }

    pub fn from_json(action: &JsonValue) -> Result<Action, ActionError> {
        // FIXME: too many unwraps
        let account = Action::conv_action_field_str(action, "account")?;
        let action_name = Action::conv_action_field_str(action, "name")?;

        // TODO: can we make this more efficient?
        let authorization: Vec<PermissionLevel> = serde_json::from_str(&action["authorization"].to_string())?;

        let data: Bytes = if action["data"].is_string() {
            // if `data` is already provided as binary data, read it as is
            Bytes::from_hex(action["data"].as_str().unwrap())?  // safe unwrap
        }
        else {
            // otherwise, we need an ABI to encode the data into a binary string
            let data = &action["data"];
            let abi = abi::registry::get_abi(account)?;
            let mut ds = ByteStream::new();
            abi.encode_variant(&mut ds, action_name, data)?;
            ds.into()
        };

        Ok(Action {
            account: Name::new(account)?,
            name: Name::new(action_name)?,
            authorization,
            data,
        })
    }

    pub fn from_json_array(actions: &JsonValue) -> Result<Vec<Action>, ActionError> {
        Ok(actions.as_array().unwrap().iter()
            .map(|v| Action::from_json(v).unwrap())
            .collect())
    }

    pub fn decode_data(&self) -> Result<JsonValue, ABIError> {
        let abi = abi::registry::get_abi(&self.account.to_string())?;
        self.decode_data_with_abi(&abi)
    }

    pub fn decode_data_with_abi(&self, abi: &ABI) -> Result<JsonValue, ABIError> {
        // FIXME: this .clone() is unnecessary once we fix deserializing from bytestream
        let mut ds = ByteStream::from(self.data.clone());
        abi.decode_variant(&mut ds, &self.name.to_string())
    }

    pub fn with_data(mut self, value: &JsonValue) -> Self {
        let mut ds = ByteStream::new();
        let abi = abi::registry::get_abi(&self.account.to_string()).unwrap();
        abi.encode_variant(&mut ds, &self.name.to_string(), value).unwrap();
        self.data = ds.into();
        self
    }

    pub fn to_json(&self) -> Result<JsonValue, ABIError> {
        let abi = abi::registry::get_abi(&self.account.to_string())?;
        self.to_json_with_abi(&abi)
    }

    pub fn to_json_with_abi(&self, abi: &ABI) -> Result<JsonValue, ABIError> {
        Ok(json!({
            "account": self.account.to_string(),
            "name": self.name.to_string(),
            "authorization": serde_json::to_value(&self.authorization)?,
            "data": self.decode_data_with_abi(abi)?,
        }))
    }
}
