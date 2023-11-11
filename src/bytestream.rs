use std::fmt::Write;
use std::mem;
use std::num::ParseIntError;

use bytemuck::cast_ref;
use thiserror::Error;

use crate::AntelopeType;


#[derive(Error, Debug)]
pub enum StreamError {
    #[error("stream ended")]
    Ended,

    #[error("invalid hex character")]
    InvalidHexChar(#[from] ParseIntError),

    #[error("odd number of chars in hex representation")]
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

pub struct ByteStream {
    // this should/could? also be made generic using the std::io::Write trait
    data: Vec<u8>,

    read_pos: usize,
}

impl ByteStream {
    pub fn new() -> Self {
        Self {
            data: vec![],
            read_pos: 0,

        }
    }

    pub fn data(&self) -> &[u8] {
        self.data.as_slice()
    }

    pub fn pop(&mut self) -> Vec<u8> {
        mem::replace(&mut self.data, vec![])
        // let mut result: Vec<u8> = vec![];
        // (self.data, result) = (result, self.data);
        // result
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn hex_data(&self) -> String {
        bin_to_hex(&self.data)
    }



    pub fn write_byte(&mut self, byte: u8) {
        self.data.push(byte)
    }

    pub fn read_byte(&mut self) -> Result<u8, StreamError> {
        let pos = self.read_pos;
        if pos != self.data.len() {
            self.read_pos += 1; Ok(self.data[pos])
        }
        else {
            Err(StreamError::Ended)
        }
    }

    pub fn read_bytes(&mut self, n: usize) -> Result<&[u8], StreamError> {
        if self.read_pos + n > self.data.len() {
            Err(StreamError::Ended)
        }
        else {
            let result = Ok(&self.data[self.read_pos..self.read_pos+n]);
            self.read_pos += n;
            result
        }
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.data.extend_from_slice(bytes)
    }

    pub fn encode(&mut self, v: &AntelopeType) {
        v.to_bin(self)
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

    pub fn write_var_u32(&mut self, n: u32) {
        // TODO: would it be better to use the `bytemuck` create here?
        let mut n = n.clone();
        loop {
            if n >> 7 != 0 {
                self.write_byte((0x80 | (n & 0x7f)) as u8);
                n = n >> 7
            }
            else {
                self.write_byte(n as u8);
                break;
            }
        }
    }

    pub fn write_str(&mut self, s: &str) {
        self.write_var_u32(s.len() as u32);
        self.data.extend_from_slice(s.as_bytes());
    }


}


pub fn hex_to_bin(s: &str) -> Result<Vec<u8>, StreamError> {
    if s.len() % 2 != 0 {
        Err(StreamError::OddLength)
    }
    else {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.into()))
            .collect()
    }
}

pub fn bin_to_hex(data: &[u8]) -> String {
    let mut result = String::with_capacity(2 * data.len());
    for byte in data {
        write!(&mut result, "{:02x}", byte).unwrap();
    }
    result
}
