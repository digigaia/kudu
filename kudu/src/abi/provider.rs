use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use serde_json::json;

use crate::{ABI, ABIDefinition, ABIError, APIClient};


//
// see tests and more: https://github.com/wharfkit/abicache/blob/master/test/tests/abi.ts
//

pub enum ABIProvider {
    API(APIClient),
    Test,
    Cached {
        provider: Box<ABIProvider>,
        cache: RefCell<HashMap<String, Rc<ABI>>>,
    },
}

// FIXME: abi_provider should be able to get ABI by `Name`, not only by `String`
impl ABIProvider {
    pub fn get_abi(&self, abi_name: &str) -> Result<Rc<ABI>, ABIError> {
        match self {
            ABIProvider::Cached { provider, cache } => {
                if let Some(abi) = cache.borrow().get(abi_name) {
                    return Ok(abi.clone());
                }

                let abi = provider.get_abi(abi_name)?;
                cache.borrow_mut().insert(abi_name.to_string(), abi.clone());
                Ok(abi)
            },
            _ => {
                let abi_def = ABIDefinition::from_str(&self.get_abi_definition(abi_name)?)?;
                Ok(Rc::new(ABI::from_definition(&abi_def)?))
            }
        }
    }

    pub fn get_abi_definition(&self, abi_name: &str) -> Result<String, ABIError> {
        if abi_name == "signing_request" {
            todo!();
            // Ok(SIGNING_REQUEST_ABI.to_owned())
            // FIXME: SIGNING_REQUEST_ABI is now inside the `kudu-esr` crate
            // if we want we can use an `OverrideProvider`, see note at the end of the file
        }
        else {
            match self {
                ABIProvider::API(client) => {
                    let abi = client.call("/v1/chain/get_abi",
                                          &json!({"account_name": abi_name})).unwrap();
                    Ok(abi["abi"].to_string())
                },
                ABIProvider::Test => {
                    match abi_name {
                        "eosio" => Ok(EOSIO_ABI.to_owned()),
                        "eosio.token" => Ok(EOSIO_TOKEN_ABI.to_owned()),
                        _ => unimplemented!(),
                    }
                },
                ABIProvider::Cached { provider, .. } => {
                    provider.get_abi_definition(abi_name)
                }
            }
        }
    }
}

// -----------------------------------------------------------------------------
//     static ABI definitions for tests
// -----------------------------------------------------------------------------

// FIXME: replace this with actual ABIs from the networks

static EOSIO_ABI: &str  = r#"{
    "version": "eosio::abi/1.2",
    "structs": [
        {
            "name": "voteproducer",
            "base": "",
            "fields": [
                { "name": "voter", "type": "name" },
                { "name": "proxy", "type": "name" },
                { "name": "producers", "type": "name[]" }
            ]
        }
    ]
}
"#;

static EOSIO_TOKEN_ABI: &str  = r#"{
    "version": "eosio::abi/1.2",
    "structs": [
        {
            "name": "transfer",
            "base": "",
            "fields": [
                { "name": "from", "type": "name" },
                { "name": "to", "type": "name" },
                { "name": "quantity", "type": "asset" },
                { "name": "memo", "type": "string" }
            ]
        }
    ]
}
"#;

// // FIXME: implement CachedProvider that takes an ABIProvider upon construction and wraps it
// // implement OverrideProvider that creates a new ABIProvider where some ABIs have been overriden, e.g.
// //    OverrideProvider::new(APICallABiprovider, "signing_request" => get_signing_request_abi)
