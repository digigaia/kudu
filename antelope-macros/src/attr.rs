use heck::ToSnakeCase;
use proc_macro2::Ident;
use syn::{Attribute, LitStr, Result, Variant};

/// Find the value of a #[serde(rename = "...")] attribute.
fn attr_rename(attrs: &[Attribute]) -> Result<Option<String>> {
    let mut rename = None;

    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                let s: LitStr = meta.value()?.parse()?;
                if rename.is_some() {
                    return Err(meta.error("duplicate rename attribute"));
                }
                rename = Some(s.value());
                Ok(())
            } else {
                Err(meta.error("unsupported attribute"))
            }
        })?;
    }

    Ok(rename)
}

/// Determine the name of a variant, respecting a rename attribute.
pub fn snake_name_of_variant(var: &Variant) -> Result<String> {
    let rename = attr_rename(&var.attrs)?;
    Ok(rename.unwrap_or_else(|| unraw(&var.ident).to_snake_case()))
}

fn unraw(ident: &Ident) -> String {
    ident.to_string().trim_start_matches("r#").to_owned()
}
