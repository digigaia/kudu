use std::sync::Once;

use color_eyre::eyre::Result;
use serde_json::json;
use tracing_subscriber::{
    EnvFilter,
    // fmt::format::FmtSpan,
};

use antelope_abi::{abidefinition::abi_schema, ABIDefinition, ABI, ABIError};
use antelope_core::convert::{variant_to_int, variant_to_uint};

static TRACING_INIT: Once = Once::new();

fn init() {
    TRACING_INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            // .with_span_events(FmtSpan::ACTIVE)
            .init();
    });
}

macro_rules! check_error {
    ($t:ident, $error_type:pat) => {
        // let result = $t ;
        assert!($t.is_err(), "expected error, found some result instead");
        match $t.err().unwrap() {
            $error_type => (),
            err => panic!("wrong error type: expected `{}`, got `{}`", stringify!($error_type), err),
        }
    };
    ($t:ident, $error_type:pat, $msg:literal) => {
        assert!($t.is_err(), "expected error, found some result instead");
        let err = $t.err().unwrap();
        match err {
            $error_type => {
                let received = err.to_string();
                if !received.contains($msg) {
                    panic!(r#"expected error with message "{}", got this instead: "{}""#,
                           $msg, received);
                }
            },
            err => panic!("wrong error type: expected `{}`, got `{}`", stringify!($error_type), err),
        }
    };
}

macro_rules! check_encode_error {
    ($t:ident, $msg:literal) => {
        check_error!($t, ABIError::EncodeError { .. }, $msg);
    }
}


fn main() -> Result<()> {
    println!("hello");

    let schema = abi_schema();
    let j = json!(schema);
    // println!("{:#}", j);

    let schema2 = ABIDefinition::from_str(&j.to_string()).unwrap();

    assert!(*schema == schema2);

    let n = u128::from_str_radix("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF", 16);
    println!("{:?}", n.unwrap() as i128);

    let n = variant_to_uint::<u128>(&json!("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF"));
    println!("{n:?}");

    let n = variant_to_int::<i128>(&json!("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF"));
    println!("{n:?}");


    let i = i16::from_str_radix("7FFF", 16);
    println!("{i:?}");

    init();

    // TODO: if fields is empty then we have a (near) infinite recursion
    let abi = ABI::from_str(r#"
    {
        "version": "eosio::abi/1.0",
        "types": [],
        "structs": [{
            "name": "hi",
            "base": "",
            "fields": [{"name": "a", "type": "checksum512"}]
        }],
        "actions": [{
            "name": "hi",
            "type": "hi[]",
            "ricardian_contract": ""
        }],
        "tables": []
    }
    "#)?;

    let data = b"\xff\xff\xff\xff\x08";

    let result = abi.binary_to_variant("hi[]", data.to_vec());
    check_encode_error!(result, "ht");

    Ok(())
}
