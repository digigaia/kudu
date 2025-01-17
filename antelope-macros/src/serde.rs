use crate::attr;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    Data, DataEnum, DataStruct, DeriveInput, Error, Fields, FieldsNamed, Result, Variant,
};

pub fn derive(input: &DeriveInput) -> TokenStream {
    match try_expand(input) {
        Ok(expanded) => expanded,
        // If there are invalid attributes in the input, expand to a Serialize
        // impl anyway to minimize spurious secondary errors in other code that
        // serializes this type.
        Err(error) => panic!("Error while using derive(BinarySerializable): {}", error),
    }
}

fn try_expand(input: &DeriveInput) -> Result<TokenStream> {
    match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => derive_struct(input, fields),
        _ => Err(Error::new(
            Span::call_site(),
            "currently only structs with named fields are supported",
        )),
    }
}

fn derive_struct(input: &DeriveInput, fields: &FieldsNamed) -> Result<TokenStream> {
    let ident = &input.ident;

    let fieldname = &fields.named.iter().map(|f| &f.ident).collect::<Vec<_>>();
    let fieldtype = &fields.named.iter().map(|f| &f.ty).collect::<Vec<_>>();

    Ok(quote! {
        #[allow(deprecated, non_upper_case_globals)]
        const _: () = {
            impl antelope::BinarySerializable for #ident {
                fn encode(&self, s: &mut antelope::ByteStream) {
                    #(
                        self.#fieldname.encode(s);
                    )*
                }
                fn decode(s: &mut antelope::ByteStream) -> ::core::result::Result<Self, antelope::SerializeError> {
                    Ok(Self {
                        #(
                            #fieldname: #fieldtype::decode(s)?,
                        )*
                    })
                }
            }
        };
    })
}


pub fn derive_serialize_enum(input: &DeriveInput) -> TokenStream {
    match try_expand_enum(input) {
        Ok(expanded) => expanded,
        // If there are invalid attributes in the input, expand to a Serialize
        // impl anyway to minimize spurious secondary errors in other code that
        // serializes this type.
        Err(error) => panic!("Error while using derive(SerializeEnum): {}", error),
    }
}

fn try_expand_enum(input: &DeriveInput) -> Result<TokenStream> {
    match &input.data {
        Data::Enum(enumeration) => derive_enum(input, enumeration),
        _ => Err(Error::new(
            Span::call_site(),
            "currently only structs with named fields are supported",
        )),
    }
}

fn derive_enum(input: &DeriveInput, enumeration: &DataEnum) -> Result<TokenStream> {
    if input.generics.lt_token.is_some() || input.generics.where_clause.is_some() {
        return Err(Error::new(
            Span::call_site(),
            "Enums with generics are not supported",
        ));
    }

    let ident = &input.ident;

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
        // match variant.fields {
        //     Fields::Unnamed(ref f) if f.unnamed.len() == 1 => true,
        //     _ => false,
        // }
    }

    fn is_unit_variant_field(variant: &&Variant) -> bool {
        matches!(variant.fields, Fields::Unit)
        // match variant.fields {
        //     Fields::Unit => true,
        //     _ => false,
        // }
    }

    let var_idents = enumeration
        .variants
        .iter()
        .filter(is_variant_field)
        .map(|variant| &variant.ident)
        .collect::<Vec<_>>();
    let names = enumeration
        .variants
        .iter()
        .filter(is_variant_field)
        .map(attr::snake_name_of_variant)
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
        .map(attr::snake_name_of_variant)
        .collect::<Result<Vec<_>>>()?;

    Ok(quote! {
        #[doc(hidden)]
        // #[allow(deprecated, non_upper_case_globals)]
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
                                serde::ser::SerializeTuple::serialize_element(&mut state, "")?;
                                serde::ser::SerializeTuple::end(state)
                            },
                        )*
                    }
                }
            }
        };
    })
}
