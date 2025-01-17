//! This module contains macros to be used in the `antelope` crates.
//!
//! ## Feature flags
//!
//! - `detailed-error`: activate this to enable the [`macro@with_location`] macro. If
//!   not enabled, the [`macro@with_location`] macro will be a no-op.

mod attr;
mod serde;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

#[cfg(feature = "detailed-error")]
use syn::ItemEnum;

#[cfg(feature = "detailed-error")]
mod error;

#[cfg(feature = "detailed-error")]
use crate::error::add_location_to_error_enum;


/// Add a `location` field to all variants of a `Snafu` error enum
///
/// This will add the `location` field to all variants, which need to be either
/// structs or the unit type (tuple variants are not allowed).
/// The location field will be automatically populated when using the error selector.
///
/// This macro will also update the display string (if defined) to also show the
/// location that has been captured.
///
/// Adding the `location` field to an error enum will increase its size by 32 bytes.
///
/// **NOTE:** you cannot use a `whatever` variant in conjunction with this, nor can you
///           manually define the `location` field yourself (it will conflict with the
///           generated one).
#[proc_macro_attribute]
pub fn with_location(attr: TokenStream, annotated_item: TokenStream) -> TokenStream {
    with_location_impl(attr, annotated_item)
}


#[cfg(feature = "detailed-error")]
fn with_location_impl(_attr: TokenStream, annotated_item: TokenStream) -> TokenStream {
    let error_enum = parse_macro_input!(annotated_item as ItemEnum);
    add_location_to_error_enum(error_enum).into()
}

#[cfg(not(feature = "detailed-error"))]
fn with_location_impl(_attr: TokenStream, annotated_item: TokenStream) -> TokenStream {
    annotated_item
}


/// Implement the `antelope::BinarySerializable` trait
///
/// This simply calls `BinarySerializable::encode()` and `BinarySerializable::decode()`
/// on all members sequentially.
#[proc_macro_derive(BinarySerializable)]
pub fn derive_binaryserializable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    serde::derive(&input).into()
}

/// Implement the `serde::Serialize` and `serde::Deserialize` trait
///
/// Antelope blockchains expect enums (variant types) to be encoded as a
/// tuple of `(discriminant, value)` which is not natively supported by `serde`,
/// so this macro fills in the gap and should be used instead of
/// `#[derive(Serialize, Deserialize)]` for enum types. By default the discriminant
/// is serialized as a `snake_case` string.
///
/// It only exposes one attribute argument for fields which is `serde(rename)`.
#[proc_macro_derive(SerializeEnum, attributes(serde))]
pub fn derive_serialize_enum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    serde::derive_serialize_enum(&input).into()
}
