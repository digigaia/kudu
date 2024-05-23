use std::str::from_utf8;

use antelope_core::{
    types::crypto::{CryptoData, CryptoDataType, KeyType},
    Asset, InvalidValue, Name, Symbol,
    types::antelopevalue::InvalidDataSnafu,
};
use bytemuck::pod_read_unaligned;
use snafu::ensure;

use crate::bytestream::ByteStream;


pub trait BinarySerializable {
    fn encode(&self, stream: &mut ByteStream);
    fn decode(stream: &mut ByteStream) -> Result<Self, InvalidValue>
    where
        Self: Sized; // FIXME: this should be a different Error type
}


// TODO: implement for other int/uint types

impl BinarySerializable for i64 {
    fn encode(&self, stream: &mut ByteStream) {
        stream.write_i64(*self)
    }
    fn decode(stream: &mut ByteStream) -> Result<Self, InvalidValue> {
        Ok(pod_read_unaligned(stream.read_bytes(8)?))
    }
}

impl BinarySerializable for u64 {
    fn encode(&self, stream: &mut ByteStream) {
        stream.write_u64(*self)
    }
    fn decode(stream: &mut ByteStream) -> Result<Self, InvalidValue> {
        Ok(pod_read_unaligned(stream.read_bytes(8)?))
    }
}

impl BinarySerializable for Name {
    fn encode(&self, stream: &mut ByteStream) {
        stream.write_u64(self.as_u64());
    }

    fn decode(stream: &mut ByteStream) -> Result<Self, InvalidValue> {
        let n = u64::decode(stream)?;
        Ok(Name::from_u64(n))
    }
}


impl BinarySerializable for Symbol {
    fn encode(&self, stream: &mut ByteStream) {
        stream.write_u64(self.as_u64());
    }

    fn decode(stream: &mut ByteStream) -> Result<Self, InvalidValue> {
        let n = u64::decode(stream)?;
        Ok(Symbol::from_u64(n))
    }
}


impl BinarySerializable for Asset {
    fn encode(&self, stream: &mut ByteStream) {
        self.amount().encode(stream);
        self.symbol().encode(stream);
    }

    fn decode(stream: &mut ByteStream) -> Result<Self, InvalidValue> {
        let amount = i64::decode(stream)?;
        let symbol = Symbol::decode(stream)?;
        Ok(Asset::new(amount, symbol))
    }
}


impl<T: CryptoDataType, const DATA_SIZE: usize> BinarySerializable for CryptoData<T, DATA_SIZE> {
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

pub fn read_var_u32(stream: &mut ByteStream) -> Result<u32, InvalidValue> {
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

pub fn read_var_i32(stream: &mut ByteStream) -> Result<i32, InvalidValue> {
    let n = read_var_u32(stream)?;
    Ok(match n & 1 {
        0 => n >> 1,
        _ => ((!n) >> 1) | 0x8000_0000,
    } as i32)
}

pub fn read_bytes(stream: &mut ByteStream) -> Result<Vec<u8>, InvalidValue> {
    let len = read_var_u32(stream)? as usize;
    Ok(Vec::from(stream.read_bytes(len)?))
}

pub fn read_str(stream: &mut ByteStream) -> Result<&str, InvalidValue> {
    let len = read_var_u32(stream)? as usize;
    Ok(from_utf8(stream.read_bytes(len)?)?)
}
