//!
//! this contains some basic types for the chain FIXME FIXME write me properly!!
//!
//! Other useful types include [`Action`], [`PermissionLevel`].
//!

mod action;
mod trace;
mod transaction;

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    ABISerializable, AccountName, ActionName, Asset, Name, contract,
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
