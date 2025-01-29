//! This module contains macros to be used in the `antelope` crates.
//!
//! ## Feature flags
//!
//! - `detailed-error`: activate this to enable the [`macro@with_location`] macro. If
//!   not enabled, the [`macro@with_location`] macro will be a no-op.

mod attr;
mod contract;
mod serde;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, ItemStruct, MetaNameValue, punctuated::Punctuated};

#[cfg(feature = "detailed-error")]
use syn::ItemEnum;

#[cfg(feature = "detailed-error")]
mod error;

#[cfg(feature = "detailed-error")]
use crate::error::add_location_to_error_enum;


// the `antelope` crate re-exports this macro and adds documentation to it
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


// the `antelope` crate re-exports this macro and adds documentation to it
#[proc_macro_attribute]
pub fn contract(attr: TokenStream, annotated_item: TokenStream) -> TokenStream {
    let attrs = parse_macro_input!(attr with Punctuated::<MetaNameValue, syn::Token![,]>::parse_terminated);
    let contract_struct = parse_macro_input!(annotated_item as ItemStruct);
    contract::add_contract_trait_impl(attrs, contract_struct).into()
}

// the `antelope` crate re-exports this macro and adds documentation to it
#[proc_macro_derive(BinarySerializable)]
pub fn derive_binaryserializable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    serde::derive(&input).into()
}

// the `antelope` crate re-exports this macro and adds documentation to it
#[proc_macro_derive(SerializeEnum, attributes(serde))]
pub fn derive_serialize_enum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    serde::derive_serialize_enum(&input, false).into()
}

// the `antelope` crate re-exports this macro and adds documentation to it
#[proc_macro_derive(SerializeEnumPrefixed, attributes(serde))]
pub fn derive_serialize_enum_prefixed(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    serde::derive_serialize_enum(&input, true).into()
}
