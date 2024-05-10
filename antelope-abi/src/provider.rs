use std::rc::Rc;
use std::sync::OnceLock;

use serde_json::json;
use thiserror::Error;

use antelope_core::{api::APIClient, InvalidValue};

use crate::{
    abi::ABIDefinition,
    abiencoder::ABIEncoder,
};


//
// FIXME: this whole module is full of inefficiencies, fix that
// see tests and more: https://github.com/wharfkit/abicache/blob/master/test/tests/abi.ts
//

// FIXME: this is not proper... (the include ../..)
pub static SIGNING_REQUEST_ABI: &str = include_str!("../../antelope-esr/src/signing_request_abi.json");

// FIXME: move this and line above inside the `antelope-esr` crate, there is no need to have it here
// if we want we can use an `OverrideProvider`, see note at the end of the file
pub fn get_signing_request_abi() -> &'static ABIEncoder {
    static SR_ABI: OnceLock<ABIEncoder> = OnceLock::new();
    SR_ABI.get_or_init(|| {
        ABIEncoder::with_abi(&ABIDefinition::from_str(SIGNING_REQUEST_ABI).unwrap())
    })
}


// fn get_abi_cache() -> &'static Mutex<HashMap<String, String>> {
//     static CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
//     CACHE.get_or_init(|| {
//         Mutex::new(HashMap::<String, String>::new())
//     })
// }



// =============================================================================
//
//     InvalidABI error
//
// =============================================================================


#[derive(Error, Debug)]
pub enum InvalidABI {
    #[error(r#"unknown ABI with name "{0}""#)]
    Unknown(String),

    #[error("could not parse ABI")]
    ParseError(#[from] InvalidValue),

}


// =============================================================================
//
//     ABIProvider trait
//
// =============================================================================

pub trait ABIProvider {
    fn get_abi_definition(&self, abi_name: &str) -> Result<String, InvalidABI>;

    fn get_abi(&self, abi_name: &str) -> Result<Rc<ABIEncoder>, InvalidABI> {
        let abi_def = ABIDefinition::from_str(&self.get_abi_definition(abi_name)?)?;
        Ok(Rc::new(ABIEncoder::from_abi(&abi_def)))
    }
}

// =============================================================================
//
//     API call-based ABIProvider
//
// =============================================================================

#[derive(Clone)]
pub struct APICallABIProvider {
    client: APIClient,
}

impl APICallABIProvider {
    pub fn new(endpoint: &str) -> Self {
        APICallABIProvider { client: APIClient::new(endpoint) }
    }
}

impl ABIProvider for APICallABIProvider {
    fn get_abi_definition(&self, abi_name: &str) -> Result<String, InvalidABI> {
        match abi_name {
            "signing_request" => Ok(SIGNING_REQUEST_ABI.to_owned()),
            _ => {
                let abi = self.client.call("/v1/chain/get_abi",
                                           &json!({"account_name": abi_name})).unwrap();
                Ok(abi["abi"].to_string())
            },
        }
    }
}


// =============================================================================
//
//     Null ABI Provider - empty provider without implementation
//
// =============================================================================

#[derive(Clone)]
pub struct NullABIProvider {}

impl NullABIProvider {
    pub fn new() -> Self { NullABIProvider {} }
}

impl Default for NullABIProvider {
    fn default() -> Self { Self::new() }
}

impl ABIProvider for NullABIProvider {
    fn get_abi_definition(&self, _abi_name: &str) -> Result<String, InvalidABI> {
        unimplemented!()
    }
}


// =============================================================================
//
//     Test ABI Provider - returns some statically-stored ABIs
//
// =============================================================================

#[derive(Clone)]
pub struct TestABIProvider {}

impl TestABIProvider {
    pub fn new() -> Self { TestABIProvider {} }
}

impl Default for TestABIProvider {
    fn default() -> Self { Self::new() }
}

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

impl ABIProvider for TestABIProvider {
    fn get_abi_definition(&self, abi_name: &str) -> Result<String, InvalidABI> {
        match abi_name {
            "signing_request" => Ok(SIGNING_REQUEST_ABI.to_owned()),
            "eosio" => Ok(EOSIO_ABI.to_owned()),
            _ => unimplemented!(),
        }
    }
}




//
// static helper functions to get most often used ABIProviders
//

// FIXME: remove this function, this is not the right place. This needs to be defined
//        closer to where it is actually used
pub fn test_provider() -> TestABIProvider {
    TestABIProvider::new()
}

// FIXME: implement CachedProvider that takes an ABIProvider upon construction and wraps it
// implement OverrideProvider that creates a new ABIProvider where some ABIs have been overriden, e.g.
//    OverrrideProvider::new(APICallABiprovider, "signing_request" => get_signing_request_abi)
