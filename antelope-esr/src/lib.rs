use std::io::prelude::*;
use std::fs::read_to_string;

use base64::prelude::*;

use flate2::{
    Compression,
    write::DeflateEncoder,
    read::DeflateDecoder,
};

use antelope_core::{Name, json, JsonValue};
use antelope_abi::{ABIDefinition, ABIEncoder, ByteStream, abi::TypeNameRef as T};


use std::sync::Once;
use tracing_subscriber::{
    EnvFilter,
    fmt::format::FmtSpan,
};

use tracing::warn;

static TRACING_INIT: Once = Once::new();

fn init() {
    TRACING_INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .with_span_events(FmtSpan::ACTIVE)
            // .pretty()
            .init();
    });
}



// static SIGNER_NAME: Name = Name::from_str("............1").unwrap();  // == Name::from_u64(1)  TODO: make this a unittest
static SIGNER_NAME: Name = Name::from_u64(1);
static SIGNER_PERMISSION: Name = Name::from_u64(2);


struct SigningRequest {
    actions: JsonValue,
}

impl SigningRequest {
    fn new(actions: JsonValue) -> Self {
        let mut result = SigningRequest { actions };
        result.encode_actions();
        result
    }

    fn encode_actions(&mut self) {
        // this needs to dynamically find the ABIs for the different actions
        let mut action = &mut self.actions[0];
        let mut dsa = ByteStream::new();
        let action_abi = ABIDefinition::from_variant(&json!({
            "version": "eosio::abi/1.2",
            "structs": [
                {
                    "name": "voteproducer",
                    "base": "",
                    "fields": [
                        { "name": "voter", "type": "name" },
                        { "name": "proxy", "type": "name" },
                        { "name": "producers", "type": "name[]" },
                    ],
                }
            ],
        })).unwrap();
        let action_encoder = ABIEncoder::from_abi(&action_abi);
        action_encoder.encode_variant(
            &mut dsa,
            T(action["name"].as_str().unwrap()),
            &action["data"]
        ).unwrap();

        action["data"] = JsonValue::String(dsa.hex_data());

    }

    fn encode(&self) -> String {
        let abi_str = read_to_string("src/signing_request_abi.json").unwrap();
        let abi: ABIDefinition = serde_json::from_str(&abi_str).unwrap();
        let encoder = ABIEncoder::from_abi(&abi);
        let mut ds = ByteStream::new();

        // first encode all actions

        encoder.encode_variant(&mut ds, T("action"), &self.actions[0]).unwrap();

        // encoder.encode_variant(&mut ds, T("signing_request"), &self.actions).unwrap(); // FIXME: remove this `unwrap`
        "".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signing() {
        init();

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
        init();

        let mut enc = DeflateEncoder::new(Vec::new(), Compression::default());
        enc.write_all(b"Hello World").unwrap();
        println!("{:?}", enc.finish().unwrap());

        let esr = "gmNgZGRkAIFXBqEFopc6760yugsVYWCA0YIwxgKjuxLSL6-mgmQA";

        let dec = BASE64_URL_SAFE.decode(esr).unwrap();

        warn!("{:?}", &dec);

        let mut deflater = DeflateDecoder::new(&dec[1..]);
        let mut dec2 = vec![];
        deflater.read_to_end(&mut dec2).unwrap();

        warn!("{:?}", &dec2);

        let abi_str = read_to_string("src/signing_request_abi.json").unwrap();
        let abi: ABIDefinition = serde_json::from_str(&abi_str).unwrap();
        let encoder = ABIEncoder::from_abi(&abi);

        let mut ds = ByteStream::from(dec2);
        let r = encoder.decode_variant(&mut ds, T("signing_request")).unwrap();

        warn!("{}", r);

        // assert!(false);
    }


}
