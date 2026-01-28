use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use snafu::OptionExt;

use crate::{ABI, ABIError, abi::error::UnknownABISnafu};

//
// see tests and more: https://github.com/wharfkit/abicache/blob/master/test/tests/abi.ts
//

// TODO: make abi name a kudu::Name instead of a String
static REGISTRY: LazyLock<Mutex<HashMap<String, Arc<ABI>>>> = LazyLock::new(|| {
    let mut reg = HashMap::new();
    reg.insert("eosio".to_string(), Arc::new(ABI::from_str(EOSIO_ABI).unwrap()));
    reg.insert("eosio.token".to_string(), Arc::new(ABI::from_str(EOSIO_TOKEN_ABI).unwrap()));
    Mutex::new(reg)
});

pub fn load_abi(abi_name: &str, abi: &str) -> Result<(), ABIError> {
    let mut reg = REGISTRY.lock().unwrap();
    reg.insert(abi_name.to_string(), Arc::new(ABI::from_str(abi)?));
    Ok(())
}

pub fn get_abi(abi_name: &str) -> Result<Arc<ABI>, ABIError> {
    REGISTRY.lock().unwrap().get(abi_name).cloned().context(UnknownABISnafu { name: abi_name.to_string() })
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
