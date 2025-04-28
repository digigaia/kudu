use proc_macro2::TokenStream;
use quote::quote;
use syn::{Expr, ItemStruct, Lit, MetaNameValue, punctuated::Punctuated};

/// control whether we want to have debugging information for the macro when compiling
const DEBUG: bool = false;

macro_rules! debug {
    ( $($elem:expr),* ) => { if DEBUG { eprintln!( $($elem),* ); } }
}


type KVPairs = Punctuated::<MetaNameValue, syn::Token![,]>;

fn parse_contract_args(attrs: KVPairs) -> (String, String) {
    let mut account = None;
    let mut name = None;

    for kv in attrs {
        if kv.path.is_ident("account") {
            if account.is_some() {
                panic!("'account' provided more than once");
            }
            // TODO: use an if-let chain when they are stabilized
            let Expr::Lit(lit) = kv.value else { panic!("'account' value should be a string"); };
            if let Lit::Str(v) = lit.lit {
                account = Some(v);
            } else {
                panic!("'account' value should be a string");
            }
        } else if kv.path.is_ident("name") {
            if name.is_some() {
                panic!("'name' provided more than once");
            }
            let Expr::Lit(lit) = kv.value else { panic!("'name' value should be a string"); };
            if let Lit::Str(v) = lit.lit {
                name = Some(v);
            } else {
                panic!("'name' value should be a string");
            }
        }
    }
    match (account, name) {
        (Some(account), Some(name)) => (account.value(), name.value()),
        (None, None) => panic!("missing both 'account' and 'name' attributes"),
        (None, _) => panic!("missing 'account' attribute"),
        (_, None) => panic!("missing 'name' attribute"),
    }
}

pub fn add_contract_trait_impl(attrs: KVPairs, contract_struct: ItemStruct) -> TokenStream {
    debug!("attr: {:?}", &attrs);

    let (account, name) = parse_contract_args(attrs);

    let struct_name = &contract_struct.ident;

    debug!("Contract {}::{}", account, name);

    quote! {
        #contract_struct

        #[doc(hidden)]
        const _: () = {
            impl kudu::Contract for #struct_name {
                fn account() -> kudu::AccountName {
                    const { kudu::AccountName::constant(#account) }
                }
                fn name() -> kudu::ActionName {
                    const { kudu:: ActionName::constant(#name) }
                }
            }
        };
    }
}
