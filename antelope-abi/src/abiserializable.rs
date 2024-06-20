use bytemuck::{cast_ref, pod_read_unaligned};
use hex::FromHexError;
use snafu::{Snafu, IntoError};
use tracing::instrument;

use annotated_error::with_location;
use antelope_core::{
    AntelopeType, AntelopeValue, Asset, InvalidValue, Name,
    PrivateKey, PublicKey, Signature, Symbol, InvalidSymbol,
    impl_auto_error_conversion,
};

use crate::{
    binaryserializable::{
        read_bytes, read_str, read_var_i32, read_var_u32, write_var_i32, write_var_u32, BinarySerializable,
    },
    ByteStream, StreamError,
};

#[with_location]
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum SerializeError {
    #[snafu(display("stream error"))]
    StreamError { source: StreamError },

    #[snafu(display("invalid value"))]
    InvalidValue { source: InvalidValue },

    #[snafu(display("invalid symbol"))]
    InvalidSymbol { source: InvalidSymbol },

    #[snafu(display("cannot decode hex data"))]
    HexDecodeError { source: FromHexError },

    #[snafu(display("{msg}"))]
    InvalidData { msg: String },  // acts as a generic error type with a given message
}

impl_auto_error_conversion!(StreamError, SerializeError, StreamSnafu);
impl_auto_error_conversion!(InvalidValue, SerializeError, InvalidValueSnafu);
impl_auto_error_conversion!(InvalidSymbol, SerializeError, InvalidSymbolSnafu);
impl_auto_error_conversion!(FromHexError, SerializeError, HexDecodeSnafu);

// FIXME: from_bin should take &str instead of AntelopeType, and we might need to register an ABI provider
pub trait ABISerializable {
    fn abi_name() -> &'static str {
        "undefined"
    }
    // abi_base, abi_fields, see https://github.com/wharfkit/antelope/blob/master/src/chain/struct.ts
    fn to_bin(&self, _stream: &mut ByteStream);
    fn from_bin(_typename: AntelopeType, _stream: &mut ByteStream) -> Result<Self, SerializeError>
    where
        Self: Sized;
}


impl ABISerializable for AntelopeValue {
    fn to_bin(&self, stream: &mut ByteStream) {
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

    #[instrument(skip(stream))]
    fn from_bin(typename: AntelopeType, stream: &mut ByteStream) -> Result<Self, SerializeError> {
        Ok(match typename {
            AntelopeType::Bool => match stream.read_byte()? {
                1 => Self::Bool(true),
                0 => Self::Bool(false),
                _ => {
                    return InvalidDataSnafu { msg: "cannot parse bool from stream".to_owned() }.fail();
                },
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
            AntelopeType::ExtendedAsset => {
                Self::ExtendedAsset(Box::new((Asset::decode(stream)?, Name::decode(stream)?)))
            },
        })
    }
}
