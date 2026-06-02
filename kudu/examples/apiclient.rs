// SPDX-FileCopyrightText: 2024-2026 DigiGaia SCCL
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::json;

use kudu::api::APIClient;


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
