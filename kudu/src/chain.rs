//!
//! this contains some basic types for the chain FIXME FIXME write me properly!!
//!
//! Other useful types include [`Action`], [`PermissionLevel`].
//!

mod action;
mod trace;
mod transaction;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json;
use tracing::{debug, trace, warn};
use transaction::TransactionError;

use crate::{
    contract, ABISerializable, APIClient, AccountName, ActionName, Asset, Bytes, JsonValue, Name, PrivateKey
};

// this is needed to be able to call the `ABISerializable` derive macro, which needs
// access to the `kudu` crate
extern crate self as kudu;

// =============================================================================
//
//     Type definitions for Antelope structs found in the
//     https://github.com/AntelopeIO/spring/blob/main/libraries/chain/ folder
//
//     Type correspondence:
//      - fc::unsigned_int -> VarUint32
//      - fc::microseconds -> i64
//      - boost::flat_map -> BTreeMap
//      - boost::flat_set -> BTreeSet
//      - std::vector<char> -> Bytes
//
//     Notes:
//      - on x86, char is usually signed char, but can be converted losslessly
//        to unsigned char, so we keep u8 as a representation
//        see: https://en.cppreference.com/w/cpp/language/types
//
// =============================================================================


pub type Map<K, V> = BTreeMap<K, V>;
pub type Set<T> = BTreeSet<T>;


pub trait Contract: ABISerializable {
    fn account() -> AccountName;
    fn name() -> ActionName;
}

pub use action::{Action, ActionError, IntoPermissionVec, PermissionLevel};
pub use trace::{
    AccountAuthSequence, AccountDelta,
    ActionReceipt, ActionReceiptV0,
    ActionTrace, ActionTraceV0, ActionTraceV1,
    PackedTransactionV0,
    Trace,
    TransactionTrace, TransactionTraceV0, TransactionTraceException, TransactionTraceMsg,
};
pub use transaction::{SignedTransaction, Transaction};


/// not a native Antelope type but normally defined through an ABI
/// It is provided here for convenience
#[derive(Clone, Debug, PartialEq, Eq, ABISerializable, Serialize, Deserialize)]
#[contract(account="eosio.token", name="transfer")]
pub struct Transfer {
    pub from: Name,
    pub to: Name,
    pub quantity: Asset,
    pub memo: String,
}


// impl Contract for Transfer {
//     fn account() -> AccountName {
//         const { AccountName::constant("eosio.token") }
//      }
//     fn name() -> ActionName {
//         const { ActionName::constant("transfer") }
//     }
// }



// TODO: move this to api.rs (or not?)

// TODO: make tracing level configurable so we can use it for both success and error logging
pub fn nodeos_log(response: &JsonValue, console_output: &[String]) {
    // this is a separate function (instead of inline) so it shows that the logs come from nodeos
    if !console_output.is_empty() {
        let tx_id = response["transaction_id"].as_str().unwrap();
        debug!("Console output for tx: {}", tx_id);
    }
    for output in console_output {
        for line in output.lines() {
            debug!(line);
        }
    }
}

/// Parse a full JSON transaction trace and return the relevant message lines
/// WARNING: this panics if the trace is malformed
pub fn parse_trace(response: &JsonValue) -> Result<Vec<String>, Vec<String>> {
    trace!("parse trace: {}", serde_json::to_string_pretty(response).unwrap());

    let mut lines = vec![];
    if let Some(processed) = response.get("processed") {
        // print console output
        for trace in processed["action_traces"].as_array().unwrap().iter() {
            if let Some(output) = trace.get("console") {
                if !output.as_str().unwrap().is_empty() {
                    lines.push(output.to_string());
                }
            }
            for inline_trace in trace["inline_traces"].as_array().unwrap().iter() {
                if let Some(output) = inline_trace.get("console") {
                    if !output.as_str().unwrap().is_empty() {
                        lines.push(output.to_string());
                    }
                }
            }
        }
        Ok(lines)
    }
    else if let Some(error) = response.get("error") {
        let msg = &error["details"][0]["message"];
        lines.push(error["what"].to_string());
        lines.push(msg.to_string());
        Err(lines)
    }
    else {
        lines.push(format!("Unhandled case!! {}", serde_json::to_string_pretty(response).unwrap()));
        Err(lines)
    }
}

/// Log the results of a sent transaction.
/// Return a `TransactionError` if there was an issue with the transaction.
pub fn log_tx_trace(response: &JsonValue) -> Result<(), TransactionError> {
    // TODO: use `nodeos_log()` instead
    match parse_trace(response) {
        Ok(lines) => {
            for l in lines.iter() {
                debug!("{}", l);
            }
            Ok(())
        },
        Err(lines) => {
            for l in lines.iter() {
                warn!("{}", l);
            }
            Err(TransactionError::NodeosError { message: lines.join("\n") })
        },
    }
}

pub fn push_action(
    client: Arc<APIClient>,
    actor: Name,
    signing_key: &PrivateKey,
    contract: Name,
    action: Name,
    args: &JsonValue,
) -> Result<(), TransactionError>
{
    debug!("PUSH ACTION: {actor} {contract} {action} {args}");

    // create a new transaction with the given action
    let action = Action {
        account: contract,
        name: action,
        authorization: vec![PermissionLevel { actor, permission: Name::constant("active") }],
        data: Bytes::new(),
    }
    .with_data(args)?;

    let mut tx = Transaction::new(vec![action]);
    tx.link(client)?;

    let signed_tx = tx.sign(signing_key)?;
    let result = signed_tx.send_unchecked()?;

    log_tx_trace(&result)
}


#[cfg(test)]
mod tests {
    use super::*;
    use kudu::{PrivateKey, json};

    #[allow(non_snake_case)]
    fn N(name: &str) -> Name { Name::constant(name) }


    #[test]
    fn test_push_action() -> Result<(), TransactionError> {
        kudu::tracing_init();

        let client = APIClient::local();
        let key = PrivateKey::eosio_dev();

        push_action(client, N("eosio"), &key, N("eosio.token"), N("transfer"), &json!({
            "from": "eosio",
            "to": "eosio.token",
            "quantity": "1.0000 SYS",
            "memo": "here's a sys for you!"
        }))?;

        Ok(())
    }
}
