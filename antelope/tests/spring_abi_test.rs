use std::str::FromStr;
use std::sync::Once;

use color_eyre::eyre::Result;
use serde_json::{json, Value as JsonValue};
use tracing_subscriber::{
    EnvFilter,
    // fmt::format::FmtSpan,
};

use antelope::*;


// =============================================================================
//
//     Unittests coming from the reference Spring implementation
//     https://github.com/AntelopeIO/spring/blob/main/unittests/abi_tests.cpp
//
//     skipping the following tests:
//      - linkauth_test, unlinkauth_test, updateauth_test, deleteauth_test,
//        newaccount_test, setcode_test, setabi_test, packed_transaction
//        -> they seem to provide relatively low value
//      - abi_serialize_detailed_error_message, abi_serialize_short_error_message,
//        abi_deserialize_detailed_error_messsage
//        -> can come at a later stage under the `detailed-error` feature flag
//      - abi_very_deep_structs, abi_very_deep_structs_1us, abi_deep_structs_validate
//        -> they use the `deep_nested_abi` and `large_nested_abis` that are inexistent
//      - abi_large_signature
//        -> use webauthn signatures which are not yet implemented
//      - abi_to_variant__add_action__good_return_value,
//        abi_to_variant__add_action__bad_return_value,
//        abi_to_variant__add_action__no_return_value,
//        -> we haven't implemented `ABI::abi_to_variant` and it seems to be
//           pretty involved with internals, we might not need it anyway
//
// =============================================================================

static TRACING_INIT: Once = Once::new();

fn init() {
    TRACING_INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            // .with_span_events(FmtSpan::ACTIVE)
            .init();
    });
}


// -----------------------------------------------------------------------------
//     Utility functions & macros
// -----------------------------------------------------------------------------

#[track_caller]
fn verify_byte_round_trip(abi: &ABI, typename: &str, value: &JsonValue) -> Result<()> {
    let encoded = abi.variant_to_binary(typename, value)?;
    let decoded = abi.binary_to_variant(typename, encoded.clone())?;
    let encoded2 = abi.variant_to_binary(typename, &decoded)?;

    // assert_eq!(value, &decoded);
    assert_eq!(encoded, encoded2);
    Ok(())
}

#[track_caller]
fn verify_round_trip2(abi: &ABI, typename: &str, value: &JsonValue,
                      hex_repr: &str, expected_json: &str) -> Result<()> {
    let encoded = abi.variant_to_binary(typename, value)?;
    assert_eq!(hex::encode(&encoded), hex_repr);
    let decoded = abi.binary_to_variant(typename, encoded.clone())?;
    assert_eq!(&decoded.to_string(), expected_json);
    let encoded2 = abi.variant_to_binary(typename, value)?;
    assert_eq!(encoded, encoded2);
    Ok(())
}

#[track_caller]
fn verify_round_trip(abi: &ABI, typename: &str, value: &JsonValue,
                     hex_repr: &str) -> Result<()> {
    verify_round_trip2(abi, typename, value, hex_repr, &value.to_string())
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

macro_rules! check_integrity_error {
    ($t:ident, $msg:literal) => {
        check_error!($t, ABIError::IntegrityError { .. }, $msg);
    }
}

macro_rules! check_invalid_abi {
    ($path:literal, $msg:literal) => {
        let abi_def = include_str!($path);
        let abi = ABI::from_str(abi_def);
        check_error!(abi, ABIError::IntegrityError { .. }, $msg);
    }
}


// -----------------------------------------------------------------------------
//     Unittests
// -----------------------------------------------------------------------------

#[test]
fn uint_types() -> Result<()> {
    init();

    let currency_abi = r#"
    {
        "version": "eosio::abi/1.0",
        "types": [],
        "structs": [{
            "name": "transfer",
            "base": "",
            "fields": [{
                "name": "amount64",
                "type": "uint64"
            },{
                "name": "amount32",
                "type": "uint32"
            },{
                "name": "amount16",
                "type": "uint16"
            },{
                "name": "amount8",
                "type": "uint8"
            }]
        }],
        "actions": [],
        "tables": [],
        "ricardian_clauses": []
    }
    "#;
    let currency_abi = ABIDefinition::from_str(currency_abi)?;
    let abi = ABI::from_definition(&currency_abi.with_contract_abi()?)?;

    let test_data = json!({
        "amount64": 64,
        "amount32": 32,
        "amount16": 16,
        "amount8": 8,
    });

    verify_byte_round_trip(&abi, "transfer", &test_data)
}


#[test]
fn general() -> Result<()> {
    init();

    let my_abi = ABIDefinition::from_str(include_str!("data/general_abi.json"))?;
    let abi = ABI::from_definition(&my_abi.with_contract_abi()?)?;
    let test_data = JsonValue::from_str(include_str!("data/general_data.json"))?;

    verify_byte_round_trip(&abi, "A", &test_data)
}

#[test]
fn duplicate_types() -> Result<()> {
    init();
    check_invalid_abi!("data/duplicate_types_abi.json", "type already exists");
    Ok(())
}

// FIXME: review me!
#[test]
fn nested_types() -> Result<()> {
    init();

    use antelope::ByteStream;

    let indirectly_nested_abi = r#"
    {
        "version": "eosio::abi/1.0",
        "types": [{
            "new_type_name": "name_arr",
            "type": "name[]"
        },{
            "new_type_name": "name_matrix",
            "type": "name_arr[]"
        }],
        "structs": [],
        "actions": [],
        "tables": [],
        "ricardian_clauses": []
    }
    "#;
    let abi = ABI::from_str(indirectly_nested_abi)?;
    let mut ds = ByteStream::new();

    let value = json!([["a", "b"],["c", "d"]]);
    abi.encode_variant(&mut ds, "name_matrix", &value)?;

    let decoded = abi.decode_variant(&mut ds, "name_matrix")?;
    println!("{:?}", decoded);

    assert_eq!(value, decoded);

    let directly_nested_abi = r#"
    {
        "version": "eosio::abi/1.0",
        "types": [{
            "new_type_name": "name_arr",
            "type": "name[]"
        },{
            "new_type_name": "name_matrix",
            "type": "name[][]"
        }],
        "structs": [],
        "actions": [],
        "tables": [],
        "ricardian_clauses": []
    }
    "#;
    let _abi = ABI::from_str(directly_nested_abi);
    // check_error!(abi, ABIError::IntegrityError { .. }, "invalid type used in typedefs");

    Ok(())
}

#[test]
fn abi_cycle() -> Result<()> {
    init();
    // NOTE: we'd like "circular reference" here in the message but the issue is caught before
    //       by a different integrity check (namely: we can define the same type twice)
    check_invalid_abi!("data/typedef_cycle_abi.json", "type already exists");
    check_invalid_abi!("data/struct_cycle_abi.json", "circular reference in struct");
    Ok(())
}

#[test]
fn abi_type_repeat() -> Result<()> {
    init();
    check_invalid_abi!("data/abi_type_repeat.json", "type already exists");
    Ok(())
}

#[test]
fn abi_struct_repeat() -> Result<()> {
    init();
    check_invalid_abi!("data/abi_struct_repeat.json", "duplicate struct definition");
    Ok(())
}

#[test]
fn abi_action_repeat() -> Result<()> {
    init();
    check_invalid_abi!("data/abi_action_repeat.json", "duplicate action definition");
    Ok(())
}

#[test]
fn abi_table_repeat() -> Result<()> {
    init();
    check_invalid_abi!("data/abi_table_repeat.json", "duplicate table definition");
    Ok(())
}

#[test]
fn abi_type_def() -> Result<()> {
    init();

    let abi = ABI::from_str(r#"
    {
        "version": "eosio::abi/1.0",
        "types": [{
            "new_type_name": "account_name",
            "type": "name"
        }],
        "structs": [{
            "name": "transfer",
            "base": "",
            "fields": [{
                "name": "from",
                "type": "account_name"
            },{
                "name": "to",
                "type": "name"
            },{
                "name": "amount",
                "type": "uint64"
            }]
        }],
        "actions": [{
            "name": "transfer",
            "type": "transfer",
            "ricardian_contract": "transfer contract"
        }],
        "tables": []
    }
    "#)?;

    assert!(abi.is_type("name".into()));
    assert!(abi.is_type("account_name".into()));

    let data = json!({
        "from": "kevin",
        "to": "dan",
        "amount": 16
    });

    verify_byte_round_trip(&abi, "transfer", &data)
}

#[test]
fn abi_type_loop() -> Result<()> {
    init();
    check_invalid_abi!("data/abi_type_loop.json", "type already exists");
    Ok(())
}

#[test]
fn abi_std_optional() -> Result<()> {
    init();

    let abi = ABI::from_str(r#"
    {
        "version": "eosio::abi/1.2",
        "types": [],
        "structs": [{
            "name": "fees",
            "base": "",
            "fields": [{
                "name": "gas_price",
                "type": "uint64?"
            },{
                "name": "miner_cut",
                "type": "uint32?"
            },{
                "name": "bridge_fee",
                "type": "uint32?"
            }]
        }],
        "actions": [{
            "name": "fees",
            "type": "fees",
            "ricardian_contract": ""
        }],
        "tables": [],
        "ricardian_clauses": [],
        "variants": [],
        "action_results": []
    }
    "#)?;

    // check conversion when all members are provided
    verify_byte_round_trip(&abi, "fees", &json!({
        "gas_price": "42",
        "miner_cut": "2",
        "bridge_fee": "2",
    }))?;

    // check conversion when the first optional member is missing
    verify_byte_round_trip(&abi, "fees", &json!({
        "miner_cut": "2",
        "bridge_fee": "2",
    }))?;

    // check conversion when the second optional member is missing
    verify_byte_round_trip(&abi, "fees", &json!({
        "gas_price": "42",
        "bridge_fee": "2",
    }))?;

    // check conversion when the last optional member is missing
    verify_byte_round_trip(&abi, "fees", &json!({
        "gas_price": "42",
        "miner_cut": "2",
    }))?;

    // check conversion when all optional members are missing
    verify_byte_round_trip(&abi, "fees", &json!({}))?;

    Ok(())
}

#[test]
fn abi_type_redefine() -> Result<()> {
    init();
    check_invalid_abi!("data/abi_type_redefine.json", "circular reference in type");
    Ok(())
}

#[test]
fn abi_type_redefine_to_name() -> Result<()> {
    init();
    check_invalid_abi!("data/abi_type_redefine_to_name.json", "type already exists");
    Ok(())
}

// NOTE: the JSON in Spring is not correct, hence the test either can't be correct
// TODO: report bug!!
// NOTE: we change the behavior of this test as we can and actually want to allow
//       recursive structs like this, can be useful eg. to represent trees
#[test]
fn abi_type_nested_in_vector() -> Result<()> {
    init();

    let abi = r#"
    {
        "version": "eosio::abi/1.0",
        "types": [],
        "structs": [{
            "name": "store_t",
            "base": "",
            "fields": [{
                "name": "id",
                "type": "uint64"
            },{
                "name": "children",
                "type": "store_t[]"
            }]
        }],
        "actions": [],
        "tables": []
    }
    "#;

    let abi = ABI::from_str(abi);
    assert!(abi.is_ok());

    Ok(())
}


#[test]
fn abi_account_name_in_eosio_abi() -> Result<()> {
    init();

    let abi_def = include_str!("data/abi_account_name_in_eosio_abi.json");

    let abi = ABI::from_definition(&ABIDefinition::from_str(abi_def)?.with_contract_abi()?);
    check_integrity_error!(abi, "type already exists");

    Ok(())
}

// Unlimited array size during abi serialization can exhaust memory and crash the process
#[test]
fn abi_large_array() -> Result<()> {
    init();

    let abi = ABI::from_str(r#"
    {
        "version": "eosio::abi/1.1",
        "types": [],
        "structs": [{
            "name": "hi",
            "base": "",
            "fields": [{"name": "a", "type": "int8"}]
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
    check_error!(result, ABIError::DecodeError { .. }, "stream ended unexpectedly; unable to unpack field 'a' of struct 'hi'");

    Ok(())
}

// Unlimited array size during abi serialization can exhaust memory and crash the process
// a non-zero struct would fail early like in the test before, but a zero-sized struct
// will seemingly loop forever (as it is deserializing them it doesn't exhaust the stream
// so can go on for a very long time)
#[test]
#[cfg(feature = "hardened")]
fn abi_large_array_hardened() -> Result<()> {
    init();

    let abi = ABI::from_str(r#"
    {
        "version": "eosio::abi/1.1",
        "types": [],
        "structs": [{
            "name": "hi",
            "base": "",
            "fields": []
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
    check_error!(result, ABIError::DecodeError { .. }, "timeout or something like that");

    Ok(())
}

#[test]
fn abi_is_type_recursion() -> Result<()> {
    init();
    check_invalid_abi!("data/abi_is_type_recursion.json", "invalid type");
    Ok(())
}

#[test]
#[cfg(feature = "hardened")]
fn abi_recursive_structs() -> Result<()> {
    init();

    let abi = ABI::from_str(include_str!("data/abi_recursive_structs.json"))?;

    let data = json!({"user": "eosio"});
    let encoded = abi.variant_to_binary("hi2", &data)?;

    let result = abi.binary_to_variant("hi", encoded);
    check_encode_error!(result, "yep");

    Ok(())
}

#[test]
fn variants() -> Result<()> {
    init();

    let duplicate_variant_abi = r#"
    {
        "version": "eosio::abi/1.1",
        "variants": [
            {"name": "v1", "types": ["int8", "string", "bool"]},
            {"name": "v1", "types": ["int8", "string", "bool"]}
        ]
    }
    "#;

    let variant_abi_invalid_type = r#"
    {
        "version": "eosio::abi/1.1",
        "variants": [
            {"name": "v1", "types": ["int91", "string", "bool"]}
        ]
    }
    "#;

    let variant_abi = r#"
    {
        "version": "eosio::abi/1.1",
        "types": [
            {"new_type_name": "foo", "type": "s"},
            {"new_type_name": "bar", "type": "s"}
        ],
        "structs": [
            {"name": "s", "base": "", "fields": [
                {"name": "i0", "type": "int8"},
                {"name": "i1", "type": "int8"}
            ]}
        ],
        "variants": [
            {"name": "v1", "types": ["int8", "string", "int16"]},
            {"name": "v2", "types": ["foo", "bar"]}
        ]
    }
    "#;

    let result = ABI::from_str(duplicate_variant_abi);
    check_integrity_error!(result, "duplicate variants definition");

    let result = ABI::from_str(variant_abi_invalid_type);
    check_integrity_error!(result, "invalid type `int91` used in variant 'v1'");

    // round-trip abi through multiple formats
    // json -> variant -> abi_def -> bin
    let mut stream = ByteStream::new();
    ABIDefinition::from_str(variant_abi)?.to_bin(&mut stream);
    // bin -> abi_def -> variant -> abi_def
    let abi = ABI::from_str(&json!(ABIDefinition::from_bin(&mut stream)?).to_string())?;

    // expected array containing variant
    let result = abi.variant_to_binary("v1", &json!(9));
    check_encode_error!(result, "expected input to be an array of 2 elements while processing variant: 9");

    let result = abi.variant_to_binary("v1", &json!([4]));
    check_encode_error!(result, "expected input to be an array of 2 elements while processing variant: [4]");

    let result = abi.variant_to_binary("v1", &json!([4, 5]));
    check_encode_error!(result, "expected variant typename to be a string: 4");

    let result = abi.variant_to_binary("v1", &json!([4, 5, 6]));
    check_encode_error!(result, "expected input to be an array of 2 elements while processing variant: [4,5,6]");

    // type is not valid within this variant
    let result = abi.variant_to_binary("v1", &json!(["int9", 21]));
    check_encode_error!(result, "specified type `int9` is not valid within the variant 'v1'");

    verify_round_trip(&abi, "v1", &json!(["int8",21]), "0015")?;
    verify_round_trip(&abi, "v1", &json!(["string","abcd"]), "010461626364")?;
    verify_round_trip(&abi, "v1", &json!(["int16",3]), "020300")?;
    verify_round_trip(&abi, "v1", &json!(["int16",4]), "020400")?;
    verify_round_trip(&abi, "v2", &json!(["foo",{"i0":5,"i1":6}]), "000506")?;
    verify_round_trip(&abi, "v2", &json!(["bar",{"i0":5,"i1":6}]), "010506")?;

    Ok(())
}

#[test]
fn aliased_variants() -> Result<()> {
    init();

    let aliased_variant = r#"
    {
        "version": "eosio::abi/1.1",
        "types": [
            { "new_type_name": "foo", "type": "foo_variant" }
        ],
        "variants": [
            {"name": "foo_variant", "types": ["int8", "string"]}
        ]
    }
    "#;

    // round-trip abi through multiple formats
    // json -> variant -> abi_def -> bin
    let mut stream = ByteStream::new();
    ABIDefinition::from_str(aliased_variant)?.to_bin(&mut stream);
    // bin -> abi_def -> variant -> abi_def
    let abi = ABI::from_str(&json!(ABIDefinition::from_bin(&mut stream)?).to_string())?;

    verify_round_trip(&abi, "foo", &json!(["int8",21]), "0015")
}

#[test]
fn variant_of_aliases() -> Result<()> {
    init();

    let aliased_variant = r#"
    {
        "version": "eosio::abi/1.1",
        "types": [
            { "new_type_name": "foo_0", "type": "int8" },
            { "new_type_name": "foo_1", "type": "string" }
        ],
        "variants": [
            {"name": "foo", "types": ["foo_0", "foo_1"]}
        ]
    }
    "#;
    let abi = ABI::from_str(aliased_variant)?;

    verify_round_trip(&abi, "foo", &json!(["foo_0",21]), "0015")
}

#[test]
fn extend() -> Result<()> {
    init();

    // NOTE: Ideally this ABI would be rejected during validation for an improper definition for struct "s2".
    //       Such a check is not yet implemented during validation, but it can check during serialization.
    let abi = ABI::from_str(r#"
    {
        "version": "eosio::abi/1.1",
        "structs": [
            {"name": "s", "base": "", "fields": [
                {"name": "i0", "type": "int8"},
                {"name": "i1", "type": "int8"},
                {"name": "i2", "type": "int8$"},
                {"name": "a", "type": "int8[]$"},
                {"name": "o", "type": "int8?$"}
            ]},
            {"name": "s2", "base": "", "fields": [
                {"name": "i0", "type": "int8"},
                {"name": "i1", "type": "int8$"},
                {"name": "i2", "type": "int8"}
            ]}
        ]
    }
    "#)?;

    // missing i1
    let result = abi.variant_to_binary("s", &json!({"i0":5}));
    check_encode_error!(result, "missing field 'i1' in input object while processing struct 's'");

    // Unexpected 'a'
    let result = abi.variant_to_binary("s", &json!({"i0":5,"i1":6,"a":[8,9,10]}));
    check_encode_error!(result, "Unexpected field 'a' found in input object while processing struct");

    verify_round_trip(&abi, "s", &json!({"i0":5,"i1":6}), "0506")?;
    verify_round_trip(&abi, "s", &json!({"i0":5,"i1":6,"i2":7}), "050607")?;
    verify_round_trip(&abi, "s", &json!({"i0":5,"i1":6,"i2":7,"a":[8,9,10]}), "0506070308090a")?;
    verify_round_trip(&abi, "s", &json!({"i0":5,"i1":6,"i2":7,"a":[8,9,10],"o":null}), "0506070308090a00")?;
    verify_round_trip(&abi, "s", &json!({"i0":5,"i1":6,"i2":7,"a":[8,9,10],"o":31}), "0506070308090a011f")?;

    verify_round_trip2(&abi, "s", &json!([5,6]), "0506", r#"{"i0":5,"i1":6}"#)?;
    verify_round_trip2(&abi, "s", &json!([5,6,7]), "050607", r#"{"i0":5,"i1":6,"i2":7}"#)?;
    verify_round_trip2(&abi, "s", &json!([5,6,7,[8,9,10]]), "0506070308090a", r#"{"i0":5,"i1":6,"i2":7,"a":[8,9,10]}"#)?;
    verify_round_trip2(&abi, "s", &json!([5,6,7,[8,9,10],null]), "0506070308090a00", r#"{"i0":5,"i1":6,"i2":7,"a":[8,9,10],"o":null}"#)?;
    verify_round_trip2(&abi, "s", &json!([5,6,7,[8,9,10],31]), "0506070308090a011f", r#"{"i0":5,"i1":6,"i2":7,"a":[8,9,10],"o":31}"#)?;

    let result = abi.variant_to_binary("s2", &json!({"i0":1}));
    check_encode_error!(result, "Encountered field 'i2' without binary extension designation while processing struct");

    Ok(())
}

#[test]
fn version() -> Result<()> {
    init();

    let abi = ABI::from_str("{}");
    check_error!(abi, ABIError::JsonError { .. }, "cannot deserialize ABIDefinition from JSON");
    // check_integrity_error!(abi, "yepyep");

    let abi = ABI::from_str(r#"{"version": ""}"#);
    check_error!(abi, ABIError::VersionError { .. }, "unsupported ABI version");

    let abi = ABI::from_str(r#"{"version": "eosio::abi/9.0"}"#);
    check_error!(abi, ABIError::VersionError { .. }, "unsupported ABI version");

    assert!(ABI::from_str(r#"{"version": "eosio::abi/1.0"}"#).is_ok());
    assert!(ABI::from_str(r#"{"version": "eosio::abi/1.1"}"#).is_ok());
    assert!(ABI::from_str(r#"{"version": "eosio::abi/1.2"}"#).is_ok());

    Ok(())
}

#[test]
fn abi_serialize_incomplete_json_array() -> Result<()> {
    init();

    let abi = ABI::from_str(r#"{
        "version": "eosio::abi/1.0",
        "structs": [
            {"name": "s", "base": "", "fields": [
                {"name": "i0", "type": "int8"},
                {"name": "i1", "type": "int8"},
                {"name": "i2", "type": "int8"}
            ]}
        ]
    }"#)?;

    let result = abi.variant_to_binary("s", &json!([]));
    check_encode_error!(result, "early end to input array specifying the fields of struct");

    let result = abi.variant_to_binary("s", &json!([1, 2]));
    check_encode_error!(result, "early end to input array specifying the fields of struct");

    verify_round_trip2(&abi, "s", &json!([1,2,3]), "010203", r#"{"i0":1,"i1":2,"i2":3}"#)?;

    Ok(())
}

#[test]
fn abi_serialize_incomplete_json_object() -> Result<()> {
    init();

    let abi = ABI::from_str(r#"
    {
        "version": "eosio::abi/1.0",
        "structs": [
            {"name": "s1", "base": "", "fields": [
                {"name": "i0", "type": "int8"},
                {"name": "i1", "type": "int8"}
            ]},
            {"name": "s2", "base": "", "fields": [
                {"name": "f0", "type": "s1"},
                {"name": "i2", "type": "int8"}
            ]}
        ]
    }
    "#)?;

    let result = abi.variant_to_binary("s2", &json!({}));
    check_encode_error!(result, "missing field 'f0' in input object");

    let result = abi.variant_to_binary("s2", &json!({"f0":{"i0":1}}));
    check_encode_error!(result, "missing field 'i1' in input object");

    verify_round_trip(&abi, "s2", &json!({"f0":{"i0":1,"i1":2},"i2":3}), "010203")?;

    Ok(())
}

#[test]
fn abi_serialize_json_mismatched_type() -> Result<()> {
    init();

    let abi = ABI::from_str(r#"
    {
        "version": "eosio::abi/1.0",
        "structs": [
            {"name": "s1", "base": "", "fields": [
                {"name": "i0", "type": "int8"}
            ]},
            {"name": "s2", "base": "", "fields": [
                {"name": "f0", "type": "s1"},
                {"name": "i1", "type": "int8"}
            ]}
        ]
    }
    "#)?;

    let result = abi.variant_to_binary("s2", &json!({"f0":1,"i1":2}));
    // FIXME: add context for ABI traversal so we can have the better error message
    // check_encode_error!(result, "unexpected input encountered while encoding struct 's2.f0'");
    check_encode_error!(result, "unexpected input while encoding struct 's1'");

    verify_round_trip(&abi, "s2", &json!({"f0":{"i0":1},"i1":2}), "0102")?;

    Ok(())
}

#[test]
fn abi_serialize_json_empty_name() -> Result<()> {
    init();

    let abi = ABI::from_str(r#"
    {
        "version": "eosio::abi/1.0",
        "structs": [
            {"name": "s1", "base": "", "fields": [
                {"name": "", "type": "int8"}
            ]}
        ]
    }
    "#)?;

    let result = abi.variant_to_binary("s1", &json!({"": 1}));
    assert!(result.is_ok());

    // check_error!(result, ABIError::EncodeError { .. }, "blip");

    verify_round_trip(&abi, "s1", &json!({"": 1}), "01")?;

    Ok(())
}

#[test]
fn serialize_optional_struct_type() -> Result<()> {
    init();

    let abi = ABI::from_str(r#"
    {
        "version": "eosio::abi/1.0",
        "structs": [
            {"name": "s", "base": "", "fields": [
                {"name": "i0", "type": "int8"}
            ]}
        ]
    }
    "#)?;

    verify_round_trip(&abi, "s?", &json!({"i0": 5}), "0105")?;
    verify_round_trip(&abi, "s?", &JsonValue::Null, "00")?;

    Ok(())
}
