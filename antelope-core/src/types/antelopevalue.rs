use std::array::TryFromSliceError;
use std::convert::From;
use std::num::TryFromIntError;

use std::str::{ParseBoolError, Utf8Error};

use chrono::ParseError as ChronoParseError;
use hex::FromHexError;
use snafu::{Snafu, IntoError, ResultExt, OptionExt};
use strum::{Display, AsRefStr, EnumDiscriminants, EnumString, VariantNames};
use tracing::instrument;

#[cfg(feature = "float128")]
use f128::f128;

use antelope_macros::with_location;

use crate::{
    json, JsonError, JsonValue,
    impl_auto_error_conversion,
};

use crate::types::{self,
    VarInt32, VarUint32, Bytes,
    TimePoint, TimePointSec, BlockTimestampType,
    Asset, InvalidAsset,
    Name, InvalidName,
    Symbol, SymbolCode, InvalidSymbol,
    Checksum160, Checksum256, Checksum512,
    PublicKey, PrivateKey, Signature, InvalidCryptoData,
};

use crate::convert::{
    variant_to_int, variant_to_uint, variant_to_float, variant_to_str,
    str_to_int, str_to_float, hex_to_boxed_array,
    ConversionError,
};
#[cfg(feature = "float128")]
use crate::convert::str_to_f128;

// see full list in: https://github.com/AntelopeIO/leap/blob/main/libraries/chain/abi_serializer.cpp#L89
#[derive(Debug, AsRefStr, EnumDiscriminants, VariantNames, Clone, PartialEq)]
#[strum(serialize_all = "snake_case")]
#[strum_discriminants(name(AntelopeType))]
#[strum_discriminants(derive(Display, EnumString))]
#[strum_discriminants(strum(serialize_all = "snake_case"))]
pub enum AntelopeValue {
    Bool(bool),

    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    Int128(i128),

    Uint8(u8),
    Uint16(u16),
    Uint32(u32),
    Uint64(u64),
    Uint128(u128),

    #[strum(serialize = "varint32")]
    #[strum_discriminants(strum(serialize = "varint32"))]
    VarInt32(VarInt32),
    #[strum(serialize = "varuint32")]
    #[strum_discriminants(strum(serialize = "varuint32"))]
    VarUint32(VarUint32),

    Float32(f32),
    Float64(f64),
    #[cfg(feature = "float128")]
    Float128(f128),

    Bytes(Bytes),
    String(types::String),

    TimePoint(TimePoint),
    TimePointSec(TimePointSec),
    BlockTimestampType(BlockTimestampType),

    Checksum160(Box<Checksum160>),
    Checksum256(Box<Checksum256>),
    Checksum512(Box<Checksum512>),

    PublicKey(Box<PublicKey>),
    PrivateKey(Box<PrivateKey>),
    Signature(Box<Signature>),

    Name(Name),
    SymbolCode(SymbolCode),
    Symbol(Symbol),
    Asset(Asset),
    ExtendedAsset(Box<(Asset, Name)>),
}


impl From<AntelopeType> for String {
    fn from(ty: AntelopeType) -> String {
        ty.to_string()
    }
}

impl AntelopeValue {
    #[instrument]
    pub fn from_str(typename: AntelopeType, repr: &str) -> Result<Self, InvalidValue> {
        Ok(match typename {
            AntelopeType::Bool => Self::Bool(repr.parse().context(BoolSnafu)?),
            AntelopeType::Int8 => Self::Int8(str_to_int(repr)?),
            AntelopeType::Int16 => Self::Int16(str_to_int(repr)?),
            AntelopeType::Int32 => Self::Int32(str_to_int(repr)?),
            AntelopeType::Int64 => Self::Int64(str_to_int(repr)?),
            AntelopeType::Int128 => Self::Int128(str_to_int(repr)?),
            AntelopeType::Uint8 => Self::Uint8(str_to_int(repr)?),
            AntelopeType::Uint16 => Self::Uint16(str_to_int(repr)?),
            AntelopeType::Uint32 => Self::Uint32(str_to_int(repr)?),
            AntelopeType::Uint64 => Self::Uint64(str_to_int(repr)?),
            AntelopeType::Uint128 => Self::Uint128(str_to_int(repr)?),
            AntelopeType::VarInt32 => Self::VarInt32(str_to_int::<i32>(repr)?.into()),
            AntelopeType::VarUint32 => Self::VarUint32(str_to_int::<u32>(repr)?.into()),
            AntelopeType::Float32 => Self::Float32(str_to_float(repr)?),
            AntelopeType::Float64 => Self::Float64(str_to_float(repr)?),
            #[cfg(feature = "float128")]
            AntelopeType::Float128 => Self::Float128(str_to_f128(repr)?),
            AntelopeType::Bytes => Self::Bytes(hex::decode(repr).context(FromHexSnafu)?),
            AntelopeType::String => Self::String(repr.to_owned()),
            AntelopeType::TimePoint => Self::TimePoint(TimePoint::from_str(repr)?),
            AntelopeType::TimePointSec => Self::TimePointSec(TimePointSec::from_str(repr)?),
            AntelopeType::BlockTimestampType => Self::BlockTimestampType(BlockTimestampType::from_str(repr)?),
            AntelopeType::Checksum160 => Self::Checksum160(hex_to_boxed_array(repr).context(FromHexSnafu)?),
            AntelopeType::Checksum256 => Self::Checksum256(hex_to_boxed_array(repr).context(FromHexSnafu)?),
            AntelopeType::Checksum512 => Self::Checksum512(hex_to_boxed_array(repr).context(FromHexSnafu)?),
            AntelopeType::PublicKey => Self::PublicKey(Box::new(PublicKey::from_str(repr).context(CryptoDataSnafu)?)),
            AntelopeType::PrivateKey => Self::PrivateKey(Box::new(PrivateKey::from_str(repr).context(CryptoDataSnafu)?)),
            AntelopeType::Signature => Self::Signature(Box::new(Signature::from_str(repr).context(CryptoDataSnafu)?)),
            AntelopeType::Name => Self::Name(Name::from_str(repr).context(NameSnafu)?),
            AntelopeType::SymbolCode => Self::SymbolCode(SymbolCode::from_str(repr).context(SymbolSnafu)?),
            AntelopeType::Symbol => Self::Symbol(Symbol::from_str(repr).context(SymbolSnafu)?),
            AntelopeType::Asset => Self::Asset(Asset::from_str(repr).context(AssetSnafu { repr })?),
            AntelopeType::ExtendedAsset => Self::from_variant(typename, &serde_json::from_str(repr).context(JsonParseSnafu)?)?,
            // _ => { return Err(InvalidValue::InvalidType(typename.to_string())); },
        })
    }

    pub fn to_variant(&self) -> JsonValue {
        match self {
            Self::Bool(b) => json!(b),
            Self::Int8(n) => json!(n),
            Self::Int16(n) => json!(n),
            Self::Int32(n) => json!(n),
            Self::Int64(n) => json!(n.to_string()),
            Self::Int128(n) => json!(n.to_string()),
            Self::Uint8(n) => json!(n),
            Self::Uint16(n) => json!(n),
            Self::Uint32(n) => json!(n),
            Self::Uint64(n) => json!(n.to_string()),
            Self::Uint128(n) => json!(n.to_string()),
            Self::VarInt32(n) => json!(i32::from(*n)),
            Self::VarUint32(n) => json!(u32::from(*n)),
            Self::Float32(x) => json!(x),
            Self::Float64(x) => json!(x),
            #[cfg(feature = "float128")]
            Self::Float128(x) => json!(x.to_string()),
            Self::Bytes(b) => json!(hex::encode_upper(b)),
            Self::String(s) => json!(s),
            Self::TimePoint(t) => t.to_json(),
            Self::TimePointSec(t) => t.to_json(),
            Self::BlockTimestampType(t) => t.to_json(),
            Self::Checksum160(c) => json!(hex::encode_upper(&c[..])),
            Self::Checksum256(c) => json!(hex::encode_upper(&c[..])),
            Self::Checksum512(c) => json!(hex::encode_upper(&c[..])),
            Self::PublicKey(sig) => json!(sig.to_string()),
            Self::PrivateKey(sig) => json!(sig.to_string()),
            Self::Signature(sig) => json!(sig.to_string()),
            Self::Name(name) => json!(name.to_string()),
            Self::SymbolCode(sym) => json!(sym.to_string()),
            Self::Symbol(sym) => json!(sym.to_string()),
            Self::Asset(asset) => json!(asset.to_string()),
            Self::ExtendedAsset(ea) => {
                let (ref quantity, ref contract) = **ea;
                json!({
                    "quantity": quantity,
                    "contract": contract,
                })
            },
        }
    }

    #[instrument]
    pub fn from_variant(typename: AntelopeType, v: &JsonValue) -> Result<Self, InvalidValue> {
        let incompatible_types = || {
            IncompatibleVariantTypesSnafu { typename, value: v.clone() }
        };

        Ok(match typename {
            AntelopeType::Bool => Self::Bool(v.as_bool().with_context(incompatible_types)?),
            AntelopeType::Int8 => Self::Int8(variant_to_int(v)?),
            AntelopeType::Int16 => Self::Int16(variant_to_int(v)?),
            AntelopeType::Int32 => Self::Int32(variant_to_int(v)?),
            AntelopeType::Int64 => Self::Int64(variant_to_int(v)?),
            AntelopeType::Int128 => Self::Int128(variant_to_int(v)?),
            AntelopeType::Uint8 => Self::Uint8(variant_to_uint(v)?),
            AntelopeType::Uint16 => Self::Uint16(variant_to_uint(v)?),
            AntelopeType::Uint32 => Self::Uint32(variant_to_uint(v)?),
            AntelopeType::Uint64 => Self::Uint64(variant_to_uint(v)?),
            AntelopeType::Uint128 => Self::Uint128(variant_to_uint(v)?),
            AntelopeType::VarInt32 => Self::VarInt32(variant_to_int::<i32>(v)?.into()),
            AntelopeType::VarUint32 => Self::VarUint32(variant_to_uint::<u32>(v)?.into()),
            AntelopeType::Float32 => Self::Float32(variant_to_float(v)?),
            AntelopeType::Float64 => Self::Float64(variant_to_float(v)?),
            #[cfg(feature = "float128")]
            AntelopeType::Float128 => Self::Float128(variant_to_f128(v)?),
            AntelopeType::Bytes => Self::Bytes(hex::decode(
                v.as_str().with_context(incompatible_types)?
            ).context(FromHexSnafu)?),
            AntelopeType::String => Self::String(v.as_str().with_context(incompatible_types)?.to_owned()),
            AntelopeType::TimePoint => {
                let repr = v.as_str().with_context(incompatible_types)?;
                Self::TimePoint(TimePoint::from_str(repr)?)
            },
            AntelopeType::TimePointSec => {
                let repr = v.as_str().with_context(incompatible_types)?;
                Self::TimePointSec(TimePointSec::from_str(repr)?)
            },
            AntelopeType::BlockTimestampType => {
                let repr = v.as_str().with_context(incompatible_types)?;
                Self::BlockTimestampType(BlockTimestampType::from_str(repr)?)
            },
            AntelopeType::Checksum160 => {
                Self::Checksum160(hex_to_boxed_array(v.as_str().with_context(incompatible_types)?)
                                  .context(FromHexSnafu)?)
            },
            AntelopeType::Checksum256 => {
                Self::Checksum256(hex_to_boxed_array(v.as_str().with_context(incompatible_types)?)
                                  .context(FromHexSnafu)?)
            },
            AntelopeType::Checksum512 => {
                Self::Checksum512(hex_to_boxed_array(v.as_str().with_context(incompatible_types)?)
                                  .context(FromHexSnafu)?)
            },
            AntelopeType::PublicKey
            | AntelopeType::PrivateKey
            | AntelopeType::Signature
            | AntelopeType::Name
            | AntelopeType::Symbol
            | AntelopeType::SymbolCode
            | AntelopeType::Asset => Self::from_str(typename, v.as_str().with_context(incompatible_types)?)?,
            AntelopeType::ExtendedAsset => {
                let ea = v.as_object().with_context(incompatible_types)?;
                let qty = variant_to_str(&ea["quantity"])?;
                Self::ExtendedAsset(Box::new((
                    Asset::from_str(qty).context(AssetSnafu { repr: qty })?,
                    Name::from_str(ea["contract"].as_str().with_context(incompatible_types)?).context(NameSnafu)?,
                )))
            },
        })
    }
}



impl From<AntelopeValue> for bool {
    fn from(n: AntelopeValue) -> bool {
        match n {
            AntelopeValue::Bool(b) => b,
            _ => unimplemented!(),
        }
    }
}

impl From<AntelopeValue> for i32 {
    fn from(n: AntelopeValue) -> i32 {
        match n {
            AntelopeValue::Int8(n) => n as i32,
            AntelopeValue::Int16(n) => n as i32,
            AntelopeValue::Int32(n) => n,
            AntelopeValue::Uint8(n) => n as i32,
            AntelopeValue::Uint16(n) => n as i32,
            AntelopeValue::Uint32(n) => n.try_into().unwrap(),
            AntelopeValue::VarUint32(n) => u32::from(n).try_into().unwrap(),
            _ => todo!(),
        }
    }
}

impl TryFrom<AntelopeValue> for usize {
    type Error = InvalidValue;

    fn try_from(n: AntelopeValue) -> Result<usize, Self::Error> {
        Ok(match n {
            AntelopeValue::Int8(n) => n as usize,
            AntelopeValue::Int16(n) => n as usize,
            AntelopeValue::Int32(n) => n as usize,
            AntelopeValue::Int64(n) => n.try_into().context(IntConversionSnafu { size: "usize" })?,
            AntelopeValue::Uint8(n) => n as usize,
            AntelopeValue::Uint16(n) => n as usize,
            AntelopeValue::Uint32(n) => n as usize,
            AntelopeValue::Uint64(n) => n as usize,
            AntelopeValue::VarInt32(n) => i32::from(n).try_into().context(IntConversionSnafu { size: "usize" })?,
            AntelopeValue::VarUint32(n) => u32::from(n) as usize,
            _ => return InvalidDataSnafu { msg: (format!("cannot convert {:?} to usize", n)) }.fail(),
        })
    }
}

impl TryFrom<AntelopeValue> for i64 {
    type Error = InvalidValue;

    fn try_from(n: AntelopeValue) -> Result<i64, Self::Error> {
        Ok(match n {
            AntelopeValue::Int8(n) => n as i64,
            AntelopeValue::Int16(n) => n as i64,
            AntelopeValue::Int32(n) => n as i64,
            AntelopeValue::Int64(n) => n,
            AntelopeValue::Uint8(n) => n as i64,
            AntelopeValue::Uint16(n) => n as i64,
            AntelopeValue::Uint32(n) => n as i64,
            AntelopeValue::Uint64(n) => n.try_into().context(IntConversionSnafu { size: "i64" })?,
            AntelopeValue::VarInt32(n) => i32::from(n) as i64,
            AntelopeValue::VarUint32(n) => u32::from(n) as i64,
            _ => return InvalidDataSnafu { msg: format!("cannot convert {:?} to i64", n) }.fail(),
        })
    }
}


impl TryFrom<AntelopeValue> for String {
    type Error = InvalidValue;

    fn try_from(s: AntelopeValue) -> Result<String, Self::Error> {
        Ok(match s {
            AntelopeValue::String(s) => s,
            AntelopeValue::Name(s) => s.to_string(),
            AntelopeValue::Symbol(s) => s.to_string(),
            AntelopeValue::Asset(s) => s.to_string(),
            _ => return InvalidDataSnafu { msg: format!("cannot convert {:?} to string", s) }.fail(),
        })
    }
}



#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use color_eyre::eyre::Report;

    use super::*;

    #[test]
    fn test_conversion() -> Result<(), Report> {
        let n = json!(23);
        let n = AntelopeValue::from_variant(AntelopeType::Int8, &n)?;
        println!("n = {n:?}");

        Ok(())
    }

    #[test]
    fn test_antelope_types() -> Result<(), Report> {
        assert_eq!(AntelopeType::from_str("int8")?, AntelopeType::Int8);
        assert_eq!(AntelopeType::from_str("varint32")?, AntelopeType::VarInt32);

        Ok(())
    }
}


// Note: serde_json doesn't support "natively" 128-bit integer types
//  see: https://github.com/serde-rs/json/issues/846

#[with_location]
#[derive(Debug, Snafu)]
pub enum InvalidValue {
    #[snafu(display(r#"cannot convert given variant {value} to Antelope type "{typename}""#))]
    IncompatibleVariantTypes {
        typename: String,
        value: Box<JsonValue>,
    },

    #[snafu(display("invalid bool"))]
    Bool { source: ParseBoolError },

    #[snafu(display("invalid conversion"))]
    Conversion { source: ConversionError },

    #[snafu(display("invalid name"))]
    Name { source: InvalidName },

    #[snafu(display("invalid symbol"))]
    Symbol { source: InvalidSymbol },

    #[snafu(display("invalid asset"))]
    Asset {
        repr: String,
        source: InvalidAsset,
    },

    #[snafu(display("invalid hex representation"))]
    FromHex { source: FromHexError },

    #[snafu(display("invalid crypto data"))]
    CryptoData { source: InvalidCryptoData },

    #[snafu(display("cannot parse bytes as UTF-8"))]
    Utf8Error { source: Utf8Error },

    #[snafu(display("cannot parse JSON string"))]
    JsonParse { source: JsonError },

    #[snafu(display("cannot parse date/time"))]
    DateTimeParse { source: ChronoParseError },

    #[snafu(display("cannot parse typename"))]
    TypenameParseError { source: strum::ParseError },

    #[snafu(display("incorrect array size for checksum"))]
    IncorrectChecksumSize { source: TryFromSliceError },

    #[snafu(display("cannot fit given number into integer type: {size}"))]
    IntConversionError {
        size: &'static str,
        source: TryFromIntError
    },

    #[snafu(display("{msg}"))]
    InvalidData { msg: String },  // acts as a generic error type with a given message
}

impl_auto_error_conversion!(ConversionError, InvalidValue, ConversionSnafu);
impl_auto_error_conversion!(FromHexError, InvalidValue, FromHexSnafu);
impl_auto_error_conversion!(strum::ParseError, InvalidValue, TypenameParseSnafu);
impl_auto_error_conversion!(JsonError, InvalidValue, JsonParseSnafu);
impl_auto_error_conversion!(Utf8Error, InvalidValue, Utf8Snafu);
impl_auto_error_conversion!(ChronoParseError, InvalidValue, DateTimeParseSnafu);
