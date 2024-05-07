use std::sync::Once;

use tracing::warn;
use tracing_subscriber::{
    EnvFilter,
    // fmt::format::FmtSpan,
};

use antelope_core::{Name, json, api};
use antelope_esr::signing_request::*;


//
// FIXME: look up more tests here: https://github.com/wharfkit/signing-request/blob/master/test/request.ts
//

static TRACING_INIT: Once = Once::new();

fn init() {
    api::set_api_endpoint(None);

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

    assert_eq!(hex::encode_upper(enc),
               concat!("000101010000000000EA30557015D289DEAA32DD",
                       "0101000000000000000200000000000000110100",
                       "000000000000A032DD181BE9D56500010000"));
}

#[test]
fn decode() {
    init();

    // NOTE: this is an old example from the v1 spec where SIGNER_PERMISSION and
    //       SIGNER_NAME would both resolve to SIGNER_NAME
    //       we don't want to use this or support it
    // let esr = "gmNgZGRkAIFXBqEFopc6760yugsVYWCA0YIwxgKjuxLSL6-mgmQA";

    let esr = "gmNgZGRkAIFXBqEFopc6760yugsVYWBggtKCMIEFRnclpF9eTWUACgAA";

    let r = SigningRequest::decode(esr).unwrap();

    assert_eq!(r.chain_id, ChainId::Alias(1));

    assert_eq!(r.actions.len(), 1);
    let a = &r.actions[0];
    assert_eq!(a["account"], "eosio");
    assert_eq!(a["name"], "voteproducer");
    let auth = &a["authorization"][0];
    assert_eq!(auth["actor"], SIGNER_NAME.to_string());
    assert_eq!(auth["permission"], SIGNER_PERMISSION.to_string());
    let data = &a["data"];
    assert_eq!(data["voter"], SIGNER_NAME.to_string());
    assert_eq!(data["proxy"], "greymassvote");
    assert_eq!(data["producers"].as_array().unwrap().len(), 0);

    assert_eq!(r.flags, 1);
    assert_eq!(r.callback, None);
    assert!(r.info.is_empty());

    // assert!(false);
}

#[test]
fn dec2() {
    init();

    let esr = "gmNgZGRkAIFXBqEFopc6760yugsVYWBggtKCMIEFRnclpF9eTWUACgAA";
    let r = json!(SigningRequest::decode(esr).unwrap());
    warn!(%esr, %r);

    let esr = "gmNgZGRkAIFXBqEFopc6760yugsVYWCA0YIwxgKjuxLSL6-mgmQA";
    let r = json!(SigningRequest::decode(esr).unwrap());
    warn!(%esr, %r);

    // assert!(false);
}
