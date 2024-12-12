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
}
