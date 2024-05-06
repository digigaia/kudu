use std::collections::HashMap;
use std::fs::read_to_string;
use std::sync::{Mutex, OnceLock};

use serde_json::json;
use tracing::warn;

use antelope_core::api::api_call;

use crate::{
    abi::ABIDefinition,
    abiencoder::ABIEncoder,
};


//
// FIXME: this whole module is full of inefficiencies, fix that
//


pub fn signing_request_abi_schema() -> &'static ABIDefinition {
    static SIGNING_REQUEST_ABI_SCHEMA: OnceLock<ABIDefinition> = OnceLock::new();
    SIGNING_REQUEST_ABI_SCHEMA.get_or_init(|| {
        // FIXME: replace this with `include_str!`
        let abi_str = read_to_string("src/signing_request_abi.json").unwrap();
        let abi: ABIDefinition = serde_json::from_str(&abi_str).unwrap();
        abi
    })
}

pub fn signing_request_abi_parser() -> &'static ABIEncoder {
    static SIGNING_REQUEST_ABI_PARSER: OnceLock<ABIEncoder> = OnceLock::new();
    SIGNING_REQUEST_ABI_PARSER.get_or_init(|| {
        ABIEncoder::with_abi(signing_request_abi_schema())
    })
}

fn get_abi_cache() -> &'static Mutex<HashMap<String, String>> {
    static CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    CACHE.get_or_init(|| {
        Mutex::new(HashMap::<String, String>::new())
    })
}


// static EOSIO_ABI: &str  = r#"{
//     "version": "eosio::abi/1.2",
//     "structs": [
//         {
//             "name": "voteproducer",
//             "base": "",
//             "fields": [
//                 { "name": "voter", "type": "name" },
//                 { "name": "proxy", "type": "name" },
//                 { "name": "producers", "type": "name[]" }
//             ]
//         }
//     ]
// }
// "#;



pub fn get_abi_definition(abi_name: &str) -> ABIDefinition {
    let mut cache = get_abi_cache().lock().unwrap();
    match cache.get(abi_name) {
        Some(def) => ABIDefinition::from_str(def).unwrap(),
        None => {
            let abi = api_call("/v1/chain/get_abi",
                               &json!({"account_name": abi_name})).unwrap();
            let abi = &abi["abi"];
            let keys: Vec<_> = abi.as_object().unwrap().keys().collect();
            warn!("{:?}", keys);
            cache.insert(abi_name.to_owned(), abi.to_string());
            ABIDefinition::from_str(&abi.to_string()).unwrap()
        },
    }
}


pub fn get_abi(abi_name: &str) -> ABIEncoder {
    match abi_name {
        "eosio" => ABIEncoder::from_abi(&get_abi_definition("eosio")),
        "signing_request" => signing_request_abi_parser().clone(),
        _ => panic!("no abi with name {}", abi_name),
    }
}
