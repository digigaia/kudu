//!
//! This library provides data types and functions to interact with
//! [Antelope](https://antelope.io) blockchains.
//!
//! The basic types can be found in the [`types`] module, and the variant type used to represent
//! values handled by Antelope blockchains is [`AntelopeValue`].
//!
//! On top of each module documentation, the following pages might be of interest:
//!
//! - [To-Do list](doc::todo): to-do list of items needing to be completed before a first release
//! - [Notes](doc::notes): list of notes and resources
//! - [Architecture](doc::architecture): overview of the architecture and design decisions
//!
//! # Feature flags
//!
//! - `cli`: whether to compile the command-line tools alongside the library.
//!          This feature is enabled by default and currently installs the `kuduconv` tool.
//! - `detailed-error`: activate this to enable the [`macro@with_location`] macro. If
//!                     not enabled, the [`macro@with_location`] macro will be a no-op.
//! - `hardened`: implement safeguards to check for execution time and recursion depth
//!               when validating ABIs. (NOT IMPLEMENTED YET!)
//! - `float128`: add support for a native `float128` type. This currently needs a nightly Rust
//!               version as `f128` support is still experimental. If this is not active,
//!               the `Float128` will still be available but as a `[u8; 16]` wrapper.
//!
//!
//! # Antelope data model
//!
//! ![Antelope data model][datamodel]
//!
#![cfg_attr(
    all(),
    doc = ::embed_doc_image::embed_image!("datamodel", "src/doc/antelope_data_model.drawio.svg")
)]
//!
//! Data used in the Antelope blockchains can be found in a variety of formats, namely:
//!  - Rust native data types (structs defined in this library)
//!  - JSON Value (`serde_json::Value`) (also called variant in Antelope terminology)
//!  - JSON string representation
//!  - binary data
//!
//! The diagram above shows those types and the different ways to convert between them.
//!  - most of the conversions are handled via the `serde::Serialize` and `serde::Deserialize`
//!    traits.
//!  - to convert between a JSON value and a binary stream you need to use an instance
//!    of the [`ABI`] class which has been initialized with a data schema
//!    ([`ABIDefinition`]).
//!  - to convert between a Rust native value and a binary stream you need to use the
//!    [`ABISerializable`] trait, which you can automatically derive using the
//!    [`ABISerializable`](macro@ABISerializable) derive macro.
//!
//! ## Traits implemented for native types
//!
//! Wherever possible, the following traits are implemented for the base types:
//!  - [`Clone`], and also [`Copy`] when the struct size isn't prohibitively big
//!  - [`Debug`](std::fmt::Debug) and [`Display`](std::fmt::Display)
//!  - [`FromStr`](std::str::FromStr), allowing most types to be constructed from their `str`
//!    representation via [`str::parse()`]
//!  - [`PartialEq`], [`Eq`], [`PartialOrd`], [`Ord`], [`Hash`]
//!
//! <div class="warning">
//!
//! ## Warnings / pitfalls
//!
//!  - when defining your own types, if you have a bytes field make sure to use the [`Bytes`]
//!    type instead of `Vec<u8>` otherwise the JSON serialization will not be correct.
//!  - when defining a variant type using a Rust enum, you need to use the [`SerializeEnum`]
//!    derive macro instead of `serde::Serialize` and `serde::Deserialize`. This is because
//!    the discriminant needs to be encoded in a specific way which cannot be achieved with
//!    the `serde::Serialize` trait.
//!
//! </div>
//!
//! # `kuduconv` CLI tool
//!
//! The `kuduconv` tool provides JSON <> hex conversion functionality, similar to what the
//! `abieos` tools `generate_hex_from_json` and `generate_json_from_hex` do.
//!
//! Example:
//! ```sh
//! $ kuduconv -h  # always nice to read the help to know what a program can do!
//!
//! $ kuduconv to-hex --abi token.abi transfer {"from": "useraaaaaaaa", "to": "useraaaaaaab", "quantity": "0.0001 SYS", "memo": ""}
//! ```
//!
//! # Differences between this library and the Antelope C++ library
//!
//!  - hex numbers here are lowercase whereas C++ outputs hex data in upper case
//!  - C++ outputs `i64` and `u64` as double-quoted, this library doesn't



// disable this lint to allow our types to implement a `from_str` constructor
// without implement the `std::str::FromStr` trait
// if we didn't, we would have to import that trait everywhere we want to build
// our types, which wouldn't be very convenient and isn't very discoverable
#![allow(clippy::should_implement_trait)]

#![cfg_attr(feature = "float128", feature(f128))]

#[cfg(not(doctest))]
pub mod doc;

pub mod abi;
pub mod api;
pub mod chain;
pub mod config;
pub mod convert;
pub mod macros;
pub mod json;
pub mod types;

// FIXME: check whether we want those typedefs? Does it make it easier or
// does it obscure where those types are coming from?
// maybe move them inside the `kudu::json` module?
pub use serde_json::{
    Map as JsonMap,
    Value as JsonValue,
    Error as JsonError,
    json
};

pub use api::APIClient;

pub use types::*;
pub use chain::*;

pub use abi::{ABI, ABIError, ABIDefinition, TypeName};

pub mod abiserializable;
pub mod bytestream;

pub use bytestream::{ByteStream, StreamError};
pub use abiserializable::{ABISerializable, SerializeError, to_bin, to_hex, from_bin};

/// Add a `location` field to all variants of a `Snafu` error enum
///
/// This will add the `location` field to all variants, which need to be either
/// structs or the unit type (tuple variants are not allowed).
/// The location field will be automatically populated when using the error selector.
///
/// This macro will also update the display string (if defined) to also show the
/// location that has been captured.
///
/// **NOTE:** Adding the `location` field to an error enum will increase its size by
///           32 bytes, and an additional 32 bytes for each variant that contains a
///           `source` field (as this latter also has the extra size), recursively.
///           This might become expensive quite quickly, that's why the corresponding
///           feature isn't enabled by default.
///
/// **NOTE:** you cannot use a `whatever` variant in conjunction with this, nor can you
///           manually define the `location` field yourself (it will conflict with the
///           generated one).
pub use kudu_macros::with_location;

/// Attribute macro to easily declare structs representing contract actions.
///
/// This implements the [`Contract`] trait so that this struct can be used where
/// [`Action`]s are expected.
///
/// # Example
///
/// ```
/// # use kudu::{Name, Asset, ABISerializable, contract};
/// #[contract(account="eosio.token", name="transfer")]
/// #[derive(ABISerializable)]
/// pub struct Transfer {
///     pub from: Name,
///     pub to: Name,
///     pub quantity: Asset,
///     pub memo: String,
/// }
/// ```
pub use kudu_macros::contract;

/// Implement the [`ABISerializable`](trait@ABISerializable) trait
///
/// This calls [`ABISerializable::to_bin()`] and [`ABISerializable::from_bin()`]
/// on all members sequentially.
pub use kudu_macros::ABISerializable;

/// Implement the `serde::Serialize` and `serde::Deserialize` trait
///
/// Antelope blockchains expect enums (variant types) to be encoded as a
/// tuple of `(discriminant, value)` which is not natively supported by `serde`,
/// so this macro fills in the gap and should be used instead of
/// `#[derive(Serialize, Deserialize)]` for enum types. By default the discriminant
/// is serialized as a `snake_case` string.
///
/// It exposes one attribute argument for fields which is `serde(rename)`.
pub use kudu_macros::SerializeEnum;

/// Implement the `serde::Serialize` and `serde::Deserialize` trait
///
/// This version of the macro generates string tags which are composed of the
/// discriminant name prefixed with the enum name.
///
/// Antelope blockchains expect enums (variant types) to be encoded as a
/// tuple of `(discriminant, value)` which is not natively supported by `serde`,
/// so this macro fills in the gap and should be used instead of
/// `#[derive(Serialize, Deserialize)]` for enum types. By default the discriminant
/// is serialized as a `snake_case` string.
///
/// It exposes one attribute argument for fields which is `serde(rename)`.
pub use kudu_macros::SerializeEnumPrefixed;
