use std::collections::{BTreeMap, BTreeSet};

use crate::{
    AccountName, ActionName, BlockID, BlockTimestampType, Digest, MicroSeconds, PermissionName, TransactionID, VarUint32
};

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

#[derive(Eq, Hash, PartialEq, Debug, Copy, Clone, Default)]
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
#[derive(Eq, Hash, PartialEq, Debug, Clone, Default)]
pub struct Action {
    pub account: AccountName,
    pub name: ActionName,
    pub auth: Vec<PermissionLevel>,
    pub data: Vec<u8>,
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
