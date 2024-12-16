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
