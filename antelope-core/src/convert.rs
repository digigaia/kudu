use std::any::type_name;
use std::str::FromStr;
use std::num::{ParseFloatError, ParseIntError, TryFromIntError};

use hex::FromHexError;
use num::{Integer, Signed, Unsigned, Float};
use serde_json::Value as JsonValue;
use snafu::prelude::*;

use antelope_macros::with_location;


// -----------------------------------------------------------------------------
//     Error type for all possible conversion errors
// -----------------------------------------------------------------------------

#[with_location]
#[derive(Debug, Snafu)]
pub enum ConversionError {
    #[snafu(display("invalid integer: {repr}"))]
    Int {
        repr: String,
        source: ParseIntError
    },

    #[snafu(display("integer out of range: cannot fit {value} in a `{target_type}`"))]
    IntPrecision {
        value: i128,  // i128 allows to represent both i64 and u64
        target_type: &'static str,
        source: TryFromIntError
    },

    #[snafu(display("invalid float: {repr}"))]
    Float {
        repr: String,
        source: ParseFloatError,
    },

    #[snafu(display("float out of range, cannot convert to f32: {value}"))]
    FloatPrecision {
        value: f64,
    },

    #[snafu(display(r#"cannot convert given variant {value} to type "{typename}""#))]
    IncompatibleVariantTypes {
        typename: &'static str,
        value: Box<JsonValue>
    },

}

type Result<T, E = ConversionError> = std::result::Result<T, E>;


// -----------------------------------------------------------------------------
//     Hex conversion functions
// -----------------------------------------------------------------------------

pub fn hex_to_boxed_array<const N: usize>(s: &str) -> Result<Box<[u8; N]>, FromHexError> {
    let mut result = [0_u8; N];
    hex::decode_to_slice(s, &mut result)?;
    Ok(Box::new(result))
}


// -----------------------------------------------------------------------------
//     Utility functions to convert numeric types
// -----------------------------------------------------------------------------

pub fn variant_to_str(v: &JsonValue) -> Result<&str> {
    v.as_str().with_context(|| IncompatibleVariantTypesSnafu {
        typename: "&str",
        value: v.clone(),
    })
}

pub fn str_to_int<T>(s: &str) -> Result<T>
where
    T: Integer + FromStr<Err = ParseIntError>,
{
    s.parse().context(IntSnafu { repr: s })
}

pub fn str_to_float<T>(s: &str) -> Result<T>
where
    T: Float + FromStr<Err = ParseFloatError>,
{
    s.parse().context(FloatSnafu { repr: s })
}

pub fn variant_to_int<T>(v: &JsonValue) -> Result<T>
where
    T: TryFromInt64 + FromStr<Err = ParseIntError>,
{
    match v {
        v if v.is_i64() => T::try_from_i64(v.as_i64().unwrap()),
        v if v.is_string() => {
            let v_str = v.as_str().unwrap();
            v_str.parse().context(IntSnafu { repr: v_str })
        },
        _ => IncompatibleVariantTypesSnafu { typename: type_name::<T>(), value: v.clone() }.fail(),
    }
}

pub fn variant_to_uint<T>(v: &JsonValue) -> Result<T>
where
    T: TryFromUint64 + FromStr<Err = ParseIntError>,
{
    match v {
        v if v.is_u64() => T::try_from_u64(v.as_u64().unwrap()),
        v if v.is_string() => {
            let v_str = v.as_str().unwrap();
            v_str.parse().context(IntSnafu { repr: v_str })
        },
        _ => IncompatibleVariantTypesSnafu { typename: type_name::<T>(), value: v.clone() }.fail(),
    }
}

pub fn variant_to_float<T>(v: &JsonValue) -> Result<T>
where
    T: TryFromFloat64 + FromStr<Err = ParseFloatError>,
{
    match v {
        v if v.is_f64() => T::try_from_f64(v.as_f64().unwrap()),
        v if v.is_string() => {
            let v_str = v.as_str().unwrap();
            v_str.parse().context(FloatSnafu { repr: v_str })
        },
        _ => IncompatibleVariantTypesSnafu { typename: type_name::<T>(), value: v.clone() }.fail(),
    }
}


// -----------------------------------------------------------------------------
//     Trait definitions to convert an i64/u64 to any int and f64 to f32
//
//     note: TryFrom doesn't work because it has `Err = TryFromIntError`
//           for all types except themselves where `Err = Infallible`
// -----------------------------------------------------------------------------

pub trait TryFromInt64 : Integer + Signed {
    fn try_from_i64(value: i64) -> Result<Self, ConversionError>;
}

pub trait TryFromUint64 : Integer + Unsigned {
    fn try_from_u64(value: u64) -> Result<Self, ConversionError>;
}

pub trait TryFromFloat64 : Float {
    fn try_from_f64(value: f64) -> Result<Self, ConversionError>;
}


// -----------------------------------------------------------------------------
//     Implementation of those traits on integer types
// -----------------------------------------------------------------------------

macro_rules! conv_from_i64 {
    ($t:ident) => {
        impl TryFromInt64 for $t {
            fn try_from_i64(value: i64) -> Result<Self, ConversionError> {
                value.try_into().context(IntPrecisionSnafu { value, target_type: stringify!($t) })
            }
        }
    };
    ($t:ident, infallible) => {
        impl TryFromInt64 for $t {
            fn try_from_i64(value: i64) -> Result<Self, ConversionError> {
                Ok(value.into())
            }
        }
    };
}

macro_rules! conv_from_u64 {
    ($t:ident) => {
        impl TryFromUint64 for $t {
            fn try_from_u64(value: u64) -> Result<Self, ConversionError> {
                value.try_into().context(IntPrecisionSnafu { value, target_type: stringify!($t) })
            }
        }
    };
    ($t:ident, infallible) => {
        impl TryFromUint64 for $t {
            fn try_from_u64(value: u64) -> Result<Self, ConversionError> {
                Ok(value.into())
            }
        }
    };
}


conv_from_i64!(i8);
conv_from_i64!(i16);
conv_from_i64!(i32);
conv_from_i64!(i64, infallible);
conv_from_i64!(i128, infallible);

conv_from_u64!(u8);
conv_from_u64!(u16);
conv_from_u64!(u32);
conv_from_u64!(u64, infallible);
conv_from_u64!(u128, infallible);

impl TryFromFloat64 for f64 {
    fn try_from_f64(value: f64) -> Result<f64, ConversionError> {
        Ok(value)
    }
}

impl TryFromFloat64 for f32 {
    fn try_from_f64(value: f64) -> Result<f32, ConversionError> {
        let result = value as f32;
        if result.is_finite() {
            Ok(result)
        }
        else {
            FloatPrecisionSnafu { value }.fail()
        }
    }
}
