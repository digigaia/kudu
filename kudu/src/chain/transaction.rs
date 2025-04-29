use serde::{Deserialize, Serialize};
use snafu::{Snafu, ResultExt};

use crate::{
    Action, ABIProvider, Extensions, JsonValue,
    TransactionId, TimePointSec, VarUint32, ABISerializable,
    json, with_location,
};

// this is needed to be able to call the `ABISerializable` derive macro, which needs
// access to the `kudu` crate
extern crate self as kudu;

#[with_location]
#[derive(Debug, Snafu)]
// TODO: rename to `InvalidAction` for consistency?
pub enum TransactionError {
    // #[snafu(display("Cannot convert action['{field_name}'] to str, actual type: {value:?}"))]
    // FieldType {
    //     field_name: String,
    //     value: JsonValue,
    // },

    // #[snafu(display("Invalid name"))]
    // Name { source: InvalidName },

    // #[snafu(display("invalid hex representation"))]
    // FromHex { source: FromHexError },

    // #[snafu(display("ABI error"))]
    // ABI { source: ABIError },

    #[snafu(display("could not match JSON object to transaction"))]
    FromJson { source: serde_json::Error },
}

// impl_auto_error_conversion!(serde_json::Error, TransactionError, FromJsonSnafu);


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
    #[serde(default)]
    pub actions: Vec<Action>,
    pub transaction_extensions: Extensions,
}



impl Transaction {
    pub fn id() -> TransactionId {
        todo!();  // sha256 hash of the serialized trx
    }

    pub fn from_json(
        abi_provider: Option<&ABIProvider>,
        tx: &JsonValue
    ) -> Result<Transaction, TransactionError> {
        // set default values if missing
        let mut tx = tx.as_object().unwrap().clone();
        tx.entry("expiration")            .or_insert("1970-01-01T00:00:00.000".into());
        tx.entry("ref_block_num")         .or_insert(json!(0));
        tx.entry("ref_block_prefix")      .or_insert(json!(0));
        tx.entry("max_cpu_usage_ms")      .or_insert(json!(0));
        tx.entry("max_net_usage_words")   .or_insert(json!(0));
        tx.entry("delay_sec")             .or_insert(json!(0));
        tx.entry("context_free_actions")  .or_insert(json!([]));
        // tx.entry("actions")               .or_insert(json!([]));
        tx.entry("transaction_extensions").or_insert(json!([]));
        tx.entry("context_free_data")     .or_insert(json!([]));  // FIXME: needed? wanted?

        // FIXME: we should do this properly
        // see: https://github.com/serde-rs/serde/issues/2065
        // let expiration = tx.remove("expiration")
        //     .unwrap_or(JsonValue::String("1970-01-01T00:00:00.000".to_owned()));

        let actions = tx.remove("actions").unwrap_or(JsonValue::Array(vec![]));

        let mut result: Transaction = serde_json::from_value(json!(tx)).context(FromJsonSnafu)?;
        result.actions = Action::from_json_array(abi_provider, &actions).unwrap();
        Ok(result)
    }

}
