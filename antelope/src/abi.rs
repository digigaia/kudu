//!
//! This module provides tools to encode/decode `Antelope` types into/from an ABI.
//!
//! ## Feature flags
//!
//! - `hardened`: implement safeguards to check for execution time and recursion depth
//!               (NOT IMPLEMENTED YET!)
//! - `float128`: add support for the `float128` type, needs a nightly Rust version
//!               as `f128` support in is still experimental


pub mod abi;
pub mod abidefinition;
pub mod abiserializer;
pub mod provider;
pub mod data;

pub use abi::{ABI, ABIError};
pub use abidefinition::ABIDefinition;
pub use provider::ABIProvider;
