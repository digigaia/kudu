//!
//! This crate provides tools to encode/decode `Antelope` types into/from an ABI.
//!
//! ## Feature flags
//!
//! - `hardened`: implement safeguards to check for execution time and recursion depth
//!               (NOT IMPLEMENTED YET!)
//! - `float128`: add support for the `float128` type, needs a nightly Rust version
//!               as `f128` support in is still experimental

#![doc = include_str!("../TODO.md")]

// disable this lint to allow our types to implement a `from_str` constructor
// without implement the `std::str::FromStr` trait
// if we didn't, we would have to import that trait everywhere we want to build
// our types, which wouldn't be very convenient and isn't very discoverable
#![allow(clippy::should_implement_trait)]

#![cfg_attr(feature = "float128", feature(f128))]


pub mod abi;
pub mod abidefinition;
pub mod abiserializable;
pub mod abiserializer;
pub mod binaryserializable;
pub mod bytestream;
pub mod provider;
pub mod data;
pub mod typenameref;

pub use abi::{ABI, ABIError};
pub use abidefinition::ABIDefinition;
pub use abiserializable::ABISerializable;
pub use bytestream::{ByteStream, StreamError};
pub use binaryserializable::{BinarySerializable, SerializeError};
pub use provider::ABIProvider;
pub use typenameref::TypeNameRef;
