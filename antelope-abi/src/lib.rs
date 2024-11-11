//!
//! This crate provides tools to encode/decode `Antelope` types into/from an ABI.
//!

// disable this lint to allow our types to implement a `from_str` constructor
// without implement the `std::str::FromStr` trait
// if we didn't, we would have to import that trait everywhere we want to build
// our types, which wouldn't be very convenient and isn't very discoverable
#![allow(clippy::should_implement_trait)]


pub mod abi;
pub mod abidefinition;
pub mod abiserializable;
pub mod binaryserializable;
pub mod bytestream;
pub mod provider;
pub mod data;
pub mod typenameref;

pub use abi::ABI;
pub use abidefinition::{ABIDefinition, ABIError};
pub use abiserializable::ABISerializable;
pub use bytestream::{ByteStream, StreamError};
pub use binaryserializable::{BinarySerializable, SerializeError};
pub use provider::ABIProvider;
