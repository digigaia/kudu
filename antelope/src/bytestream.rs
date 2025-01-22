use std::num::ParseIntError;

use hex;
use snafu::{ensure, Snafu};
use tracing::trace;

use antelope_macros::with_location;

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
///
/// This is different that both `std::io::Read`/`std::io::Write` and the `bytes`
/// crate as this is supposed to be used for reading from files/streams that have
/// an end, so the `read` operation is fallible, but when writing we assume everything
/// is fine so the `write` operation is infallible.
#[derive(Default)]
pub struct ByteStream {
    data: Vec<u8>,

    read_pos: usize,
}

impl From<ByteStream> for Vec<u8> {
    fn from(stream: ByteStream) -> Vec<u8> {
        stream.data
    }
}

impl ByteStream {
    pub fn new() -> Self {
        Self {
            data: vec![],
            read_pos: 0,
        }
    }

    pub fn from(data: Vec<u8>) -> Self {
        Self { data, read_pos: 0 }
    }

    pub fn data(&self) -> &[u8] {
        self.data.as_slice()
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.data
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn hex_data(&self) -> String {
        hex::encode(&self.data)
    }

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

    #[inline]
    pub fn write_byte(&mut self, byte: u8) {
        self.data.push(byte)
    }

    #[inline]
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.data.extend_from_slice(bytes)
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
