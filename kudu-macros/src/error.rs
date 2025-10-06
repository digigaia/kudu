use proc_macro2::{TokenStream, TokenTree, Group};
use quote::quote;
use syn::{
    Attribute, Fields, FieldsNamed, LitStr, Meta, Variant, Field, ItemEnum, parse2,
    visit_mut::{
        VisitMut, visit_attribute_mut,
    }
};


/// control whether we want to have debugging information for the macro when compiling
const DEBUG: bool = false;

macro_rules! debug {
    ( $($elem:expr),* ) => { if DEBUG { eprintln!( $($elem),* ); } }
}


// =============================================================================
//
//     Visitor for adding a `location` field to all Enum variants
//
// =============================================================================

fn location_as_fields_named() -> FieldsNamed {
    syn::parse_str("{ #[snafu(implicit)] location: snafu::Location }").unwrap()
}

pub fn location_field() -> Field {
    let fs: FieldsNamed = location_as_fields_named();
    let location_field = &fs.named[0];
    location_field.clone()
}

pub struct AddLocationField;

impl VisitMut for AddLocationField {
    fn visit_variant_mut(&mut self, node: &mut Variant) {
        match &mut node.fields {
            Fields::Named(fields) => {
                if fields.named.iter().any(|f| f.ident.as_ref().unwrap() == "location") {
                    panic!("variant '{}' already defines a `location` field, please remove it so it can be added automatically",
                           &node.ident);
                }
                fields.named.push(location_field());
            },
            Fields::Unit => {
                node.fields = location_as_fields_named().into()
            },
            _ => {
                panic!("variant '{}' needs to be a struct or unit type to be able to add `location` to it!", &node.ident);
            }

        }
    }
}

// =============================================================================
//
//     Visitor for adding the location at which an error was constructed
//     to the display string associated with a given variant
//
// =============================================================================

pub struct AddLocationToDisplay;

impl VisitMut for AddLocationToDisplay {
    fn visit_attribute_mut(&mut self, node: &mut Attribute) {
        // debug!("+++ visiting attr: {:#?}", node.path().get_ident().unwrap().to_string());

        if node.path().is_ident("snafu") {
            visit_snafu_attr(node);
        }

        // Delegate to the default impl to visit nested expressions.
        visit_attribute_mut(self, node);
    }
}

fn update_group_message_with_location(group: &TokenTree) -> TokenTree {
    let TokenTree::Group(group) = group else {
        panic!("expected TokenTree::Group, got something else");
    };

    // parse the display message as a `LitStr` string literal to get its value
    let lit: LitStr = parse2(group.stream())
        .expect("display group needs to contain a string literal");
    let disp = lit.value();

    let new_disp = format!(r#"{disp} (at: {{location}})"#);
    let quoted = format!(r##"r#"{new_disp}"#"##);
    // debug!("==> {}", &quoted);

    TokenTree::Group(
        Group::new(
            group.delimiter(),
            quoted.parse().unwrap()
        )
    )
}

fn visit_snafu_attr(node: &mut Attribute) {
    let node_str: String = quote! { #node }.to_string();
    debug!("=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=");
    debug!("found snafu attr on: \"{node_str}\"");
    // debug!("NODE: {:#?}", node);

    let Meta::List(snafu_attrs) = &mut node.meta else {
        panic!("expected a Meta::List instance for the snafu attribute");
    };

    let old_tokens = &snafu_attrs.tokens;
    debug!("OLD: {}", quote! { #old_tokens }.to_string());

    // we want to iterate over all the original tokens,
    // find the one that is about `display`, and change the display
    // string in the token following that one
    let mut orig_tokens = snafu_attrs.tokens.clone().into_iter();
    let mut out: Vec<TokenTree> = vec![];

    loop {
        match orig_tokens.next() {
            Some(token_tree) => {
                match token_tree {
                    // found the `display` token
                    TokenTree::Ident(ref i) if i == "display" => {
                        // copy it to the output
                        out.push(token_tree);

                        // create a replacement token that contains the new
                        // display string and add it to the output
                        let old_group = orig_tokens.next().unwrap();
                        let new_group = update_group_message_with_location(&old_group);
                        out.push(new_group);
                    },

                    // found the `display` token
                    TokenTree::Ident(ref i) if i == "whatever" => {
                        panic!(concat!(r#"found `whatever` attribute on "{}" which is "#,
                                       r#"incompatible with adding a `location` field"#),
                               node_str);
                    },

                    // other token, just copy it to the output
                    _ => out.push(token_tree),
                }
            },
            None => break,
        }
    }

    let new_tokens = TokenStream::from_iter(out);
    debug!("NEW: {}", quote! { #new_tokens }.to_string());
    snafu_attrs.tokens = new_tokens;
}


// =============================================================================
//
//     Function adding the location field to an error enum
//
// =============================================================================

pub fn add_location_to_error_enum(mut error_enum: ItemEnum) -> TokenStream {
    AddLocationField.visit_item_enum_mut(&mut error_enum);
    AddLocationToDisplay.visit_item_enum_mut(&mut error_enum);

    quote! { #error_enum }
}
