use serde_json::{json, Value};

use antelope_core::api::api_call;


fn main() {
    println!("hello!");

    let resp = api_call("/v1/chain/get_info", &Value::Null).unwrap();
    println!("{:#?}", resp);

    let resp2 = api_call("/v1/chain/get_abi", &json!({"account_name": "eosio"})).unwrap();
    println!("{:#?}", &resp2);
}
