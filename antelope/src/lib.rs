//!
//! This library provides data types and functions to interact with
//! [Antelope](https://antelope.io) blockchains.
//!
//! The main type used to represent values handled by Antelope blockchains is [`AntelopeValue`]
//!

#![doc = include_str!("../../TODO.md")]

//! ----

#![doc = include_str!("../TODO.md")]

//! ----

//! # Antelope data model
//!
//! ![Antelope data model][datamodel]
//!
#![cfg_attr(
    all(),
    doc = ::embed_doc_image::embed_image!("datamodel", "doc/antelope_data_model.drawio.svg")
)]
//!
//! Data used in the Antelope blockchains can be found in a variety of formats.
//! Including this library, we can find:
//!  - Rust native data types
//!  - JSON Value (`serde_json::Value`) (also called variant in Antelope terminology)
//!  - JSON string representation
//!  - binary data
//!
//! The above diagram shows those types and the different ways to convert between them.
//!  - most of the conversions are handled via the serde `Serialize` and `Deserialize` trait,
//!    however care should be taken when deriving that trait on a Rust native enum, as
//!    the discriminant needs to be encoded in a specific way which cannot be achieved
//!    by the `serde::Serialize` trait so you need to use the `antelope::SerializeEnum`
//!    trait instead.
//!  - to convert between a JSON value and a binary stream you need to use an instance
//!    of the `antelope::ABI` class which has been initialized with a data schema
//!    (`ABIDefinition`).
//!  - to convert between a Rust native value and a binary stream you need to use the
//!    [`BinarySerializable`] trait, which you can automatically derive using the
//!    `BinarySerializable` derive macro.
//!
//! ## WARNINGS / PITFALLS
//!
//!  - when defining your own types, use the [`antelope::Bytes`] type instead of `Vec<u8>`
//!    otherwise the JSON serialization will not be correct
//!  - when defining your own types that contain a Rust enum types, use the `antelope::SerializeEnum`
//!    derive macro instead of `serde::Serialize` and `serde::Deserialize`
//! ----

#![doc = include_str!("../../NOTES.md")]

// disable this lint to allow our types to implement a `from_str` constructor
// without implement the `std::str::FromStr` trait
// if we didn't, we would have to import that trait everywhere we want to build
// our types, which wouldn't be very convenient and isn't very discoverable
#![allow(clippy::should_implement_trait)]

#![cfg_attr(feature = "float128", feature(f128))]

pub mod abi;
pub mod api;
pub mod chain;
pub mod config;
pub mod convert;
pub mod error;
pub mod json;
pub mod types;

// FIXME: check whether we want those typedefs? Does it make it easier or
// does it obscure where those types are coming from?
pub use serde_json::{
    Map as JsonMap,
    Value as JsonValue,
    Error as JsonError,
    json
};

pub use api::APIClient;

pub use types::*;
pub use chain::*;


pub use abi::*;




pub mod binaryserializable;
pub mod bytestream;
pub mod typenameref;

pub use bytestream::{ByteStream, StreamError};
pub use binaryserializable::{BinarySerializable, SerializeError};
pub use typenameref::TypeNameRef;

pub use antelope_macros::{with_location, BinarySerializable, SerializeEnum};
