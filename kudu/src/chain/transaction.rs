use chrono::ParseError as ChronoParseError;
use serde::{Deserialize, Serialize};
use snafu::Snafu;

use crate::{
    ABIProvider, Action, ActionError, Extensions, JsonValue,
    TransactionId, TimePointSec, VarUint32, ABISerializable,
    convert::{variant_to_object, variant_to_str, variant_to_uint, ConversionError},
    impl_auto_error_conversion, with_location,
};

// this is needed to be able to call the `ABISerializable` derive macro, which needs
// access to the `kudu` crate
extern crate self as kudu;

#[with_location]
#[derive(Debug, Snafu)]
// TODO: rename to `InvalidAction` for consistency?
pub enum TransactionError {
    #[snafu(display("unknown field: '{field}'"))]
    UnknownField { field: String },

    #[snafu(display("cannot convert field to required target type"))]
    Conversion { source: ConversionError },

    #[snafu(display("cannot parse date/time"))]
    DateTimeParse { source: ChronoParseError },

    #[snafu(display("invalid action"))]
    InvalidAction { source: ActionError },

    #[snafu(display("could not match JSON object to transaction"))]
    FromJson { source: serde_json::Error },
}

impl_auto_error_conversion!(ChronoParseError, TransactionError, DateTimeParseSnafu);
impl_auto_error_conversion!(ConversionError, TransactionError, ConversionSnafu);
impl_auto_error_conversion!(ActionError, TransactionError, InvalidActionSnafu);
impl_auto_error_conversion!(serde_json::Error, TransactionError, FromJsonSnafu);


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

    /// Create a new `Transaction` from a JSON value containing the non-default fields.
    /// You should pass an `ABIProvider` if the data fields for the `Actions` are not
    /// encoded yet, it is unnecessary otherwise.
    pub fn from_json(
        abi_provider: Option<&ABIProvider>,
        tx: &JsonValue
    ) -> Result<Transaction, TransactionError> {
        let mut result = Transaction::default();
        for (field, value) in variant_to_object(tx)?.iter() {
            match field.as_str() {
                "expiration"           => result.expiration           = variant_to_str(value)?.parse()?,
                "ref_block_num"        => result.ref_block_num        = variant_to_uint(value)?,
                "ref_block_prefix"     => result.ref_block_prefix     = variant_to_uint(value)?,
                "max_cpu_usage_ms"     => result.max_cpu_usage_ms     = variant_to_uint(value)?,
                "max_net_usage_words"  => result.max_net_usage_words  = variant_to_uint::<u32>(value)?.into(),
                "delay_sec"            => result.delay_sec            = variant_to_uint::<u32>(value)?.into(),
                "context_free_actions" => result.context_free_actions = Action::from_json_array(abi_provider, value)?,
                "actions"              => result.actions              = Action::from_json_array(abi_provider, value)?,
                "transaction_extensions" => result.transaction_extensions = serde_json::from_value(value.clone())?,
                other => UnknownFieldSnafu { field: other }.fail()?,
            }
        }
        Ok(result)
    }
}
