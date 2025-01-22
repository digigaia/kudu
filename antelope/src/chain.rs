//!
//! this contains some basic types for the chain FIXME FIXME write me properly!!
//!
//! Other useful types include [`Action`], [`PermissionLevel`].
//!


use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    AccountName, ActionName, BlockID, BlockTimestampType, Digest, Extensions, MicroSeconds,
    PermissionName, TransactionID, TimePointSec, VarUint32, Name, Asset, BinarySerializable,
    binaryserializable::to_bin, Bytes, Signature,
};

extern crate self as antelope;

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
//      - std::vector<char> -> Vec<u8>
//
//     Notes:
//      - on x86, char is usually signed char, but can be converted losslessly
//        to unsigned char, so we keep u8 as a representation
//        see: https://en.cppreference.com/w/cpp/language/types
//
// =============================================================================


// from: https://github.com/AntelopeIO/spring/blob/main/libraries/chain/include/eosio/chain/action.hpp

pub trait Contract: BinarySerializable {
    fn account() -> AccountName;
    fn name() -> ActionName;
}

#[derive(Eq, Hash, PartialEq, Debug, Copy, Clone, Default, Deserialize, Serialize, BinarySerializable)]
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
#[derive(Eq, Hash, PartialEq, Debug, Clone, Default, Deserialize, Serialize, BinarySerializable)]
pub struct Action {
    pub account: AccountName,
    pub name: ActionName,
    pub authorization: Vec<PermissionLevel>,
    pub data: Bytes,
}

impl Action {
    pub fn new<T: Contract>(authorization: Vec<PermissionLevel>, contract: T) -> Action {
        Action {
            account: T::account(),
            name: T::name(),
            authorization,
            data: to_bin(&contract)
        }
    }
}

// from: https://github.com/AntelopeIO/spring/blob/main/libraries/chain/include/eosio/chain/action_receipt.hpp

/// For each action dispatched this receipt is generated.
#[derive(Eq, Hash, PartialEq, Debug, Clone, Default)]
pub struct ActionReceipt {
    receiver: AccountName,
    act_digest: Digest,
    global_sequence: u64,
    recv_sequence: u64,
    auth_sequence: BTreeMap<AccountName, u64>,
    code_sequence: VarUint32,
    abi_sequence: VarUint32,
}

#[derive(Eq, Hash, PartialEq, Debug, Clone, Default)]
pub struct AccountDelta {
    account: AccountName,
    delta: i64,
}

#[derive(Eq, Hash, PartialEq, Debug, Clone, Default)]
pub struct Trace {
    action_ordinal: VarUint32,
    creator_action_ordinal: VarUint32,
    closest_unnotified_ancestor_action_ordinal: VarUint32,
    receipt: Option<ActionReceipt>,
    receiver: ActionName,
    act: Action,
    context_free: bool, // = false;
    elapsed: MicroSeconds,
    console: String,
    trx_id: TransactionID, /// the transaction that generated this action
    block_num: u32, // = 0;
    block_time: BlockTimestampType,
    producer_block_id: Option<BlockID>,
    account_ram_deltas: BTreeSet<AccountDelta>,
      // std::optional<fc::exception>    except;  // TODO / FIXME
    error_code: Option<u64>,
    return_value: Vec<u8>,
}


#[derive(Eq, Hash, PartialEq, Debug, Clone, Default, Serialize, Deserialize, BinarySerializable)]
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
    pub fn id() -> TransactionID {
        todo!();  // sha256 hash of the serialized trx
    }
}



/// not a native Antelope type but normally defined through an ABI
/// It is provided here for convenience
#[derive(Clone, Debug, PartialEq, Eq, BinarySerializable, Serialize, Deserialize)]
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


#[derive(Eq, Hash, PartialEq, Debug, Clone, Default, Serialize, Deserialize, BinarySerializable)]
pub struct PackedTransactionV0 {
    pub signatures: Vec<Signature>,
    pub compression: u8,
    pub packed_context_free_data: Bytes,
    pub packed_trx: Transaction,
}
