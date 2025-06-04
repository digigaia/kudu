use serde_json::Value as JsonValue;
use ureq;

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

    fn fullpath(&self, path: &str) -> String {
        format!("{}{}", &self.endpoint, path)
    }

    pub fn get(&self, path: &str) -> Result<JsonValue, ureq::Error> {
        ureq::get(self.fullpath(path))
            .call()?
            .body_mut()
            .read_json()
    }

    pub fn call(&self, path: &str, params: &JsonValue) -> Result<JsonValue, ureq::Error> {
        ureq::post(self.fullpath(path))
            .send_json(params)?
            .body_mut()
            .read_json()
    }


    // -----------------------------------------------------------------------------
    //     helper functions for known endpoints
    //     TODO: maybe this is not the best place to define them?
    // -----------------------------------------------------------------------------

    pub fn jungle() -> Self {
        APIClient::new("https://jungle4.greymass.com")
    }

    pub fn vaulta() -> Self {
        APIClient::new("https://vaulta.greymass.com")
    }
}
