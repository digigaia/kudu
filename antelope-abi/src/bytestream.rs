use std::mem;
use std::num::ParseIntError;

use bytemuck::cast_ref;
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

// TODO: we could provide default impl for u16, u32, etc. using only write_byte
/*
pub trait ByteStream {
    fn write_byte(&mut self, byte: u8);

    fn write_u8(&mut self, n: u8);
    fn write_u16(&mut self, n: u16);
    fn write_u32(&mut self, n: u32);
    fn write_u64(&mut self, n: u64);
    fn write_u128(&mut self, n: u128);

    fn write_i8(&mut self, n: i8);
    fn write_i16(&mut self, n: i16);
    fn write_i32(&mut self, n: i32);
    fn write_i64(&mut self, n: i64);
    fn write_i128(&mut self, n: i128);

    fn write_var_u32(&mut self, n: u32);
    fn write_str(&mut self, s: &str);
}
*/

#[derive(Default)]
pub struct ByteStream {
    // this should/could? also be made generic using the std::io::Write trait
    // or maybe use the `bytes` crate to have an efficient copy-on-write bytes strucs?
    data: Vec<u8>,

    read_pos: usize,
}

impl From<ByteStream> for Vec<u8> {
    fn from(ds: ByteStream) -> Vec<u8> {
        ds.data
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

    pub fn pop(&mut self) -> Vec<u8> {
        mem::take(&mut self.data)
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn hex_data(&self) -> String {
        hex::encode_upper(&self.data)
    }


    pub fn write_byte(&mut self, byte: u8) {
        self.data.push(byte)
    }

    pub fn leftover(&self) -> &[u8] {
        &self.data[self.read_pos..]
    }

    pub fn read_byte(&mut self) -> Result<u8, StreamError> {
        let pos = self.read_pos;
        ensure!(pos != self.data.len(), EndedSnafu { wanted: 1_usize, available: 0_usize });

        trace!("read 1 byte - hex: {}", hex::encode_upper(&self.data[pos..pos + 1]));
        self.read_pos += 1;
        Ok(self.data[pos])
    }

    pub fn read_bytes(&mut self, n: usize) -> Result<&[u8], StreamError> {
        let available = self.data.len() - self.read_pos;
        ensure!(n <= available, EndedSnafu { wanted: n, available });

        let result = &self.data[self.read_pos..self.read_pos + n];
        trace!("read {n} bytes - hex: {}", hex::encode_upper(result));
        self.read_pos += n;
        Ok(result)
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.data.extend_from_slice(bytes)
    }

    ////////////////
    // FIXME FIXME
    //
    // deprecate everything that's under this line

    // pub fn write_bool(&mut self, b: bool) {
    //     self.write_byte(match b {
    //         true => 1,
    //         false => 0,
    //     });
    // }

    pub fn write_u8(&mut self, n: u8) {
        self.write_byte(n);
    }


    pub fn write_u16(&mut self, n: u16) {
        self.data.extend_from_slice(cast_ref::<u16, [u8; 2]>(&n));
    }

    pub fn write_u32(&mut self, n: u32) {
        self.data.extend_from_slice(cast_ref::<u32, [u8; 4]>(&n));
    }

    pub fn write_u64(&mut self, n: u64) {
        self.data.extend_from_slice(cast_ref::<u64, [u8; 8]>(&n));
    }

    pub fn write_u128(&mut self, n: u128) {
        self.data.extend_from_slice(cast_ref::<u128, [u8; 16]>(&n));
    }

    pub fn write_i8(&mut self, n: i8) {
        // FIXME: check that this is correct
        self.data.push(n as u8);
    }

    pub fn write_i16(&mut self, n: i16) {
        self.data.extend_from_slice(cast_ref::<i16, [u8; 2]>(&n));
    }

    pub fn write_i32(&mut self, n: i32) {
        self.data.extend_from_slice(cast_ref::<i32, [u8; 4]>(&n));
    }

    pub fn write_i64(&mut self, n: i64) {
        self.data.extend_from_slice(cast_ref::<i64, [u8; 8]>(&n));
    }

    pub fn write_i128(&mut self, n: i128) {
        self.data.extend_from_slice(cast_ref::<i128, [u8; 16]>(&n));
    }

    pub fn write_f32(&mut self, x: f32) {
        self.data.extend_from_slice(cast_ref::<f32, [u8; 4]>(&x));
    }

    pub fn write_f64(&mut self, x: f64) {
        self.data.extend_from_slice(cast_ref::<f64, [u8; 8]>(&x));
    }
}
