use serde_json::json;

use antelope::api::APIClient;


fn main() {
    let client = APIClient::new("https://jungle4.greymass.com");

    let id = 1;

    let resp = match id {
        1 => client.get("/v1/chain/get_info"),
        2 => client.call("/v1/chain/get_abi", &json!({"account_name": "eosio"})),
        3 => client.call("/v1/chain/get_abi", &json!({"account_name": "eosio.token"})),
        _ => unimplemented!(),
    }.unwrap();

    println!("{}", &resp);
}
