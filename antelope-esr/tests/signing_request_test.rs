use std::sync::Once;

use tracing::warn;
use tracing_subscriber::{
    EnvFilter,
    // fmt::format::FmtSpan,
};
use color_eyre::Result;

use antelope_core::{json, Name};
use antelope_abi::ABIProvider;
use antelope_esr::signing_request::*;


//
// FIXME: look up more tests here: https://github.com/wharfkit/signing-request/blob/master/test/request.ts
//

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

    // let opts = EncodeOptions::default();
    // let opts = EncodeOptions::with_abi_provider("test");
    // let req = SigningRequest::new(json!({ "actions": actions }), opts);
    let mut req = SigningRequest::from_actions(actions)
        .with_abi_provider(ABIProvider::Test);
    warn!("{:?}", req);
    // assert!(false);
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

    let r = SigningRequest::decode(esr, Some(ABIProvider::Test)).unwrap();

    assert_eq!(r.chain_id, ChainId::Alias(1));

    let Request::Actions(actions) = r.request else {
        panic!("invalid request type, should be `actions[]`");
    };

    assert_eq!(actions.len(), 1);
    let a = &actions[0];
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
    let r = json!(SigningRequest::decode(esr, Some(ABIProvider::Test)).unwrap());
    warn!(%esr, %r);

    let esr = "gmNgZGRkAIFXBqEFopc6760yugsVYWCA0YIwxgKjuxLSL6-mgmQA";
    let r = json!(SigningRequest::decode(esr, Some(ABIProvider::Test)).unwrap());
    warn!(%esr, %r);

    // assert!(false);
}


//
// following tests mirror those in
// https://github.com/wharfkit/signing-request/blob/master/test/request.ts
//

#[test]
fn create_from_action() -> Result<()> {
    init();

    // let provider = ABIProvider::API(APIClient::jungle());
    let provider = ABIProvider::Test;
    let provider = ABIProvider::Cached {
        provider: Box::new(provider),
        cache: Default::default(),
    };

    let mut req = SigningRequest::from_action(
        json!({
            "account": "eosio.token",
            "name": "transfer",
            "authorization": [{"actor": "foo", "permission": "active"}],
            "data": {"from": "foo", "to": "bar", "quantity": "1.000 EOS", "memo": "hello there"},
        }))
        .with_abi_provider(provider);


    assert_eq!(json!(req), json!({
        "chain_id": ["chain_alias", 1],
        "req": [
            "action",
            {
                "account": "eosio.token",
                "name": "transfer",
                "authorization": [{"actor": "foo", "permission": "active"}],
                "data": {"from": "foo", "to": "bar", "quantity": "1.000 EOS", "memo": "hello there"},
            },
        ],
        "callback": "",
        "flags": 1,
        "info": [],
    }));

    req.encode_actions();
    req.decode_actions();
    req.encode_actions();

    assert_eq!(json!(req), json!({
        "chain_id": ["chain_alias", 1],
        "req": [
            "action",
            {
                "account": "eosio.token",
                "name": "transfer",
                "authorization": [{"actor": "foo", "permission": "active"}],
                "data": "000000000000285D000000000000AE39E80300000000000003454F53000000000B68656C6C6F207468657265",
            },
        ],
        "callback": "",
        "flags": 1,
        "info": [],
    }));

    Ok(())
}


#[test]
fn create_from_actions() -> Result<()> {
    init();

    let provider = ABIProvider::Test;

    let mut req = SigningRequest::from_actions(
        json!([
            {
                "account": "eosio.token",
                "name": "transfer",
                "authorization": [{"actor": "foo", "permission": "active"}],
                "data": {"from": "foo", "to": "bar", "quantity": "1.000 EOS", "memo": "hello there"},
            },
            {
                "account": "eosio.token",
                "name": "transfer",
                "authorization": [{"actor": "baz", "permission": "active"}],
                "data": {"from": "baz", "to": "bar", "quantity": "1.000 EOS", "memo": "hello there"},
            }
        ]))
        .with_callback("https://example.com/?tx={{tx}}", true)
        .with_abi_provider(provider);

    req.encode_actions();


    assert_eq!(json!(req), json!({
        "chain_id": ["chain_alias", 1],
        "req": [
            "action[]",
            [
                {
                    "account": "eosio.token",
                    "name": "transfer",
                    "authorization": [{"actor": "foo", "permission": "active"}],
                    "data": "000000000000285D000000000000AE39E80300000000000003454F53000000000B68656C6C6F207468657265"
                },
                {
                    "account": "eosio.token",
                    "name": "transfer",
                    "authorization": [{"actor": "baz", "permission": "active"}],
                    "data": "000000000000BE39000000000000AE39E80300000000000003454F53000000000B68656C6C6F207468657265"
                }
            ]
        ],
        "callback": "https://example.com/?tx={{tx}}",
        "flags": 3,
        "info": [],
    }));


    Ok(())
}


#[test]
fn create_from_transaction() -> Result<()> {
    init();

    let timestamp = "2018-02-15T00:00:00";

    let mut req = SigningRequest::from_transaction(
        json!({
            "broadcast": false,
            "callback": "https://example.com/?tx={{tx}}",
            "transaction": {
                "delay_sec": 123,
                "expiration": timestamp,
                "max_cpu_usage_ms": 99,
                "actions": [
                    {
                        "account": "eosio.token",
                        "name": "transfer",
                        "authorization": [{"actor": "foo", "permission": "active"}],
                        "data": "000000000000285D000000000000AE39E80300000000000003454F53000000000B68656C6C6F207468657265",
                    }
                ]
            }
        }));

    // we should be able to call `SigningRequest::encode_actions()` without
    // having to provide an ABIProvider as the action is already encoded
    req.encode_actions();

    assert_eq!(json!(req), json!({
        "chain_id": ["chain_alias", 1],
        "req": [
            "transaction",
            {
                "actions": [
                        {
                            "account": "eosio.token",
                            "name": "transfer",
                            "authorization": [{"actor": "foo", "permission": "active"}],
                            "data": "000000000000285d000000000000ae39e80300000000000003454f53000000000b68656c6c6f207468657265",
                        },
                    ],
                    "context_free_actions": [],
                    "delay_sec": 123,
                    "expiration": timestamp,
                    "max_cpu_usage_ms": 99,
                    "max_net_usage_words": 0,
                    "ref_block_num": 0,
                    "ref_block_prefix": 0,
                    "transaction_extensions": [],
            },
        ],
        "callback": "https://example.com/?tx={{tx}}",
        "flags": 0,
        "info": [],
    }));

    Ok(())
}
