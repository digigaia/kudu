use reqwest::Result;
use serde::ser::Serialize;
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
