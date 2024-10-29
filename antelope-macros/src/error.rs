use proc_macro2::TokenStream as TokenStream2;
use syn::{
    parenthesized, Attribute, Fields, FieldsNamed, LitStr, Meta, Variant, Field,
    visit_mut::{
        VisitMut, visit_attribute_mut,
    }
};


fn location_as_fields_named() -> FieldsNamed {
    syn::parse_str("{ #[snafu(implicit)] location: snafu::Location }").unwrap()
}

pub fn location_field() -> Field {
    let fs: FieldsNamed = location_as_fields_named();
    let location_field = &fs.named[0];
    location_field.clone()
}



// =============================================================================
//
//     Visitor for adding a `location` field to all Enum variants
//
// =============================================================================

pub struct AddLocationField;

impl VisitMut for AddLocationField {
    fn visit_variant_mut(&mut self, node: &mut Variant) {
        match &mut node.fields {
            Fields::Named(ref mut fields) => {
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
//     Visitor for adding the location an error was constructed to the display
//     string associated with a given variant
//
// =============================================================================

pub struct AddLocationToDisplay;

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
                    // let msg = format!("unrecognized attr for snafu: `{:?}`", meta.path.get_ident());
                    // println!("{}", msg);
                    Ok(())
                }
            })
            // .unwrap_or(());
            .unwrap_or_else(|e| {
                println!("cannot parse nested meta on attribute {:?}", node);
                println!("{:?}", e);
            });

            if let Some(disp) = disp_str {
                // println!("=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=");
                // println!("found disp str while visiting: {disp}");
                // println!("NODE = {:#?}", node);

                let new_disp = format!(r#"{disp} (at: {{location}})"#);
                let new_display_attr = format!(r##"display(r#"{new_disp}"#)"##); // prefer raw strings to escaped
                // let new_display_attr = format!(r#"display({new_disp:?})"#);   // works too

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
        visit_attribute_mut(self, node);
    }
}
