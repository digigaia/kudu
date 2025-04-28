use serde::{Deserialize, Serialize};

use crate::{
    Action, Extensions,
    TransactionId, TimePointSec, VarUint32, ABISerializable,
};

// this is needed to be able to call the `ABISerializable` derive macro, which needs
// access to the `kudu` crate
extern crate self as kudu;


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
