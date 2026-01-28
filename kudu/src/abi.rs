//!
//! This module provides tools to encode/decode `Antelope` types into/from an ABI.
//!

mod definition;
mod error;
mod provider;
pub mod registry;
mod serializer;
mod typename;
pub mod data;

pub use definition::{ABIDefinition, abi_schema};
pub use error::ABIError;
pub use provider::ABIProvider;
// pub use registry::{get_abi, load_abi};
pub use serializer::ABI;
pub use typename::TypeName;
