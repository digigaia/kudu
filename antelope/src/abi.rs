//!
//! This module provides tools to encode/decode `Antelope` types into/from an ABI.
//!
//! ## Feature flags
//!
//! - `hardened`: implement safeguards to check for execution time and recursion depth
//!               (NOT IMPLEMENTED YET!)
//! - `float128`: add support for the `float128` type, needs a nightly Rust version
//!               as `f128` support in is still experimental


mod definition;
mod error;
mod provider;
mod serializer;
pub mod data;

pub use error::ABIError;
pub use serializer::ABI;
pub use definition::{ABIDefinition, abi_schema};
pub use provider::{ABIProvider, get_signing_request_abi};
