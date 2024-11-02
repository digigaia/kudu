use tracing::instrument;

use antelope_core::{
    AntelopeType, AntelopeValue, Asset, Symbol,
    Name, PrivateKey, PublicKey, Signature,
};

use crate::{
    binaryserializable::{
        read_bytes, read_str, read_var_i32, read_var_u32, write_var_i32, write_var_u32,
    },
    ByteStream, BinarySerializable, SerializeError,
};


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
            Self::VarInt32(n) => write_var_i32(stream, *n),
            Self::VarUint32(n) => write_var_u32(stream, *n),
            Self::Float32(x) => x.encode(stream),
            Self::Float64(x) => x.encode(stream),
            Self::Bytes(b) => {
                write_var_u32(stream, b.len() as u32);
                stream.write_bytes(&b[..]);
            },
            Self::String(s) => {
                write_var_u32(stream, s.len() as u32);
                stream.write_bytes(s.as_bytes());
            },
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
            AntelopeType::VarInt32 => Self::VarInt32(read_var_i32(stream)?),
            AntelopeType::VarUint32 => Self::VarUint32(read_var_u32(stream)?),
            AntelopeType::Float32 => Self::Float32(f32::decode(stream)?),
            AntelopeType::Float64 => Self::Float64(f64::decode(stream)?),
            AntelopeType::Bytes => Self::Bytes(read_bytes(stream)?),
            AntelopeType::String => Self::String(read_str(stream)?.to_owned()),
            AntelopeType::TimePoint => Self::TimePoint(i64::decode(stream)?),
            AntelopeType::TimePointSec => Self::TimePointSec(u32::decode(stream)?),
            AntelopeType::BlockTimestampType => Self::BlockTimestampType(u32::decode(stream)?),
            AntelopeType::Checksum160 => Self::Checksum160(Box::new(stream.read_bytes(20)?.try_into().unwrap())),
            AntelopeType::Checksum256 => Self::Checksum256(Box::new(stream.read_bytes(32)?.try_into().unwrap())),
            AntelopeType::Checksum512 => Self::Checksum512(Box::new(stream.read_bytes(64)?.try_into().unwrap())),
            AntelopeType::PublicKey => Self::PublicKey(Box::new(PublicKey::decode(stream)?)),
            AntelopeType::PrivateKey => Self::PrivateKey(Box::new(PrivateKey::decode(stream)?)),
            AntelopeType::Signature => Self::Signature(Box::new(Signature::decode(stream)?)),
            AntelopeType::Name => Self::Name(Name::decode(stream)?),
            AntelopeType::Symbol => Self::Symbol(Symbol::decode(stream)?),
            AntelopeType::SymbolCode => Self::SymbolCode(u64::decode(stream)?),
            AntelopeType::Asset => Self::Asset(Asset::decode(stream)?),
            AntelopeType::ExtendedAsset => {
                Self::ExtendedAsset(Box::new((Asset::decode(stream)?, Name::decode(stream)?)))
            },
        })
    }
}
