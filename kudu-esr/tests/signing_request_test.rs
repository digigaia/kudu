use std::sync::Once;

use tracing::warn;
use tracing_subscriber::{
    EnvFilter,
    // fmt::format::FmtSpan,
};
use color_eyre::{Result, eyre::bail};

use kudu::{Action, Name, json};
use kudu::ABIProvider;
use kudu_esr::signing_request::*;


//
// FIXME: look up more tests here: https://github.com/wharfkit/signing-request/blob/master/test/request.ts
//

static INIT: Once = Once::new();

fn init() {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
        // .with_span_events(FmtSpan::ACTIVE)
        // .pretty()
            .init();

        color_eyre::install().unwrap();
    });
}

#[test]
fn placeholder_value() {
    assert_eq!(SIGNER_NAME,       Name::new("............1").unwrap());
    assert_eq!(SIGNER_PERMISSION, Name::new("............2").unwrap());
}

#[test]
fn encode() -> Result<()> {
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

    let abi = ABIProvider::Test.get_abi("eosio")?;
    let actions2 = vec![Action {
        account: Name::constant("eosio"),
        name: Name::constant("voteproducer"),
        authorization: vec![SIGNER_AUTH],
        data: abi.variant_to_binary("voteproducer", &json!({
            "voter": SIGNER_NAME,
            "proxy": "greymassvote",
            "producers": [],
        }))?,
    }];

    let expected = concat!("000101010000000000ea30557015d289deaa32dd",
                           "0101000000000000000200000000000000110100",
                           "000000000000a032dd181be9d56500010000");

    let req = SigningRequest::from_actions_json(ABIProvider::Test, &actions);
    assert_eq!(req.encode().to_hex(), expected);

    let req = SigningRequest::from_actions(actions2);
    assert_eq!(req.encode().to_hex(), expected);

    Ok(())
}

#[test]
fn decode() -> Result<()> {
    init();

    // NOTE: this is an old example from the v1 spec where SIGNER_PERMISSION and
    //       SIGNER_NAME would both resolve to SIGNER_NAME
    //       we don't want to use this or support it
    // let esr = "gmNgZGRkAIFXBqEFopc6760yugsVYWCA0YIwxgKjuxLSL6-mgmQA";

    let abi_provider = ABIProvider::Test;

    let esr = "gmNgZGRkAIFXBqEFopc6760yugsVYWBggtKCMIEFRnclpF9eTWUACgAA";

    let r = SigningRequest::decode(esr, Some(ABIProvider::Test)).unwrap();

    assert_eq!(r.chain_id, ChainId::Alias(1));

    let Request::Actions(actions) = r.request else {
        bail!("invalid request type, should be `actions[]`");
    };
    assert_eq!(actions.len(), 1);

    let a = &actions[0];
    assert_eq!(a.account, "eosio");
    assert_eq!(a.name, "voteproducer");

    let auth = &a.authorization[0];
    assert_eq!(auth.actor, SIGNER_NAME);
    assert_eq!(auth.permission, SIGNER_PERMISSION);

    let data = a.decode_data(&abi_provider);
    assert!(data.is_object());
    assert_eq!(data["voter"], SIGNER_NAME.to_string());
    assert_eq!(data["proxy"], "greymassvote");
    assert_eq!(data["producers"].as_array().unwrap().len(), 0);

    assert_eq!(r.flags, RequestFlags::Broadcast);
    assert_eq!(r.callback, None);
    assert!(r.info.is_empty());

    Ok(())
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

    let req = SigningRequest::from_action_json(
        provider,
        &json!({
            "account": "eosio.token",
            "name": "transfer",
            "authorization": [{"actor": "foo", "permission": "active"}],
            "data": {"from": "foo", "to": "bar", "quantity": "1.000 EOS", "memo": "hello there"},
        }));

    assert_eq!(json!(req), json!({
        "chain_id": ["chain_alias", 1],
        "req": [
            "action",
            {
                "account": "eosio.token",
                "name": "transfer",
                "authorization": [{"actor": "foo", "permission": "active"}],
                "data": "000000000000285d000000000000ae39e80300000000000003454f53000000000b68656c6c6f207468657265",
            },
        ],
        "callback": "",
        "flags": 1,
        "info": [],
    }));

    assert_eq!(req.to_json(), json!({
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

    Ok(())
}


#[test]
fn create_from_actions() -> Result<()> {
    init();

    let provider = ABIProvider::Test;

    let req = SigningRequest::from_actions_json(
        provider,
        &json!([
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
        .with_callback("https://example.com/?tx={{tx}}", true);

    assert_eq!(json!(req), json!({
        "chain_id": ["chain_alias", 1],
        "req": [
            "action[]",
            [
                {
                    "account": "eosio.token",
                    "name": "transfer",
                    "authorization": [{"actor": "foo", "permission": "active"}],
                    "data": "000000000000285d000000000000ae39e80300000000000003454f53000000000b68656c6c6f207468657265"
                },
                {
                    "account": "eosio.token",
                    "name": "transfer",
                    "authorization": [{"actor": "baz", "permission": "active"}],
                    "data": "000000000000be39000000000000ae39e80300000000000003454f53000000000b68656c6c6f207468657265"
                }
            ]
        ],
        "callback": "https://example.com/?tx={{tx}}",
        "flags": 3,
        "info": [],
    }));

    assert_eq!(req.to_json(), json!({
        "chain_id": ["chain_alias", 1],
        "req": [
            "action[]",
            [
                {
                    "account": "eosio.token",
                    "name": "transfer",
                    "authorization": [{"actor": "foo", "permission": "active"}],
                    "data": {"from": "foo", "to": "bar", "quantity": "1.000 EOS", "memo": "hello there"}
                },
                {
                    "account": "eosio.token",
                    "name": "transfer",
                    "authorization": [{"actor": "baz", "permission": "active"}],
                    "data": {"from": "baz", "to": "bar", "quantity": "1.000 EOS", "memo": "hello there"}
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

    let timestamp = "2018-02-15T00:00:00.000";

    let req = SigningRequest::from_transaction_json(
        None,
        json!({
            "delay_sec": 123,
            "expiration": timestamp,
            "max_cpu_usage_ms": 99,
            "actions": [
                {
                    "account": "eosio.token",
                    "name": "transfer",
                    "authorization": [{"actor": "foo", "permission": "active"}],
                    "data": "000000000000285d000000000000ae39e80300000000000003454f53000000000b68656c6c6f207468657265",
                }
            ]
        }))
        .with_broadcast(false)
        .with_callback("https://example.com/?tx={{tx}}", false);

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
                // "context_free_data": [],  // FIXME: this is in wharfkit test, but should only appear in a signed transaction
                // see: https://github.com/AntelopeIO/spring/blob/main/libraries/chain/include/eosio/chain/transaction.hpp#L121
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


#[test]
fn create_from_uri() -> Result<()> {
    init();

    let provider = ABIProvider::Test;
    let uri = "esr://gmNgZGBY1mTC_MoglIGBIVzX5uxZRqAQGMBoExgDAjRi4fwAVz93ICUckpGYl12skJZfpFCSkaqQllmcwczAAAA";

    let req = SigningRequest::from_uri(uri)?.with_abi_provider(provider);

    let expected = json!({
        "chain_id": ["chain_alias", 1],
        "req": [
            "action",
            {
                "account": "eosio.token",
                "name": "transfer",
                "authorization": [{"actor": "............1", "permission": "............1"}],
                "data": "0100000000000000000000000000285d01000000000000000050454e47000000135468616e6b7320666f72207468652066697368",
            },
        ],
        "callback": "",
        "flags": 3,
        "info": [],
    });

    assert_eq!(json!(req), expected);

    Ok(())
}
