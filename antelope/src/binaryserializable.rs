use std::str::{from_utf8, Utf8Error};

use bytemuck::{cast_ref, pod_read_unaligned};
use snafu::{ensure, Snafu, ResultExt};

use antelope_macros::with_location;

use crate::{
    ByteStream, StreamError, ABIError,
    types::*,
    impl_auto_error_conversion,
};


#[with_location]
#[derive(Debug, Snafu)]
pub enum SerializeError {
    #[snafu(display("stream error"))]
    StreamError { source: StreamError },

    #[snafu(display("invalid symbol"))]
    InvalidSymbol { source: InvalidSymbol },

    #[snafu(display("cannot decode bytes as utf-8"))]
    Utf8Error { source: Utf8Error },

    #[snafu(display("invalid crypto data"))]
    InvalidCryptoData { source: InvalidCryptoData },

    #[snafu(display("{msg}"))]
    InvalidData { msg: String },  // acts as a generic error type with a given message

    #[snafu(display("ABI error"), visibility(pub(crate)))]
    ABIError {
        #[snafu(source(from(ABIError, Box::new)))]
        source: Box<ABIError>
    },
}

impl_auto_error_conversion!(StreamError, SerializeError, StreamSnafu);
impl_auto_error_conversion!(InvalidSymbol, SerializeError, InvalidSymbolSnafu);
impl_auto_error_conversion!(InvalidCryptoData, SerializeError, InvalidCryptoDataSnafu);


/// Define methods required to (de)serialize a struct to a [`ByteStream`]
pub trait BinarySerializable {
    fn encode(&self, stream: &mut ByteStream);
    fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError>
    where
        Self: Sized;
}

// FIXME! Derive `BinarySerializable` for all builtin types

pub fn to_bin<T: BinarySerializable>(value: &T) -> Bytes {
    let mut s = ByteStream::new();
    value.encode(&mut s);
    Bytes(s.into_bytes())
}

pub fn to_hex<T: BinarySerializable>(value: &T) -> String {
    let mut s = ByteStream::new();
    value.encode(&mut s);
    s.hex_data()
}

// pub fn from_bin<T: BinarySerializable>(mut s: ByteStream) -> Result<T, SerializeError> {
//     T::decode(&mut s)
// }

// FIXME: this makes an unnecessary copy
pub fn from_bin<T: BinarySerializable>(bin: impl AsRef<[u8]>) -> Result<T, SerializeError> {
    let mut s = ByteStream::from(bin.as_ref().to_vec());
    T::decode(&mut s)
}

// -----------------------------------------------------------------------------
//     Boilerplate macros
// -----------------------------------------------------------------------------

macro_rules! impl_pod_serialization {
    ($typ:ty, $size:literal) => {
        impl BinarySerializable for $typ {
            #[inline]
            fn encode(&self, stream: &mut ByteStream) {
                stream.write_bytes(cast_ref::<$typ, [u8; $size]>(self))
            }
            #[inline]
            fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError> {
                Ok(pod_read_unaligned(stream.read_bytes($size)?))
            }
        }
    }
}

macro_rules! impl_wrapped_serialization {
    ($typ:ty, $inner:ty) => {
        impl BinarySerializable for $typ {
            #[inline]
            fn encode(&self, stream: &mut ByteStream) {
                <$inner>::from(*self).encode(stream)
            }
            #[inline]
            fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError> {
                Ok(<$typ>::from(<$inner>::decode(stream)?))
            }
        }
    }
}

macro_rules! impl_array_serialization {
    ($typ:ty, $size:literal) => {
        impl BinarySerializable for $typ {
            #[inline]
            fn encode(&self, stream: &mut ByteStream) {
                stream.write_bytes(&self.0[..])
            }
            #[inline]
            fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError> {
                let arr: [u8; $size] = stream.read_bytes($size)?.try_into().unwrap();  // safe unwrap
                Ok(<$typ>::from(arr))
            }
        }
    }
}


// -----------------------------------------------------------------------------
//     Serialization of ints and native Rust types
// -----------------------------------------------------------------------------

impl BinarySerializable for bool {
    #[inline]
    fn encode(&self, stream: &mut ByteStream) {
        stream.write_byte(match *self {
            true => 1u8,
            false => 0u8,
        })
    }
    #[inline]
    fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError> {
        match stream.read_byte()? {
            1 => Ok(true),
            0 => Ok(false),
            _ => InvalidDataSnafu { msg: "cannot parse bool from stream".to_owned() }.fail(),
        }
    }
}

impl BinarySerializable for i8 {
    #[inline]
    fn encode(&self, stream: &mut ByteStream) {
        stream.write_byte(*self as u8)
    }
    #[inline]
    fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError> {
        Ok(stream.read_byte()? as i8)
    }
}

impl_pod_serialization!(i16, 2);
impl_pod_serialization!(i32, 4);
impl_pod_serialization!(i64, 8);
impl_pod_serialization!(i128, 16);

impl BinarySerializable for u8 {
    #[inline]
    fn encode(&self, stream: &mut ByteStream) {
        stream.write_byte(*self)
    }
    #[inline]
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
#[cfg(feature = "float128")]
impl_pod_serialization!(f128, 16);


impl BinarySerializable for VarInt32 {
    #[inline]
    fn encode(&self, stream: &mut ByteStream) {
        write_var_i32(stream, i32::from(*self))
    }
    #[inline]
    fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError> {
        Ok(read_var_i32(stream)?.into())
    }
}

impl BinarySerializable for VarUint32 {
    #[inline]
    fn encode(&self, stream: &mut ByteStream) {
        write_var_u32(stream, u32::from(*self))
    }
    #[inline]
    fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError> {
        Ok(read_var_u32(stream)?.into())
    }
}


// -----------------------------------------------------------------------------
//     Serialization of string types
// -----------------------------------------------------------------------------

impl BinarySerializable for Bytes {
    fn encode(&self, stream: &mut ByteStream) {
        write_var_u32(stream, self.0.len() as u32);
        stream.write_bytes(&self.0[..]);
    }
    fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError> {
        let len = read_var_u32(stream)? as usize;
        Ok(Bytes::from(stream.read_bytes(len)?))
    }
}

// convenience implementation to avoid allocating when encoding a &[u8]
impl BinarySerializable for &[u8] {
    fn encode(&self, stream: &mut ByteStream) {
        write_var_u32(stream, self.len() as u32);
        stream.write_bytes(self);
    }
    fn decode(_stream: &mut ByteStream) -> Result<Self, SerializeError> {
        unimplemented!();
    }
}

impl BinarySerializable for String {
    fn encode(&self, stream: &mut ByteStream) {
        write_var_u32(stream, self.len() as u32);
        stream.write_bytes(self.as_bytes());
    }
    fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError> {
        let len = read_var_u32(stream)? as usize;
        from_utf8(stream.read_bytes(len)?).context(Utf8Snafu).map(|s| s.to_owned())
    }
}

// convenience implementation to avoid allocating encoding a &str
impl BinarySerializable for &str {
    fn encode(&self, stream: &mut ByteStream) {
        write_var_u32(stream, self.len() as u32);
        stream.write_bytes(self.as_bytes());
    }
    fn decode(_stream: &mut ByteStream) -> Result<Self, SerializeError> {
        unimplemented!()
    }
}


// -----------------------------------------------------------------------------
//     Serialization of time types
// -----------------------------------------------------------------------------

impl_wrapped_serialization!(TimePoint, i64);
impl_wrapped_serialization!(TimePointSec, u32);
impl_wrapped_serialization!(BlockTimestampType, u32);


// -----------------------------------------------------------------------------
//     Serialization of checksum types
// -----------------------------------------------------------------------------

impl_array_serialization!(Checksum160, 20);
impl_array_serialization!(Checksum256, 32);
impl_array_serialization!(Checksum512, 64);


// -----------------------------------------------------------------------------
//     Serialization of Antelope types
// -----------------------------------------------------------------------------

impl BinarySerializable for Name {
    #[inline]
    fn encode(&self, stream: &mut ByteStream) {
        self.as_u64().encode(stream)
    }

    #[inline]
    fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError> {
        let n = u64::decode(stream)?;
        Ok(Name::from_u64(n))
    }
}

impl BinarySerializable for Symbol {
    #[inline]
    fn encode(&self, stream: &mut ByteStream) {
        self.as_u64().encode(stream)
    }

    #[inline]
    fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError> {
        let n = u64::decode(stream)?;
        Ok(Symbol::from_u64(n)?)
    }
}

impl BinarySerializable for SymbolCode {
    #[inline]
    fn encode(&self, stream: &mut ByteStream) {
        self.as_u64().encode(stream)
    }

    #[inline]
    fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError> {
        let n = u64::decode(stream)?;
        Ok(SymbolCode::from_u64(n))
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

impl BinarySerializable for ExtendedAsset {
    fn encode(&self, stream: &mut ByteStream) {
        self.quantity.encode(stream);
        self.contract.encode(stream);
    }

    fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError> {
        let quantity = Asset::decode(stream)?;
        let contract = Name::decode(stream)?;
        Ok(ExtendedAsset { quantity, contract })
    }
}

impl<T: CryptoDataType, const DATA_SIZE: usize> BinarySerializable for CryptoData<T, DATA_SIZE> {
    fn encode(&self, stream: &mut ByteStream) {
        stream.write_byte(self.key_type().index());
        stream.write_bytes(self.data());
    }

    fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError> {
        let key_type = KeyType::from_index(stream.read_byte()?)?;
        let data = stream.read_bytes(DATA_SIZE)?.try_into().unwrap();  // safe unwrap
        Ok(Self::new(key_type, data))
    }
}

// this, coupled with the blanket impl for Vec, gives us the impl for the `Extensions` type
impl BinarySerializable for (u16, Bytes) {
    fn encode(&self, stream: &mut ByteStream) {
        self.0.encode(stream);
        self.1.encode(stream);
    }

    fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError> {
        let id = u16::decode(stream)?;
        let data = Bytes::decode(stream)?;
        Ok((id, data))
    }
}


// -----------------------------------------------------------------------------
//     util functions for varints and reading/writing str/bytes
// -----------------------------------------------------------------------------

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

fn read_var_u32(stream: &mut ByteStream) -> Result<u32, SerializeError> {
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

fn read_var_i32(stream: &mut ByteStream) -> Result<i32, SerializeError> {
    let n = read_var_u32(stream)?;
    Ok(match n & 1 {
        0 => n >> 1,
        _ => ((!n) >> 1) | 0x8000_0000,
    } as i32)
}


// -----------------------------------------------------------------------------
//     other useful blanket implementations for containers
// -----------------------------------------------------------------------------

// NOTE: we have 2 choices here:
//  - blanket impl, at the cost of a non-optimized impl for Vec<u8>
//  - optimized impl for Vec<u8>, but we have to manually implement
//    (possibly with the help of a macro) all the other needed types
impl<T: BinarySerializable> BinarySerializable for Vec<T> {
    fn encode(&self, stream: &mut ByteStream) {
        write_var_u32(stream, self.len() as u32);
        for elem in self {
            elem.encode(stream);
        }
    }

    fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError> {
        let len: u32 = VarUint32::decode(stream)?.into();
        let mut result = Vec::with_capacity(len as usize);
        for _ in 0..len {
            result.push(T::decode(stream)?);
        }
        Ok(result)
    }
}

impl<T: BinarySerializable> BinarySerializable for Option<T> {
    fn encode(&self, stream: &mut ByteStream) {
        match self {
            Some(v) => {
                true.encode(stream);
                v.encode(stream);
            },
            None => {
                false.encode(stream);
            }
        }
    }

    fn decode(stream: &mut ByteStream) -> Result<Self, SerializeError> {
        Ok(match bool::decode(stream)? {
            true => Some(T::decode(stream)?),
            false => None,
        })
    }
}
