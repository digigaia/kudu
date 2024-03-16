//!
//! This crate provides tools to encode/decode `Antelope` types into/from an ABI.
//!

pub mod abi;
pub mod abiencoder;
pub mod bytestream;
pub mod abiserializable;

pub use bytestream::{ByteStream, StreamError};
pub use abiencoder::ABIEncoder;
