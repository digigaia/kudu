// use std::collections::HashMap;
use std::sync::Mutex;

use lazy_static::lazy_static;
use reqwest::Result;
use serde::ser::Serialize;
use serde_json::{Value as JsonValue};


// FIXME: remove me!!!
const DEFAULT_PROVIDER: &str = "https://jungle4.greymass.com";
lazy_static! {
    static ref API_ENDPOINT: Mutex<Option<String>> = Mutex::new(Some(DEFAULT_PROVIDER.to_owned()));
}


pub fn set_api_endpoint(endpoint: Option<String>) {
    *API_ENDPOINT.lock().unwrap() = endpoint;
}

pub fn api_endpoint() -> Option<String> {
    (*API_ENDPOINT.lock().unwrap()).clone()
}

pub fn api_call<T>(path: &str, params: &T) -> Result<JsonValue>
where
    T: Serialize + ?Sized
 {
    match api_endpoint().as_deref() {
        Some(endpoint) => {
            let fullpath = format!("{}{}", endpoint, path);
            let client = reqwest::blocking::Client::new();
            client
                .post(fullpath)
                .json(params)
                .send()?
                .json()
        },
        None => {
            // Ok(JsonValue::Null) // ???
            unimplemented!();
        },
    }
}


#[derive(Clone)]
pub struct APIClient {
    endpoint: String,
}

impl APIClient {
    pub fn new(endpoint: &str) -> Self {
        APIClient {
            endpoint: endpoint.to_owned(),
        }
    }

    pub fn call<T>(&self, path: &str, params: &T) -> Result<JsonValue>
    where
        T: Serialize + ?Sized
    {
        let fullpath = format!("{}{}", &self.endpoint, path);
        let client = reqwest::blocking::Client::new();
        client
            .post(fullpath)
            .json(params)
            .send()?
            .json()
    }
}
