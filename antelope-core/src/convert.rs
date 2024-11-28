use core::num::dec2flt::pfe_invalid;
use std::any::type_name;
use std::str::FromStr;
use std::num::{ParseFloatError, ParseIntError, TryFromIntError};

use hex::FromHexError;
use num::{Integer, Signed, Unsigned, Float};
use serde_json::Value as JsonValue;
use snafu::prelude::*;

#[cfg(feature = "float128")]
use f128::f128;

use antelope_macros::with_location;


// -----------------------------------------------------------------------------
//     Error type for all possible conversion errors
// -----------------------------------------------------------------------------

#[with_location]
#[derive(Debug, Snafu)]
pub enum ConversionError {
    #[snafu(display("invalid integer: {repr} - target type: {target}"))]
    Int {
        repr: String,
        target: &'static str,
        source: ParseIntError
    },

    #[snafu(display("invalid hex integer: {repr} - target type: {target}"))]
    HexInt {
        repr: String,
        target: &'static str,
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

/// Trait for signed integers that allows parsing negative integers
/// from their hex representation
pub trait NegativeHex : Integer + Signed {
    fn from_hex_str(repr: &str) -> Result<Self>;
}

macro_rules! impl_negative_hex {
    ($t:ident, $unsigned:ty) => {
        impl NegativeHex for $t {
            fn from_hex_str(repr: &str) -> Result<Self> {
                <$unsigned>::from_str_radix(repr, 16)
                    .map(|n| n as $t)
                    .map_err(|_| HexIntSnafu { repr, target: stringify!($t) }.build())
            }
        }
    }
}

impl_negative_hex!(i8, u8);
impl_negative_hex!(i16, u16);
impl_negative_hex!(i32, u32);
impl_negative_hex!(i64, u64);
impl_negative_hex!(i128, u128);


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
    s.parse().context(IntSnafu { repr: s, target: type_name::<T>() })
}

pub fn str_to_float<T>(s: &str) -> Result<T>
where
    T: Float + FromStr<Err = ParseFloatError>,
{
    s.parse().context(FloatSnafu { repr: s })
}

#[cfg(feature = "float128")]
pub fn str_to_f128(s: &str) -> Result<f128> {
    f128::parse(s).map_err(|_| pfe_invalid()).context(FloatSnafu { repr: s })
}

pub fn variant_to_int<T>(v: &JsonValue) -> Result<T>
where
    T: TryFromInt64 + FromStr<Err = ParseIntError> + NegativeHex,
{
    if let Some(n) = v.as_i64()      { T::try_from_i64(n) }
    else if let Some(s) = v.as_str() {
        if let Some(hex_repr) = s.strip_prefix("0x") {
            T::from_hex_str(hex_repr)
        }
        else {
            s.parse().context(IntSnafu { repr: s, target: type_name::<T>() })
        }
    }
    else {
        IncompatibleVariantTypesSnafu { typename: type_name::<T>(), value: v.clone() }.fail()
    }
}

pub fn variant_to_uint<T>(v: &JsonValue) -> Result<T>
where
    T: TryFromUint64 + FromStr<Err = ParseIntError>,
{
    if let Some(n) = v.as_u64()      { T::try_from_u64(n) }
    else if let Some(s) = v.as_str() {
        if let Some(hex_repr) = s.strip_prefix("0x") {
            T::from_str_radix(hex_repr, 16).map_err(|_| HexIntSnafu { repr: s, target: type_name::<T>() }.build())
        }
        else {
            s.parse().context(IntSnafu { repr: s, target: type_name::<T>() })
        }
    }
    else {
        IncompatibleVariantTypesSnafu { typename: type_name::<T>(), value: v.clone() }.fail()
    }
}

pub fn variant_to_float<T>(v: &JsonValue) -> Result<T>
where
    T: TryFromFloat64 + FromStr<Err = ParseFloatError>,
{
    if let Some(x) = v.as_f64()      { T::try_from_f64(x) }
    else if let Some(s) = v.as_str() { s.parse().context(FloatSnafu { repr: s }) }
    else {
        IncompatibleVariantTypesSnafu { typename: type_name::<T>(), value: v.clone() }.fail()
    }
}

pub fn variant_to_f128(v: &JsonValue) -> Result<f128> {

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
