use std::str::{from_utf8, Utf8Error};

use antelope_core::{
    Asset, Name, Symbol, InvalidSymbol, InvalidValue, impl_auto_error_conversion,
    types::crypto::{CryptoData, CryptoDataType, KeyType, InvalidCryptoData},
    types::builtin,
};
use bytemuck::{cast_ref, pod_read_unaligned};
use hex::FromHexError;
use snafu::{ensure, Snafu, IntoError, ResultExt};

use antelope_macros::with_location;
use crate::{ByteStream, StreamError};


#[with_location]
#[derive(Debug, Snafu)]
pub enum SerializeError {
    #[snafu(display("stream error"))]
    StreamError { source: StreamError },

    #[snafu(display("invalid value"))]
    InvalidValue { source: InvalidValue },

    #[snafu(display("invalid symbol"))]
    InvalidSymbol { source: InvalidSymbol },

    #[snafu(display("cannot decode hex data"))]
    HexDecodeError { source: FromHexError },

    #[snafu(display("cannot decode bytes as utf-8"))]
    Utf8Error { source: Utf8Error },

    #[snafu(display("invalid crypto data"))]
    InvalidCryptoData { source: InvalidCryptoData },

    #[snafu(display("{msg}"))]
    InvalidData { msg: String },  // acts as a generic error type with a given message
}

impl_auto_error_conversion!(StreamError, SerializeError, StreamSnafu);
impl_auto_error_conversion!(InvalidValue, SerializeError, InvalidValueSnafu);
impl_auto_error_conversion!(InvalidSymbol, SerializeError, InvalidSymbolSnafu);
impl_auto_error_conversion!(FromHexError, SerializeError, HexDecodeSnafu);
impl_auto_error_conversion!(InvalidCryptoData, SerializeError, InvalidCryptoDataSnafu);


/// Define methods required to (de)serialize a struct to a [`ByteStream`]
pub trait BinarySerializable {
    fn encode(&self, stream: &mut ByteStream);
    fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError>
    where
        Self: Sized;
}

// FIXME! Derive `BinarySerializable` for all builtin types

// -----------------------------------------------------------------------------
//     Boilerplate macros
// -----------------------------------------------------------------------------

macro_rules! impl_pod_serialization {
    ($typ:ty, $size:literal) => {
        impl BinarySerializable for $typ {
            fn encode(&self, stream: &mut ByteStream) {
                stream.write_bytes(cast_ref::<$typ, [u8; $size]>(self))
            }
            fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError> {
                Ok(pod_read_unaligned(stream.read_bytes($size)?))
            }
        }
    }
}

macro_rules! impl_wrapped_serialization {
    ($typ:ty, $inner:ty) => {
        impl BinarySerializable for $typ {
            fn encode(&self, stream: &mut ByteStream) {
                self.0.encode(stream)
            }
            fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError> {
                Ok(<$typ>::from(<$inner>::decode(stream)?))
            }
        }
    }
}

macro_rules! impl_array_serialization {
    ($typ:ty, $size:literal) => {
        impl BinarySerializable for $typ {
            fn encode(&self, stream: &mut ByteStream) {
                stream.write_bytes(&self[..])
            }
            fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError> {
                Ok(Box::new(stream.read_bytes($size)?.try_into().unwrap()))
            }
        }
    }
}


// -----------------------------------------------------------------------------
//     Serialization of ints and native Rust types
// -----------------------------------------------------------------------------

impl BinarySerializable for bool {
    fn encode(&self, stream: &mut ByteStream) {
        stream.write_byte(match *self {
            true => 1u8,
            false => 0u8,
        })
    }
    fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError> {
        match stream.read_byte()? {
            1 => Ok(true),
            0 => Ok(false),
            _ => InvalidDataSnafu { msg: "cannot parse bool from stream".to_owned() }.fail(),
        }
    }
}

impl BinarySerializable for i8 {
    fn encode(&self, stream: &mut ByteStream) {
        stream.write_byte(*self as u8)
    }
    fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError> {
        Ok(stream.read_byte()? as i8)
    }
}

impl_pod_serialization!(i16, 2);
impl_pod_serialization!(i32, 4);
impl_pod_serialization!(i64, 8);
impl_pod_serialization!(i128, 16);

impl BinarySerializable for u8 {
    fn encode(&self, stream: &mut ByteStream) {
        stream.write_byte(*self)
    }
    fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError> {
        Ok(stream.read_byte()?)
    }
}

impl_pod_serialization!(u16, 2);
impl_pod_serialization!(u32, 4);
impl_pod_serialization!(u64, 8);
impl_pod_serialization!(u128, 16);

impl_pod_serialization!(f32, 4);
impl_pod_serialization!(f64, 8);


impl BinarySerializable for builtin::VarInt32 {
    fn encode(&self, stream: &mut ByteStream) {
        write_var_i32(stream, self.0)
    }
    fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError> {
        Ok(Self(read_var_i32(stream)?))
    }
}

impl BinarySerializable for builtin::VarUint32 {
    fn encode(&self, stream: &mut ByteStream) {
        write_var_u32(stream, self.0)
    }
    fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError> {
        Ok(Self(read_var_u32(stream)?))
    }
}

// -----------------------------------------------------------------------------
//     Serialization of string types
// -----------------------------------------------------------------------------

impl BinarySerializable for builtin::Bytes {
    fn encode(&self, stream: &mut ByteStream) {
        write_var_u32(stream, self.len() as u32);
        stream.write_bytes(&self[..]);
    }
    fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError> {
        let len = read_var_u32(stream)? as usize;
        Ok(Vec::from(stream.read_bytes(len)?))
    }
}
impl BinarySerializable for builtin::String {
    fn encode(&self, stream: &mut ByteStream) {
        write_var_u32(stream, self.len() as u32);
        stream.write_bytes(self.as_bytes());
    }
    fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError> {
        let len = read_var_u32(stream)? as usize;
        from_utf8(stream.read_bytes(len)?).context(Utf8Snafu).map(|s| s.to_owned())
    }
}


// -----------------------------------------------------------------------------
//     Serialization of time types
// -----------------------------------------------------------------------------

impl_wrapped_serialization!(builtin::TimePoint, i64);
impl_wrapped_serialization!(builtin::TimePointSec, u32);
impl_wrapped_serialization!(builtin::BlockTimestampType, u32);

// -----------------------------------------------------------------------------
//     Serialization of checksum types
// -----------------------------------------------------------------------------

impl_array_serialization!(builtin::Checksum160, 20);
impl_array_serialization!(builtin::Checksum256, 32);
impl_array_serialization!(builtin::Checksum512, 64);

// -----------------------------------------------------------------------------
//     Serialization of Antelope types
// -----------------------------------------------------------------------------

impl BinarySerializable for Name {
    fn encode(&self, stream: &mut ByteStream) {
        self.as_u64().encode(stream)
    }

    fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError> {
        let n = u64::decode(stream)?;
        Ok(Name::from_u64(n))
    }
}

impl BinarySerializable for Symbol {
    fn encode(&self, stream: &mut ByteStream) {
        self.as_u64().encode(stream)
    }

    fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError> {
        let n = u64::decode(stream)?;
        Ok(Symbol::from_u64(n)?)
    }
}

impl BinarySerializable for Asset {
    fn encode(&self, stream: &mut ByteStream) {
        self.amount().encode(stream);
        self.symbol().encode(stream);
    }

    fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError> {
        let amount = i64::decode(stream)?;
        let symbol = Symbol::decode(stream)?;
        Ok(Asset::new(amount, symbol))
    }
}

impl BinarySerializable for builtin::ExtendedAsset {
    fn encode(&self, stream: &mut ByteStream) {
        self.0.encode(stream);
        self.1.encode(stream);
    }

    fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError> {
        let asset = Asset::decode(stream)?;
        let name = Name::decode(stream)?;
        Ok((asset, name))
    }
}

impl<T: CryptoDataType, const DATA_SIZE: usize> BinarySerializable for CryptoData<T, DATA_SIZE> {
    fn encode(&self, stream: &mut ByteStream) {
        stream.write_byte(self.key_type().index());
        stream.write_bytes(self.data());
    }

    fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError> {
        let key_type = KeyType::from_index(stream.read_byte()?)?;
        let data = stream.read_bytes(DATA_SIZE)?.try_into().unwrap();
        Ok(Self::new(key_type, data))
    }
}


// -----------------------------------------------------------------------------
//     util functions for varints and reading/writing str/bytes
// -----------------------------------------------------------------------------

// TODO: remove pub visibility on those functions?

pub fn write_var_u32(stream: &mut ByteStream, n: u32) {
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

pub fn write_var_i32(stream: &mut ByteStream, n: i32) {
    let unsigned = ((n as u32) << 1) ^ ((n >> 31) as u32);
    write_var_u32(stream, unsigned)
}

pub fn read_var_u32(stream: &mut ByteStream) -> Result<u32, SerializeError> {
    let mut offset = 0;
    let mut result = 0;
    loop {
        let byte = stream.read_byte()?;
        result |= (byte as u32 & 0x7F) << offset;
        offset += 7;
        if (byte & 0x80) == 0 { break; }

        ensure!(offset < 32, InvalidDataSnafu { msg: "varint too long to fit in u32" });
    }
    Ok(result)
}

pub fn read_var_i32(stream: &mut ByteStream) -> Result<i32, SerializeError> {
    let n = read_var_u32(stream)?;
    Ok(match n & 1 {
        0 => n >> 1,
        _ => ((!n) >> 1) | 0x8000_0000,
    } as i32)
}
