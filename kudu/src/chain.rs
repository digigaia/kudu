//!
//! this contains some basic types for the chain FIXME FIXME write me properly!!
//!
//! Other useful types include [`Action`], [`PermissionLevel`].
//!


use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    AccountName, ActionName, BlockId, BlockTimestamp, Digest, Extensions, MicroSeconds,
    PermissionName, TransactionId, TimePointSec, VarUint32, Name, Asset, ABISerializable,
    abiserializable::to_bin, Bytes, Signature, SerializeEnumPrefixed,
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

// from: https://github.com/AntelopeIO/spring/blob/main/libraries/chain/include/eosio/chain/action.hpp

pub trait Contract: ABISerializable {
    fn account() -> AccountName;
    fn name() -> ActionName;
}

#[derive(Eq, Hash, PartialEq, Debug, Copy, Clone, Default, Deserialize, Serialize, ABISerializable)]
pub struct PermissionLevel {
    pub actor: AccountName,
    pub permission: PermissionName,
}

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

impl IntoPermissionVec for (&str, &str) {
    fn into_permission_vec(self) -> Vec<PermissionLevel> {
        vec![PermissionLevel { actor: AccountName::constant(self.0), permission: PermissionName::constant(self.1) }]
    }
}

impl Action {
    pub fn new<T: Contract>(authorization: impl IntoPermissionVec, contract: T) -> Action {
        Action {
            account: T::account(),
            name: T::name(),
            authorization: authorization.into_permission_vec(),
            data: to_bin(&contract)
        }
    }
}

// from: https://github.com/AntelopeIO/spring/blob/main/libraries/chain/include/eosio/chain/action_receipt.hpp

#[derive(Eq, Hash, PartialEq, Debug, Clone, Default, Serialize, Deserialize, ABISerializable)]
pub struct AccountAuthSequence {
    pub account: Name,
    pub sequence: u64,
}

/// For each action dispatched this receipt is generated.
#[derive(Eq, Hash, PartialEq, Debug, Clone, Default, Serialize, Deserialize, ABISerializable)]
pub struct ActionReceiptV0 {
    pub receiver: AccountName,
    pub act_digest: Digest,
    pub global_sequence: u64,
    pub recv_sequence: u64,
    // FIXME: check this field
    pub auth_sequence: Vec<AccountAuthSequence>,
    // pub auth_sequence: Vec<PermissionLevel>,
    pub code_sequence: VarUint32,
    pub abi_sequence: VarUint32,
}

#[derive(Eq, Hash, PartialEq, Debug, Clone, SerializeEnumPrefixed, ABISerializable)]
pub enum ActionReceipt {
    V0(ActionReceiptV0),
}

// FIXME: check that the `Ord` is correct, as it differs from the C++ one which only compares on "account"
//        (which is probably an optimization, and we should be fine)
#[derive(Eq, Hash, PartialEq, Ord, PartialOrd, Debug, Clone, Default, Serialize, Deserialize, ABISerializable)]
pub struct AccountDelta {
    pub account: AccountName,
    pub delta: i64,
}



#[derive(Eq, Hash, PartialEq, Debug, Clone, Default, ABISerializable, Serialize)]
pub struct Trace {
    pub action_ordinal: VarUint32,
    pub creator_action_ordinal: VarUint32,
    pub closest_unnotified_ancestor_action_ordinal: VarUint32,
    pub receipt: Option<ActionReceipt>,
    pub receiver: ActionName,
    pub act: Action,
    pub context_free: bool, // = false;
    pub elapsed: MicroSeconds,
    pub console: String,
    pub trx_id: TransactionId, /// the transaction that generated this action
    pub block_num: u32, // = 0;
    pub block_time: BlockTimestamp,
    pub producer_block_id: Option<BlockId>,
    pub account_ram_deltas: Set<AccountDelta>,
      // std::optional<fc::exception>    except;  // TODO / FIXME
    pub error_code: Option<u64>,
    pub return_value: Bytes,
}


#[derive(Eq, Hash, PartialEq, Debug, Clone, Default, Serialize, Deserialize, ABISerializable)]
pub struct Transaction {
    // -----------------------------------------------------------------------------
    //     TransactionHeader fields
    // -----------------------------------------------------------------------------

    /// The time at which a transaction expires.
    pub expiration: TimePointSec,
    /// Specifies a block num in the last 2^16 blocks.
    pub ref_block_num: u16,
    /// Specifies the lower 32 bits of the block id.
    pub ref_block_prefix: u32,
    /// Upper limit on total network bandwidth (in 8 byte words) billed for this transaction.
    pub max_net_usage_words: VarUint32,
    /// Upper limit on the total CPU time billed for this transaction.
    pub max_cpu_usage_ms: u8,
    /// Number of seconds to delay this transaction for during which it may be canceled.
    pub delay_sec: VarUint32,

    // -----------------------------------------------------------------------------
    //     Transaction fields
    // -----------------------------------------------------------------------------

    pub context_free_actions: Vec<Action>,
    pub actions: Vec<Action>,
    pub transaction_extensions: Extensions,
}



impl Transaction {
    pub fn id() -> TransactionId {
        todo!();  // sha256 hash of the serialized trx
    }
}



/// not a native Antelope type but normally defined through an ABI
/// It is provided here for convenience
#[derive(Clone, Debug, PartialEq, Eq, ABISerializable, Serialize, Deserialize)]
pub struct Transfer {
    pub from: Name,
    pub to: Name,
    pub quantity: Asset,
    pub memo: String,
}


impl Contract for Transfer {
    fn account() -> AccountName {
        const { AccountName::constant("eosio.token") }
     }
    fn name() -> ActionName {
        const { ActionName::constant("transfer") }
    }
}


#[derive(Eq, Hash, PartialEq, Debug, Clone, Default, Serialize, Deserialize, ABISerializable)]
pub struct PackedTransactionV0 {
    pub signatures: Vec<Signature>,
    pub compression: u8,
    pub packed_context_free_data: Bytes,
    pub packed_trx: Transaction,
}

#[derive(Eq, Hash, PartialEq, Debug, Clone, Default, Serialize, Deserialize, ABISerializable)]
pub struct TransactionTraceException {
    pub error_code: i64,
    pub error_message: String,
}

#[derive(Eq, Hash, PartialEq, Debug, Clone, Default, Serialize, Deserialize, ABISerializable)]
pub struct ActionTraceV0 {
    pub action_ordinal: VarUint32,
    pub creator_action_ordinal: VarUint32,
    pub receipt: Option<ActionReceipt>,
    pub receiver: Name,
    pub act: Action,
    pub context_free: bool,
    pub elapsed: i64,
    pub console: String,
    pub account_ram_deltas: Vec<AccountDelta>,  // FIXME: replace me with flat_set
    pub except: Option<String>,
    pub error_code: Option<u64>,
}

#[derive(Eq, Hash, PartialEq, Debug, Clone, Default, Serialize, Deserialize, ABISerializable)]
pub struct ActionTraceV1 {
    pub action_ordinal: VarUint32,
    pub creator_action_ordinal: VarUint32,
    pub receipt: Option<ActionReceipt>,
    pub receiver: Name,
    pub act: Action,
    pub context_free: bool,
    pub elapsed: i64,
    pub console: String,
    pub account_ram_deltas: Vec<AccountDelta>,  // FIXME: replace me with flat_set
    pub account_disk_deltas: Vec<AccountDelta>,
    pub except: Option<String>,
    pub error_code: Option<u64>,
    pub return_value: Bytes,
}

#[derive(Eq, Hash, PartialEq, Debug, Clone, SerializeEnumPrefixed, ABISerializable)]
pub enum ActionTrace {
    V0(ActionTraceV0),
    V1(ActionTraceV1),
}

// FIXME: defined in spring:libraries/state_history
#[derive(Eq, Hash, PartialEq, Debug, Clone, Default, Serialize, Deserialize, ABISerializable)]
pub struct PartialTransaction {

}

// NOTE: this is the one used in the tests with the ship_abi.json ABI
//       it seems to be an old one as the one defined in "spring:chain/trace.hpp" differs significantly
// TODO: we should also define the new one corresponding to the current Antelope version
#[derive(Eq, Hash, PartialEq, Debug, Clone, Default, Serialize, Deserialize, ABISerializable)]
pub struct TransactionTraceV0 {
    pub id: TransactionId,
    pub status: u8,
    pub cpu_usage_us: u32,
    pub net_usage_words: VarUint32,
    pub elapsed: i64,
    pub net_usage: u64,
    pub scheduled: bool,
    pub action_traces: Vec<ActionTrace>,
    pub account_ram_delta: Option<AccountDelta>,
    pub except: Option<String>,
    pub error_code: Option<u64>,
    pub failed_dtrx_trace: Option<Box<TransactionTraceV0>>,
    pub partial: Option<PartialTransaction>,
}

#[derive(Eq, Hash, PartialEq, Debug, Clone, SerializeEnumPrefixed, ABISerializable)]
pub enum TransactionTrace {
    V0(TransactionTraceV0),
}

#[derive(Eq, Hash, PartialEq, Debug, Clone, SerializeEnumPrefixed, ABISerializable)]
pub enum TransactionTraceMsg {
    #[serde(rename="transaction_trace_exception")]
    Exception(TransactionTraceException),

    #[serde(rename="transaction_trace")]
    Trace(TransactionTrace),
}
