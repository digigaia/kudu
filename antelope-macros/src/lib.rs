use proc_macro::TokenStream;

#[cfg(feature = "detailed-error")]
use syn::{parse_macro_input, ItemEnum};

#[cfg(feature = "detailed-error")]
mod error;

#[cfg(feature = "detailed-error")]
use crate::error::add_location_to_error_enum;

// FIXME: add those comments in a module level doc (or on the `with_location` macro)
// FIXME: print a proper error if we already have a field named `location`
// FIXME: print a proper error if we don't define a display string
// FIXME: do not use `node.parse_nested_meta(|meta| {})` on the `snafu` attribute,
//        as some nested attrs can't be parsed like this, use `parse_args` instead
// FIXME: cannot use location or source with Whatever, see: https://docs.rs/snafu/latest/snafu/struct.Whatever.html#limitations

#[cfg(feature = "detailed-error")]
#[proc_macro_attribute]
pub fn with_location(_input: TokenStream, annotated_item: TokenStream) -> TokenStream {
    let error_enum = parse_macro_input!(annotated_item as ItemEnum);
    add_location_to_error_enum(error_enum).into()
}

#[cfg(not(feature = "detailed-error"))]
#[proc_macro_attribute]
pub fn with_location(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
