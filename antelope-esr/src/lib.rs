use std::io::prelude::*;
use std::fs::read_to_string;

use flate2::{Compression, write::DeflateEncoder};
use serde_json::{json, Value};

use antelope_abi::{ABIDefinition, ABIEncoder, ByteStream};

static SIGNER_NAME: &str = "............1";
static SIGNER_PERMISSION: &str = "............2";


struct SigningRequest {
    actions: Value,
}

impl SigningRequest {
    fn new(actions: Value) -> Self {
        SigningRequest { actions }
    }

    fn encode(&self) -> String {
        let abi_str = read_to_string("src/signing_request_abi.json").unwrap();
        let abi: ABIDefinition = serde_json::from_str(&abi_str).unwrap();
        let encoder = ABIEncoder::from_abi(&abi);
        let mut ds = ByteStream::new();

        encoder.encode_variant(&mut ds, "signing_request", &self.actions).unwrap(); // FIXME: remove this `unwrap`
        "".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signing() {

        // TODO: check whether we need a specific type for this or if we want to just use JSON
        let actions = json!([{
            "account": "eosio",
            "name": "voteproducer",
            "authorization": [{
                "actor": SIGNER_NAME,
                "permission": SIGNER_PERMISSION,
            }],
            "data": {
                "voter": SIGNER_NAME,
                "proxy": "greymassvote",
                "producers": [],
            }
        }]);

        let req = SigningRequest::new(actions);
        let enc = req.encode();

        assert_eq!(enc, "");
    }

    #[test]
    fn deflate_compression() {
        let mut enc = DeflateEncoder::new(Vec::new(), Compression::default());
        enc.write_all(b"Hello World").unwrap();
        println!("{:?}", enc.finish().unwrap());

        // assert!(false);
    }


}
