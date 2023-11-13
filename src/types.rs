pub mod name;
pub mod symbol;
pub mod asset;

pub use name::{Name, InvalidName};
pub use symbol::{Symbol, InvalidSymbol};
pub use asset::{Asset, InvalidAsset};

use std::num::{ParseFloatError, ParseIntError, TryFromIntError};
use std::str::{from_utf8, Utf8Error, ParseBoolError};

use bytemuck::cast_ref;
use serde_json::{json, Value};
use thiserror::Error;
use strum::EnumVariantNames;

use super::{ByteStream, StreamError};

// see full list in: https://github.com/AntelopeIO/leap/blob/main/libraries/chain/abi_serializer.cpp#L89
#[derive(Debug, EnumVariantNames)]
#[strum(serialize_all = "lowercase")]
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

    VarUint32(u32),

    Float32(f32),
    Float64(f64),
    // Float128(??),

    String(String),

    Name(Name),
    Symbol(Symbol),
    Asset(Asset),

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
            "varuint32" => Self::VarUint32(repr.parse()?),
            "float32" => Self::Float32(repr.parse()?),
            "float64" => Self::Float64(repr.parse()?),
            "string" => Self::String(repr.to_owned()),
            "name" => Self::Name(Name::from_str(repr)?),
            "symbol" => Self::Symbol(Symbol::from_str(repr)?),
            "asset" => Self::Asset(Asset::from_str(repr)?),
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
            Self::Int128(_n) => todo!(),
            Self::Uint8(n) => json!(n),
            Self::Uint16(n) => json!(n),
            Self::Uint32(n) => json!(n),
            Self::Uint64(n) => json!(n),
            Self::Uint128(_n) => todo!(),
            Self::VarUint32(n) => json!(n),
            Self::Float32(x) => json!(x),
            Self::Float64(x) => json!(x),
            Self::String(s) => json!(s),
            Self::Name(name) => json!(name.to_string()),
            Self::Symbol(sym) => json!(sym.to_string()),
            Self::Asset(asset) => json!(asset.to_string()),
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
            "uint8" => Self::Uint8(v.as_u64().ok_or_else(incompatible_types)?.try_into()?),
            "uint16" => Self::Uint16(v.as_u64().ok_or_else(incompatible_types)?.try_into()?),
            "uint32" => Self::Uint32(v.as_u64().ok_or_else(incompatible_types)?.try_into()?),
            "uint64" => Self::Uint64(v.as_u64().ok_or_else(incompatible_types)?),
            "varuint32" => Self::VarUint32(v.as_i64().ok_or_else(incompatible_types)?.try_into()?),
            "float32" => Self::Float32(f64_to_f32(v.as_f64().ok_or_else(incompatible_types)?)?),
            "float64" => Self::Float64(v.as_f64().ok_or_else(incompatible_types)?),
            "string" => Self::String(v.as_str().ok_or_else(incompatible_types)?.to_owned()),
            "name" => Self::from_str("name", v.as_str().ok_or_else(incompatible_types)?)?,
            "symbol" => Self::from_str("symbol", v.as_str().ok_or_else(incompatible_types)?)?,
            "asset" => Self::from_str("asset", v.as_str().ok_or_else(incompatible_types)?)?,
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
            Self::Int128(_n) => todo!(),
            Self::Uint8(n) => stream.write_byte(*n),
            Self::Uint16(n) => stream.write_bytes(cast_ref::<u16, [u8; 2]>(&n)),
            Self::Uint32(n) => stream.write_bytes(cast_ref::<u32, [u8; 4]>(&n)),
            Self::Uint64(n) => stream.write_bytes(cast_ref::<u64, [u8; 8]>(&n)),
            Self::Uint128(_n) => todo!(),
            Self::VarUint32(n) => write_var_u32(stream, *n),
            Self::Float32(x) => stream.write_bytes(cast_ref::<f32, [u8; 4]>(&x)),
            Self::Float64(x) => stream.write_bytes(cast_ref::<f64, [u8; 8]>(&x)),
            Self::String(s) => {
                write_var_u32(stream, s.len() as u32);
                stream.write_bytes(&s.as_bytes()[..s.len()]);
            },
            Self::Name(name) => name.encode(stream),
            Self::Symbol(sym) => sym.encode(stream),
            Self::Asset(asset) => asset.encode(stream),
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
            "int16" => Self::Int16(*cast_ref::<[u8; 2], i16>(stream.read_bytes(2)?.try_into().unwrap())),
            "int32" => Self::Int32(*cast_ref::<[u8; 4], i32>(stream.read_bytes(4)?.try_into().unwrap())),
            "int64" => Self::Int64(*cast_ref::<[u8; 8], i64>(stream.read_bytes(8)?.try_into().unwrap())),
            "uint8" => Self::Uint8(stream.read_byte()?),
            "uint16" => Self::Uint16(*cast_ref::<[u8; 2], u16>(stream.read_bytes(2)?.try_into().unwrap())),
            "uint32" => Self::Uint32(*cast_ref::<[u8; 4], u32>(stream.read_bytes(4)?.try_into().unwrap())),
            "uint64" => Self::Uint64(*cast_ref::<[u8; 8], u64>(stream.read_bytes(8)?.try_into().unwrap())),
            "varuint32" => Self::VarUint32(read_var_u32(stream)?),
            "float32" => Self::Float32(*cast_ref::<[u8; 4], f32>(stream.read_bytes(4)?.try_into().unwrap())),
            "float64" => Self::Float64(*cast_ref::<[u8; 8], f64>(stream.read_bytes(8)?.try_into().unwrap())),
            "string" => Self::String(read_str(stream)?.to_owned()),
            "name" => Self::Name(Name::from_str(read_str(stream)?)?),
            "symbol" => Self::Symbol(Symbol::from_str(read_str(stream)?)?),
            "asset" => Self::Asset(Asset::from_str(read_str(stream)?)?),
            _ => { return Err(InvalidValue::InvalidType(typename.to_owned())); },
        })
    }
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

    #[error(r#"cannot convert given variant "{1}" to Antelope type "{0}"""#)]
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

    #[error("stream ended while reading bytes")]
    StreamEnded(#[from] StreamError),

    #[error("cannot parse bytes as UTF-8")]
    Utf8Error(#[from] Utf8Error),

    #[error("{0}")]
    InvalidData(String),  // acts as a generic error type with a given message
}


/*
/// This is the main trait that Antelope types need to implement. This allows
/// them to be serialized to an ABI using an ABIEncoder
pub trait ABISerializable {
    fn encode(&self, stream: &mut ByteStream);

    fn decode(stream: &mut ByteStream) -> Self;

}


impl ABISerializable for bool {
    fn encode(&self, stream: &mut ByteStream) {
        stream.write_byte(match *self {
            true => 1u8,
            false => 0u8,
        })
    }
    fn decode(_stream: &mut ByteStream) -> Self {
        todo!();
    }
}

impl ABISerializable for i8 {
    fn encode(&self, stream: &mut ByteStream) {
        stream.write_i8(*self);
    }
    fn decode(_stream: &mut ByteStream) -> Self {
        todo!();
    }
}

impl ABISerializable for i16 {
    fn encode(&self, stream: &mut ByteStream) {
        stream.write_i16(*self);
    }
    fn decode(_stream: &mut ByteStream) -> Self {
        todo!();
    }
}

impl ABISerializable for i32 {
    fn encode(&self, stream: &mut ByteStream) {
        stream.write_i32(*self);
    }
    fn decode(_stream: &mut ByteStream) -> Self {
        todo!();
    }
}

impl ABISerializable for i64 {
    fn encode(&self, stream: &mut ByteStream) {
        stream.write_i64(*self);
    }
    fn decode(_stream: &mut ByteStream) -> Self {
        todo!();
    }
}

// impl ABISerializable for i128 {
//     fn encode(&self, stream: &mut ByteStream) {
//         stream.write_i128(*self);
//     }
// }

impl ABISerializable for u8 {
    fn encode(&self, stream: &mut ByteStream) {
        stream.write_u8(*self);
    }
    fn decode(_stream: &mut ByteStream) -> Self {
        todo!();
    }
}

impl ABISerializable for u16 {
    fn encode(&self, stream: &mut ByteStream) {
        stream.write_u16(*self);
    }
    fn decode(_stream: &mut ByteStream) -> Self {
        todo!();
    }
}

impl ABISerializable for u32 {
    fn encode(&self, stream: &mut ByteStream) {
        stream.write_u32(*self);
    }
    fn decode(_stream: &mut ByteStream) -> Self {
        todo!();
    }
}

impl ABISerializable for u64 {
    fn encode(&self, stream: &mut ByteStream) {
        stream.write_u64(*self);
    }
    fn decode(_stream: &mut ByteStream) -> Self {
        todo!();
    }
}

impl ABISerializable for f32 {
    fn encode(&self, stream: &mut ByteStream) {
        stream.write_f32(*self);
    }
    fn decode(_stream: &mut ByteStream) -> Self {
        todo!();
    }
}

impl ABISerializable for f64 {
    fn encode(&self, stream: &mut ByteStream) {
        stream.write_f64(*self);
    }
    fn decode(_stream: &mut ByteStream) -> Self {
        todo!();
    }
}

// impl ABISerializable for u128 {
//     fn encode(&self, stream: &mut ByteStream) {
//         stream.write_u128(*self);
//     }
// }

impl ABISerializable for &str {
    fn encode(&self, stream: &mut ByteStream) {
        stream.write_str(*self);
    }
    fn decode(_stream: &mut ByteStream) -> Self {
        todo!();
    }
}


impl ABISerializable for &[&str] {
    fn encode(&self,  stream: &mut ByteStream) {
        stream.write_var_u32(self.len() as u32);
        for &s in *self {
            stream.write_str(s);
        }
    }
    fn decode(_stream: &mut ByteStream) -> Self {
        todo!();
    }
}
*/
