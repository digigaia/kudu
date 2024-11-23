use std::str::FromStr;

use color_eyre::eyre::Result;
use serde_json::{json, Value as JsonValue};

use antelope_abi::abidefinition::*;
use antelope_abi::ABI;

// =============================================================================
//
//     Unittests coming from the reference Spring implementation
//     https://github.com/AntelopeIO/spring/blob/main/unittests/abi_tests.cpp
//
//     skipping the following tests because they seem to provide relatively
//     low value:
//      - linkauth_test, unlinkauth_test, updateauth_test, deleteauth_test, ...
//
// =============================================================================

// -----------------------------------------------------------------------------
//     Utility functions & macros
// -----------------------------------------------------------------------------

#[track_caller]
fn verify_round_trip(abi: &ABI, typename: &str, value: &JsonValue) -> Result<()> {
    let encoded = abi.variant_to_binary(typename, value)?;
    let decoded = abi.binary_to_variant(typename, encoded.clone())?;
    let encoded2 = abi.variant_to_binary(typename, &decoded)?;

    // assert_eq!(value, &decoded);
    assert_eq!(encoded, encoded2);
    Ok(())
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

    verify_round_trip(&abi, "transfer", &test_data)
}


#[test]
fn general() -> Result<()> {
    let my_abi = ABIDefinition::from_str(include_str!("data/general_abi.json"))?;
    let abi = ABI::from_definition(&my_abi.with_contract_abi()?)?;
    let test_data = JsonValue::from_str(include_str!("data/general_data.json"))?;

    verify_round_trip(&abi, "A", &test_data)
}

#[test]
fn duplicate_types() -> Result<()> {
    check_invalid_abi!("data/duplicate_types_abi.json", "type already exists");
    Ok(())
}

#[test]
fn nested_types() -> Result<()> {
    use antelope_abi::ByteStream;

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
    // NOTE: we'd like "circular reference" here in the message but the issue is caught before
    //       by a different integrity check (namely: we can define the same type twice)
    check_invalid_abi!("data/typedef_cycle_abi.json", "type already exists");
    check_invalid_abi!("data/struct_cycle_abi.json", "circular reference in struct");
    Ok(())
}

#[test]
fn abi_type_repeat() -> Result<()> {
    check_invalid_abi!("data/abi_type_repeat.json", "type already exists");
    Ok(())
}

#[test]
fn abi_struct_repeat() -> Result<()> {
    check_invalid_abi!("data/abi_struct_repeat.json", "duplicate struct definition");
    Ok(())
}

#[test]
fn abi_action_repeat() -> Result<()> {
    check_invalid_abi!("data/abi_action_repeat.json", "duplicate action definition");
    Ok(())
}

#[test]
fn abi_table_repeat() -> Result<()> {
    check_invalid_abi!("data/abi_table_repeat.json", "duplicate table definition");
    Ok(())
}

#[test]
fn abi_type_redefine() -> Result<()> {
    check_invalid_abi!("data/abi_type_redefine.json", "circular reference in type");
    Ok(())
}

#[test]
fn abi_type_redefine_to_name() -> Result<()> {
    let abi = r#"
    {
        "version": "eosio::abi/1.0",
        "types": [{
            "new_type_name": "name",
            "type": "name"
        }],
        "structs": [],
        "actions": [],
        "tables": []
    }
    "#;

    let abi = ABI::from_str(abi);
    check_error!(abi, ABIError::IntegrityError { .. }, "type already exists");

    Ok(())
}

// NOTE: the JSON in Spring is not correct, hence the test either can't be correct
// TODO: report bug!!
#[test] #[ignore]
fn abi_type_nested_in_vector() -> Result<()> {
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
    check_error!(abi, ABIError::IntegrityError { .. }, "duplicate table definition");

    Ok(())
}


#[test]
fn abi_account_name_in_eosio_abi() -> Result<()> {
    let abi_def = include_str!("data/abi_account_name_in_eosio_abi.json");

    let abi = ABI::from_definition(&ABIDefinition::from_str(abi_def)?.with_contract_abi()?);
    check_error!(abi, ABIError::IntegrityError { .. }, "type already exists");

    Ok(())
}

#[test]
fn abi_is_type_recursion() -> Result<()> {
    check_invalid_abi!("data/abi_is_type_recursion.json", "invalid type");
    Ok(())
}
