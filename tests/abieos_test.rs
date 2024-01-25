pub mod abi_files;

use serde_json::Value;
// use anyhow::Result;
use color_eyre::eyre::Result;
use log::debug;

use antelope::abi::*;
use antelope::{
    ABIEncoder, ByteStream, bin_to_hex,
    types::InvalidValue,
};

use abi_files::{
    test_abi::TEST_ABI,
    trx_abi::{TRANSACTION_ABI, PACKED_TRANSACTION_ABI},
    ship_abi::STATE_HISTORY_PLUGIN_ABI,
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
//  - extended_asset                                                          //
//  - transaction_trace                                                       //
//  - transaction_trace_msg                                                   //
//                                                                            //
////////////////////////////////////////////////////////////////////////////////


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
fn roundtrip_strings() -> Result<()> {
    init();

    let transaction_abi_def = ABIDefinition::from_str(TRANSACTION_ABI)?;
    let transaction_abi = ABIEncoder::from_abi(&transaction_abi_def);
    let abi = &transaction_abi;

    check_round_trip(abi, "string", r#""""#, "00");
    check_round_trip(abi, "string", r#""z""#, "017A");
    check_round_trip(abi, "string", r#""This is a string.""#, "1154686973206973206120737472696E672E");
    check_round_trip(abi, "string", r#""' + '*'.repeat(128) + '""#, "1727202B20272A272E7265706561742831323829202B2027");
    check_round_trip(abi, "string", r#""\u0000  这是一个测试  Это тест  هذا اختبار 👍""#, "40002020E8BF99E698AFE4B880E4B8AAE6B58BE8AF952020D0ADD182D0BE20D182D0B5D181D1822020D987D8B0D8A720D8A7D8AED8AAD8A8D8A7D8B120F09F918D");

    Ok(())
}

#[test]
fn roundtrip_crypto_types() -> Result<()> {
    init();

    let transaction_abi_def = ABIDefinition::from_str(TRANSACTION_ABI)?;
    let transaction_abi = ABIEncoder::from_abi(&transaction_abi_def);
    let abi = &transaction_abi;

    check_round_trip(abi, "checksum160",
                     r#""0000000000000000000000000000000000000000""#,
                     "0000000000000000000000000000000000000000");
    check_round_trip(abi, "checksum160",
                     r#""123456789abcdef01234567890abcdef70123456""#,
                     "123456789abcdef01234567890abcdef70123456");
    check_round_trip(abi, "checksum256",
                     r#""0000000000000000000000000000000000000000000000000000000000000000""#,
                     "0000000000000000000000000000000000000000000000000000000000000000");
    check_round_trip(abi, "checksum256",
                     r#""0987654321abcdef0987654321ffff1234567890abcdef001234567890abcdef""#,
                     "0987654321abcdef0987654321ffff1234567890abcdef001234567890abcdef");
    check_round_trip(abi, "checksum512",
                     r#""00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000""#,
                     "00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000");
    check_round_trip(abi, "checksum512",
                     r#""0987654321abcdef0987654321ffff1234567890abcdef001234567890abcdef0987654321abcdef0987654321ffff1234567890abcdef001234567890abcdef""#,
                     "0987654321abcdef0987654321ffff1234567890abcdef001234567890abcdef0987654321abcdef0987654321ffff1234567890abcdef001234567890abcdef");

    check_round_trip2(abi, "public_key", r#""EOS1111111111111111111111111111111114T1Anm""#, "00000000000000000000000000000000000000000000000000000000000000000000", r#""PUB_K1_11111111111111111111111111111111149Mr2R""#);
    check_round_trip2(abi, "public_key", r#""EOS11111111111111111111111115qCHTcgbQwptSz99m""#, "0000000000000000000000000000000000000000000000000000FFFFFFFFFFFFFFFF", r#""PUB_K1_11111111111111111111111115qCHTcgbQwpvP72Uq""#);
    check_round_trip2(abi, "public_key", r#""EOS111111111111111114ZrjxJnU1LA5xSyrWMNuXTrYSJ57""#, "000000000000000000000000000000000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF", r#""PUB_K1_111111111111111114ZrjxJnU1LA5xSyrWMNuXTrVub2r""#);
    check_round_trip2(abi, "public_key", r#""EOS1111111113diW7pnisfdBvHTXP7wvW5k5Ky1e5DVuF23dosU""#, "00000000000000000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF", r#""PUB_K1_1111111113diW7pnisfdBvHTXP7wvW5k5Ky1e5DVuF4PizpM""#);
    check_round_trip2(abi, "public_key", r#""EOS11DsZ6Lyr1aXpm9aBqqgV4iFJpNbSw5eE9LLTwNAxqjJgmjgbT""#, "00000080FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF", r#""PUB_K1_11DsZ6Lyr1aXpm9aBqqgV4iFJpNbSw5eE9LLTwNAxqjJgXSdB8""#);
    check_round_trip2(abi, "public_key", r#""EOS12wkBET2rRgE8pahuaczxKbmv7ciehqsne57F9gtzf1PVYNMRa2""#, "0000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF", r#""PUB_K1_12wkBET2rRgE8pahuaczxKbmv7ciehqsne57F9gtzf1PVb7Rf7o""#);
    check_round_trip2(abi, "public_key", r#""EOS1yp8ebBuKZ13orqUrZsGsP49e6K3ThVK1nLutxSyU5j9SaXz9a""#, "000080FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF", r#""PUB_K1_1yp8ebBuKZ13orqUrZsGsP49e6K3ThVK1nLutxSyU5j9Tx1r96""#);
    check_round_trip2(abi, "public_key", r#""EOS9adaAMuB9v8yX1mZ5PtoB6VFSCeqRGjASd8ZTM6VUkiHL7mue4K""#, "00FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF", r#""PUB_K1_9adaAMuB9v8yX1mZ5PtoB6VFSCeqRGjASd8ZTM6VUkiHLB5XEdw""#);
    check_round_trip2(abi, "public_key", r#""EOS69X3383RzBZj41k73CSjUNXM5MYGpnDxyPnWUKPEtYQmTBWz4D""#, "0002A5D2400AF24411F64C29DA2FE893FF2B6681A3B6FFBE980B2EE42AD10CC2E994", r#""PUB_K1_69X3383RzBZj41k73CSjUNXM5MYGpnDxyPnWUKPEtYQmVzqTY7""#);
    check_round_trip2(abi, "public_key", r#""EOS7yBtksm8Kkg85r4in4uCbfN77uRwe82apM8jjbhFVDgEgz3w8S""#, "000395C2020968E922EB4319FB56EB4FB0E7543D4B84AD367D8DC1B922338EB7232B", r#""PUB_K1_7yBtksm8Kkg85r4in4uCbfN77uRwe82apM8jjbhFVDgEcarGb8""#);
    check_round_trip2(abi, "public_key", r#""EOS7WnhaKwHpbSidYuh2DF1qAExTRUtPEdZCaZqt75cKcixuQUtdA""#, "000359D04E6519311041B10FE9E828A226B48F3F27A52F071F8E364CD317785ABEBC", r#""PUB_K1_7WnhaKwHpbSidYuh2DF1qAExTRUtPEdZCaZqt75cKcixtU7gEn""#);
    check_round_trip2(abi, "public_key", r#""EOS7Bn1YDeZ18w2N9DU4KAJxZDt6hk3L7eUwFRAc1hb5bp6xJwxNV""#, "00032EA514C6B834DBDD6520D0AC420BCF2335FE138DE3D2DC5B7B2F03F9F99E9FAC", r#""PUB_K1_7Bn1YDeZ18w2N9DU4KAJxZDt6hk3L7eUwFRAc1hb5bp6uEBZA8""#);
    check_round_trip(abi, "public_key", r#""PUB_K1_11111111111111111111111111111111149Mr2R""#, "00000000000000000000000000000000000000000000000000000000000000000000");
    check_round_trip(abi, "public_key", r#""PUB_K1_11111111111111111111111115qCHTcgbQwpvP72Uq""#, "0000000000000000000000000000000000000000000000000000FFFFFFFFFFFFFFFF");
    check_round_trip(abi, "public_key", r#""PUB_K1_111111111111111114ZrjxJnU1LA5xSyrWMNuXTrVub2r""#, "000000000000000000000000000000000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF");
    check_round_trip(abi, "public_key", r#""PUB_K1_1111111113diW7pnisfdBvHTXP7wvW5k5Ky1e5DVuF4PizpM""#, "00000000000000000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF");
    check_round_trip(abi, "public_key", r#""PUB_K1_11DsZ6Lyr1aXpm9aBqqgV4iFJpNbSw5eE9LLTwNAxqjJgXSdB8""#, "00000080FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF");
    check_round_trip(abi, "public_key", r#""PUB_K1_12wkBET2rRgE8pahuaczxKbmv7ciehqsne57F9gtzf1PVb7Rf7o""#, "0000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF");
    check_round_trip(abi, "public_key", r#""PUB_K1_1yp8ebBuKZ13orqUrZsGsP49e6K3ThVK1nLutxSyU5j9Tx1r96""#, "000080FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF");
    check_round_trip(abi, "public_key", r#""PUB_K1_9adaAMuB9v8yX1mZ5PtoB6VFSCeqRGjASd8ZTM6VUkiHLB5XEdw""#, "00FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF");
    check_round_trip(abi, "public_key", r#""PUB_K1_69X3383RzBZj41k73CSjUNXM5MYGpnDxyPnWUKPEtYQmVzqTY7""#, "0002A5D2400AF24411F64C29DA2FE893FF2B6681A3B6FFBE980B2EE42AD10CC2E994");
    check_round_trip(abi, "public_key", r#""PUB_K1_7yBtksm8Kkg85r4in4uCbfN77uRwe82apM8jjbhFVDgEcarGb8""#, "000395C2020968E922EB4319FB56EB4FB0E7543D4B84AD367D8DC1B922338EB7232B");
    check_round_trip(abi, "public_key", r#""PUB_K1_7WnhaKwHpbSidYuh2DF1qAExTRUtPEdZCaZqt75cKcixtU7gEn""#, "000359D04E6519311041B10FE9E828A226B48F3F27A52F071F8E364CD317785ABEBC");
    check_round_trip(abi, "public_key", r#""PUB_K1_7Bn1YDeZ18w2N9DU4KAJxZDt6hk3L7eUwFRAc1hb5bp6uEBZA8""#, "00032EA514C6B834DBDD6520D0AC420BCF2335FE138DE3D2DC5B7B2F03F9F99E9FAC");

    check_round_trip(abi, "private_key", r#""PVT_R1_PtoxLPzJZURZmPS4e26pjBiAn41mkkLPrET5qHnwDvbvqFEL6""#, "0133FB621E78D5DC78F0029B6FD714BFE3B42FE4B72BC109051591E71F204D2813");
    check_round_trip(abi, "private_key", r#""PVT_R1_vbRKUuE34hjMVQiePj2FEjM8FvuG7yemzQsmzx89kPS9J8Coz""#, "0179B0C1811BF83356F3FA2DEDB76494D8D2BBA188FAE9C286F118E5E9F0621760");
    check_round_trip2(abi, "private_key", r#""5KQwrPbwdL6PhXujxW37FSSQZ1JiwsST4cqQzDeyXtP79zkvFD3""#, "00D2653FF7CBB2D8FF129AC27EF5781CE68B2558C41A74AF1F2DDCA635CBEEF07D", r#""PVT_K1_2bfGi9rYsXQSXXTvJbDAPhHLQUojjaNLomdm3cEJ1XTzMqUt3V""#);

    check_round_trip(abi, "signature", r#""SIG_K1_Kg2UKjXTX48gw2wWH4zmsZmWu3yarcfC21Bd9JPj7QoDURqiAacCHmtExPk3syPb2tFLsp1R4ttXLXgr7FYgDvKPC5RCkx""#, "002056355ED1079822D2728886B449F0F4A2BBF48BF38698C0EBE8C7079768882B1C64AC07D7A4BD85CF96B8A74FDCAFEF1A4805F946177C609FDF31ABE2463038E5");
    check_round_trip(abi, "signature", r#""SIG_R1_Kfh19CfEcQ6pxkMBz6xe9mtqKuPooaoyatPYWtwXbtwHUHU8YLzxPGvZhkqgnp82J41e9R6r5mcpnxy1wAf1w9Vyo9wybZ""#, "012053A48D3BB9A321E4AE8F079EAB72EFA778C8C09BC4C2F734DE6D19AD9BCE6A137495D877D4E51A585376AA6C1A174295DABDB25286E803BF553735CD2D31B1FC");

    Ok(())
}

#[test]
fn roundtrip_symbol() -> Result<()> {
    init();

    let transaction_abi_def = ABIDefinition::from_str(TRANSACTION_ABI)?;
    let transaction_abi = ABIEncoder::from_abi(&transaction_abi_def);
    let abi = &transaction_abi;

    check_round_trip(abi, "symbol_code", r#""A""#, "4100000000000000");
    check_round_trip(abi, "symbol_code", r#""B""#, "4200000000000000");
    check_round_trip(abi, "symbol_code", r#""SYS""#, "5359530000000000");
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

    check_round_trip(abi, "extended_asset", r#"{"quantity":"0 FOO","contract":"bar"}"#, "000000000000000000464F4F00000000000000000000AE39");
    check_round_trip(abi, "extended_asset", r#"{"quantity":"0.123456 SIX","contract":"seven"}"#, "40E201000000000006534958000000000000000080A9B6C2");

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
