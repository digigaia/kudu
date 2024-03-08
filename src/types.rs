pub mod name;
pub mod symbol;
pub mod asset;
pub mod crypto;

pub use name::{Name, InvalidName};
pub use symbol::{Symbol, InvalidSymbol, string_to_symbol_code, symbol_code_to_string};
pub use asset::{Asset, InvalidAsset};
pub use crypto::{PublicKey, PrivateKey, Signature, InvalidCryptoData};

use std::array::TryFromSliceError;
use std::num::{ParseFloatError, ParseIntError, TryFromIntError};
use std::convert::From;
use std::any::type_name;
use std::str::{FromStr, from_utf8, Utf8Error, ParseBoolError};

use bytemuck::{cast_ref, pod_read_unaligned};
use thiserror::Error;
use strum::{VariantNames, EnumDiscriminants, EnumString, Display};
use chrono::{NaiveDateTime, ParseError as ChronoParseError};
use num::{Integer, Signed, Unsigned};
use hex::FromHexError;

use super::{json, JsonValue, JsonError, ByteStream, StreamError, config};

const DATE_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.3f";

// see full list in: https://github.com/AntelopeIO/leap/blob/main/libraries/chain/abi_serializer.cpp#L89
#[derive(Debug, EnumDiscriminants, VariantNames)]
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
    VarInt32(i32),
    #[strum(serialize = "varuint32")]
    #[strum_discriminants(strum(serialize = "varuint32"))]
    VarUint32(u32),

    Float32(f32),
    Float64(f64),
    // Float128(??),

    Bytes(Vec<u8>),
    String(String),

    TimePoint(i64),
    TimePointSec(u32),
    BlockTimestampType(u32),

    Checksum160(Box<[u8; 20]>),
    Checksum256(Box<[u8; 32]>),
    Checksum512(Box<[u8; 64]>),

    PublicKey(Box<PublicKey>),
    PrivateKey(Box<PrivateKey>),
    Signature(Box<Signature>),

    Name(Name),
    SymbolCode(u64),
    Symbol(Symbol),
    Asset(Asset),
    ExtendedAsset(Box<(Asset, Name)>),

}


impl AntelopeValue {
    #[tracing::instrument]
    pub fn from_str(typename: AntelopeType, repr: &str) -> Result<Self, InvalidValue> {
        Ok(match typename {
            AntelopeType::Bool => Self::Bool(repr.parse()?),
            AntelopeType::Int8 => Self::Int8(repr.parse()?),
            AntelopeType::Int16 => Self::Int16(repr.parse()?),
            AntelopeType::Int32 => Self::Int32(repr.parse()?),
            AntelopeType::Int64 => Self::Int64(variant_to_int(&json!(repr))?),
            AntelopeType::Int128 => Self::Int128(variant_to_int(&json!(repr))?),
            AntelopeType::Uint8 => Self::Uint8(repr.parse()?),
            AntelopeType::Uint16 => Self::Uint16(repr.parse()?),
            AntelopeType::Uint32 => Self::Uint32(repr.parse()?),
            AntelopeType::Uint64 => Self::Uint64(variant_to_uint(&json!(repr))?),
            AntelopeType::Uint128 => Self::Uint128(variant_to_uint(&json!(repr))?),
            AntelopeType::VarInt32 => Self::VarInt32(repr.parse()?),
            AntelopeType::VarUint32 => Self::VarUint32(repr.parse()?),
            AntelopeType::Float32 => Self::Float32(repr.parse()?),
            AntelopeType::Float64 => Self::Float64(repr.parse()?),
            AntelopeType::Bytes => Self::Bytes(hex::decode(repr)?),
            AntelopeType::String => Self::String(repr.to_owned()),
            AntelopeType::TimePoint => Self::TimePoint(parse_date(repr)?.timestamp_micros()),
            AntelopeType::TimePointSec => Self::TimePointSec(parse_date(repr)?.timestamp() as u32),
            AntelopeType::BlockTimestampType => Self::BlockTimestampType(timestamp_to_block_slot(&parse_date(repr)?)),
            AntelopeType::Checksum160 => Self::Checksum160(hex_to_boxed_array(repr)?),
            AntelopeType::Checksum256 => Self::Checksum256(hex_to_boxed_array(repr)?),
            AntelopeType::Checksum512 => Self::Checksum512(hex_to_boxed_array(repr)?),
            AntelopeType::PublicKey => Self::PublicKey(Box::new(PublicKey::from_str(repr)?)),
            AntelopeType::PrivateKey => Self::PrivateKey(Box::new(PrivateKey::from_str(repr)?)),
            AntelopeType::Signature => Self::Signature(Box::new(Signature::from_str(repr)?)),
            AntelopeType::Name => Self::Name(Name::from_str(repr)?),
            AntelopeType::SymbolCode => Self::SymbolCode(string_to_symbol_code(repr.as_bytes())?),
            AntelopeType::Symbol => Self::Symbol(Symbol::from_str(repr)?),
            AntelopeType::Asset => Self::Asset(Asset::from_str(repr)?),
            AntelopeType::ExtendedAsset => Self::from_variant(typename, &serde_json::from_str(repr)?)?,
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
            Self::VarInt32(n) => json!(n),
            Self::VarUint32(n) => json!(n),
            Self::Float32(x) => json!(x),
            Self::Float64(x) => json!(x),
            Self::Bytes(b) => json!(hex::encode_upper(b)),
            Self::String(s) => json!(s),
            Self::TimePoint(t) => {
                let dt = NaiveDateTime::from_timestamp_micros(*t).unwrap();
                json!(format!("{}", dt.format(DATE_FORMAT)))
            },
            Self::TimePointSec(t) => {
                let dt = NaiveDateTime::from_timestamp_micros(*t as i64 * 1_000_000).unwrap();
                json!(format!("{}", dt.format(DATE_FORMAT)))
            },
            Self::BlockTimestampType(t) => {
                let dt = NaiveDateTime::from_timestamp_micros(
                    ((*t as i64 * config::BLOCK_INTERVAL_MS as i64) + config::BLOCK_TIMESTAMP_EPOCH as i64) * 1000
                ).unwrap();
                json!(format!("{}", dt.format(DATE_FORMAT)))
            }
            Self::Checksum160(c) => json!(hex::encode_upper(&c[..])),
            Self::Checksum256(c) => json!(hex::encode_upper(&c[..])),
            Self::Checksum512(c) => json!(hex::encode_upper(&c[..])),
            Self::PublicKey(sig) => json!(sig.to_string()),
            Self::PrivateKey(sig) => json!(sig.to_string()),
            Self::Signature(sig) => json!(sig.to_string()),
            Self::Name(name) => json!(name.to_string()),
            Self::SymbolCode(sym) => json!(symbol_code_to_string(*sym)),
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

    #[tracing::instrument]
    pub fn from_variant(typename: AntelopeType, v: &JsonValue) -> Result<Self, InvalidValue> {
        let incompatible_types = || {
            InvalidValue::IncompatibleVariantTypes(typename.to_string(), v.clone())
        };
        Ok(match typename {
            AntelopeType::Bool => Self::Bool(v.as_bool().ok_or_else(incompatible_types)?),
            AntelopeType::Int8 => Self::Int8(v.as_i64().ok_or_else(incompatible_types)?.try_into()?),
            AntelopeType::Int16 => Self::Int16(v.as_i64().ok_or_else(incompatible_types)?.try_into()?),
            AntelopeType::Int32 => Self::Int32(v.as_i64().ok_or_else(incompatible_types)?.try_into()?),
            AntelopeType::Int64 => Self::Int64(variant_to_int(v)?),
            AntelopeType::Int128 => Self::Int128(variant_to_int(v)?),
            AntelopeType::Uint8 => Self::Uint8(v.as_u64().ok_or_else(incompatible_types)?.try_into()?),
            AntelopeType::Uint16 => Self::Uint16(v.as_u64().ok_or_else(incompatible_types)?.try_into()?),
            AntelopeType::Uint32 => Self::Uint32(v.as_u64().ok_or_else(incompatible_types)?.try_into()?),
            AntelopeType::Uint64 => Self::Uint64(variant_to_uint(v)?),
            AntelopeType::Uint128 => Self::Uint128(variant_to_uint(v)?),
            AntelopeType::VarInt32 => Self::VarInt32(v.as_i64().ok_or_else(incompatible_types)?.try_into()?),
            AntelopeType::VarUint32 => Self::VarUint32(v.as_u64().ok_or_else(incompatible_types)?.try_into()?),
            AntelopeType::Float32 => Self::Float32(f64_to_f32(v.as_f64().ok_or_else(incompatible_types)?)?),
            AntelopeType::Float64 => Self::Float64(v.as_f64().ok_or_else(incompatible_types)?),
            AntelopeType::Bytes => Self::Bytes(hex::decode(v.as_str().ok_or_else(incompatible_types)?)?),
            AntelopeType::String => Self::String(v.as_str().ok_or_else(incompatible_types)?.to_owned()),
            AntelopeType::TimePoint => {
                let dt = parse_date(v.as_str().ok_or_else(incompatible_types)?)?;
                Self::TimePoint(dt.timestamp_micros())
            },
            AntelopeType::TimePointSec => {
                let dt = parse_date(v.as_str().ok_or_else(incompatible_types)?)?;
                Self::TimePointSec(dt.timestamp() as u32)
            },
            AntelopeType::BlockTimestampType => {
                let dt = parse_date(v.as_str().ok_or_else(incompatible_types)?)?;
                Self::BlockTimestampType(timestamp_to_block_slot(&dt))
            },
            AntelopeType::Checksum160 => Self::Checksum160(hex_to_boxed_array(v.as_str().ok_or_else(incompatible_types)?)?),
            AntelopeType::Checksum256 => Self::Checksum256(hex_to_boxed_array(v.as_str().ok_or_else(incompatible_types)?)?),
            AntelopeType::Checksum512 => Self::Checksum512(hex_to_boxed_array(v.as_str().ok_or_else(incompatible_types)?)?),
            AntelopeType::PublicKey |
            AntelopeType::PrivateKey |
            AntelopeType::Signature |
            AntelopeType::Name |
            AntelopeType::Symbol |
            AntelopeType::SymbolCode |
            AntelopeType::Asset => Self::from_str(typename, v.as_str().ok_or_else(incompatible_types)?)?,
            AntelopeType::ExtendedAsset => {
                let ea = v.as_object().ok_or_else(incompatible_types)?;
                Self::ExtendedAsset(Box::new((
                    Asset::from_str(ea["quantity"].as_str().ok_or_else(incompatible_types)?)?,
                    Name::from_str(ea["contract"].as_str().ok_or_else(incompatible_types)?)?,
                )))
            },
        })
    }

    pub fn to_bin(&self, stream: &mut ByteStream) {
        match self {
            Self::Bool(b) => stream.write_byte(match b {
                true => 1u8,
                false => 0u8,
            }),
            Self::Int8(n) => stream.write_byte(*n as u8), // FIXME: check that this is correct
            Self::Int16(n) => stream.write_bytes(cast_ref::<i16, [u8; 2]>(n)),
            Self::Int32(n) => stream.write_bytes(cast_ref::<i32, [u8; 4]>(n)),
            Self::Int64(n) => stream.write_bytes(cast_ref::<i64, [u8; 8]>(n)),
            Self::Int128(n) => stream.write_bytes(cast_ref::<i128, [u8; 16]>(n)),
            Self::Uint8(n) => stream.write_byte(*n),
            Self::Uint16(n) => stream.write_bytes(cast_ref::<u16, [u8; 2]>(n)),
            Self::Uint32(n) => stream.write_bytes(cast_ref::<u32, [u8; 4]>(n)),
            Self::Uint64(n) => stream.write_bytes(cast_ref::<u64, [u8; 8]>(n)),
            Self::Uint128(n) => stream.write_bytes(cast_ref::<u128, [u8; 16]>(n)),
            Self::VarInt32(n) => write_var_i32(stream, *n),
            Self::VarUint32(n) => write_var_u32(stream, *n),
            Self::Float32(x) => stream.write_bytes(cast_ref::<f32, [u8; 4]>(x)),
            Self::Float64(x) => stream.write_bytes(cast_ref::<f64, [u8; 8]>(x)),
            Self::Bytes(b) => {
                write_var_u32(stream, b.len() as u32);
                stream.write_bytes(&b[..]);
            },
            Self::String(s) => {
                write_var_u32(stream, s.len() as u32);
                stream.write_bytes(s.as_bytes());
            },
            Self::TimePoint(t) => stream.write_bytes(cast_ref::<i64, [u8; 8]>(t)),
            Self::TimePointSec(t) => stream.write_bytes(cast_ref::<u32, [u8; 4]>(t)),
            Self::BlockTimestampType(t) => stream.write_bytes(cast_ref::<u32, [u8; 4]>(t)),
            Self::Checksum160(c) => stream.write_bytes(&c[..]),
            Self::Checksum256(c) => stream.write_bytes(&c[..]),
            Self::Checksum512(c) => stream.write_bytes(&c[..]),
            Self::PublicKey(sig) => sig.encode(stream),
            Self::PrivateKey(sig) => sig.encode(stream),
            Self::Signature(sig) => sig.encode(stream),
            Self::Name(name) => name.encode(stream),
            Self::Symbol(sym) => sym.encode(stream),
            Self::SymbolCode(sym) => stream.write_bytes(cast_ref::<u64, [u8; 8]>(sym)),
            Self::Asset(asset) => asset.encode(stream),
            Self::ExtendedAsset(ea) => {
                let (ref quantity, ref contract) = **ea;
                quantity.encode(stream);
                contract.encode(stream);
            },
        }
    }

    #[tracing::instrument]
    pub fn from_bin(typename: AntelopeType, stream: &mut ByteStream) -> Result<Self, InvalidValue> {
        Ok(match typename {
            AntelopeType::Bool => match stream.read_byte()? {
                1 => Self::Bool(true),
                0 => Self::Bool(false),
                _ => { return Err(InvalidValue::InvalidData("cannot parse bool from stream".to_owned())); },
            },
            AntelopeType::Int8 => Self::Int8(stream.read_byte()? as i8),
            AntelopeType::Int16 => Self::Int16(pod_read_unaligned(stream.read_bytes(2)?)),
            AntelopeType::Int32 => Self::Int32(pod_read_unaligned(stream.read_bytes(4)?)),
            AntelopeType::Int64 => Self::Int64(pod_read_unaligned(stream.read_bytes(8)?)),
            AntelopeType::Int128 => Self::Int128(pod_read_unaligned(stream.read_bytes(16)?)),
            AntelopeType::Uint8 => Self::Uint8(stream.read_byte()?),
            AntelopeType::Uint16 => Self::Uint16(pod_read_unaligned(stream.read_bytes(2)?)),
            AntelopeType::Uint32 => Self::Uint32(pod_read_unaligned(stream.read_bytes(4)?)),
            AntelopeType::Uint64 => Self::Uint64(pod_read_unaligned(stream.read_bytes(8)?)),
            AntelopeType::Uint128 => Self::Uint128(pod_read_unaligned(stream.read_bytes(16)?)),
            AntelopeType::VarInt32 => Self::VarInt32(read_var_i32(stream)?),
            AntelopeType::VarUint32 => Self::VarUint32(read_var_u32(stream)?),
            AntelopeType::Float32 => Self::Float32(pod_read_unaligned(stream.read_bytes(4)?)),
            AntelopeType::Float64 => Self::Float64(pod_read_unaligned(stream.read_bytes(8)?)),
            AntelopeType::Bytes => Self::Bytes(read_bytes(stream)?),
            AntelopeType::String => Self::String(read_str(stream)?.to_owned()),
            AntelopeType::TimePoint => Self::TimePoint(pod_read_unaligned(stream.read_bytes(8)?)),
            AntelopeType::TimePointSec => Self::TimePointSec(pod_read_unaligned(stream.read_bytes(4)?)),
            AntelopeType::BlockTimestampType => Self::BlockTimestampType(pod_read_unaligned(stream.read_bytes(4)?)),
            AntelopeType::Checksum160 => Self::Checksum160(Box::new(stream.read_bytes(20)?.try_into().unwrap())),
            AntelopeType::Checksum256 => Self::Checksum256(Box::new(stream.read_bytes(32)?.try_into().unwrap())),
            AntelopeType::Checksum512 => Self::Checksum512(Box::new(stream.read_bytes(64)?.try_into().unwrap())),
            AntelopeType::PublicKey => Self::PublicKey(Box::new(PublicKey::decode(stream)?)),
            AntelopeType::PrivateKey => Self::PrivateKey(Box::new(PrivateKey::decode(stream)?)),
            AntelopeType::Signature => Self::Signature(Box::new(Signature::decode(stream)?)),
            AntelopeType::Name => Self::Name(Name::decode(stream)?),
            AntelopeType::Symbol => Self::Symbol(Symbol::decode(stream)?),
            AntelopeType::SymbolCode => Self::SymbolCode(pod_read_unaligned(stream.read_bytes(8)?)),
            AntelopeType::Asset => Self::Asset(Asset::decode(stream)?),
            AntelopeType::ExtendedAsset => Self::ExtendedAsset(Box::new((
                Asset::decode(stream)?,
                Name::decode(stream)?,
            ))),
        })
    }
}


fn timestamp_to_block_slot(dt: &NaiveDateTime) -> u32 {
    let ms_since_epoch = (dt.timestamp_micros() / 1000) as u64 - config::BLOCK_TIMESTAMP_EPOCH;
    let result = ms_since_epoch / (config::BLOCK_INTERVAL_MS as u64);
    result as u32
}


fn f64_to_f32(x: f64) -> Result<f32, InvalidValue> {
    let result = x as f32;
    if result.is_finite() { Ok(result) } else { Err(InvalidValue::FloatPrecision) }
}

fn write_var_u32(stream: &mut ByteStream, n: u32) {
    let mut n = n;
    loop {
        if n >> 7 != 0 {
            stream.write_byte((0x80 | (n & 0x7f)) as u8);
            n >>= 7
        }
        else {
            stream.write_byte(n as u8);
            break;
        }
    }
}

fn write_var_i32(stream: &mut ByteStream, n: i32) {
    let unsigned = ((n as u32) << 1) ^ ((n >> 31) as u32);
    write_var_u32(stream, unsigned)
}

fn read_var_u32(stream: &mut ByteStream) -> Result<u32, InvalidValue> {
    let mut offset = 0;
    let mut result = 0;
    loop {
        let byte = stream.read_byte()?;
        result |= (byte as u32 & 0x7F) << offset;
        offset += 7;
        if (byte & 0x80) == 0 { break; }
        if offset >= 32 {
            return Err(InvalidValue::InvalidData(
                "varint too long to fit in u32".to_owned()
            ));
        }
    }
    Ok(result)
}

fn read_var_i32(stream: &mut ByteStream) -> Result<i32, InvalidValue> {
    let n = read_var_u32(stream)?;
    Ok(match n & 1 {
        0 => n >> 1,
        _ => ((!n) >> 1) | 0x8000_0000,
    } as i32)
}

fn read_bytes(stream: &mut ByteStream) -> Result<Vec<u8>, InvalidValue> {
    let len = read_var_u32(stream)? as usize;
    Ok(Vec::from(stream.read_bytes(len)?))
}

fn read_str(stream: &mut ByteStream) -> Result<&str, InvalidValue> {
    let len = read_var_u32(stream)? as usize;
    Ok(from_utf8(stream.read_bytes(len)?)?)
}


impl From<AntelopeValue> for bool {
    fn from(n: AntelopeValue) -> bool {
        match n {
            AntelopeValue::Bool(b) => b,
            _ => todo!(),
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
            AntelopeValue::Uint32(n) => n as i32,
            AntelopeValue::VarUint32(n) => n as i32,
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
            AntelopeValue::Int64(n) => n as usize,
            AntelopeValue::Uint8(n) => n as usize,
            AntelopeValue::Uint16(n) => n as usize,
            AntelopeValue::Uint32(n) => n as usize,
            AntelopeValue::Uint64(n) => n as usize,
            AntelopeValue::VarInt32(n) => n as usize,
            AntelopeValue::VarUint32(n) => n as usize,
            _ => return Err(InvalidValue::InvalidData( format!("cannot convert {:?} to usize", n))),
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
            AntelopeValue::Uint64(n) => n as i64,
            AntelopeValue::VarInt32(n) => n as i64,
            AntelopeValue::VarUint32(n) => n as i64,
            _ => return Err(InvalidValue::InvalidData( format!("cannot convert {:?} to i64", n))),
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
            _ => return Err(InvalidValue::InvalidData( format!("cannot convert {:?} to string", s))),
        })
    }
}


fn variant_to_int<T>(v: &JsonValue) -> Result<T, InvalidValue>
where
    T: Integer + Signed + FromStr + From<i64>,
    InvalidValue: From<<T as FromStr>::Err>,
{
    match v {
        v if v.is_i64() => Ok(v.as_i64().unwrap().into()),
        v if v.is_string() => Ok(v.as_str().unwrap().parse()?),
        _ => Err(InvalidValue::IncompatibleVariantTypes(type_name::<T>().to_owned(), v.clone())),
    }
}

fn variant_to_uint<T>(v: &JsonValue) -> Result<T, InvalidValue>
where
    T: Integer + Unsigned + FromStr + From<u64>,
    InvalidValue: From<<T as FromStr>::Err>,
{
    match v {
        v if v.is_u64() => Ok(v.as_u64().unwrap().into()),
        v if v.is_string() => Ok(v.as_str().unwrap().parse()?),
        _ => Err(InvalidValue::IncompatibleVariantTypes(type_name::<T>().to_owned(), v.clone())),
    }
}

/// return a date in microseconds
fn parse_date(s: &str) -> Result<NaiveDateTime, InvalidValue> {
    Ok(NaiveDateTime::parse_from_str(s, DATE_FORMAT)?)
}

pub fn hex_to_boxed_array<const N: usize>(s: &str) -> Result<Box<[u8; N]>, FromHexError> {
    let mut result = [0_u8; N];
    hex::decode_to_slice(s, &mut result)?;
    Ok(Box::new(result))
}


#[cfg(test)]
mod tests {
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


#[derive(Error, Debug)]
pub enum InvalidValue {
    #[error(r#"cannot convert given variant {1} to Antelope type "{0}""#)]
    IncompatibleVariantTypes(String, JsonValue),

    #[error("invalid bool")]
    Bool(#[from] ParseBoolError),

    #[error("invalid integer")]
    Int(#[from] ParseIntError),

    #[error("integer out of range")]
    IntPrecision(#[from] TryFromIntError),

    #[error("invalid float")]
    Float(#[from] ParseFloatError),

    #[error("float out of range")]
    FloatPrecision,

    #[error("invalid name")]
    Name(#[from] InvalidName),

    #[error("invalid symbol")]
    Symbol(#[from] InvalidSymbol),

    #[error("invalid asset")]
    Asset(#[from] InvalidAsset),

    #[error("invalid hex representation")]
    FromHex(#[from] FromHexError),

    #[error("invalid crypto data")]
    CryptoData(#[from] InvalidCryptoData),

    #[error("stream error")]
    StreamError(#[from] StreamError),

    #[error("cannot parse bytes as UTF-8")]
    Utf8Error(#[from] Utf8Error),

    #[error("cannot parse JSON string")]
    JsonParseError(#[from] JsonError),

    #[error("cannot parse date/time")]
    DateTimeParseError(#[from] ChronoParseError),

    #[error("cannot parse typename")]
    TypenameParseError(#[from] strum::ParseError),

    #[error("incorrect array size for checksum")]
    IncorrectChecksumSize(#[from] TryFromSliceError),

    #[error("{0}")]
    InvalidData(String),  // acts as a generic error type with a given message
}
