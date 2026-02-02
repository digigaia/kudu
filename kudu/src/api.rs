use std::hash::{Hash, Hasher};

use serde_json::Value as JsonValue;
use snafu::{Snafu, ResultExt, ensure};
use ureq;

use kudu_macros::with_location;

// see API endpoints from greymass here: https://www.greymass.com/endpoints

#[derive(Clone, Debug)]
pub struct APIClient {
    pub endpoint: String,
    pub agent: ureq::Agent,
}

#[with_location]
#[derive(Debug, Snafu)]
pub enum HttpError {
    #[snafu(display("http status: {code} - error: {message}"))]
    HttpError { code: u16, message: String },

    #[snafu(display("{source}"))]
    ConnectionError { source: ureq::Error },

    #[snafu(display("{source}"))]
    JsonError { source: ureq::Error },
}


impl APIClient {
    pub fn new(endpoint: &str) -> Self {
        APIClient {
            endpoint: endpoint.trim_end_matches('/').to_owned(),
            agent: ureq::Agent::config_builder()
                .http_status_as_error(false)
                // FIXME: user_agent seems to not work? InvalidHeaderValue???
                // .user_agent(&format!("kudu/{}", crate::config::VERSION))
                // no way to specify content-type?
            // .accept("application/json")  // is it necessary?
                .build()
                .into()
        }
    }

    fn fullpath(&self, path: &str) -> String {
        format!("{}{}", &self.endpoint, path)
    }

    fn return_checked_result(&self, mut response: ureq::http::Response<ureq::Body>) -> Result<JsonValue, HttpError> {
        let code = response.status().as_u16();
        let result: JsonValue = response
            .body_mut()
            .read_json().context(JsonSnafu)?;

        // HTTP status code 4xx and 5xx need to raise an error
        // we do it manually to add more information to the error than just the status code
        ensure!(!(400..600).contains(&code), HttpSnafu { code, message: result["error"].to_string() });

        Ok(result)
    }

    pub fn get(&self, path: &str) -> Result<JsonValue, HttpError> {
        let response = self.agent.get(self.fullpath(path))
            .call().context(ConnectionSnafu)?;
        self.return_checked_result(response)
        // let code = result.status().as_u16();
        // let result: JsonValue = result
        //     .body_mut()
        //     .read_json().context(JsonSnafu)?;
        // // HTTP status code 4xx and 5xx need to raise an error
        // // we do it manually to add more information to the error than just the status code
        // ensure!(!(400..600).contains(&code), HttpSnafu { code, message: result["error"].to_string() });
        // Ok(result)
    }

    pub fn call(&self, path: &str, params: &JsonValue) -> Result<JsonValue, HttpError> {
        let response = self.agent.post(self.fullpath(path))
            .send_json(params).context(ConnectionSnafu)?;
        self.return_checked_result(response)
        // let code = result.status().as_u16();
        // let result: JsonValue = result
        //     .body_mut()
        //     .read_json().context(JsonSnafu)?;
        // // HTTP status code 4xx and 5xx need to raise an error
        // // we do it manually to add more information to the error than just the status code
        // ensure!(!(400..600).contains(&code), HttpSnafu { code, message: result["error"].to_string() });
        // Ok(result)
    }


    // -----------------------------------------------------------------------------
    //     helper functions for known endpoints
    //     TODO: maybe this is not the best place to define them?
    // -----------------------------------------------------------------------------

    pub fn local() -> Self {
        APIClient::new("http://127.0.0.1:8888")
    }

    pub fn jungle() -> Self {
        APIClient::new("https://jungle4.greymass.com")
    }

    pub fn vaulta() -> Self {
        APIClient::new("https://vaulta.greymass.com")
    }
}

impl PartialEq for APIClient {
    fn eq(&self, other: &Self) -> bool {
        self.endpoint == other.endpoint
    }
}

impl Eq for APIClient {}

impl Hash for APIClient {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.endpoint.hash(state);
    }
}

impl Default for APIClient {
    fn default() -> Self { Self::vaulta() }
}
