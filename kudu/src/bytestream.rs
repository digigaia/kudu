// SPDX-FileCopyrightText: 2023-2026 DigiGaia SCCL
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::num::ParseIntError;

use hex;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use snafu::{ensure, Snafu};
use tracing::trace;

use kudu_macros::with_location;


#[with_location]
#[derive(Debug, Snafu)]
pub enum StreamError {
    #[snafu(display("stream ended, tried to read {wanted} byte(s) but only {available} available"))]
    Ended { wanted: usize, available: usize },

    #[snafu(display("invalid hex character"))]
    InvalidHexChar { source: ParseIntError },

    #[snafu(display("odd number of chars in hex representation"))]
    OddLength,

    #[snafu(display("varint too long to fit in u32"))]
    InvalidVarInt,
}


/// Provide access to a byte stream along with a cursor to read into it.
/// This is a non-owning type, the string type equivalent would be `&str`.
///
/// This is different that both `std::io::Read`/`std::io::Write` and the `bytes`
/// crate as this is supposed to be used for reading from files/streams that have
/// an end, so the `read` operation is fallible, but when writing we assume everything
/// is fine (we usually write into memory) so the `write` operation is infallible.
#[derive(Default)]
pub struct ByteStreamView<'a> {
    data: &'a [u8],

    read_pos: usize,
}

impl<'a> From<&'a [u8]> for ByteStreamView<'a> {
    fn from(data: &'a [u8]) -> Self {
        ByteStreamView { data, read_pos: 0 }
    }
}

impl<'a> From<&'a Vec<u8>> for ByteStreamView<'a> {
    fn from(data: &'a Vec<u8>) -> Self {
        ByteStreamView { data: data.as_ref(), read_pos: 0 }
    }
}

impl<'a> ByteStreamView<'a> {
    pub fn leftover(&self) -> &[u8] {
        &self.data[self.read_pos..]
    }

    pub fn read_byte(&mut self) -> Result<u8, StreamError> {
        let pos = self.read_pos;
        ensure!(pos != self.data.len(), EndedSnafu { wanted: 1_usize, available: 0_usize });

        trace!("read 1 byte - hex: {}", hex::encode(&self.data[pos..pos + 1]));
        self.read_pos += 1;
        Ok(self.data[pos])
    }

    pub fn read_bytes(&mut self, n: usize) -> Result<&[u8], StreamError> {
        let available = self.data.len() - self.read_pos;
        ensure!(n <= available, EndedSnafu { wanted: n, available });

        let result = &self.data[self.read_pos..self.read_pos + n];
        trace!("read {n} bytes - hex: {}", hex::encode(result));
        self.read_pos += n;
        Ok(result)
    }

    pub fn read_var_u32(&mut self) -> Result<u32, StreamError> {
        let mut offset = 0;
        let mut result = 0;
        loop {
            let byte = self.read_byte()?;
            result |= (byte as u32 & 0x7F) << offset;
            offset += 7;
            if (byte & 0x80) == 0 { break; }

            ensure!(offset < 32, InvalidVarIntSnafu);
        }
        Ok(result)
    }

    pub fn read_var_i32(&mut self) -> Result<i32, StreamError> {
        let n = self.read_var_u32()?;
        Ok(match n & 1 {
            0 => n >> 1,
            _ => ((!n) >> 1) | 0x8000_0000,
        } as i32)
    }
}

// FIXME: rename ByteStream to ByteBuffer
//        actually: use Bytes instead of ByteStream for writing data, and only
//                  ByteStreamView (rename to ByteStream then) for reading

/// Provide access to a byte stream along with a cursor to read into it.
/// This is an owning type, the string type equivalent would be `String`.
///
/// This is different that both `std::io::Read`/`std::io::Write` and the `bytes`
/// crate as this is supposed to be used for reading from files/streams that have
/// an end, so the `read` operation is fallible, but when writing we assume everything
/// is fine (we usually write into memory) so the `write` operation is infallible.
#[derive(Default)]
pub struct ByteStream {
    data: Vec<u8>,

    read_pos: usize,
}

impl<'a> From<&'a ByteStream> for ByteStreamView<'a> {
    fn from(stream: &'a ByteStream) -> ByteStreamView<'a> {
        ByteStreamView { data: &stream.data, read_pos: stream.read_pos }
    }
}

impl From<ByteStream> for Vec<u8> {
    fn from(stream: ByteStream) -> Vec<u8> {
        stream.data
    }
}

impl From<Vec<u8>> for ByteStream {
    fn from(data: Vec<u8>) -> Self {
        Self { data, read_pos: 0 }
    }
}

impl From<Bytes> for ByteStream {
    fn from(data: Bytes) -> Self {
        Self { data: data.0, read_pos: 0 }
    }
}

impl From<ByteStream> for Bytes {
    fn from(stream: ByteStream) -> Bytes {
        Bytes(stream.data)
    }
}

impl ByteStream {
    pub fn new() -> Self {
        Self {
            data: vec![],
            read_pos: 0,
        }
    }
}



#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Bytes(Vec<u8>);

impl Bytes {
    pub fn new() -> Self { Bytes(vec![]) }

    pub fn from_hex<T: AsRef<[u8]>>(data: T) -> Result<Bytes, hex::FromHexError> {
        Ok(Bytes(hex::decode(data)?))
    }

    pub fn to_hex(&self) -> String {
        hex::encode(&self.0)
    }

    pub fn clear(&mut self) {
        self.0.clear()
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn view<'a>(&'a self) -> ByteStreamView<'a> {
        self.into()
    }


    #[inline]
    pub fn write_byte(&mut self, byte: u8) {
        self.0.push(byte)
    }

    #[inline]
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.0.extend_from_slice(bytes)
    }

    pub fn write_var_u32(&mut self, n: u32) {
        let mut n = n;
        loop {
            if n >> 7 != 0 {
                self.write_byte((0x80 | (n & 0x7f)) as u8);
                n >>= 7
            }
            else {
                self.write_byte(n as u8);
                break;
            }
        }
    }

    pub fn write_var_i32(&mut self, n: i32) {
        let unsigned = ((n as u32) << 1) ^ ((n >> 31) as u32);
        self.write_var_u32(unsigned)
    }

}

impl From<Vec<u8>> for Bytes {
    fn from(v: Vec<u8>) -> Bytes {
        Bytes(v)
    }
}

impl From<&[u8]> for Bytes {
    fn from(s: &[u8]) -> Bytes {
        Bytes(s.to_vec())
    }
}

impl<const N: usize> From<&[u8; N]> for Bytes {
    fn from(s: &[u8; N]) -> Bytes {
        Bytes(s.to_vec())
    }
}

// This should probably be using Bytes::from_hex if we define it, however this
// conversion is probably prone to error so we don't define it for now unless
// a good reason comes up that we should
// impl From<&str> for Bytes {
//     fn from(s: &str) -> Bytes {
//         Bytes(s.as_bytes().to_vec())
//     }
// }

impl From<Bytes> for Vec<u8> {
    fn from(b: Bytes) -> Vec<u8> {
        b.0
    }
}

impl AsRef<[u8]> for Bytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl<'a> From<&'a Bytes> for ByteStreamView<'a> {
    fn from(data: &'a Bytes) -> ByteStreamView<'a> {
        ByteStreamView { data: data.as_bytes(), read_pos: 0 }
    }
}


impl Serialize for Bytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        self.to_hex().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Bytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let hex_repr: &str = <&str>::deserialize(deserializer)?;
        Bytes::from_hex(hex_repr).map_err(|e| de::Error::custom(e.to_string()))
    }
}
