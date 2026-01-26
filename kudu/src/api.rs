use serde_json::Value as JsonValue;
use snafu::{Snafu, ResultExt};
use ureq;

use kudu_macros::with_location;

// see API endpoints from greymass here: https://www.greymass.com/endpoints

#[derive(Clone)]
pub struct APIClient {
    pub endpoint: String,
}

#[with_location]
#[derive(Debug, Snafu)]
pub enum HttpError {
    #[snafu(display("{source}"))]
    ConnectionError { source: ureq::Error },

    #[snafu(display("{source}"))]
    JsonError { source: ureq::Error },
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

    pub fn get(&self, path: &str) -> Result<JsonValue, HttpError> {
        ureq::get(self.fullpath(path))
            .call().context(ConnectionSnafu)?
            .body_mut()
            .read_json().context(JsonSnafu)
    }

    pub fn call(&self, path: &str, params: &JsonValue) -> Result<JsonValue, HttpError> {
        ureq::post(self.fullpath(path))
            .send_json(params).context(ConnectionSnafu)?
            .body_mut()
            .read_json().context(JsonSnafu)
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
