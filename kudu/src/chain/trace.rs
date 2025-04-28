use serde::{Deserialize, Serialize};

use crate::{
    AccountName, Action, ActionName, BlockId, BlockTimestamp, Digest, MicroSeconds,
    TransactionId, VarUint32, Name, ABISerializable,
    Bytes, Signature, SerializeEnumPrefixed, Transaction, Set,
};

// this is needed to be able to call the `ABISerializable` derive macro, which needs
// access to the `kudu` crate
extern crate self as kudu;

// pub type _Map<K, V> = BTreeMap<K, V>;
// pub type Set<T> = BTreeSet<T>;


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
