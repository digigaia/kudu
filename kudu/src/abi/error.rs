use hex::FromHexError;
use serde_json::{
    Error as JsonError,
    Value as JsonValue,
};
use snafu::Snafu;

use kudu_macros::with_location;

use crate::{InvalidValue, impl_auto_error_conversion, SerializeError};

#[with_location]
#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum ABIError {
    #[snafu(display("cannot deserialize {what} from stream"))]
    DeserializeError { what: String, source: SerializeError },

    #[snafu(display(r#"unsupported ABI version: "{version}""#))]
    VersionError { version: String },

    #[snafu(display(r#"incompatible versions: "{a}" vs. "{b}""#))]
    IncompatibleVersionError { a: String, b: String },

    #[snafu(display("integrity error: {message}"))]
    IntegrityError { message: String },

    #[snafu(display("encode error: {message}"))]
    EncodeError { message: String },

    #[snafu(display("decode error: {message}"))]
    DecodeError { message: String },

    #[snafu(display("cannot deserialize ABIDefinition from JSON"))]
    JsonError { source: JsonError },

    #[snafu(display("cannot decode hex representation for hex ABI"))]
    HexABIError { source: FromHexError },

    #[snafu(display("unknown ABI with name: '{name}'"))]
    UnknownABIError { name: String },

    #[snafu(display("cannot convert variant to AntelopeValue: {v}"))]
    VariantConversionError { v: Box<JsonValue>, source: InvalidValue },

    #[snafu(display(r#"cannot convert given variant {value} to Antelope type "{typename}""#))]
    IncompatibleVariantTypes {
        typename: String,
        value: Box<JsonValue>,
    },
}

impl_auto_error_conversion!(FromHexError, ABIError, HexABISnafu);
impl_auto_error_conversion!(JsonError, ABIError, JsonSnafu);
