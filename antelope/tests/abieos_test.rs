use std::sync::{Once, OnceLock};

use color_eyre::eyre::Result;
use tracing::{debug, info, instrument};
use tracing_subscriber::{
    EnvFilter,
    fmt::format::FmtSpan,
};

// use antelope_abi::abidefinition::{ABIDefinition, TypeNameRef};
use antelope::{
    ABI, ByteStream, ABIDefinition, TypeNameRef,
    JsonValue, InvalidValue,
    data::{
        TEST_ABI, TOKEN_HEX_ABI,
        TRANSACTION_ABI, PACKED_TRANSACTION_ABI,
    }
};


#[cfg(feature = "float128")]
use antelope_abi::data::STATE_HISTORY_PLUGIN_ABI;


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
//                                                                            //
////////////////////////////////////////////////////////////////////////////////

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

fn transaction_abi() -> &'static ABI {
    static TRX_ABI: OnceLock<ABI> = OnceLock::new();
    TRX_ABI.get_or_init(|| {
        let transaction_abi_def = ABIDefinition::from_str(TRANSACTION_ABI).unwrap();
        ABI::from_definition(&transaction_abi_def).unwrap()
    })
}

#[instrument(skip(ds, abi))]
fn try_encode_stream(ds: &mut ByteStream, abi: &ABI, typename: TypeNameRef, data: &str) -> Result<()> {
    let value: JsonValue = serde_json::from_str(data).map_err(InvalidValue::from)?;
    info!("{:?}", &value);
    abi.encode_variant(ds, typename, &value)?;
    Ok(())
}

fn try_encode(abi: &ABI, typename: &str, data: &str) -> Result<()> {
    let mut ds = ByteStream::new();
    try_encode_stream(&mut ds, abi, typename.into(), data)
}

fn try_decode_stream(ds: &mut ByteStream, abi: &ABI, typename: TypeNameRef) -> Result<JsonValue> {
    let decoded = abi.decode_variant(ds, typename)?;
    assert!(ds.leftover().is_empty());
    Ok(decoded)
}

fn try_decode<T: AsRef<[u8]>>(abi: &ABI, typename: &str, data: T) -> Result<JsonValue> {
    let mut ds = ByteStream::from(hex::decode(data).map_err(InvalidValue::from)?);
    try_decode_stream(&mut ds, abi, typename.into())
}

#[track_caller]
fn round_trip(abi: &ABI, typename: &str, data: &str, hex: &str, expected: &str) -> Result<()> {
    debug!(r#"==== round-tripping type "{typename}" with value {data}"#);
    let mut ds = ByteStream::new();

    try_encode_stream(&mut ds, abi, typename.into(), data)?;
    assert_eq!(ds.hex_data(), hex);

    let decoded = try_decode_stream(&mut ds, abi, typename.into())?;
    let repr = decoded.to_string();

    // if we have a number which representation would use scientific notation,
    // first convert it to an `f64` and then call `to_string()` in order to get
    // the representation with only digits
    if let Some(x) = decoded.as_f64() {
        if repr.contains('e') {
            assert_eq!(x.to_string(), expected);
            return Ok(());
        }
    }
    assert_eq!(repr, expected);

    Ok(())
}

fn check_error<F, T>(f: F, expected_error_msg: &str)
    where F: FnOnce() -> Result<T>
{
    match f() {
        Ok(_) => {
            panic!(r#"expected error with message "{}" but everything went fine..."#,
                   expected_error_msg);
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
#[track_caller]
fn check_round_trip(abi: &ABI, typename: &str, data: &str, hex: &str) {
    round_trip(abi, typename, data, hex, data).unwrap()
}

#[track_caller]
fn check_round_trip2(abi: &ABI, typename: &str, data: &str, hex: &str, expected: &str) {
    round_trip(abi, typename, data, hex, expected).unwrap()
}


///// FIXME FIXME: what about the expected hex?
fn _check_error_trip(abi: &ABI, typename: &str, data: &str, error_msg: &str) {
    check_error(|| round_trip(abi, typename, data, "", data), error_msg);
}

fn str_to_hex(s: &str) -> String {
    format!("{:02x}{}", s.len(), hex::encode(s.as_bytes()))
}


#[test]
fn integration_test() -> Result<()> {
    init();

    let _test_abi_def = ABIDefinition::from_str(TEST_ABI)?;
    let _test_abi = ABI::from_definition(&_test_abi_def)?;

    let _transaction_abi_def = ABIDefinition::from_str(TRANSACTION_ABI)?;
    let _transaction_abi = ABI::from_definition(&_transaction_abi_def);

    let _token_abi = ABI::from_hex_abi(TOKEN_HEX_ABI)?;

    let _abi = &_transaction_abi;

    check_error(|| Ok(ABIDefinition::from_str("")?), "cannot deserialize ABIDefinition");
    check_error(|| Ok(ABI::from_hex_abi("")?), "stream ended");
    check_error(|| Ok(ABI::from_hex_abi("00")?), "unsupported ABI version");
    check_error(|| Ok(ABI::from_hex_abi(&str_to_hex("eosio::abi/9.0"))?), "unsupported ABI version");
    check_error(|| Ok(ABI::from_hex_abi(&str_to_hex("eosio::abi/1.0"))?), "stream ended");
    check_error(|| Ok(ABI::from_hex_abi(&str_to_hex("eosio::abi/1.1"))?), "stream ended");

    Ok(())
}

#[test]
fn test_type_properties() -> Result<()> {
    init();

    let transaction_abi_def = ABIDefinition::from_str(TRANSACTION_ABI)?;
    let transaction_abi = ABI::from_definition(&transaction_abi_def)?;
    let _abi = &transaction_abi;

    // check_error(|| try_encode(abi, "int8?[]", "[]"), "failed test type properties");
    // FIXME: check more tests at: test.cpp:1007
    //        these seem to be only implemented in abieos, but we can find no
    //        trace of equivalent checks in Spring...
    //        Leaving empty for now

    Ok(())
}

#[test]
fn roundtrip_bool() -> Result<()> {
    init();

    let abi = transaction_abi();

    check_round_trip(abi, "bool", "true", "01");
    check_round_trip(abi, "bool", "false", "00");

    check_error(|| try_decode(abi, "bool", ""), "stream ended");
    check_error(|| try_encode(abi, "bool", ""), "cannot parse JSON string");
    check_error(|| try_encode(abi, "bool", "trues"), "cannot parse JSON string");
    check_error(|| try_encode(abi, "bool", "null"), "cannot convert given variant");
    check_error(|| try_encode(abi, "bool", r#""foo""#), "cannot convert given variant");

    Ok(())
}

#[test]
fn roundtrip_i8() -> Result<()> {
    init();

    let abi = transaction_abi();

    check_round_trip(abi, "int8", "0", "00");
    check_round_trip(abi, "int8", "127", "7f");
    check_round_trip(abi, "int8", "-128", "80");
    check_round_trip(abi, "uint8", "0", "00");
    check_round_trip(abi, "uint8", "1", "01");
    check_round_trip(abi, "uint8", "254", "fe");
    check_round_trip(abi, "uint8", "255", "ff");

    check_error(|| try_encode(abi, "int8", "128"), "integer out of range");
    check_error(|| try_encode(abi, "int8", "-129"), "integer out of range");
    check_error(|| try_encode(abi, "uint8", "-1"), "cannot convert given variant");
    check_error(|| try_encode(abi, "uint8", "256"), "integer out of range");

    check_round_trip(abi, "uint8[]", "[]", "00");
    check_round_trip(abi, "uint8[]", "[10]", "010a");
    check_round_trip(abi, "uint8[]", "[10,9]", "020a09");
    check_round_trip(abi, "uint8[]", "[10,9,8]", "030a0908");

    Ok(())
}

#[test]
fn roundtrip_i16() -> Result<()> {
    init();

    let abi = transaction_abi();

    check_round_trip(abi, "int16", "0", "0000");
    check_round_trip(abi, "int16", "32767", "ff7f");
    check_round_trip(abi, "int16", "-32768", "0080");
    check_round_trip(abi, "uint16", "0", "0000");
    check_round_trip(abi, "uint16", "65535", "ffff");

    check_error(|| try_decode(abi, "int16", "01"), "stream ended");
    check_error(|| try_encode(abi, "int16", "32768"), "integer out of range");
    check_error(|| try_encode(abi, "int16", "-32769"), "integer out of range");
    check_error(|| try_encode(abi, "uint16", "-1"), "cannot convert given variant");
    check_error(|| try_encode(abi, "uint16", "65536"), "integer out of range");

    Ok(())
}

#[test]
fn roundtrip_i32() -> Result<()> {
    init();

    let abi = transaction_abi();

    check_round_trip(abi, "int32", "0", "00000000");
    check_round_trip(abi, "int32", "2147483647", "ffffff7f");
    check_round_trip(abi, "int32", "-2147483648", "00000080");
    check_round_trip(abi, "uint32", "0", "00000000");
    check_round_trip(abi, "uint32", "4294967295", "ffffffff");

    check_error(|| try_encode(abi, "int32", "2147483648"), "integer out of range");
    check_error(|| try_encode(abi, "int32", "-2147483649"), "integer out of range");
    check_error(|| try_encode(abi, "uint32", "-1"), "cannot convert given variant");
    check_error(|| try_encode(abi, "uint32", "4294967296"), "integer out of range");

    Ok(())
}

#[test]
fn roundtrip_i64() -> Result<()> {
    init();

    let abi = transaction_abi();

    check_round_trip(abi, "int64", r#""0""#, "0000000000000000");
    check_round_trip(abi, "int64", r#""1""#, "0100000000000000");
    check_round_trip(abi, "int64", r#""-1""#, "ffffffffffffffff");
    check_round_trip(abi, "int64", r#""9223372036854775807""#, "ffffffffffffff7f");
    check_round_trip(abi, "int64", r#""-9223372036854775808""#, "0000000000000080");
    check_round_trip(abi, "uint64", r#""0""#, "0000000000000000");
    check_round_trip(abi, "uint64", r#""18446744073709551615""#, "ffffffffffffffff");

    check_error(|| try_encode(abi, "int64", r#""9223372036854775808""#), "number too large to fit in target type");
    check_error(|| try_encode(abi, "int64", r#""-9223372036854775809""#), "number too small to fit in target type");
    check_error(|| try_encode(abi, "uint64", r#""-1""#), "invalid digit");
    check_error(|| try_encode(abi, "uint64", r#""18446744073709551616""#), "number too large to fit in target type");

    Ok(())
}

#[test]
fn roundtrip_i128() -> Result<()> {
    init();

    let abi = transaction_abi();

    check_round_trip(abi, "int128", r#""0""#, "00000000000000000000000000000000");
    check_round_trip(abi, "int128", r#""1""#, "01000000000000000000000000000000");
    check_round_trip(abi, "int128", r#""-1""#, "ffffffffffffffffffffffffffffffff");
    check_round_trip(abi, "int128", r#""18446744073709551615""#, "ffffffffffffffff0000000000000000");
    check_round_trip(abi, "int128", r#""-18446744073709551615""#, "0100000000000000ffffffffffffffff");
    check_round_trip(abi, "int128", r#""170141183460469231731687303715884105727""#, "ffffffffffffffffffffffffffffff7f");
    check_round_trip(abi, "int128", r#""-170141183460469231731687303715884105727""#, "01000000000000000000000000000080");
    check_round_trip(abi, "int128", r#""-170141183460469231731687303715884105728""#, "00000000000000000000000000000080");
    check_round_trip(abi, "uint128", r#""0""#, "00000000000000000000000000000000");
    check_round_trip(abi, "uint128", r#""18446744073709551615""#, "ffffffffffffffff0000000000000000");
    check_round_trip(abi, "uint128", r#""340282366920938463463374607431768211454""#, "feffffffffffffffffffffffffffffff");
    check_round_trip(abi, "uint128", r#""340282366920938463463374607431768211455""#, "ffffffffffffffffffffffffffffffff");

    check_error(|| try_encode(abi, "int128", r#""170141183460469231731687303715884105728""#), "number too large");
    check_error(|| try_encode(abi, "int128", r#""-170141183460469231731687303715884105729""#), "number too small");
    check_error(|| try_encode(abi, "uint128", r#""-1""#), "invalid integer");
    check_error(|| try_encode(abi, "uint128", r#""340282366920938463463374607431768211456""#), "number too large");
    check_error(|| try_encode(abi, "uint128", r#""true""#), "invalid integer");

    Ok(())
}

#[test]
fn roundtrip_varints() -> Result<()> {
    init();

    let abi = transaction_abi();

    check_round_trip(abi, "varuint32", "0", "00");
    check_round_trip(abi, "varuint32", "127", "7f");
    check_round_trip(abi, "varuint32", "128", "8001");
    check_round_trip(abi, "varuint32", "129", "8101");
    check_round_trip(abi, "varuint32", "16383", "ff7f");
    check_round_trip(abi, "varuint32", "16384", "808001");
    check_round_trip(abi, "varuint32", "16385", "818001");
    check_round_trip(abi, "varuint32", "2097151", "ffff7f");
    check_round_trip(abi, "varuint32", "2097152", "80808001");
    check_round_trip(abi, "varuint32", "2097153", "81808001");
    check_round_trip(abi, "varuint32", "268435455", "ffffff7f");
    check_round_trip(abi, "varuint32", "268435456", "8080808001");
    check_round_trip(abi, "varuint32", "268435457", "8180808001");
    check_round_trip(abi, "varuint32", "4294967294", "feffffff0f");
    check_round_trip(abi, "varuint32", "4294967295", "ffffffff0f");

    check_round_trip(abi, "varint32", "0", "00");
    check_round_trip(abi, "varint32", "-1", "01");
    check_round_trip(abi, "varint32", "1", "02");
    check_round_trip(abi, "varint32", "-2", "03");
    check_round_trip(abi, "varint32", "2", "04");
    check_round_trip(abi, "varint32", "-2147483647", "fdffffff0f");
    check_round_trip(abi, "varint32", "2147483647", "feffffff0f");
    check_round_trip(abi, "varint32", "-2147483648", "ffffffff0f");

    check_error(|| try_encode(abi, "varuint32", "4294967296"), "integer out of range");
    check_error(|| try_encode(abi, "varuint32", "-1"), "cannot convert given variant");
    check_error(|| try_encode(abi, "varint32", "2147483648"), "integer out of range");
    check_error(|| try_encode(abi, "varint32", "-2147483649"), "integer out of range");

    Ok(())
}

#[test]
fn roundtrip_floats() -> Result<()> {
    init();

    let abi = transaction_abi();

    check_round_trip(abi, "float32", "0.0", "00000000");
    check_round_trip(abi, "float32", "0.125", "0000003e");
    check_round_trip(abi, "float32", "-0.125", "000000be");
    check_round_trip(abi, "float64", "0.0", "0000000000000000");
    check_round_trip(abi, "float64", "0.125", "000000000000c03f");
    check_round_trip(abi, "float64", "-0.125", "000000000000c0bf");
    check_round_trip2(abi, "float64", "151115727451828646838272.0", "000000000000c044", "151115727451828650000000");
    check_round_trip2(abi, "float64", "-151115727451828646838272.0", "000000000000c0c4", "-151115727451828650000000");

    Ok(())
}

#[test]
#[cfg(feature = "float128")]
fn roundtrip_float128() -> Result<()> {
    init();

    let abi = transaction_abi();

    check_round_trip(abi, "float128", r#""00000000000000000000000000000000""#, "00000000000000000000000000000000");
    check_round_trip(abi, "float128", r#""ffffffffffffffffffffffffffffffff""#, "ffffffffffffffffffffffffffffffff");
    check_round_trip(abi, "float128", r#""12345678abcdef12345678abcdef1234""#, "12345678abcdef12345678abcdef1234");

    Ok(())
}

#[test]
fn roundtrip_datetimes() -> Result<()> {
    init();

    let abi = transaction_abi();

    check_round_trip(abi, "time_point_sec", r#""1970-01-01T00:00:00.000""#, "00000000");
    check_round_trip(abi, "time_point_sec", r#""2018-06-15T19:17:47.000""#, "db10245b");
    check_round_trip(abi, "time_point_sec", r#""2030-06-15T19:17:47.000""#, "5b6fb671");

    check_round_trip(abi, "time_point", r#""1970-01-01T00:00:00.000""#, "0000000000000000");
    check_round_trip(abi, "time_point", r#""1970-01-01T00:00:00.001""#, "e803000000000000");
    check_round_trip(abi, "time_point", r#""1970-01-01T00:00:00.002""#, "d007000000000000");
    check_round_trip(abi, "time_point", r#""1970-01-01T00:00:00.010""#, "1027000000000000");
    check_round_trip(abi, "time_point", r#""1970-01-01T00:00:00.100""#, "a086010000000000");
    check_round_trip(abi, "time_point", r#""2018-06-15T19:17:47.000""#, "c0ac3112b36e0500");
    check_round_trip(abi, "time_point", r#""2018-06-15T19:17:47.999""#, "18eb4012b36e0500");
    check_round_trip(abi, "time_point", r#""2030-06-15T19:17:47.999""#, "188bb5fc1dc70600");
    check_round_trip2(abi, "time_point", r#""2000-12-31T23:59:59.999999""#, "ff1f23e5c3790300", r#""2000-12-31T23:59:59.999""#);

    check_round_trip(abi, "block_timestamp_type", r#""2000-01-01T00:00:00.000""#, "00000000");
    check_round_trip(abi, "block_timestamp_type", r#""2000-01-01T00:00:00.500""#, "01000000");
    check_round_trip(abi, "block_timestamp_type", r#""2000-01-01T00:00:01.000""#, "02000000");
    check_round_trip(abi, "block_timestamp_type", r#""2018-06-15T19:17:47.500""#, "b79a6d45");
    check_round_trip(abi, "block_timestamp_type", r#""2018-06-15T19:17:48.000""#, "b89a6d45");

    check_error(|| try_encode(abi, "time_point_sec", "true"), "cannot convert given variant");
    check_error(|| try_encode(abi, "time_point", "true"), "cannot convert given variant");
    check_error(|| try_encode(abi, "block_timestamp_type", "true"), "cannot convert given variant");

    Ok(())
}

#[test]
fn roundtrip_names() -> Result<()> {
    init();

    let abi = transaction_abi();

    check_round_trip(abi, "name", r#""""#, "0000000000000000");
    check_round_trip(abi, "name", r#""1""#, "0000000000000008");
    check_round_trip(abi, "name", r#""abcd""#, "000000000090d031");
    check_round_trip(abi, "name", r#""ab.cd.ef""#, "0000004b8184c031");
    check_round_trip(abi, "name", r#""ab.cd.ef.1234""#, "3444004b8184c031");
    check_round_trip(abi, "name", r#""zzzzzzzzzzzz""#, "f0ffffffffffffff");

    check_error(|| try_encode(abi, "name", "true"), "cannot convert given variant");
    check_error(|| try_encode(abi, "name", r#""..ab.cd.ef..""#), "Name not properly normalized");

    Ok(())
}

#[test]
fn roundtrip_bytes() -> Result<()> {
    init();

    let abi = transaction_abi();

    check_round_trip(abi, "bytes", r#""""#, "00");
    check_round_trip(abi, "bytes", r#""00""#, "0100");
    check_round_trip(abi, "bytes", r#""aabbccddeeff00010203040506070809""#, "10aabbccddeeff00010203040506070809");

    check_error(|| try_decode(abi, "bytes", "01"), "stream ended");
    check_error(|| try_encode(abi, "bytes", r#""0""#), "Odd number of digits");
    check_error(|| try_encode(abi, "bytes", r#""yz""#), "Invalid character");
    check_error(|| try_encode(abi, "bytes", "true"), "cannot convert given variant");

    Ok(())
}

#[test]
fn roundtrip_strings() -> Result<()> {
    init();

    let abi = transaction_abi();

    check_round_trip(abi, "string", r#""""#, "00");
    check_round_trip(abi, "string", r#""z""#, "017a");
    check_round_trip(abi, "string", r#""This is a string.""#, "1154686973206973206120737472696e672e");
    check_round_trip(abi, "string", r#""' + '*'.repeat(128) + '""#, "1727202b20272a272e7265706561742831323829202b2027");
    check_round_trip(abi, "string", r#""\u0000  这是一个测试  Это тест  هذا اختبار 👍""#, "40002020e8bf99e698afe4b880e4b8aae6b58be8af952020d0add182d0be20d182d0b5d181d1822020d987d8b0d8a720d8a7d8aed8aad8a8d8a7d8b120f09f918d");

    check_error(|| try_decode(abi, "string", "01"), "stream ended");
    check_error(|| try_decode(abi, "string", hex::encode(b"\x11invalid utf8: \xff\xfe\xfd")), "invalid utf-8 sequence");
    check_error(|| try_encode(abi, "time_point_sec", "true"), "cannot convert given variant");

    Ok(())
}

#[test]
fn roundtrip_crypto_types() -> Result<()> {
    init();

    let abi = transaction_abi();

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
    check_round_trip2(abi, "public_key", r#""EOS11111111111111111111111115qCHTcgbQwptSz99m""#, "0000000000000000000000000000000000000000000000000000ffffffffffffffff", r#""PUB_K1_11111111111111111111111115qCHTcgbQwpvP72Uq""#);
    check_round_trip2(abi, "public_key", r#""EOS111111111111111114ZrjxJnU1LA5xSyrWMNuXTrYSJ57""#, "000000000000000000000000000000000000ffffffffffffffffffffffffffffffff", r#""PUB_K1_111111111111111114ZrjxJnU1LA5xSyrWMNuXTrVub2r""#);
    check_round_trip2(abi, "public_key", r#""EOS1111111113diW7pnisfdBvHTXP7wvW5k5Ky1e5DVuF23dosU""#, "00000000000000000000ffffffffffffffffffffffffffffffffffffffffffffffff", r#""PUB_K1_1111111113diW7pnisfdBvHTXP7wvW5k5Ky1e5DVuF4PizpM""#);
    check_round_trip2(abi, "public_key", r#""EOS11DsZ6Lyr1aXpm9aBqqgV4iFJpNbSw5eE9LLTwNAxqjJgmjgbT""#, "00000080ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff", r#""PUB_K1_11DsZ6Lyr1aXpm9aBqqgV4iFJpNbSw5eE9LLTwNAxqjJgXSdB8""#);
    check_round_trip2(abi, "public_key", r#""EOS12wkBET2rRgE8pahuaczxKbmv7ciehqsne57F9gtzf1PVYNMRa2""#, "0000ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff", r#""PUB_K1_12wkBET2rRgE8pahuaczxKbmv7ciehqsne57F9gtzf1PVb7Rf7o""#);
    check_round_trip2(abi, "public_key", r#""EOS1yp8ebBuKZ13orqUrZsGsP49e6K3ThVK1nLutxSyU5j9SaXz9a""#, "000080ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff", r#""PUB_K1_1yp8ebBuKZ13orqUrZsGsP49e6K3ThVK1nLutxSyU5j9Tx1r96""#);
    check_round_trip2(abi, "public_key", r#""EOS9adaAMuB9v8yX1mZ5PtoB6VFSCeqRGjASd8ZTM6VUkiHL7mue4K""#, "00ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff", r#""PUB_K1_9adaAMuB9v8yX1mZ5PtoB6VFSCeqRGjASd8ZTM6VUkiHLB5XEdw""#);
    check_round_trip2(abi, "public_key", r#""EOS69X3383RzBZj41k73CSjUNXM5MYGpnDxyPnWUKPEtYQmTBWz4D""#, "0002a5d2400af24411f64c29da2fe893ff2b6681a3b6ffbe980b2ee42ad10cc2e994", r#""PUB_K1_69X3383RzBZj41k73CSjUNXM5MYGpnDxyPnWUKPEtYQmVzqTY7""#);
    check_round_trip2(abi, "public_key", r#""EOS7yBtksm8Kkg85r4in4uCbfN77uRwe82apM8jjbhFVDgEgz3w8S""#, "000395c2020968e922eb4319fb56eb4fb0e7543d4b84ad367d8dc1b922338eb7232b", r#""PUB_K1_7yBtksm8Kkg85r4in4uCbfN77uRwe82apM8jjbhFVDgEcarGb8""#);
    check_round_trip2(abi, "public_key", r#""EOS7WnhaKwHpbSidYuh2DF1qAExTRUtPEdZCaZqt75cKcixuQUtdA""#, "000359d04e6519311041b10fe9e828a226b48f3f27a52f071f8e364cd317785abebc", r#""PUB_K1_7WnhaKwHpbSidYuh2DF1qAExTRUtPEdZCaZqt75cKcixtU7gEn""#);
    check_round_trip2(abi, "public_key", r#""EOS7Bn1YDeZ18w2N9DU4KAJxZDt6hk3L7eUwFRAc1hb5bp6xJwxNV""#, "00032ea514c6b834dbdd6520d0ac420bcf2335fe138de3d2dc5b7b2f03f9f99e9fac", r#""PUB_K1_7Bn1YDeZ18w2N9DU4KAJxZDt6hk3L7eUwFRAc1hb5bp6uEBZA8""#);
    check_round_trip(abi, "public_key", r#""PUB_K1_11111111111111111111111111111111149Mr2R""#, "00000000000000000000000000000000000000000000000000000000000000000000");
    check_round_trip(abi, "public_key", r#""PUB_K1_11111111111111111111111115qCHTcgbQwpvP72Uq""#, "0000000000000000000000000000000000000000000000000000ffffffffffffffff");
    check_round_trip(abi, "public_key", r#""PUB_K1_111111111111111114ZrjxJnU1LA5xSyrWMNuXTrVub2r""#, "000000000000000000000000000000000000ffffffffffffffffffffffffffffffff");
    check_round_trip(abi, "public_key", r#""PUB_K1_1111111113diW7pnisfdBvHTXP7wvW5k5Ky1e5DVuF4PizpM""#, "00000000000000000000ffffffffffffffffffffffffffffffffffffffffffffffff");
    check_round_trip(abi, "public_key", r#""PUB_K1_11DsZ6Lyr1aXpm9aBqqgV4iFJpNbSw5eE9LLTwNAxqjJgXSdB8""#, "00000080ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
    check_round_trip(abi, "public_key", r#""PUB_K1_12wkBET2rRgE8pahuaczxKbmv7ciehqsne57F9gtzf1PVb7Rf7o""#, "0000ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
    check_round_trip(abi, "public_key", r#""PUB_K1_1yp8ebBuKZ13orqUrZsGsP49e6K3ThVK1nLutxSyU5j9Tx1r96""#, "000080ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
    check_round_trip(abi, "public_key", r#""PUB_K1_9adaAMuB9v8yX1mZ5PtoB6VFSCeqRGjASd8ZTM6VUkiHLB5XEdw""#, "00ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
    check_round_trip(abi, "public_key", r#""PUB_K1_69X3383RzBZj41k73CSjUNXM5MYGpnDxyPnWUKPEtYQmVzqTY7""#, "0002a5d2400af24411f64c29da2fe893ff2b6681a3b6ffbe980b2ee42ad10cc2e994");
    check_round_trip(abi, "public_key", r#""PUB_K1_7yBtksm8Kkg85r4in4uCbfN77uRwe82apM8jjbhFVDgEcarGb8""#, "000395c2020968e922eb4319fb56eb4fb0e7543d4b84ad367d8dc1b922338eb7232b");
    check_round_trip(abi, "public_key", r#""PUB_K1_7WnhaKwHpbSidYuh2DF1qAExTRUtPEdZCaZqt75cKcixtU7gEn""#, "000359d04e6519311041b10fe9e828a226b48f3f27a52f071f8e364cd317785abebc");
    check_round_trip(abi, "public_key", r#""PUB_K1_7Bn1YDeZ18w2N9DU4KAJxZDt6hk3L7eUwFRAc1hb5bp6uEBZA8""#, "00032ea514c6b834dbdd6520d0ac420bcf2335fe138de3d2dc5b7b2f03f9f99e9fac");

    check_round_trip(abi, "private_key", r#""PVT_R1_PtoxLPzJZURZmPS4e26pjBiAn41mkkLPrET5qHnwDvbvqFEL6""#, "0133fb621e78d5dc78f0029b6fd714bfe3b42fe4b72bc109051591e71f204d2813");
    check_round_trip(abi, "private_key", r#""PVT_R1_vbRKUuE34hjMVQiePj2FEjM8FvuG7yemzQsmzx89kPS9J8Coz""#, "0179b0c1811bf83356f3fa2dedb76494d8d2bba188fae9c286f118e5e9f0621760");
    check_round_trip2(abi, "private_key", r#""5KQwrPbwdL6PhXujxW37FSSQZ1JiwsST4cqQzDeyXtP79zkvFD3""#, "00d2653ff7cbb2d8ff129ac27ef5781ce68b2558c41a74af1f2ddca635cbeef07d", r#""PVT_K1_2bfGi9rYsXQSXXTvJbDAPhHLQUojjaNLomdm3cEJ1XTzMqUt3V""#);

    check_round_trip(abi, "signature", r#""SIG_K1_Kg2UKjXTX48gw2wWH4zmsZmWu3yarcfC21Bd9JPj7QoDURqiAacCHmtExPk3syPb2tFLsp1R4ttXLXgr7FYgDvKPC5RCkx""#, "002056355ed1079822d2728886b449f0f4a2bbf48bf38698c0ebe8c7079768882b1c64ac07d7a4bd85cf96b8a74fdcafef1a4805f946177c609fdf31abe2463038e5");
    check_round_trip(abi, "signature", r#""SIG_R1_Kfh19CfEcQ6pxkMBz6xe9mtqKuPooaoyatPYWtwXbtwHUHU8YLzxPGvZhkqgnp82J41e9R6r5mcpnxy1wAf1w9Vyo9wybZ""#, "012053a48d3bb9a321e4ae8f079eab72efa778c8c09bc4c2f734de6d19ad9bce6a137495d877d4e51a585376aa6c1a174295dabdb25286e803bf553735cd2d31b1fc");

    check_error(|| try_encode(abi, "checksum256", r#""xy""#), "Invalid string length");
    check_error(|| try_encode(abi, "checksum256", r#""xy00000000000000000000000000000000000000000000000000000000000000""#), "Invalid character");
    check_error(|| try_encode(abi, "checksum256", "true"), "cannot convert given variant");
    check_error(|| try_encode(abi, "checksum256", r#""a0""#), "Invalid string length");

    check_error(|| try_encode(abi, "public_key", r#""foo""#), "not crypto data");
    check_error(|| try_encode(abi, "public_key", "true"), "cannot convert given variant");
    check_error(|| try_encode(abi, "private_key", r#""foo""#), "not crypto data");
    check_error(|| try_encode(abi, "private_key", "true"), "cannot convert given variant");
    check_error(|| try_encode(abi, "signature", r#""foo""#), "not crypto data");
    check_error(|| try_encode(abi, "signature", "true"), "cannot convert given variant");

    Ok(())
}

#[test]
fn roundtrip_symbol() -> Result<()> {
    init();

    let abi = transaction_abi();

    check_round_trip(abi, "symbol_code", r#""A""#, "4100000000000000");
    check_round_trip(abi, "symbol_code", r#""B""#, "4200000000000000");
    check_round_trip(abi, "symbol_code", r#""SYS""#, "5359530000000000");
    check_round_trip(abi, "symbol", r#""0,A""#, "0041000000000000");
    check_round_trip(abi, "symbol", r#""1,Z""#, "015a000000000000");
    check_round_trip(abi, "symbol", r#""4,SYS""#, "0453595300000000");

    check_error(|| try_encode(abi, "symbol_code", r#""foo""#), "invalid symbol");
    check_error(|| try_encode(abi, "symbol_code", "true"), "cannot convert given variant");
    check_error(|| try_encode(abi, "symbol_code", "null"), "cannot convert given variant");
    check_error(|| try_encode(abi, "symbol", "null"), "cannot convert given variant");

    Ok(())
}

#[test]
fn roundtrip_asset() -> Result<()> {
    init();

    let abi = transaction_abi();

    check_round_trip(abi, "asset", r#""0 FOO""#, "000000000000000000464f4f00000000");
    check_round_trip(abi, "asset", r#""0.0 FOO""#, "000000000000000001464f4f00000000");
    check_round_trip(abi, "asset", r#""0.00 FOO""#, "000000000000000002464f4f00000000");
    check_round_trip(abi, "asset", r#""0.000 FOO""#, "000000000000000003464f4f00000000");
    check_round_trip(abi, "asset", r#""1.2345 SYS""#, "39300000000000000453595300000000");
    check_round_trip(abi, "asset", r#""-1.2345 SYS""#, "c7cfffffffffffff0453595300000000");

    check_round_trip(abi, "asset[]", r#"[]"#, "00");
    check_round_trip(abi, "asset[]", r#"["0 FOO"]"#, "01000000000000000000464f4f00000000");
    check_round_trip(abi, "asset[]", r#"["0 FOO","0.000 FOO"]"#, "02000000000000000000464f4f00000000000000000000000003464f4f00000000");
    check_round_trip(abi, "asset?", "null", "00");
    check_round_trip(abi, "asset?", r#""0.123456 SIX""#, "0140e20100000000000653495800000000");

    check_round_trip(abi, "extended_asset", r#"{"quantity":"0 FOO","contract":"bar"}"#, "000000000000000000464f4f00000000000000000000ae39");
    check_round_trip(abi, "extended_asset", r#"{"quantity":"0.123456 SIX","contract":"seven"}"#, "40e201000000000006534958000000000000000080a9b6c2");

    check_error(|| try_encode(abi, "symbol", "null"), "cannot convert given variant");

    Ok(())
}

#[test]
fn roundtrip_transaction() -> Result<()> {
    init();

    let trx_abi = transaction_abi();
    let token_abi = &ABI::from_hex_abi(TOKEN_HEX_ABI)?;
    let packed_trx_abi_def = ABIDefinition::from_str(PACKED_TRANSACTION_ABI)?;
    let packed_trx_abi = &ABI::from_definition(&packed_trx_abi_def)?;

    check_round_trip(token_abi, "transfer",
                     r#"{"from":"useraaaaaaaa","to":"useraaaaaaab","quantity":"0.0001 SYS","memo":"test memo"}"#,
                     "608c31c6187315d6708c31c6187315d6010000000000000004535953000000000974657374206d656d6f");

    check_round_trip(trx_abi, "transaction",
                     r#"{"expiration":"2009-02-13T23:31:31.000","ref_block_num":1234,"ref_block_prefix":5678,"max_net_usage_words":0,"max_cpu_usage_ms":0,"delay_sec":0,"context_free_actions":[],"actions":[{"account":"eosio.token","name":"transfer","authorization":[{"actor":"useraaaaaaaa","permission":"active"}],"data":"608c31c6187315d6708c31c6187315d60100000000000000045359530000000000"}],"transaction_extensions":[]}"#,
                     "d3029649d2042e160000000000000100a6823403ea3055000000572d3ccdcd01608c31c6187315d600000000a8ed323221608c31c6187315d6708c31c6187315d6010000000000000004535953000000000000");

    check_round_trip2(
        token_abi, "transfer",
        r#"{"to":"useraaaaaaab","memo":"test memo","from":"useraaaaaaaa","quantity":"0.0001 SYS"}"#,
        "608c31c6187315d6708c31c6187315d6010000000000000004535953000000000974657374206d656d6f",
        r#"{"from":"useraaaaaaaa","to":"useraaaaaaab","quantity":"0.0001 SYS","memo":"test memo"}"#,
    );

    check_round_trip2(
        trx_abi, "transaction",
        r#"{"ref_block_num":1234,"ref_block_prefix":5678,"expiration":"2009-02-13T23:31:31.000","max_net_usage_words":0,"max_cpu_usage_ms":0,"delay_sec":0,"context_free_actions":[],"actions":[{"account":"eosio.token","name":"transfer","authorization":[{"actor":"useraaaaaaaa","permission":"active"}],"data":"608c31c6187315d6708c31c6187315d60100000000000000045359530000000000"}],"transaction_extensions":[]}"#,
        "d3029649d2042e160000000000000100a6823403ea3055000000572d3ccdcd01608c31c6187315d600000000a8ed323221608c31c6187315d6708c31c6187315d6010000000000000004535953000000000000",
        r#"{"expiration":"2009-02-13T23:31:31.000","ref_block_num":1234,"ref_block_prefix":5678,"max_net_usage_words":0,"max_cpu_usage_ms":0,"delay_sec":0,"context_free_actions":[],"actions":[{"account":"eosio.token","name":"transfer","authorization":[{"actor":"useraaaaaaaa","permission":"active"}],"data":"608c31c6187315d6708c31c6187315d60100000000000000045359530000000000"}],"transaction_extensions":[]}"#,
    );

    check_round_trip(
        packed_trx_abi, "packed_transaction_v0",
        r#"{"signatures":["SIG_K1_K5PGhrkUBkThs8zdTD9mGUJZvxL4eU46UjfYJSEdZ9PXS2Cgv5jAk57yTx4xnrdSocQm6DDvTaEJZi5WLBsoZC4XYNS8b3"],"compression":0,"packed_context_free_data":"","packed_trx":{"expiration":"2009-02-13T23:31:31.000","ref_block_num":1234,"ref_block_prefix":5678,"max_net_usage_words":0,"max_cpu_usage_ms":0,"delay_sec":0,"context_free_actions":[],"actions":[{"account":"eosio.token","name":"transfer","authorization":[{"actor":"useraaaaaaaa","permission":"active"}],"data":"608c31c6187315d6708c31c6187315d60100000000000000045359530000000000"}],"transaction_extensions":[]}}"#,
        "01001f4d6c791d32e38ca1a0a5f3139b8d1d521b641fe2ee675311fca4c755acdfca2d13fe4dee9953d2504fcb4382eeacbcef90e3e8034bdd32eba11f1904419df6af0000d3029649d2042e160000000000000100a6823403ea3055000000572d3ccdcd01608c31c6187315d600000000a8ed323221608c31c6187315d6708c31c6187315d6010000000000000004535953000000000000"
    );

    Ok(())
}

#[test]
#[cfg(feature = "float128")]
fn roundtrip_transaction_traces() -> Result<()> {
    init();

    let ship_abi_def = ABIDefinition::from_str(STATE_HISTORY_PLUGIN_ABI)?;
    let ship_abi = &ABI::from_definition(&ship_abi_def)?;

    check_round_trip(ship_abi, "transaction_trace",
                     r#"["transaction_trace_v0",{"id":"3098EA9476266BFA957C13FA73C26806D78753099CE8DEF2A650971F07595A69","status":0,"cpu_usage_us":2000,"net_usage_words":25,"elapsed":"194","net_usage":"200","scheduled":false,"action_traces":[["action_trace_v1",{"action_ordinal":1,"creator_action_ordinal":0,"receipt":["action_receipt_v0",{"receiver":"eosio","act_digest":"F2FDEEFF77EFC899EED23EE05F9469357A096DC3083D493571CF68A422C69EFE","global_sequence":"11","recv_sequence":"11","auth_sequence":[{"account":"eosio","sequence":"11"}],"code_sequence":2,"abi_sequence":0}],"receiver":"eosio","act":{"account":"eosio","name":"newaccount","authorization":[{"actor":"eosio","permission":"active"}],"data":"0000000000EA305500409406A888CCA501000000010002C0DED2BC1F1305FB0FAAC5E6C03EE3A1924234985427B6167CA569D13DF435CF0100000001000000010002C0DED2BC1F1305FB0FAAC5E6C03EE3A1924234985427B6167CA569D13DF435CF01000000"},"context_free":false,"elapsed":"83","console":"","account_ram_deltas":[{"account":"oracle.aml","delta":"2724"}],"account_disk_deltas":[],"except":null,"error_code":null,"return_value":""}]],"account_ram_delta":null,"except":null,"error_code":null,"failed_dtrx_trace":null,"partial":null}]"#,
                     "003098EA9476266BFA957C13FA73C26806D78753099CE8DEF2A650971F07595A6900D007000019C200000000000000C800000000000000000101010001000000000000EA3055F2FDEEFF77EFC899EED23EE05F9469357A096DC3083D493571CF68A422C69EFE0B000000000000000B00000000000000010000000000EA30550B0000000000000002000000000000EA30550000000000EA305500409E9A2264B89A010000000000EA305500000000A8ED3232660000000000EA305500409406A888CCA501000000010002C0DED2BC1F1305FB0FAAC5E6C03EE3A1924234985427B6167CA569D13DF435CF0100000001000000010002C0DED2BC1F1305FB0FAAC5E6C03EE3A1924234985427B6167CA569D13DF435CF01000000005300000000000000000100409406A888CCA5A40A000000000000000000000000000000");

    check_round_trip(ship_abi, "transaction_trace_msg",
                     r#"["transaction_trace_exception",{"error_code":"3","error_message":"error happens"}]"#,
                     "0003000000000000000D6572726F722068617070656E73");

    check_round_trip(ship_abi, "transaction_trace_msg",
                     r#"["transaction_trace",["transaction_trace_v0",{"id":"B2C8D46F161E06740CFADABFC9D11F013A1C90E25337FF3E22840B195E1ADC4B","status":0,"cpu_usage_us":2000,"net_usage_words":12,"elapsed":"7670","net_usage":"96","scheduled":false,"action_traces":[["action_trace_v1",{"action_ordinal":1,"creator_action_ordinal":0,"receipt":["action_receipt_v0",{"receiver":"eosio","act_digest":"7670940C29EC0A4C573EF052C5A29236393F587F208222B3C1B6A9C8FEA2C66A","global_sequence":"27","recv_sequence":"1","auth_sequence":[{"account":"eosio","sequence":"2"}],"code_sequence":1,"abi_sequence":0}],"receiver":"eosio","act":{"account":"eosio","name":"doit","authorization":[{"actor":"eosio","permission":"active"}],"data":"00"},"context_free":false,"elapsed":"7589","console":"","account_ram_deltas":[],"account_disk_deltas":[],"except":null,"error_code":null,"return_value":"01FFFFFFFFFFFFFFFF00"}]],"account_ram_delta":null,"except":null,"error_code":null,"failed_dtrx_trace":null,"partial":null}]]"#,
                     "0100B2C8D46F161E06740CFADABFC9D11F013A1C90E25337FF3E22840B195E1ADC4B00D00700000CF61D0000000000006000000000000000000101010001000000000000EA30557670940C29EC0A4C573EF052C5A29236393F587F208222B3C1B6A9C8FEA2C66A1B000000000000000100000000000000010000000000EA3055020000000000000001000000000000EA30550000000000EA30550000000000901D4D010000000000EA305500000000A8ED3232010000A51D00000000000000000000000A01FFFFFFFFFFFFFFFF000000000000");


    Ok(())
}
