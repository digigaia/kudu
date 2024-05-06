// use std::collections::HashMap;

use reqwest::Result;
use serde::ser::Serialize;
use serde_json::Value;

const API_ENDPOINT: &str = "https://jungle4.greymass.com";


pub fn api_call<T>(path: &str, params: &T) -> Result<Value>
where
    T: Serialize + ?Sized
{
    let fullpath = format!("{}{}", API_ENDPOINT, path);
    let client = reqwest::blocking::Client::new();
    client
        .post(fullpath)
        .json(params)
        .send()?
        .json()
}
