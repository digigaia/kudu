use serde_json::Value;
// use anyhow::Result;
use color_eyre::eyre::Result;
use log::debug;

use antelope::abi::*;
use antelope::{
    ABIEncoder, ByteStream, bin_to_hex,
    types::InvalidValue,
};


////////////////////////////////////////////////////////////////////////////////
//                                                                            //
// following tests are coming from                                            //
// https://github.com/AntelopeIO/abieos/blob/main/src/test.cpp#L577           //
//                                                                            //
// to get the hex representation of each test, you need to compile and run    //
// the `test_abieos` binary from this repo                                    //
//                                                                            //
////////////////////////////////////////////////////////////////////////////////

////////////////////////////////////////////////////////////////////////////////
//                                                                            //
// TODO: MISSING TYPES                                                        //
//  - f128                                                                    //
//  - string                                                                  //
//  - checksum{160,256,512}                                                   //
//  - public_key                                                              //
//  - private_key                                                             //
//  - signature                                                               //
//  - symbol_code                                                             //
//  - extended_asset                                                          //
//  - transaction_trace                                                       //
//  - transaction_trace_msg                                                   //
//  -                                                                         //

// implement .prefix() for name type
// check unittests and validity of non-normalized names

//                                                                            //
////////////////////////////////////////////////////////////////////////////////


static TEST_ABI: &str = r#"{
    "version": "eosio::abi/1.1",
    "structs": [
        {
            "name": "s1",
            "fields": [
                {
                    "name": "x1",
                    "type": "int8"
                }
            ]
        },
        {
            "name": "s2",
            "fields": [
                {
                    "name": "y1",
                    "type": "int8$"
                },
                {
                    "name": "y2",
                    "type": "int8$"
                }
            ]
        },
        {
            "name": "s3",
            "fields": [
                {
                    "name": "z1",
                    "type": "int8$"
                },
                {
                    "name": "z2",
                    "type": "v1$"
                },
                {
                    "name": "z3",
                    "type": "s2$"
                }
            ]
        },
        {
            "name": "s4",
            "fields": [
                {
                    "name": "a1",
                    "type": "int8?$"
                },
                {
                    "name": "b1",
                    "type": "int8[]$"
                }
            ]
        },
        {
            "name": "s5",
            "fields": [
                {
                    "name": "x1",
                    "type": "int8"
                },
                {
                    "name": "x2",
                    "type": "int8"
                },
                {
                    "name": "x3",
                    "type": "s6"
                }
            ]
        },
        {
            "name": "s6",
            "fields": [
                {
                    "name": "c1",
                    "type": "int8"
                },
                {
                    "name": "c2",
                    "type": "s5[]"
                },
                {
                    "name": "c3",
                    "type": "int8"
                }
            ]
        }
    ],
    "variants": [
        {
            "name": "v1",
            "types": ["int8","s1","s2"]
        }
    ]
}"#;

static TOKEN_HEX_ABI: &str = concat!(
    "0e656f73696f3a3a6162692f312e30010c6163636f756e745f6e616d65046e61",
    "6d6505087472616e7366657200040466726f6d0c6163636f756e745f6e616d65",
    "02746f0c6163636f756e745f6e616d65087175616e7469747905617373657404",
    "6d656d6f06737472696e67066372656174650002066973737565720c6163636f",
    "756e745f6e616d650e6d6178696d756d5f737570706c79056173736574056973",
    "737565000302746f0c6163636f756e745f6e616d65087175616e746974790561",
    "73736574046d656d6f06737472696e67076163636f756e7400010762616c616e",
    "63650561737365740e63757272656e63795f7374617473000306737570706c79",
    "0561737365740a6d61785f737570706c79056173736574066973737565720c61",
    "63636f756e745f6e616d6503000000572d3ccdcd087472616e73666572000000",
    "000000a531760569737375650000000000a86cd4450663726561746500020000",
    "00384f4d113203693634010863757272656e6379010675696e74363407616363",
    "6f756e740000000000904dc603693634010863757272656e6379010675696e74",
    "36340e63757272656e63795f7374617473000000");


static TRANSACTION_ABI: &str = r#"{
    "version": "eosio::abi/1.0",
    "types": [
        {
            "new_type_name": "account_name",
            "type": "name"
        },
        {
            "new_type_name": "action_name",
            "type": "name"
        },
        {
            "new_type_name": "permission_name",
            "type": "name"
        }
    ],
    "structs": [
        {
            "name": "permission_level",
            "base": "",
            "fields": [
                {
                    "name": "actor",
                    "type": "account_name"
                },
                {
                    "name": "permission",
                    "type": "permission_name"
                }
            ]
        },
        {
            "name": "action",
            "base": "",
            "fields": [
                {
                    "name": "account",
                    "type": "account_name"
                },
                {
                    "name": "name",
                    "type": "action_name"
                },
                {
                    "name": "authorization",
                    "type": "permission_level[]"
                },
                {
                    "name": "data",
                    "type": "bytes"
                }
            ]
        },
        {
            "name": "extension",
            "base": "",
            "fields": [
                {
                    "name": "type",
                    "type": "uint16"
                },
                {
                    "name": "data",
                    "type": "bytes"
                }
            ]
        },
        {
            "name": "transaction_header",
            "base": "",
            "fields": [
                {
                    "name": "expiration",
                    "type": "time_point_sec"
                },
                {
                    "name": "ref_block_num",
                    "type": "uint16"
                },
                {
                    "name": "ref_block_prefix",
                    "type": "uint32"
                },
                {
                    "name": "max_net_usage_words",
                    "type": "varuint32"
                },
                {
                    "name": "max_cpu_usage_ms",
                    "type": "uint8"
                },
                {
                    "name": "delay_sec",
                    "type": "varuint32"
                }
            ]
        },
        {
            "name": "transaction",
            "base": "transaction_header",
            "fields": [
                {
                    "name": "context_free_actions",
                    "type": "action[]"
                },
                {
                    "name": "actions",
                    "type": "action[]"
                },
                {
                    "name": "transaction_extensions",
                    "type": "extension[]"
                }
            ]
        }
    ]
}"#;

static _TEST_KV_TABLES_ABI: &str = r#"{
    "version": "eosio::abi/1.2",
    "types": [],
    "structs": [
        {
            "name": "get",
            "base": "",
            "fields": []
        },
        {
            "name": "iteration",
            "base": "",
            "fields": []
        },
        {
            "name": "my_struct",
            "base": "",
            "fields": [
                {
                    "name": "primary",
                    "type": "name"
                },
                {
                    "name": "foo",
                    "type": "string"
                },
                {
                    "name": "bar",
                    "type": "uint64"
                },
                {
                    "name": "fullname",
                    "type": "string"
                },
                {
                    "name": "age",
                    "type": "uint32"
                }
            ]
        },
        {
            "name": "nonunique",
            "base": "",
            "fields": []
        },
        {
            "name": "setup",
            "base": "",
            "fields": []
        },
        {
            "name": "tuple_string_uint32",
            "base": "",
            "fields": [
                {
                    "name": "field_0",
                    "type": "string"
                },
                {
                    "name": "field_1",
                    "type": "uint32"
                }
            ]
        },
        {
            "name": "update",
            "base": "",
            "fields": []
        },
        {
            "name": "updateerr1",
            "base": "",
            "fields": []
        },
        {
            "name": "updateerr2",
            "base": "",
            "fields": []
        }
    ],
    "actions": [
        {
            "name": "get",
            "type": "get",
            "ricardian_contract": ""
        },
        {
            "name": "iteration",
            "type": "iteration",
            "ricardian_contract": ""
        },
        {
            "name": "nonunique",
            "type": "nonunique",
            "ricardian_contract": ""
        },
        {
            "name": "setup",
            "type": "setup",
            "ricardian_contract": ""
        },
        {
            "name": "update",
            "type": "update",
            "ricardian_contract": ""
        },
        {
            "name": "updateerr1",
            "type": "updateerr1",
            "ricardian_contract": ""
        },
        {
            "name": "updateerr2",
            "type": "updateerr2",
            "ricardian_contract": ""
        }
    ],
    "tables": []
}"#;

static PACKED_TRANSACTION_ABI: &str = r#"{
    "version": "eosio::abi/1.0",
    "types": [
        {
            "new_type_name": "account_name",
            "type": "name"
        },
        {
            "new_type_name": "action_name",
            "type": "name"
        },
        {
            "new_type_name": "permission_name",
            "type": "name"
        }
    ],
    "structs": [
        {
            "name": "permission_level",
            "base": "",
            "fields": [
                {
                    "name": "actor",
                    "type": "account_name"
                },
                {
                    "name": "permission",
                    "type": "permission_name"
                }
            ]
        },
        {
            "name": "action",
            "base": "",
            "fields": [
                {
                    "name": "account",
                    "type": "account_name"
                },
                {
                    "name": "name",
                    "type": "action_name"
                },
                {
                    "name": "authorization",
                    "type": "permission_level[]"
                },
                {
                    "name": "data",
                    "type": "bytes"
                }
            ]
        },
        {
            "name": "extension",
            "base": "",
            "fields": [
                {
                    "name": "type",
                    "type": "uint16"
                },
                {
                    "name": "data",
                    "type": "bytes"
                }
            ]
        },
        {
            "name": "transaction_header",
            "base": "",
            "fields": [
                {
                    "name": "expiration",
                    "type": "time_point_sec"
                },
                {
                    "name": "ref_block_num",
                    "type": "uint16"
                },
                {
                    "name": "ref_block_prefix",
                    "type": "uint32"
                },
                {
                    "name": "max_net_usage_words",
                    "type": "varuint32"
                },
                {
                    "name": "max_cpu_usage_ms",
                    "type": "uint8"
                },
                {
                    "name": "delay_sec",
                    "type": "varuint32"
                }
            ]
        },
        {
            "name": "transaction",
            "base": "transaction_header",
            "fields": [
                {
                    "name": "context_free_actions",
                    "type": "action[]"
                },
                {
                    "name": "actions",
                    "type": "action[]"
                },
                {
                    "name": "transaction_extensions",
                    "type": "extension[]"
                }
            ]
        },
        {
            "name": "packed_transaction_v0",
            "base": "",
            "fields": [
                {
                    "name": "signatures",
                    "type": "signature[]"
                },
                {
                    "name": "compression",
                    "type": "uint8"
                },
                {
                    "name": "packed_context_free_data",
                    "type": "bytes"
                },
                {
                    "name": "packed_trx",
                    "type": "transaction"
                }
            ]
        }
    ]
}"#;

fn init() {
    let _ = env_logger::builder().is_test(true).try_init();
}

fn try_encode(abi: &ABIEncoder, typename: &str, data: &str) -> Result<()> {
    let mut ds = ByteStream::new();
    let value: Value = serde_json::from_str(data).map_err(InvalidValue::from)?;
    abi.encode_variant(&mut ds, typename, &value)?;
    Ok(())
}

fn round_trip(abi: &ABIEncoder, typename: &str, data: &str, hex: &str, expected: &str) -> Result<()> {
    debug!(r#"==== round-tripping type "{typename}" with value {data}"#);
    let mut ds = ByteStream::new();
    let value: Value = serde_json::from_str(data)?;
    abi.encode_variant(&mut ds, typename, &value)?;

    assert_eq!(ds.hex_data(), hex.to_ascii_lowercase());

    let decoded = abi.decode_variant(&mut ds, typename)?;

    assert!(ds.leftover().is_empty());
    assert_eq!(decoded.to_string(), expected);

    Ok(())
}

fn check_error<F, T>(f: F, expected_error_msg: &str)
    where F: FnOnce() -> Result<T>
{
    match f() {
        Ok(_) => {
            panic!("expected error but everything went fine...");
        },
        Err(e) => {
            let received_msg = format!("{:?}", e);
            if !received_msg.contains(expected_error_msg) {
                eprintln!("{:?}\n", e);
                panic!(r#"expected error message with "{}", got: {}"#,
                       expected_error_msg, received_msg);
            }
        },
    }
}

/// check roundtrip JSON -> variant -> bin -> variant -> JSON
fn check_round_trip(abi: &ABIEncoder, typename: &str, data: &str, hex: &str) {
    round_trip(abi, typename, data, hex, data).unwrap()
}

fn check_round_trip2(abi: &ABIEncoder, typename: &str, data: &str, hex: &str, expected: &str) {
    round_trip(abi, typename, data, hex, expected).unwrap()
}


///// FIXME FIXME: what about the expected hex?
fn _check_error_trip(abi: &ABIEncoder, typename: &str, data: &str, error_msg: &str) {
    check_error(|| round_trip(abi, typename, data, "", data), error_msg);
}

fn str_to_hex(s: &str) -> String {
    format!("{:02x}{}", s.len(), bin_to_hex(s.as_bytes()))
}


#[test]
fn integration_test() -> Result<()> {
    init();

    let _test_abi_def = ABIDefinition::from_str(TEST_ABI)?;
    let _test_abi = ABIEncoder::from_abi(&_test_abi_def);

    let _transaction_abi_def = ABIDefinition::from_str(TRANSACTION_ABI)?;
    let _transaction_abi = ABIEncoder::from_abi(&_transaction_abi_def);

    let _token_abi = ABIEncoder::from_hex_abi(TOKEN_HEX_ABI)?;

    let _abi = &_transaction_abi;

    check_error(|| Ok(ABIDefinition::from_str("")?), "cannot parse JSON string");
    check_error(|| Ok(ABIEncoder::from_hex_abi("")?), "stream ended");
    check_error(|| Ok(ABIEncoder::from_hex_abi("00")?), "unsupported ABI version");
    check_error(|| Ok(ABIEncoder::from_hex_abi(&str_to_hex("eosio::abi/9.0"))?), "unsupported ABI version");
    check_error(|| Ok(ABIEncoder::from_hex_abi(&str_to_hex("eosio::abi/1.0"))?), "stream ended");
    check_error(|| Ok(ABIEncoder::from_hex_abi(&str_to_hex("eosio::abi/1.1"))?), "stream ended");

    Ok(())
}

#[test]
fn roundtrip_bool() -> Result<()> {
    init();

    let transaction_abi_def = ABIDefinition::from_str(TRANSACTION_ABI)?;
    let transaction_abi = ABIEncoder::from_abi(&transaction_abi_def);
    let abi = &transaction_abi;

    check_round_trip(abi, "bool", "true", "01");
    check_round_trip(abi, "bool", "false", "00");

    check_error(|| try_encode(abi, "bool", ""), "cannot parse JSON string");
    check_error(|| try_encode(abi, "bool", "trues"), "cannot parse JSON string");
    check_error(|| try_encode(abi, "bool", "null"), "cannot convert given variant");
    check_error(|| try_encode(abi, "bool", r#""foo""#), "cannot convert given variant");

    Ok(())
}

#[test]
fn roundtrip_i8() -> Result<()> {
    init();

    let transaction_abi_def = ABIDefinition::from_str(TRANSACTION_ABI)?;
    let transaction_abi = ABIEncoder::from_abi(&transaction_abi_def);
    let abi = &transaction_abi;


    check_round_trip(abi, "int8", "0", "00");
    check_round_trip(abi, "int8", "127", "7F");
    check_round_trip(abi, "int8", "-128", "80");
    check_round_trip(abi, "uint8", "0", "00");
    check_round_trip(abi, "uint8", "1", "01");
    check_round_trip(abi, "uint8", "254", "FE");
    check_round_trip(abi, "uint8", "255", "FF");

    check_error(|| try_encode(abi, "int8", "128"), "integer out of range");
    check_error(|| try_encode(abi, "int8", "-129"), "integer out of range");
    check_error(|| try_encode(abi, "uint8", "-1"), "cannot convert given variant");
    check_error(|| try_encode(abi, "uint8", "256"), "integer out of range");

    check_round_trip(abi, "uint8[]", "[]", "00");
    check_round_trip(abi, "uint8[]", "[10]", "010A");
    check_round_trip(abi, "uint8[]", "[10,9]", "020A09");
    check_round_trip(abi, "uint8[]", "[10,9,8]", "030A0908");

    Ok(())
}

#[test]
fn roundtrip_i16() -> Result<()> {
    init();

    let transaction_abi_def = ABIDefinition::from_str(TRANSACTION_ABI)?;
    let transaction_abi = ABIEncoder::from_abi(&transaction_abi_def);
    let abi = &transaction_abi;

    check_round_trip(abi, "int16", "0", "0000");
    check_round_trip(abi, "int16", "32767", "FF7F");
    check_round_trip(abi, "int16", "-32768", "0080");
    check_round_trip(abi, "uint16", "0", "0000");
    check_round_trip(abi, "uint16", "65535", "FFFF");

    check_error(|| try_encode(abi, "int16", "32768"), "integer out of range");
    check_error(|| try_encode(abi, "int16", "-32769"), "integer out of range");
    check_error(|| try_encode(abi, "uint16", "-1"), "cannot convert given variant");
    check_error(|| try_encode(abi, "uint16", "65536"), "integer out of range");

    Ok(())
}

#[test]
fn roundtrip_i32() -> Result<()> {
    init();

    let transaction_abi_def = ABIDefinition::from_str(TRANSACTION_ABI)?;
    let transaction_abi = ABIEncoder::from_abi(&transaction_abi_def);
    let abi = &transaction_abi;


    check_round_trip(abi, "int32", "0", "00000000");
    check_round_trip(abi, "int32", "2147483647", "FFFFFF7F");
    check_round_trip(abi, "int32", "-2147483648", "00000080");
    check_round_trip(abi, "uint32", "0", "00000000");
    check_round_trip(abi, "uint32", "4294967295", "FFFFFFFF");

    check_error(|| try_encode(abi, "int32", "2147483648"), "integer out of range");
    check_error(|| try_encode(abi, "int32", "-2147483649"), "integer out of range");
    check_error(|| try_encode(abi, "uint32", "-1"), "cannot convert given variant");
    check_error(|| try_encode(abi, "uint32", "4294967296"), "integer out of range");

    Ok(())
}

#[test]
fn roundtrip_i64() -> Result<()> {
    init();

    let transaction_abi_def = ABIDefinition::from_str(TRANSACTION_ABI)?;
    let transaction_abi = ABIEncoder::from_abi(&transaction_abi_def);
    let abi = &transaction_abi;


    check_round_trip(abi, "int64", "0", "0000000000000000");
    check_round_trip(abi, "int64", "1", "0100000000000000");
    check_round_trip(abi, "int64", "-1", "FFFFFFFFFFFFFFFF");
    check_round_trip(abi, "int64", "9223372036854775807", "FFFFFFFFFFFFFF7F");
    check_round_trip(abi, "int64", "-9223372036854775808", "0000000000000080");
    check_round_trip(abi, "uint64", "0", "0000000000000000");
    check_round_trip(abi, "uint64", "18446744073709551615", "FFFFFFFFFFFFFFFF");

    check_error(|| try_encode(abi, "int64", "9223372036854775808"), "cannot convert given variant");
    check_error(|| try_encode(abi, "int64", "-9223372036854775809"), "cannot convert given variant");
    check_error(|| try_encode(abi, "uint64", "-1"), "cannot convert given variant");
    check_error(|| try_encode(abi, "uint64", "18446744073709551616"), "cannot convert given variant");

    Ok(())
}

#[test]
fn roundtrip_i128() -> Result<()> {
    init();

    let transaction_abi_def = ABIDefinition::from_str(TRANSACTION_ABI)?;
    let transaction_abi = ABIEncoder::from_abi(&transaction_abi_def);
    let abi = &transaction_abi;


    check_round_trip(abi, "int128", r#""0""#, "00000000000000000000000000000000");
    check_round_trip(abi, "int128", r#""1""#, "01000000000000000000000000000000");
    check_round_trip(abi, "int128", r#""-1""#, "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF");
    check_round_trip(abi, "int128", r#""18446744073709551615""#, "FFFFFFFFFFFFFFFF0000000000000000");
    check_round_trip(abi, "int128", r#""-18446744073709551615""#, "0100000000000000FFFFFFFFFFFFFFFF");
    check_round_trip(abi, "int128", r#""170141183460469231731687303715884105727""#, "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFF7F");
    check_round_trip(abi, "int128", r#""-170141183460469231731687303715884105727""#, "01000000000000000000000000000080");
    check_round_trip(abi, "int128", r#""-170141183460469231731687303715884105728""#, "00000000000000000000000000000080");
    check_round_trip(abi, "uint128", r#""0""#, "00000000000000000000000000000000");
    check_round_trip(abi, "uint128", r#""18446744073709551615""#, "FFFFFFFFFFFFFFFF0000000000000000");
    check_round_trip(abi, "uint128", r#""340282366920938463463374607431768211454""#, "FEFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF");
    check_round_trip(abi, "uint128", r#""340282366920938463463374607431768211455""#, "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF");

    check_error(|| try_encode(abi, "int128", r#""170141183460469231731687303715884105728""#), "invalid integer");
    check_error(|| try_encode(abi, "int128", r#""-170141183460469231731687303715884105729""#), "invalid integer");
    check_error(|| try_encode(abi, "uint128", r#""-1""#), "invalid integer");
    check_error(|| try_encode(abi, "uint128", r#""340282366920938463463374607431768211456""#), "invalid integer");

    Ok(())
}

#[test]
fn roundtrip_varints() -> Result<()> {
    init();

    let transaction_abi_def = ABIDefinition::from_str(TRANSACTION_ABI)?;
    let transaction_abi = ABIEncoder::from_abi(&transaction_abi_def);
    let abi = &transaction_abi;


    check_round_trip(abi, "varuint32", "0", "00");
    check_round_trip(abi, "varuint32", "127", "7F");
    check_round_trip(abi, "varuint32", "128", "8001");
    check_round_trip(abi, "varuint32", "129", "8101");
    check_round_trip(abi, "varuint32", "16383", "FF7F");
    check_round_trip(abi, "varuint32", "16384", "808001");
    check_round_trip(abi, "varuint32", "16385", "818001");
    check_round_trip(abi, "varuint32", "2097151", "FFFF7F");
    check_round_trip(abi, "varuint32", "2097152", "80808001");
    check_round_trip(abi, "varuint32", "2097153", "81808001");
    check_round_trip(abi, "varuint32", "268435455", "FFFFFF7F");
    check_round_trip(abi, "varuint32", "268435456", "8080808001");
    check_round_trip(abi, "varuint32", "268435457", "8180808001");
    check_round_trip(abi, "varuint32", "4294967294", "FEFFFFFF0F");
    check_round_trip(abi, "varuint32", "4294967295", "FFFFFFFF0F");

    check_round_trip(abi, "varint32", "0", "00");
    check_round_trip(abi, "varint32", "-1", "01");
    check_round_trip(abi, "varint32", "1", "02");
    check_round_trip(abi, "varint32", "-2", "03");
    check_round_trip(abi, "varint32", "2", "04");
    check_round_trip(abi, "varint32", "-2147483647", "FDFFFFFF0F");
    check_round_trip(abi, "varint32", "2147483647", "FEFFFFFF0F");
    check_round_trip(abi, "varint32", "-2147483648", "FFFFFFFF0F");

    check_error(|| try_encode(abi, "varuint32", "4294967296"), "out of range");
    check_error(|| try_encode(abi, "varuint32", "-1"), "cannot convert given variant");
    check_error(|| try_encode(abi, "varint32", "2147483648"), "out of range");
    check_error(|| try_encode(abi, "varint32", "-2147483649"), "out of range");

    Ok(())
}

#[test]
fn roundtrip_floats() -> Result<()> {
    init();

    let transaction_abi_def = ABIDefinition::from_str(TRANSACTION_ABI)?;
    let transaction_abi = ABIEncoder::from_abi(&transaction_abi_def);
    let abi = &transaction_abi;


    check_round_trip(abi, "float32", "0.0", "00000000");
    check_round_trip(abi, "float32", "0.125", "0000003E");
    check_round_trip(abi, "float32", "-0.125", "000000BE");
    check_round_trip(abi, "float64", "0.0", "0000000000000000");
    check_round_trip(abi, "float64", "0.125", "000000000000C03F");
    check_round_trip(abi, "float64", "-0.125", "000000000000C0BF");
    check_round_trip2(abi, "float64", "151115727451828646838272.0", "000000000000C044", "1.5111572745182865e23");
    check_round_trip2(abi, "float64", "-151115727451828646838272.0", "000000000000C0C4", "-1.5111572745182865e23");

    Ok(())
}


#[test]
fn roundtrip_datetimes() -> Result<()> {
    init();

    let transaction_abi_def = ABIDefinition::from_str(TRANSACTION_ABI)?;
    let transaction_abi = ABIEncoder::from_abi(&transaction_abi_def);
    let abi = &transaction_abi;


    check_round_trip(abi, "time_point_sec", r#""1970-01-01T00:00:00.000""#, "00000000");
    check_round_trip(abi, "time_point_sec", r#""2018-06-15T19:17:47.000""#, "DB10245B");
    check_round_trip(abi, "time_point_sec", r#""2030-06-15T19:17:47.000""#, "5B6FB671");

    check_round_trip(abi, "time_point", r#""1970-01-01T00:00:00.000""#, "0000000000000000");
    check_round_trip(abi, "time_point", r#""1970-01-01T00:00:00.001""#, "E803000000000000");
    check_round_trip(abi, "time_point", r#""1970-01-01T00:00:00.002""#, "D007000000000000");
    check_round_trip(abi, "time_point", r#""1970-01-01T00:00:00.010""#, "1027000000000000");
    check_round_trip(abi, "time_point", r#""1970-01-01T00:00:00.100""#, "A086010000000000");
    check_round_trip(abi, "time_point", r#""2018-06-15T19:17:47.000""#, "C0AC3112B36E0500");
    check_round_trip(abi, "time_point", r#""2018-06-15T19:17:47.999""#, "18EB4012B36E0500");
    check_round_trip(abi, "time_point", r#""2030-06-15T19:17:47.999""#, "188BB5FC1DC70600");
    check_round_trip2(abi, "time_point", r#""2000-12-31T23:59:59.999999""#, "FF1F23E5C3790300", r#""2000-12-31T23:59:59.999""#);

    check_round_trip(abi, "block_timestamp_type", r#""2000-01-01T00:00:00.000""#, "00000000");
    check_round_trip(abi, "block_timestamp_type", r#""2000-01-01T00:00:00.500""#, "01000000");
    check_round_trip(abi, "block_timestamp_type", r#""2000-01-01T00:00:01.000""#, "02000000");
    check_round_trip(abi, "block_timestamp_type", r#""2018-06-15T19:17:47.500""#, "B79A6D45");
    check_round_trip(abi, "block_timestamp_type", r#""2018-06-15T19:17:48.000""#, "B89A6D45");

    check_error(|| try_encode(abi, "time_point_sec", "true"), "cannot convert given variant");
    check_error(|| try_encode(abi, "time_point", "true"), "cannot convert given variant");
    check_error(|| try_encode(abi, "block_timestamp_type", "true"), "cannot convert given variant");

    Ok(())
}

#[test]
fn roundtrip_names() -> Result<()> {
    init();

    let transaction_abi_def = ABIDefinition::from_str(TRANSACTION_ABI)?;
    let transaction_abi = ABIEncoder::from_abi(&transaction_abi_def);
    let abi = &transaction_abi;

    check_round_trip(abi, "name", r#""""#, "0000000000000000");
    check_round_trip(abi, "name", r#""1""#, "0000000000000008");
    check_round_trip(abi, "name", r#""abcd""#, "000000000090D031");
    check_round_trip(abi, "name", r#""ab.cd.ef""#, "0000004B8184C031");
    check_round_trip(abi, "name", r#""ab.cd.ef.1234""#, "3444004B8184C031");
    // check_round_trip2(abi, "name", r#""..ab.cd.ef..""#, "00C0522021700C00", r#""..ab.cd.ef""#);
    check_round_trip(abi, "name", r#""zzzzzzzzzzzz""#, "F0FFFFFFFFFFFFFF");

    // check_error(|| try_encode(abi, "bytes", r#""0""#), "odd number of chars");

    Ok(())
}

#[test]
fn roundtrip_bytes() -> Result<()> {
    init();

    let transaction_abi_def = ABIDefinition::from_str(TRANSACTION_ABI)?;
    let transaction_abi = ABIEncoder::from_abi(&transaction_abi_def);
    let abi = &transaction_abi;

    check_round_trip(abi, "bytes", r#""""#, "00");
    check_round_trip(abi, "bytes", r#""00""#, "0100");
    check_round_trip(abi, "bytes", r#""AABBCCDDEEFF00010203040506070809""#, "10AABBCCDDEEFF00010203040506070809");

    check_error(|| try_encode(abi, "bytes", r#""0""#), "odd number of chars");
    check_error(|| try_encode(abi, "bytes", r#""yz""#), "invalid hex character");

    Ok(())
}

#[test]
fn roundtrip_symbol() -> Result<()> {
    init();

    let transaction_abi_def = ABIDefinition::from_str(TRANSACTION_ABI)?;
    let transaction_abi = ABIEncoder::from_abi(&transaction_abi_def);
    let abi = &transaction_abi;

    check_round_trip(abi, "symbol", r#""0,A""#, "0041000000000000");
    check_round_trip(abi, "symbol", r#""1,Z""#, "015A000000000000");
    check_round_trip(abi, "symbol", r#""4,SYS""#, "0453595300000000");

    Ok(())
}

#[test]
fn roundtrip_asset() -> Result<()> {
    init();

    let transaction_abi_def = ABIDefinition::from_str(TRANSACTION_ABI)?;
    let transaction_abi = ABIEncoder::from_abi(&transaction_abi_def);
    let abi = &transaction_abi;

    check_round_trip(abi, "asset", r#""0 FOO""#, "000000000000000000464F4F00000000");
    check_round_trip(abi, "asset", r#""0.0 FOO""#, "000000000000000001464F4F00000000");
    check_round_trip(abi, "asset", r#""0.00 FOO""#, "000000000000000002464F4F00000000");
    check_round_trip(abi, "asset", r#""0.000 FOO""#, "000000000000000003464F4F00000000");
    check_round_trip(abi, "asset", r#""1.2345 SYS""#, "39300000000000000453595300000000");
    check_round_trip(abi, "asset", r#""-1.2345 SYS""#, "C7CFFFFFFFFFFFFF0453595300000000");

    check_round_trip(abi, "asset[]", r#"[]"#, "00");
    check_round_trip(abi, "asset[]", r#"["0 FOO"]"#, "01000000000000000000464F4F00000000");
    check_round_trip(abi, "asset[]", r#"["0 FOO","0.000 FOO"]"#, "02000000000000000000464F4F00000000000000000000000003464F4F00000000");
    check_round_trip(abi, "asset?", "null", "00");
    check_round_trip(abi, "asset?", r#""0.123456 SIX""#, "0140E20100000000000653495800000000");

    Ok(())
}

#[test]
fn roundtrip_transaction() -> Result<()> {
    init();

    let trx_abi_def = ABIDefinition::from_str(TRANSACTION_ABI)?;
    let trx_abi = &ABIEncoder::from_abi(&trx_abi_def);
    let token_abi = &ABIEncoder::from_hex_abi(TOKEN_HEX_ABI)?;
    let packed_trx_abi_def = ABIDefinition::from_str(PACKED_TRANSACTION_ABI)?;
    let packed_trx_abi = &ABIEncoder::from_abi(&packed_trx_abi_def);

    check_round_trip(token_abi, "transfer",
                     r#"{"from":"useraaaaaaaa","to":"useraaaaaaab","quantity":"0.0001 SYS","memo":"test memo"}"#,
                     "608C31C6187315D6708C31C6187315D6010000000000000004535953000000000974657374206D656D6F");

    check_round_trip(trx_abi, "transaction",
                     r#"{"expiration":"2009-02-13T23:31:31.000","ref_block_num":1234,"ref_block_prefix":5678,"max_net_usage_words":0,"max_cpu_usage_ms":0,"delay_sec":0,"context_free_actions":[],"actions":[{"account":"eosio.token","name":"transfer","authorization":[{"actor":"useraaaaaaaa","permission":"active"}],"data":"608C31C6187315D6708C31C6187315D60100000000000000045359530000000000"}],"transaction_extensions":[]}"#,
                     "D3029649D2042E160000000000000100A6823403EA3055000000572D3CCDCD01608C31C6187315D600000000A8ED323221608C31C6187315D6708C31C6187315D6010000000000000004535953000000000000");

    check_round_trip2(
        token_abi, "transfer",
        r#"{"to":"useraaaaaaab","memo":"test memo","from":"useraaaaaaaa","quantity":"0.0001 SYS"}"#,
        "608C31C6187315D6708C31C6187315D6010000000000000004535953000000000974657374206D656D6F",
        r#"{"from":"useraaaaaaaa","to":"useraaaaaaab","quantity":"0.0001 SYS","memo":"test memo"}"#,
    );

    check_round_trip2(
        trx_abi, "transaction",
        r#"{"ref_block_num":1234,"ref_block_prefix":5678,"expiration":"2009-02-13T23:31:31.000","max_net_usage_words":0,"max_cpu_usage_ms":0,"delay_sec":0,"context_free_actions":[],"actions":[{"account":"eosio.token","name":"transfer","authorization":[{"actor":"useraaaaaaaa","permission":"active"}],"data":"608C31C6187315D6708C31C6187315D60100000000000000045359530000000000"}],"transaction_extensions":[]}"#,
        "D3029649D2042E160000000000000100A6823403EA3055000000572D3CCDCD01608C31C6187315D600000000A8ED323221608C31C6187315D6708C31C6187315D6010000000000000004535953000000000000",
        r#"{"expiration":"2009-02-13T23:31:31.000","ref_block_num":1234,"ref_block_prefix":5678,"max_net_usage_words":0,"max_cpu_usage_ms":0,"delay_sec":0,"context_free_actions":[],"actions":[{"account":"eosio.token","name":"transfer","authorization":[{"actor":"useraaaaaaaa","permission":"active"}],"data":"608C31C6187315D6708C31C6187315D60100000000000000045359530000000000"}],"transaction_extensions":[]}"#,
    );

    check_round_trip(
        packed_trx_abi, "packed_transaction_v0",
        r#"{"signatures":["SIG_K1_K5PGhrkUBkThs8zdTD9mGUJZvxL4eU46UjfYJSEdZ9PXS2Cgv5jAk57yTx4xnrdSocQm6DDvTaEJZi5WLBsoZC4XYNS8b3"],"compression":0,"packed_context_free_data":"","packed_trx":{"expiration":"2009-02-13T23:31:31.000","ref_block_num":1234,"ref_block_prefix":5678,"max_net_usage_words":0,"max_cpu_usage_ms":0,"delay_sec":0,"context_free_actions":[],"actions":[{"account":"eosio.token","name":"transfer","authorization":[{"actor":"useraaaaaaaa","permission":"active"}],"data":"608C31C6187315D6708C31C6187315D60100000000000000045359530000000000"}],"transaction_extensions":[]}}"#,
        "01001F4D6C791D32E38CA1A0A5F3139B8D1D521B641FE2EE675311FCA4C755ACDFCA2D13FE4DEE9953D2504FCB4382EEACBCEF90E3E8034BDD32EBA11F1904419DF6AF0000D3029649D2042E160000000000000100A6823403EA3055000000572D3CCDCD01608C31C6187315D600000000A8ED323221608C31C6187315D6708C31C6187315D6010000000000000004535953000000000000"
    );

    Ok(())
}
