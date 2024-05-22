use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parenthesized, parse_macro_input, Attribute, Fields, FieldsNamed, ItemEnum, LitStr, Meta, Variant, Field
};
use syn::visit_mut::{self, VisitMut};


// TODO: use proc_macro_attribute from proc_macro2 (?)
// FIXME: print a proper error if we already have a field named `location`
// FIXME: print a proper error if we don't define a display string


fn location_field() -> Field {
    let fs: FieldsNamed = syn::parse_str("{ #[snafu(implicit)] location: snafu::Location }").unwrap();
    let location_field = &fs.named[0];
    location_field.clone()
}

// =============================================================================
//
//     Visitor for adding a `location` field to all Enum variants
//
// =============================================================================

struct AddLocationField;

impl VisitMut for AddLocationField {
    fn visit_variant_mut(&mut self, node: &mut Variant) {
        match &mut node.fields {
            Fields::Named(ref mut fields) => {
                fields.named.push(location_field());
            },
            _ => {
                panic!("variant '{}' needs to be a struct type to be able to add `location` to it!", &node.ident);
            }

        }
    }
}

// =============================================================================
//
//     Visitor for adding the location an error was constructed to the display
//     string associated with a given variant
//
// =============================================================================

struct AddLocationToDisplay;

impl VisitMut for AddLocationToDisplay {
    fn visit_attribute_mut(&mut self, node: &mut Attribute) {

        // println!("+++ visiting attr: {:#?}", node.path().get_ident().unwrap().to_string());

        // FIXME: this doesn't work if we have more than 1 meta attribute which is `display`
        // we will drop the others when reconstructing the meta tokens

        if node.path().is_ident("snafu") {
            let mut disp_str: Option<String> = None;

            node.parse_nested_meta(|meta| {
                if meta.path.is_ident("display") {
                    let content;
                    parenthesized!(content in meta.input);
                    let lit: LitStr = content.parse()?;

                    disp_str = Some(lit.value());

                    Ok(())
                }
                else {
                    let msg = format!("unrecognized attr for snafu: `{:?}`", meta.path.get_ident());
                    println!("{}", msg);
                    // Err(meta.error(msg))
                    Ok(())
                }
            }).unwrap();

            if let Some(disp) = disp_str {
                // println!("=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=");
                // println!("found disp str while visiting: {disp}");
                // println!("NODE = {:#?}", node);

                let new_disp = format!("{disp} (at: {{location}})");
                let new_display_attr = format!(r#"display("{new_disp}")"#);

                match &mut node.meta {
                    Meta::List(ref mut snafu_display) => {
                        let new_tokens: TokenStream2 = new_display_attr.parse().unwrap();
                        // println!("OLD: {:?}", &snafu_display.tokens);
                        // println!("NEW: {:?}", &new_tokens);
                        snafu_display.tokens = new_tokens;
                    },
                    _ => unreachable!()
                }
            }

        }

        // Delegate to the default impl to visit nested expressions.
        visit_mut::visit_attribute_mut(self, node);
    }
}



#[proc_macro_attribute]
pub fn with_location(_input: TokenStream, annotated_item: TokenStream) -> TokenStream {
    let mut error_enum = parse_macro_input!(annotated_item as ItemEnum);

    AddLocationToDisplay.visit_item_enum_mut(&mut error_enum);
    AddLocationField.visit_item_enum_mut(&mut error_enum);

    quote! { #error_enum }.into()
}

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
