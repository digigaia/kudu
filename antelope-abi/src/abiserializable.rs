use std::str::from_utf8;

use bytemuck::{cast_ref, pod_read_unaligned};
use tracing;

use antelope_core::{
    AntelopeType, AntelopeValue, InvalidValue,
    Name, Symbol, Asset,
    types::{PrivateKey, PublicKey, Signature, crypto::{CryptoData, CryptoDataType, KeyType}},
};

use crate::bytestream::ByteStream;

// FIXME: maybe this shouldn't be called ABISerializable as it doesn't have anything to do with an ABI yet
//        we are only encoding some base types in binary into a bytestream
pub trait ABISerializable  {
    fn encode(&self, stream: &mut ByteStream);
    fn decode(stream: &mut ByteStream) -> Result<Self, InvalidValue> where Self: Sized;  // FIXME: this should be a different Error type

    // FIXME: this doesn't belong here, we only have it temporarily during refactoring
    fn to_bin(&self, _stream: &mut ByteStream) {
        unimplemented!();
    }
    fn from_bin(_typename: AntelopeType, _stream: &mut ByteStream) -> Result<Self, InvalidValue> where Self: Sized {
        unimplemented!();
    }
}


impl ABISerializable for i64 {
    fn encode(&self, stream: &mut ByteStream) {
        stream.write_i64(*self)
    }
    fn decode(stream: &mut ByteStream) -> Result<Self, InvalidValue> {
        Ok(pod_read_unaligned(stream.read_bytes(8)?))
    }
}

impl ABISerializable for Name {
    fn encode(&self, stream: &mut ByteStream) {
        AntelopeValue::Uint64(self.as_u64()).to_bin(stream);
    }

    fn decode(stream: &mut ByteStream) -> Result<Self, InvalidValue> {
        let n: usize = AntelopeValue::from_bin(AntelopeType::Uint64, stream)?.try_into()?;
        Ok(Name::from_u64(n as u64))
    }
}


impl ABISerializable for Symbol {
    fn encode(&self, stream: &mut ByteStream) {
        stream.write_u64(self.as_u64());
        // AntelopeValue::Uint64(self.as_u64()).to_bin(stream);
    }

    fn decode(stream: &mut ByteStream) -> Result<Self, InvalidValue> {
        let n: usize = AntelopeValue::from_bin(AntelopeType::Uint64, stream)?.try_into()?;
        Ok(Symbol::from_u64(n as u64))
    }
}


impl ABISerializable for Asset {
    fn encode(&self, stream: &mut ByteStream) {
        self.amount().encode(stream);
        // AntelopeValue::Int64(self.amount).to_bin(stream);
        self.symbol().encode(stream);
    }

    fn decode(stream: &mut ByteStream) -> Result<Self, InvalidValue> {
        let amount: i64 = AntelopeValue::from_bin(AntelopeType::Int64, stream)?.try_into()?;
        let symbol = Symbol::decode(stream)?;
        Ok(Asset::new(amount, symbol))
    }
}


impl<T: CryptoDataType, const DATA_SIZE: usize> ABISerializable for CryptoData<T, DATA_SIZE> {
    fn encode(&self, stream: &mut ByteStream) {
        stream.write_byte(self.key_type().index());
        stream.write_bytes(self.data());
    }

    fn decode(stream: &mut ByteStream) -> Result<Self, InvalidValue> {
        let key_type = KeyType::from_index(stream.read_byte()?);
        let data = stream.read_bytes(DATA_SIZE)?.try_into().unwrap();
        Ok(Self::new(key_type, data))
    }


}

impl ABISerializable for AntelopeValue {
    fn encode(&self, _stream: &mut ByteStream) {
        unimplemented!();
    }

    fn decode(_stream: &mut ByteStream) -> Result<Self, InvalidValue> {
        unimplemented!();
    }

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

    #[tracing::instrument]
    fn from_bin(typename: AntelopeType, stream: &mut ByteStream) -> Result<Self, InvalidValue> {
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
