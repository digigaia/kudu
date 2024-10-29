mod error;

use proc_macro::TokenStream;

#[cfg(feature = "detailed-error")]
use syn::{
    parse_macro_input, Fields, ItemEnum,
    visit_mut::VisitMut,
};

#[cfg(feature = "detailed-error")]
use quote::quote;

#[cfg(feature = "detailed-error")]
use crate::error::{AddLocationField, AddLocationToDisplay, location_field};

// FIXME: print a proper error if we already have a field named `location`
// FIXME: print a proper error if we don't define a display string
// FIXME: do not use `node.parse_nested_meta(|meta| {})` on the `snafu` attribute,
//        as some nested attrs can't be parsed like this, use `parse_args` instead


#[cfg(feature = "detailed-error")]
#[proc_macro_attribute]
pub fn with_location(_input: TokenStream, annotated_item: TokenStream) -> TokenStream {
    let mut error_enum = parse_macro_input!(annotated_item as ItemEnum);

    AddLocationToDisplay.visit_item_enum_mut(&mut error_enum);
    AddLocationField.visit_item_enum_mut(&mut error_enum);

    quote! { #error_enum }.into()
}

#[cfg(not(feature = "detailed-error"))]
#[proc_macro_attribute]
pub fn with_location(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}



#[cfg(feature = "detailed-error")]
#[proc_macro_attribute]
pub fn with_location2(_input: TokenStream, annotated_item: TokenStream) -> TokenStream {

    // println!("{:#?}", &annotated_item);

    // let ItemEnum {
    //     attrs,
    //     vis,
    //     enum_token,
    //     ident,
    //     generics,
    //     brace_token,
    //     variants
    // } = parse_macro_input!(annotated_item as ItemEnum);

    let mut ast = parse_macro_input!(annotated_item as ItemEnum);

    AddLocationToDisplay.visit_item_enum_mut(&mut ast);

    // println!("enum {} - variants: {:#?}", &ast.ident, &ast.variants);
    // println!("-----------------------------------------------");

    let variants_with_loc = &mut ast.variants;


    // let implicit_attr:Attribute = syn::parse_str("#[snafu(implicit)]").unwrap();

    for v in variants_with_loc {
        // println!("inspecting variant: {}", &v.ident);

        // TODO: fail if there is no `snafu(display())` attribute

        for attr in &mut v.attrs {
            // is our attribute a `snafu` attribute?
            if attr.path().is_ident("snafu") {
                // attr.parse_nested_meta(|meta| {
                //     if meta.path.is_ident("display") {
                //         let content;
                //         parenthesized!(content in meta.input);
                //         let lit: LitStr = content.parse()?;

                //         disp_str = Some(lit.value());

                //         attr.meta
                //     }

                //     Err(meta.error("unrecognized repr"))
                // }).unwrap();

                // match &mut attr.meta {
                //     Meta::List(ref mut meta) => {
                //         let tok = &mut meta.tokens;

                //         let toks: Vec<_> = tok.clone().into_iter().collect();

                //         // check whether we are defining the `snafu(display)` attribute
                //         match (toks.len(), &toks[0], &toks[1]) {
                //             (2, TokenTree::Ident(ident), TokenTree::Group(grp)) => {
                //                 assert!(*ident == "display");

                //                 // check that the `snafu(display())` attr contains a single string
                //                 let toks2: Vec<_> = grp.stream().clone().into_iter().collect();
                //                 match (toks2.len(), &toks2[0]) {
                //                     (1, TokenTree::Literal(lit)) => {
                //                         let disp_str = lit.to_string();

                //                         println!("=-=-=-=-=-=-=-=-=- {disp_str}");
                //                     },
                //                     _ => unimplemented!(),
                //                 }
                //             },
                //             _ => unimplemented!(),
                //         }

                //         let t0 = &toks[0];
                //     },
                //     // bad => Err(Error::new_spanned(bad, "unrecognized attribute for snafu")),
                //     _ => todo!(),
                // }
            }
        }

        match &mut v.fields {
            Fields::Named(ref mut fields) => {
                fields.named.push(location_field());
            },
            _ => {
                panic!("variant '{}' needs to be a struct type to be able to add `location` to it!", &v.ident);
            }

        }
    }

    // let with_loc = quote! {
    //     #(#attrs)*
    //     #vis enum #ident {
    //         #variants_with_loc

    //         // location: Location,
    //     }
    // };

    // with_loc.into()

    // ast.to_tokens().into()
    quote! { #ast }.into()
}
