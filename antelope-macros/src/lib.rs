//! This module contains macros to be used in the `antelope` crates.

use proc_macro::TokenStream;

#[cfg(feature = "detailed-error")]
use syn::{parse_macro_input, ItemEnum};

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
/// It will also update the display string (if defined) to also show the location
/// that has been captured.
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
