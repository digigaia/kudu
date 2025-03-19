use crate::attr;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    Data, DataEnum, DataStruct, DeriveInput, Error, Fields, FieldsNamed, Ident, Result, Variant, Field,
};


/// control whether we want to have debugging information for the macro when compiling
const DEBUG: bool = false;

macro_rules! debug {
    ( $($elem:expr),* ) => { if DEBUG { eprintln!( $($elem),* ); } }
}


// =============================================================================
//
//     `Binaryserializable`
//
// =============================================================================

pub fn derive(input: &DeriveInput) -> TokenStream {
    match try_expand(input) {
        Ok(expanded) => expanded,
        Err(error) => panic!("Error while using derive(BinarySerializable): {}", error),
    }
}

fn try_expand(input: &DeriveInput) -> Result<TokenStream> {
    match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => derive_binaryserializable_struct(input, fields),
        Data::Enum(enumeration) => derive_binaryserializable_enum(input, enumeration),
        _ => Err(Error::new(
            Span::call_site(),
            "currently only structs with named fields are supported",
        )),
    }
}

fn derive_binaryserializable_struct(input: &DeriveInput, fields: &FieldsNamed) -> Result<TokenStream> {
    let ident = &input.ident;

    let fieldname = &fields.named.iter().map(|f| &f.ident).collect::<Vec<_>>();
    let fieldtype = &fields.named.iter().map(|f| &f.ty).collect::<Vec<_>>();

    debug!("field names: {:?}", &fieldname);
    debug!("field types: {:?}", &fieldtype);

    Ok(quote! {
        #[doc(hidden)]
        const _: () = {
            impl kudu::BinarySerializable for #ident {
                fn to_bin(&self, s: &mut kudu::ByteStream) {
                    #(
                        self.#fieldname.to_bin(s);
                    )*
                }
                fn from_bin(s: &mut kudu::ByteStream) -> ::core::result::Result<Self, kudu::SerializeError> {
                    Ok(Self {
                        #(
                            #fieldname: <#fieldtype>::from_bin(s)?,
                        )*
                    })
                }
            }
        };
    })
}

fn derive_binaryserializable_enum(input: &DeriveInput, enumeration: &DataEnum) -> Result<TokenStream> {
    if input.generics.lt_token.is_some() || input.generics.where_clause.is_some() {
        return Err(Error::new(
            Span::call_site(),
            "Enums with generics are not supported",
        ));
    }

    let ident = &input.ident;
    let ident_str = ident.to_string();

    let variants = enumeration
        .variants
        .iter()
        .map(|variant| match variant.fields {
            Fields::Unnamed(ref f) if f.unnamed.len() == 1 => Ok((&variant.ident, &f.unnamed[0])),
            // Fields::Unit => Ok(&variant.ident),
            _ => Err(Error::new_spanned(
                variant,
                "Invalid variant: only simple enum variants with 1 field are supported",
            )),
        })
        .collect::<Result<Vec<_>>>()?;
    let var_idents: Vec<_> = variants.iter().map(|v: &(&Ident, &Field)| v.0).collect();
    let var_type: Vec<_> = variants.iter().map(|v: &(&Ident, &Field)| &v.1.ty).collect();
    // let names = enumeration
    //     .variants
    //     .iter()
    //     .map(attr::snake_name_of_variant)
    //     .collect::<Result<Vec<_>>>()?;

    debug!("field idents: {:?}", &var_idents);
    debug!("field types: {:?}", &var_type);
    // debug!("field names: {:?}", &names);

    let index: Vec<_> = (0..(var_idents.len() as u32)).collect();

    Ok(quote! {
        #[doc(hidden)]
        const _: () = {
            impl kudu::BinarySerializable for #ident {
                fn to_bin(&self, s: &mut kudu::ByteStream) {
                    match *self {
                        #(
                            #ident::#var_idents(ref __field0) => {
                                kudu::VarUint32(#index).to_bin(s);
                                __field0.to_bin(s);
                            }
                        )*
                    }
                }
                fn from_bin(s: &mut kudu::ByteStream) -> ::core::result::Result<Self, kudu::SerializeError> {
                    Ok(match kudu::VarUint32::from_bin(s)?.0 {
                        #(
                            #index => #ident::#var_idents(<#var_type>::from_bin(s)?),
                        )*
                        t => kudu::binaryserializable::InvalidTagSnafu { tag: t, variant: #ident_str }.fail()?,
                    })
                }
            }
        };
    })
}


// =============================================================================
//
//     `SerializeEnum`
//
// =============================================================================

pub fn derive_serialize_enum(input: &DeriveInput, prefixed: bool) -> TokenStream {
    match try_expand_enum(input, prefixed) {
        Ok(expanded) => expanded,
        Err(error) => panic!("Error while using derive(SerializeEnum): {}", error),
    }
}

fn try_expand_enum(input: &DeriveInput, prefixed: bool) -> Result<TokenStream> {
    match &input.data {
        Data::Enum(enumeration) => derive_enum(input, enumeration, prefixed),
        _ => Err(Error::new(
            Span::call_site(),
            "currently only structs with named fields are supported",
        )),
    }
}

fn derive_enum(input: &DeriveInput, enumeration: &DataEnum, prefixed: bool) -> Result<TokenStream> {
    if input.generics.lt_token.is_some() || input.generics.where_clause.is_some() {
        return Err(Error::new(
            Span::call_site(),
            "Enums with generics are not supported",
        ));
    }

    let ident = &input.ident;
    let ident_str = ident.to_string();

    let _valid = enumeration.variants.iter().map(|variant| match variant.fields {
        Fields::Unit => Ok(()),
        Fields::Unnamed(ref f) if f.unnamed.len() == 1 => Ok(()),
        _ => Err(Error::new_spanned(
            variant,
            "Invalid variant: only simple enum variants with 0 or 1 field are supported",
        )),
    }).collect::<Result<Vec<_>>>()?;

    fn is_variant_field(variant: &&Variant) -> bool {
        matches!(variant.fields, Fields::Unnamed(ref f) if f.unnamed.len() == 1)
    }

    fn is_unit_variant_field(variant: &&Variant) -> bool {
        matches!(variant.fields, Fields::Unit)
    }

    let variants = enumeration
        .variants
        .iter()
        .filter(is_variant_field)
        .map(|variant| match variant.fields {
            Fields::Unnamed(ref f) => Ok((&variant.ident, &f.unnamed[0])),
            _ => panic!("cannot happen"),
        })
        .collect::<Result<Vec<_>>>()?;
    let var_idents: Vec<_> = variants.iter().map(|v: &(&Ident, &Field)| v.0).collect();
    let var_type: Vec<_> = variants.iter().map(|v: &(&Ident, &Field)| &v.1.ty).collect();

    let variant_name = |variant| {
        if prefixed { attr::prefixed_snake_name_of_variant(&ident_str, variant) }
        else        { attr::snake_name_of_variant(variant) }
    };

    // let var_idents = enumeration
    //     .variants
    //     .iter()
    //     .filter(is_variant_field)
    //     .map(|variant| &variant.ident)
    //     .collect::<Vec<_>>();
    let names = enumeration
        .variants
        .iter()
        .filter(is_variant_field)
        .map(variant_name)
        // .map(|variant| attr::prefixed_snake_name_of_variant(&ident_str, variant))
        .collect::<Result<Vec<_>>>()?;

    let unit_var_idents = enumeration
        .variants
        .iter()
        .filter(is_unit_variant_field)
        .map(|variant| &variant.ident)
        .collect::<Vec<_>>();
    let unit_names = enumeration
        .variants
        .iter()
        .filter(is_unit_variant_field)
        .map(variant_name)
        // .map(|variant| attr::prefixed_snake_name_of_variant(&ident_str, variant))
        .collect::<Result<Vec<_>>>()?;

    debug!("variant idents: {:?}", &var_idents);
    debug!("variant types: {:?}", &var_type);
    debug!("variant names: {:?}", &names);

    Ok(quote! {
        #[doc(hidden)]
        const _: () = {
            impl serde::Serialize for #ident {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where S: serde::Serializer
                {
                    match *self {
                        #(
                            #ident::#var_idents(ref __field0) => {
                                let mut state = serde::Serializer::serialize_tuple(serializer, 2)?;
                                serde::ser::SerializeTuple::serialize_element(&mut state, #names)?;
                                serde::ser::SerializeTuple::serialize_element(&mut state, __field0)?;
                                serde::ser::SerializeTuple::end(state)
                            },
                        )*
                        #(
                            #ident::#unit_var_idents => {
                                let mut state = serde::Serializer::serialize_tuple(serializer, 2)?;
                                serde::ser::SerializeTuple::serialize_element(&mut state, #unit_names)?;
                                // FIXME: is this the correct behavior?
                                serde::ser::SerializeTuple::serialize_element(&mut state, "")?;
                                serde::ser::SerializeTuple::end(state)
                            },
                        )*
                    }
                }
            }

            impl<'de> serde::Deserialize<'de> for #ident {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: serde::Deserializer<'de>,
                {
                    let tup: serde_json::Value = serde_json::Value::deserialize(deserializer)?;
                    // let tag: Option<&str> = tup[0].as_str();
                    // let tag: &str = tag.ok_or(serde::de::Error::custom("Tag (discriminant) needs to be a string"))?;
                    let tag: &str = tup[0].as_str()
                        .ok_or(serde::de::Error::custom("Tag (discriminant) needs to be a string"))?;
                    Ok(match tag {
                        #(
                            #names => {
                                let v: #var_type = serde_json::from_str(&tup[1].to_string())
                                    .map_err(|e| serde::de::Error::custom(e.to_string()))?;
                                #ident::#var_idents(v)
                            },
                        )*
                        #(
                            #unit_names => #ident::#unit_var_idents,
                        )*
                        _ => {
                            let msg = format!("Invalid tag (discriminant) for type {}: {}", #ident_str, tag);
                            return Err(serde::de::Error::custom(msg));
                        }
                    })
                }
            }
        };
    })
}
