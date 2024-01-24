pub mod name;
pub mod symbol;
pub mod asset;
pub mod crypto;

pub use name::{Name, InvalidName};
pub use symbol::{Symbol, InvalidSymbol, string_to_symbol_code, symbol_code_to_string};
pub use asset::{Asset, InvalidAsset};
pub use crypto::{Signature, InvalidSignature};

use std::array::TryFromSliceError;
use std::num::{ParseFloatError, ParseIntError, TryFromIntError};
use std::str::{from_utf8, Utf8Error, ParseBoolError};

use bytemuck::{cast_ref, pod_read_unaligned};
use serde_json::{json, Value, Error as JsonError};
use thiserror::Error;
use strum::EnumVariantNames;
use chrono::{NaiveDateTime, ParseError as ChronoParseError};

use super::{ByteStream, StreamError, bin_to_hex, hex_to_bin, hex_to_boxed_array, config};

const DATE_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.3f";

// see full list in: https://github.com/AntelopeIO/leap/blob/main/libraries/chain/abi_serializer.cpp#L89
#[derive(Debug, EnumVariantNames)]
#[strum(serialize_all = "snake_case")]
pub enum AntelopeType {
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
    VarInt32(i32),
    #[strum(serialize = "varuint32")]
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

    Name(Name),
    SymbolCode(u64),
    Symbol(Symbol),
    Asset(Asset),

    Signature(Signature),
}


impl AntelopeType {
    pub fn from_str(typename: &str, repr: &str) -> Result<Self, InvalidValue> {
        Ok(match typename {
            "bool" => Self::Bool(repr.parse()?),
            "int8" => Self::Int8(repr.parse()?),
            "int16" => Self::Int16(repr.parse()?),
            "int32" => Self::Int32(repr.parse()?),
            "int64" => Self::Int64(repr.parse()?),
            "int128" => Self::Int128(repr.parse()?),
            "uint8" => Self::Uint8(repr.parse()?),
            "uint16" => Self::Uint16(repr.parse()?),
            "uint32" => Self::Uint32(repr.parse()?),
            "uint64" => Self::Uint64(repr.parse()?),
            "uint128" => Self::Uint128(repr.parse()?),
            "varint32" => Self::VarInt32(repr.parse()?),
            "varuint32" => Self::VarUint32(repr.parse()?),
            "float32" => Self::Float32(repr.parse()?),
            "float64" => Self::Float64(repr.parse()?),
            "bytes" => Self::Bytes(hex_to_bin(repr)?),
            "string" => Self::String(repr.to_owned()),
            "time_point" => Self::TimePoint(parse_date(repr)?.timestamp_micros()),
            "time_point_sec" => Self::TimePointSec(parse_date(repr)?.timestamp() as u32),
            "block_timestamp_type" => Self::BlockTimestampType(timestamp_to_block_slot(&parse_date(repr)?)),
            "checksum160" => Self::Checksum160(hex_to_boxed_array(repr)?),
            "checksum256" => Self::Checksum256(hex_to_boxed_array(repr)?),
            "checksum512" => Self::Checksum512(hex_to_boxed_array(repr)?),
            "name" => Self::Name(Name::from_str(repr)?),
            "symbol_code" => Self::SymbolCode(string_to_symbol_code(repr.as_bytes())?),
            "symbol" => Self::Symbol(Symbol::from_str(repr)?),
            "asset" => Self::Asset(Asset::from_str(repr)?),
            "signature" => Self::Signature(Signature::from_str(repr)?),
            _ => { return Err(InvalidValue::InvalidType(typename.to_owned())); },
        })
    }

    pub fn to_variant(&self) -> Value {
        match self {
            Self::Bool(b) => json!(b),
            Self::Int8(n) => json!(n),
            Self::Int16(n) => json!(n),
            Self::Int32(n) => json!(n),
            Self::Int64(n) => json!(n),
            Self::Int128(n) => json!(n.to_string()),
            Self::Uint8(n) => json!(n),
            Self::Uint16(n) => json!(n),
            Self::Uint32(n) => json!(n),
            Self::Uint64(n) => json!(n),
            Self::Uint128(n) => json!(n.to_string()),
            Self::VarInt32(n) => json!(n),
            Self::VarUint32(n) => json!(n),
            Self::Float32(x) => json!(x),
            Self::Float64(x) => json!(x),
            Self::Bytes(b) => json!(bin_to_hex(b).to_ascii_uppercase()),
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
            Self::Checksum160(c) => json!(bin_to_hex(&c[..])),
            Self::Checksum256(c) => json!(bin_to_hex(&c[..])),
            Self::Checksum512(c) => json!(bin_to_hex(&c[..])),
            Self::Name(name) => json!(name.to_string()),
            Self::SymbolCode(sym) => json!(symbol_code_to_string(*sym)),
            Self::Symbol(sym) => json!(sym.to_string()),
            Self::Asset(asset) => json!(asset.to_string()),
            Self::Signature(sig) => json!(sig.to_string()),
        }
    }

    pub fn from_variant(typename: &str, v: &Value) -> Result<Self, InvalidValue> {
        let incompatible_types = || {
            InvalidValue::IncompatibleVariantTypes(typename.to_owned(), v.clone())
        };
        Ok(match typename {
            "bool" => Self::Bool(v.as_bool().ok_or_else(incompatible_types)?),
            "int8" => Self::Int8(v.as_i64().ok_or_else(incompatible_types)?.try_into()?),
            "int16" => Self::Int16(v.as_i64().ok_or_else(incompatible_types)?.try_into()?),
            "int32" => Self::Int32(v.as_i64().ok_or_else(incompatible_types)?.try_into()?),
            "int64" => Self::Int64(v.as_i64().ok_or_else(incompatible_types)?),
            "int128" => Self::Int128(variant_to_i128(v)?),
            "uint8" => Self::Uint8(v.as_u64().ok_or_else(incompatible_types)?.try_into()?),
            "uint16" => Self::Uint16(v.as_u64().ok_or_else(incompatible_types)?.try_into()?),
            "uint32" => Self::Uint32(v.as_u64().ok_or_else(incompatible_types)?.try_into()?),
            "uint64" => Self::Uint64(v.as_u64().ok_or_else(incompatible_types)?),
            "uint128" => Self::Uint128(variant_to_u128(v)?),
            "varint32" => Self::VarInt32(v.as_i64().ok_or_else(incompatible_types)?.try_into()?),
            "varuint32" => Self::VarUint32(v.as_u64().ok_or_else(incompatible_types)?.try_into()?),
            "float32" => Self::Float32(f64_to_f32(v.as_f64().ok_or_else(incompatible_types)?)?),
            "float64" => Self::Float64(v.as_f64().ok_or_else(incompatible_types)?),
            "bytes" => Self::Bytes(hex_to_bin(v.as_str().ok_or_else(incompatible_types)?)?),
            "string" => Self::String(v.as_str().ok_or_else(incompatible_types)?.to_owned()),
            "time_point" => {
                let dt = parse_date(v.as_str().ok_or_else(incompatible_types)?)?;
                Self::TimePoint(dt.timestamp_micros())
            },
            "time_point_sec" => {
                let dt = parse_date(v.as_str().ok_or_else(incompatible_types)?)?;
                Self::TimePointSec(dt.timestamp() as u32)
            },
            "block_timestamp_type" => {
                let dt = parse_date(v.as_str().ok_or_else(incompatible_types)?)?;
                Self::BlockTimestampType(timestamp_to_block_slot(&dt))
            },
            "checksum160" => Self::Checksum160(hex_to_boxed_array(v.as_str().ok_or_else(incompatible_types)?)?),
            "checksum256" => Self::Checksum256(hex_to_boxed_array(v.as_str().ok_or_else(incompatible_types)?)?),
            "checksum512" => Self::Checksum512(hex_to_boxed_array(v.as_str().ok_or_else(incompatible_types)?)?),
            "name" => Self::from_str("name", v.as_str().ok_or_else(incompatible_types)?)?,
            "symbol" => Self::from_str("symbol", v.as_str().ok_or_else(incompatible_types)?)?,
            "symbol_code" => Self::from_str("symbol_code", v.as_str().ok_or_else(incompatible_types)?)?,
            "asset" => Self::from_str("asset", v.as_str().ok_or_else(incompatible_types)?)?,
            "signature" => Self::from_str("signature", v.as_str().ok_or_else(incompatible_types)?)?,
            _ => { return Err(InvalidValue::InvalidType(typename.to_owned())); },
        })
    }

    pub fn to_bin(&self, stream: &mut ByteStream) {
        match self {
            Self::Bool(b) => stream.write_byte(match b {
                true => 1u8,
                false => 0u8,
            }),
            Self::Int8(n) => stream.write_byte(*n as u8), // FIXME: check that this is correct
            Self::Int16(n) => stream.write_bytes(cast_ref::<i16, [u8; 2]>(&n)),
            Self::Int32(n) => stream.write_bytes(cast_ref::<i32, [u8; 4]>(&n)),
            Self::Int64(n) => stream.write_bytes(cast_ref::<i64, [u8; 8]>(&n)),
            Self::Int128(n) => stream.write_bytes(cast_ref::<i128, [u8; 16]>(&n)),
            Self::Uint8(n) => stream.write_byte(*n),
            Self::Uint16(n) => stream.write_bytes(cast_ref::<u16, [u8; 2]>(&n)),
            Self::Uint32(n) => stream.write_bytes(cast_ref::<u32, [u8; 4]>(&n)),
            Self::Uint64(n) => stream.write_bytes(cast_ref::<u64, [u8; 8]>(&n)),
            Self::Uint128(n) => stream.write_bytes(cast_ref::<u128, [u8; 16]>(&n)),
            Self::VarInt32(n) => write_var_i32(stream, *n),
            Self::VarUint32(n) => write_var_u32(stream, *n),
            Self::Float32(x) => stream.write_bytes(cast_ref::<f32, [u8; 4]>(&x)),
            Self::Float64(x) => stream.write_bytes(cast_ref::<f64, [u8; 8]>(&x)),
            Self::Bytes(b) => {
                write_var_u32(stream, b.len() as u32);
                stream.write_bytes(&b[..]);
            },
            Self::String(s) => {
                write_var_u32(stream, s.len() as u32);
                stream.write_bytes(&s.as_bytes()[..]);
            },
            Self::TimePoint(t) => stream.write_bytes(cast_ref::<i64, [u8; 8]>(&t)),
            Self::TimePointSec(t) => stream.write_bytes(cast_ref::<u32, [u8; 4]>(&t)),
            Self::BlockTimestampType(t) => stream.write_bytes(cast_ref::<u32, [u8; 4]>(&t)),
            Self::Checksum160(c) => stream.write_bytes(&c[..]),
            Self::Checksum256(c) => stream.write_bytes(&c[..]),
            Self::Checksum512(c) => stream.write_bytes(&c[..]),
            Self::Name(name) => name.encode(stream),
            Self::Symbol(sym) => sym.encode(stream),
            Self::SymbolCode(sym) => stream.write_bytes(cast_ref::<u64, [u8; 8]>(&sym)),
            Self::Asset(asset) => asset.encode(stream),
            Self::Signature(sig) => sig.encode(stream),
        }
    }

    pub fn from_bin(typename: &str, stream: &mut ByteStream) -> Result<Self, InvalidValue> {
        Ok(match typename {
            "bool" => match stream.read_byte()? {
                1 => Self::Bool(true),
                0 => Self::Bool(false),
                _ => { return Err(InvalidValue::InvalidData("cannot parse bool from stream".to_owned())); },
            },
            "int8" => Self::Int8(stream.read_byte()? as i8),
            "int16" => Self::Int16(pod_read_unaligned(stream.read_bytes(2)?)),
            "int32" => Self::Int32(pod_read_unaligned(stream.read_bytes(4)?)),
            "int64" => Self::Int64(pod_read_unaligned(stream.read_bytes(8)?)),
            "int128" => Self::Int128(pod_read_unaligned(stream.read_bytes(16)?)),
            "uint8" => Self::Uint8(stream.read_byte()?),
            "uint16" => Self::Uint16(pod_read_unaligned(stream.read_bytes(2)?)),
            "uint32" => Self::Uint32(pod_read_unaligned(stream.read_bytes(4)?)),
            "uint64" => Self::Uint64(pod_read_unaligned(stream.read_bytes(8)?)),
            "uint128" => Self::Uint128(pod_read_unaligned(stream.read_bytes(16)?)),
            "varint32" => Self::VarInt32(read_var_i32(stream)?),
            "varuint32" => Self::VarUint32(read_var_u32(stream)?),
            "float32" => Self::Float32(pod_read_unaligned(stream.read_bytes(4)?)),
            "float64" => Self::Float64(pod_read_unaligned(stream.read_bytes(8)?)),
            "bytes" => Self::Bytes(read_bytes(stream)?),
            "string" => Self::String(read_str(stream)?.to_owned()),
            "time_point" => Self::TimePoint(pod_read_unaligned(stream.read_bytes(8)?)),
            "time_point_sec" => Self::TimePointSec(pod_read_unaligned(stream.read_bytes(4)?)),
            "block_timestamp_type" => Self::BlockTimestampType(pod_read_unaligned(stream.read_bytes(4)?)),
            "checksum160" => Self::Checksum160(Box::new(stream.read_bytes(20)?.try_into().unwrap())),
            "checksum256" => Self::Checksum256(Box::new(stream.read_bytes(32)?.try_into().unwrap())),
            "checksum512" => Self::Checksum512(Box::new(stream.read_bytes(64)?.try_into().unwrap())),
            "name" => Self::Name(Name::decode(stream)?),
            "symbol" => Self::Symbol(Symbol::decode(stream)?),
            "symbol_code" => Self::SymbolCode(pod_read_unaligned(stream.read_bytes(8)?)),
            "asset" => Self::Asset(Asset::decode(stream)?),
            "signature" => Self::Signature(Signature::decode(stream)?),
            _ => { return Err(InvalidValue::InvalidType(typename.to_owned())); },
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
    let mut n = n.clone();
    loop {
        if n >> 7 != 0 {
            stream.write_byte((0x80 | (n & 0x7f)) as u8);
            n = n >> 7
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


impl From<AntelopeType> for bool {
    fn from(n: AntelopeType) -> bool {
        match n {
            AntelopeType::Bool(b) => b,
            _ => todo!(),
        }
    }
}

impl From<AntelopeType> for i32 {
    fn from(n: AntelopeType) -> i32 {
        match n {
            AntelopeType::Int8(n) => n as i32,
            AntelopeType::Int16(n) => n as i32,
            AntelopeType::Int32(n) => n,
            AntelopeType::Uint8(n) => n as i32,
            AntelopeType::Uint16(n) => n as i32,
            AntelopeType::Uint32(n) => n as i32,
            AntelopeType::VarUint32(n) => n as i32,
            _ => todo!(),
        }
    }
}

impl TryFrom<AntelopeType> for usize {
    type Error = InvalidValue;

    fn try_from(n: AntelopeType) -> Result<usize, Self::Error> {
        Ok(match n {
            AntelopeType::Int8(n) => n as usize,
            AntelopeType::Int16(n) => n as usize,
            AntelopeType::Int32(n) => n as usize,
            AntelopeType::Int64(n) => n as usize,
            AntelopeType::Uint8(n) => n as usize,
            AntelopeType::Uint16(n) => n as usize,
            AntelopeType::Uint32(n) => n as usize,
            AntelopeType::Uint64(n) => n as usize,
            AntelopeType::VarUint32(n) => n as usize,
            _ => return Err(InvalidValue::InvalidData( format!("cannot convert {:?} to usize", n))),
        })
    }
}

impl TryFrom<AntelopeType> for i64 {
    type Error = InvalidValue;

    fn try_from(n: AntelopeType) -> Result<i64, Self::Error> {
        Ok(match n {
            AntelopeType::Int8(n) => n as i64,
            AntelopeType::Int16(n) => n as i64,
            AntelopeType::Int32(n) => n as i64,
            AntelopeType::Int64(n) => n,
            AntelopeType::Uint8(n) => n as i64,
            AntelopeType::Uint16(n) => n as i64,
            AntelopeType::Uint32(n) => n as i64,
            AntelopeType::Uint64(n) => n as i64,
            AntelopeType::VarUint32(n) => n as i64,
            _ => return Err(InvalidValue::InvalidData( format!("cannot convert {:?} to i64", n))),
        })
    }
}


impl TryFrom<AntelopeType> for String {
    type Error = InvalidValue;

    fn try_from(s: AntelopeType) -> Result<String, Self::Error> {
        Ok(match s {
            AntelopeType::String(s) => s,
            AntelopeType::Name(s) => s.to_string(),
            AntelopeType::Symbol(s) => s.to_string(),
            AntelopeType::Asset(s) => s.to_string(),
            _ => return Err(InvalidValue::InvalidData( format!("cannot convert {:?} to string", s))),
        })
    }
}



fn variant_to_i128(v: &Value) -> Result<i128, InvalidValue> {
    match v {
        v if v.is_i64() => Ok(v.as_i64().unwrap() as i128),
        v if v.is_string() => Ok(v.as_str().unwrap().parse()?),
        _ => Err(InvalidValue::IncompatibleVariantTypes("i128".to_owned(), v.clone())),
    }
}

fn variant_to_u128(v: &Value) -> Result<u128, InvalidValue> {
    match v {
        v if v.is_u64() => Ok(v.as_u64().unwrap() as u128),
        v if v.is_string() => Ok(v.as_str().unwrap().parse()?),
        _ => Err(InvalidValue::IncompatibleVariantTypes("u128".to_owned(), v.clone())),
    }
}

/// return a date in microseconds
fn parse_date(s: &str) -> Result<NaiveDateTime, InvalidValue> {
    Ok(NaiveDateTime::parse_from_str(s, DATE_FORMAT)?)
}


#[cfg(test)]
mod tests {
    use color_eyre::eyre::Report;
    use super::*;

    #[test]
    fn test_conversion() -> Result<(), Report> {
        let n = json!(23);
        let n = AntelopeType::from_variant("int8", &n)?;
        println!("n = {n:?}");

        Ok(())
    }
}


// Note: serde_json doesn't support "natively" 128-bit integer types
//  see: https://github.com/serde-rs/json/issues/846


#[derive(Error, Debug)]
pub enum InvalidValue {
    #[error("invalid type {0}")]
    InvalidType(String),

    #[error(r#"cannot convert given variant {1} to Antelope type "{0}""#)]
    IncompatibleVariantTypes(String, Value),

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

    #[error("invalid signature")]
    Signature(#[from] InvalidSignature),

    #[error("stream error")]
    StreamError(#[from] StreamError),

    #[error("cannot parse bytes as UTF-8")]
    Utf8Error(#[from] Utf8Error),

    #[error("cannot parse JSON string")]
    JsonParseError(#[from] JsonError),

    #[error("cannot parse date/time")]
    DateTimeParseError(#[from] ChronoParseError),

    #[error("incorrect array size for checksum")]
    IncorrectChecksumSize(#[from] TryFromSliceError),

    #[error("{0}")]
    InvalidData(String),  // acts as a generic error type with a given message
}
