use tracing::instrument;

use antelope_core::types::*;

use crate::{
    ByteStream, BinarySerializable, SerializeError,
};


// FIXME: from_bin should take &str instead of AntelopeType, and we might need to register an ABI provider
//        we can actually get rid of `typename` by moving it as a generic trait type, or (better) by
//        just calling it on the appropriate type that implements ABISerialize (if that is possible)
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

// T is Target type (eg: int32, varint32, name, etc.)
pub trait ABISerialize<T = Self> {
    fn to_bin(&self, _stream: &mut ByteStream);
    fn from_bin(_stream: &mut ByteStream) -> Result<Self, SerializeError>
    where
        Self: Sized;
}

include!("abiserialize_builtin.rs");

impl<T: BinarySerializable> ABISerializable for T {
    fn to_bin(&self, stream: &mut ByteStream) {
        self.encode(stream)
    }
    fn from_bin(_typename: AntelopeType, stream: &mut ByteStream) -> Result<T, SerializeError> {
        // FIXME: we should check that the implement type (here: T) is compatible with
        //        the `typename` type
        T::decode(stream)
    }
}

impl ABISerializable for AntelopeValue {
    fn to_bin(&self, stream: &mut ByteStream) {
        match self {
            Self::Bool(b) => b.encode(stream),
            Self::Int8(n) => n.encode(stream),
            Self::Int16(n) => n.encode(stream),
            Self::Int32(n) => n.encode(stream),
            Self::Int64(n) => n.encode(stream),
            Self::Int128(n) => n.encode(stream),
            Self::Uint8(n) => n.encode(stream),
            Self::Uint16(n) => n.encode(stream),
            Self::Uint32(n) => n.encode(stream),
            Self::Uint64(n) => n.encode(stream),
            Self::Uint128(n) => n.encode(stream),
            Self::VarInt32(n) => n.encode(stream),
            Self::VarUint32(n) => n.encode(stream),
            Self::Float32(x) => x.encode(stream),
            Self::Float64(x) => x.encode(stream),
            #[cfg(feature = "float128")]
            Self::Float128(x) => x.encode(stream),
            Self::Bytes(b) => b.encode(stream),
            Self::String(s) => s.encode(stream),
            Self::TimePoint(t) => t.encode(stream),
            Self::TimePointSec(t) => t.encode(stream),
            Self::BlockTimestampType(t) => t.encode(stream),
            Self::Checksum160(c) => stream.write_bytes(&c[..]),
            Self::Checksum256(c) => stream.write_bytes(&c[..]),
            Self::Checksum512(c) => stream.write_bytes(&c[..]),
            Self::PublicKey(sig) => sig.encode(stream),
            Self::PrivateKey(sig) => sig.encode(stream),
            Self::Signature(sig) => sig.encode(stream),
            Self::Name(name) => name.encode(stream),
            Self::Symbol(sym) => sym.encode(stream),
            Self::SymbolCode(sym) => sym.encode(stream),
            Self::Asset(asset) => asset.encode(stream),
            Self::ExtendedAsset(ea) => ea.encode(stream),
        }
    }

    #[instrument(skip(stream))]
    fn from_bin(typename: AntelopeType, stream: &mut ByteStream) -> Result<Self, SerializeError> {
        Ok(match typename {
            AntelopeType::Bool => Self::Bool(bool::decode(stream)?),
            AntelopeType::Int8 => Self::Int8(i8::decode(stream)?),
            AntelopeType::Int16 => Self::Int16(i16::decode(stream)?),
            AntelopeType::Int32 => Self::Int32(i32::decode(stream)?),
            AntelopeType::Int64 => Self::Int64(i64::decode(stream)?),
            AntelopeType::Int128 => Self::Int128(i128::decode(stream)?),
            AntelopeType::Uint8 => Self::Uint8(u8::decode(stream)?),
            AntelopeType::Uint16 => Self::Uint16(u16::decode(stream)?),
            AntelopeType::Uint32 => Self::Uint32(u32::decode(stream)?),
            AntelopeType::Uint64 => Self::Uint64(u64::decode(stream)?),
            AntelopeType::Uint128 => Self::Uint128(u128::decode(stream)?),
            AntelopeType::VarInt32 => Self::VarInt32(VarInt32::decode(stream)?),
            AntelopeType::VarUint32 => Self::VarUint32(VarUint32::decode(stream)?),
            AntelopeType::Float32 => Self::Float32(f32::decode(stream)?),
            AntelopeType::Float64 => Self::Float64(f64::decode(stream)?),
            #[cfg(feature = "float128")]
            AntelopeType::Float128 => Self::Float128(f128::decode(stream)?),
            AntelopeType::Bytes => Self::Bytes(Bytes::decode(stream)?),
            AntelopeType::String => Self::String(String::decode(stream)?),
            AntelopeType::TimePoint => Self::TimePoint(TimePoint::decode(stream)?),
            AntelopeType::TimePointSec => Self::TimePointSec(TimePointSec::decode(stream)?),
            AntelopeType::BlockTimestampType => Self::BlockTimestampType(BlockTimestampType::decode(stream)?),
            AntelopeType::Checksum160 => Self::Checksum160(Box::new(Checksum160::decode(stream)?)),
            AntelopeType::Checksum256 => Self::Checksum256(Box::new(Checksum256::decode(stream)?)),
            AntelopeType::Checksum512 => Self::Checksum512(Box::new(Checksum512::decode(stream)?)),
            AntelopeType::PublicKey => Self::PublicKey(Box::new(PublicKey::decode(stream)?)),
            AntelopeType::PrivateKey => Self::PrivateKey(Box::new(PrivateKey::decode(stream)?)),
            AntelopeType::Signature => Self::Signature(Box::new(Signature::decode(stream)?)),
            AntelopeType::Name => Self::Name(Name::decode(stream)?),
            AntelopeType::Symbol => Self::Symbol(Symbol::decode(stream)?),
            AntelopeType::SymbolCode => Self::SymbolCode(SymbolCode::decode(stream)?),
            AntelopeType::Asset => Self::Asset(Asset::decode(stream)?),
            AntelopeType::ExtendedAsset => {
                Self::ExtendedAsset(Box::new((Asset::decode(stream)?, Name::decode(stream)?)))
            },
        })
    }
}
