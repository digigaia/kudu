use std::io::prelude::*;
use std::sync::Once;

use flate2::{
    Compression,
    write::DeflateEncoder,
    // read::DeflateDecoder,
};

use tracing_subscriber::{
    EnvFilter,
    // fmt::format::FmtSpan,
};

use antelope_core::{Name, json};
use antelope_esr::signing_request::*;


static TRACING_INIT: Once = Once::new();

fn init() {
    TRACING_INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
        // .with_span_events(FmtSpan::ACTIVE)
        // .pretty()
            .init();
    });
}

#[test]
fn placeholder_value() {
    assert_eq!(SIGNER_NAME,       Name::from_str("............1").unwrap());
    assert_eq!(SIGNER_PERMISSION, Name::from_str("............2").unwrap());
}

#[test]
fn encode() {
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
fn decode() {
    init();

    let esr = "gmNgZGRkAIFXBqEFopc6760yugsVYWCA0YIwxgKjuxLSL6-mgmQA";

    let r = SigningRequest::decode(esr).unwrap();

    assert_eq!(r.chain_id, ChainId::Alias(1));
    assert_eq!(r.flags, 1);

    // FIXME: add more tests here

    // assert!(false);
}
