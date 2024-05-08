use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use std::process;

use dyn_clone::DynClone;
use serde_json::json;
use thiserror::Error;
use tracing::warn;

use antelope_core::{api, api::APIClient, InvalidValue};

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



pub fn get_abi_definition_uncached(abi_name: &str) -> String {
    match api::api_endpoint().as_deref() {
        Some(_) => {
            match abi_name {
                "signing_request" => {
                    SIGNING_REQUEST_ABI.to_owned()
                },
                _ => {
                    let abi = api::api_call("/v1/chain/get_abi",
                                            &json!({"account_name": abi_name})).unwrap();
                    abi["abi"].to_string()
                },
            }
        },
        None => {
            // no endpoint set for API calls, return static data
            match abi_name {
                "signing_request" => {
                    SIGNING_REQUEST_ABI.to_owned()
                },
                "eosio" => {
                    EOSIO_ABI.to_owned()
                },
                _ => {
                    unimplemented!()
                }
            }
        },
    }
}


fn get_abi_cache() -> &'static Mutex<HashMap<String, String>> {
    static CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    CACHE.get_or_init(|| {
        Mutex::new(HashMap::<String, String>::new())
    })
}

pub fn get_abi_definition(abi_name: &str) -> ABIDefinition {
    let mut cache = get_abi_cache().lock().unwrap();
    match cache.get(abi_name) {
        Some(def) => {
            warn!("getting abi def `{}` from cache, pid = {}", abi_name, process::id());
            ABIDefinition::from_str(def).unwrap()
        },
        None => {
            warn!("getting abi def `{}` from API client, pid = {}", abi_name, process::id());
            let abi = get_abi_definition_uncached(abi_name);
            cache.insert(abi_name.to_owned(), abi.clone());
            ABIDefinition::from_str(&abi).unwrap()
        },
    }
}


pub fn get_abi(abi_name: &str) -> ABIEncoder {
    // TODO: cache this and return &'static ABIEncoder (if we can)
    ABIEncoder::from_abi(&get_abi_definition(abi_name))
}

// pub fn get_abi(abi_name: &str) -> ABIEncoder {
//     match abi_name {
//         "eosio" => ABIEncoder::from_abi(&get_abi_definition("eosio")),
//         "signing_request" => signing_request_abi_parser().clone(),
//         _ => panic!("no abi with name {}", abi_name),
//     }
// }


#[derive(Error, Debug)]
pub enum InvalidABI {
    #[error(r#"unknown ABI with name "{0}""#)]
    Unknown(String),

    #[error("could not parse ABI")]
    ParseError(#[from] InvalidValue),

}

pub trait ABIProvider: DynClone {
    fn get_abi_definition(&self, abi_name: &str) -> Result<String, InvalidABI>;

    fn get_abi(&self, abi_name: &str) -> Result<ABIEncoder, InvalidABI> {
        let abi_def = ABIDefinition::from_str(&self.get_abi_definition(abi_name)?)?;
        Ok(ABIEncoder::from_abi(&abi_def))
    }
}

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
