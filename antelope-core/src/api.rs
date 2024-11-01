use reqwest::Result;
use serde_json::Value as JsonValue;

// see API endpoints from greymass here: https://www.greymass.com/endpoints

#[derive(Clone)]
pub struct APIClient {
    endpoint: String,
}

impl APIClient {
    pub fn new(endpoint: &str) -> Self {
        APIClient {
            endpoint: endpoint.trim_end_matches('/').to_owned(),
        }
    }

    pub fn get(&self, path: &str) -> Result<JsonValue> {
        self.call(path, &JsonValue::Null)
    }

    pub fn call(&self, path: &str, params: &JsonValue) -> Result<JsonValue> {
        let fullpath = format!("{}{}", &self.endpoint, path);
        let client = reqwest::blocking::Client::new();
        let req = client.post(fullpath);
        let req = if !params.is_null() { req.json(params) } else { req };
        req.send()?.json()
    }


    // -----------------------------------------------------------------------------
    //     helper functions for known endpoints
    //     TODO: maybe this is not the best place to define them?
    // -----------------------------------------------------------------------------

    pub fn jungle() -> Self {
        APIClient::new("https://jungle4.greymass.com")
    }

    pub fn eos() -> Self {
        APIClient::new("https://eos.greymass.com")
    }
}
