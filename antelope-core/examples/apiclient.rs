use serde_json::{json, Value};

use antelope_core::api::APIClient;


fn main() {
    let client = APIClient::new("https://jungle4.greymass.com");

    let resp = client.call("/v1/chain/get_info", &Value::Null).unwrap();
    println!("{:#?}", resp);

    let resp2 = client.call("/v1/chain/get_abi", &json!({"account_name": "eosio"})).unwrap();
    println!("{:#?}", &resp2);
}
