//!
//! This crate provides tools to encode/decode `Antelope` types into/from an ABI.
//!

// disable this lint to allow our types to implement a `from_str` constructor
// without implement the `std::str::FromStr` trait
// if we didn't, we would have to import that trait everywhere we want to build
// our types, which wouldn't be very convenient and isn't very discoverable
#![allow(clippy::should_implement_trait)]


pub mod abi;
pub mod abiencoder;
pub mod bytestream;
pub mod abiserializable;
pub mod binaryserializable;

pub use bytestream::{ByteStream, StreamError};
pub use abiencoder::ABIEncoder;
