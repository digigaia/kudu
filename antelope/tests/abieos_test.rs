#![cfg_attr(feature = "float128", feature(f128))]

use std::any::type_name_of_val;
use std::fmt::Debug;
use std::str::FromStr;
use std::sync::{Once, OnceLock};

use color_eyre::eyre::Result;
use serde::{de::DeserializeOwned, Serialize};
use tracing::{trace, debug, info, instrument};
use tracing_subscriber::{
    EnvFilter,
    // fmt::format::FmtSpan,
};

use antelope::{
    binaryserializable::{to_bin, from_bin, BinarySerializable},
    abi::data::{
        PACKED_TRANSACTION_ABI, TEST_ABI, TOKEN_HEX_ABI, TRANSACTION_ABI
    },
    ABIDefinition, Asset, Bytes, ByteStream, ExtendedAsset, InvalidValue, JsonValue, Name,
    Symbol, SymbolCode, TimePoint, TimePointSec, TypeName, VarInt32, VarUint32, ABI,
    Checksum160, Checksum256, Checksum512, PublicKey, PrivateKey, Signature,
    Transaction, Action, AccountName, Transfer, BlockTimestamp, PackedTransactionV0
};

#[cfg(feature = "float128")]
use antelope::{
    data::STATE_HISTORY_PLUGIN_ABI,
    TransactionTraceV0, ActionTrace, ActionTraceV1, ActionReceipt, ActionReceiptV0,
    AccountDelta, TransactionTrace, AccountAuthSequence,
};

// =============================================================================
//
// The following tests are coming mainly from
// https://github.com/AntelopeIO/abieos/blob/main/src/test.cpp#L577
//
// To get the hex representation for the values of each test, you need to
// compile and run the `test_abieos` binary from the abieos repo.
//
// The tests have been augmented to include all possible conversions from the
// Antelope data model, namely:
//
//  1- JSON -> variant -> bin -> variant -> JSON
//  2- Rust -> bin -> Rust
//  3- Rust -> JSON -> Rust
//
// =============================================================================


////////////////////////////////////////////////////////////////////////////////
//                                                                            //
// TODO:                                                                      //
//  - check integration_test, test_type_properties                            //
//                                                                            //
//                                                                            //
////////////////////////////////////////////////////////////////////////////////

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

fn transaction_abi() -> &'static ABI {
    static TRX_ABI: OnceLock<ABI> = OnceLock::new();
    TRX_ABI.get_or_init(|| {
        let transaction_abi_def = ABIDefinition::from_str(TRANSACTION_ABI).unwrap();
        ABI::from_definition(&transaction_abi_def).unwrap()
    })
}


// =============================================================================
//
//     Helper functions
//
// =============================================================================

#[instrument(skip(ds, abi))]
fn try_encode_stream(ds: &mut ByteStream, abi: &ABI, typename: TypeName, data: &str) -> Result<()> {
    let value: JsonValue = antelope::json::from_str(data).map_err(InvalidValue::from)?;
    info!("{:?}", &value);
    abi.encode_variant(ds, typename, &value)?;
    Ok(())
}

fn try_encode(abi: &ABI, typename: &str, data: &str) -> Result<()> {
    let mut ds = ByteStream::new();
    try_encode_stream(&mut ds, abi, typename.into(), data)
}

fn try_decode_stream(ds: &mut ByteStream, abi: &ABI, typename: TypeName) -> Result<JsonValue> {
    let decoded = abi.decode_variant(ds, typename)?;
    assert!(ds.leftover().is_empty(), "leftover data in stream after decoding");
    Ok(decoded)
}

fn try_decode<T: AsRef<[u8]>>(abi: &ABI, typename: &str, data: T) -> Result<JsonValue> {
    let mut ds = ByteStream::from(hex::decode(data).map_err(InvalidValue::from)?);
    try_decode_stream(&mut ds, abi, typename.into())
}

/// check roundtrip JSON -> variant -> bin -> variant -> JSON
#[track_caller]
fn check_round_trip(abi: &ABI, typename: &str, data: &str, hex: &str, expected: &str) -> Result<()> {
    debug!(r#"==== round-tripping type "{typename}" with value {data}"#);
    let mut ds = ByteStream::new();

    try_encode_stream(&mut ds, abi, typename.into(), data)?;
    assert_eq!(ds.hex_data(), hex, "variant to binary");

    let decoded = try_decode_stream(&mut ds, abi, typename.into())?;
    let repr = antelope::json::to_string(&decoded)?;

    assert_eq!(repr, expected, "variant to JSON");

    Ok(())
}

#[track_caller]
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

macro_rules! check_cross_conversion {
    ($abi:ident, $value:expr, $typ:ty, $typename:expr, $data:expr, $hex:expr, $expected:expr) => {
        check_cross_conversion!($abi, $value, $typ, $typename, $data, $hex, $expected, DEFAULT);
    };

    ($abi:ident, $value:expr, $typ:ty, $typename:expr, $data:expr, $hex:expr, $expected:expr, NO_SERDE) => {
        // 1- JSON -> variant -> bin -> variant -> JSON
        trace!(r#"checking JSON: {} -> variant -> bin: "{}" -> variant -> JSON"#, $data, $hex);
        check_round_trip($abi, $typename, $data, $hex, $expected).unwrap();

        // 2- Rust -> bin -> Rust
        trace!("checking Rust native ({:?}: {}) -> bin: {}", &$value, type_name_of_val(&$value), $hex);
        let bin = to_bin(&$value);
        let hex_data = bin.to_hex();
        assert_eq!(hex_data, $hex, "rust native to binary");
        let value2: $typ = from_bin(&bin).unwrap();
        assert_eq!(value2, $value, "binary to rust native");
    };

    ($abi:ident, $value:expr, $typ:ty, $typename:expr, $data:expr, $hex:expr, $expected:expr, DEFAULT) => {
        check_cross_conversion!($abi, $value, $typ, $typename, $data, $hex, $expected, NO_SERDE);

        // 3- Rust -> JSON -> Rust
        trace!("checking Rust native -> JSON");
        let repr = antelope::json::to_string(&$value).unwrap();
        assert_eq!(repr, $expected, "rust native to JSON");
        let value3: $typ = antelope::json::from_str(&repr).unwrap();
        assert_eq!(value3, $value, "JSON to rust native");
    };

    ($abi:ident, $value:expr, $typ:ty, $typename:expr, $data:expr, $hex:expr, $expected:expr, NO_JSON_TO_NATIVE) => {
        check_cross_conversion!($abi, $value, $typ, $typename, $data, $hex, $expected, NO_SERDE);

        // 3- Rust -> JSON -> Rust
        trace!("checking Rust native -> JSON");
        let repr = antelope::json::to_string(&$value).unwrap();
        assert_eq!(repr, $expected, "rust native to JSON");
    };
}

#[track_caller]
fn check_cross_conversion2<T>(abi: &ABI, value: T, typename: &str, data: &str, hex: &str, expected: &str)
where
    T: Serialize + DeserializeOwned + BinarySerializable + PartialEq + Debug
{
    check_cross_conversion!(abi, value, T, typename, data, hex, expected);
}

#[track_caller]
fn check_cross_conversion<T>(abi: &ABI, value: T, typename: &str, data: &str, hex: &str)
where
    T: Serialize + DeserializeOwned + BinarySerializable + PartialEq + Debug
{
    check_cross_conversion2(abi, value, typename, data, hex, data)
}

// This is weird, we can't seem to get the owned type via <T as ToOwned>,
// so we're making a new trait just for these tests with the proper associated type
trait HasOwned {
    type Owned;
}

impl HasOwned for &str {
    type Owned = String;
}

impl<T: BinarySerializable> HasOwned for &[T] {
    type Owned = Vec<T>;
}

#[track_caller]
fn check_cross_conversion_borrowed<T>(abi: &ABI, value: T, typename: &str, data: &str, hex: &str)
where
    T: Serialize + BinarySerializable + PartialEq + Debug + HasOwned + ToOwned,
    <T as HasOwned>::Owned: BinarySerializable + Debug + DeserializeOwned + PartialEq<T>,
{
    check_cross_conversion!(abi, value, <T as HasOwned>::Owned, typename, data, hex, data);
}


// =============================================================================
//
//     Tests
//
// =============================================================================

#[test]
fn integration_test() -> Result<()> {
    init();

    let _test_abi_def = ABIDefinition::from_str(TEST_ABI)?;
    let _test_abi = ABI::from_definition(&_test_abi_def)?;

    let _transaction_abi_def = ABIDefinition::from_str(TRANSACTION_ABI)?;
    let _transaction_abi = ABI::from_definition(&_transaction_abi_def);

    let _token_abi = ABI::from_hex_abi(TOKEN_HEX_ABI)?;

    let _abi = &_transaction_abi;

    fn str_to_hex(s: &str) -> String {
        format!("{:02x}{}", s.len(), hex::encode(s.as_bytes()))
    }

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

    check_cross_conversion(abi, true,  "bool", "true",  "01");
    check_cross_conversion(abi, false, "bool", "false", "00");

    check_error(|| try_decode(abi, "bool",      ""), "stream ended");
    check_error(|| try_encode(abi, "bool",      ""), "cannot parse JSON string");
    check_error(|| try_encode(abi, "bool", "trues"), "cannot parse JSON string");
    check_error(|| try_encode(abi, "bool",  "null"), "cannot convert given variant");
    check_error(|| try_encode(abi, "bool", r#""foo""#), "cannot convert given variant");

    Ok(())
}

#[test]
fn roundtrip_i8() -> Result<()> {
    init();

    let abi = transaction_abi();

    check_cross_conversion(abi,    0i8, "int8",    "0", "00");
    check_cross_conversion(abi,  127i8, "int8",  "127", "7f");
    check_cross_conversion(abi, -128i8, "int8", "-128", "80");
    check_cross_conversion(abi,    0u8, "uint8",   "0", "00");
    check_cross_conversion(abi,    1u8, "uint8",   "1", "01");
    check_cross_conversion(abi,  254u8, "uint8", "254", "fe");
    check_cross_conversion(abi,  255u8, "uint8", "255", "ff");

    check_error(|| try_encode(abi, "int8",  "128"), "integer out of range");
    check_error(|| try_encode(abi, "int8", "-129"), "integer out of range");
    check_error(|| try_encode(abi, "uint8",  "-1"), "cannot convert given variant");
    check_error(|| try_encode(abi, "uint8", "256"), "integer out of range");

    // NOTE: we can use either a `Vec`, an array or an array slice
    let v = vec![10u8, 9, 8];
    let a =     [10u8, 9, 8];
    check_cross_conversion_borrowed(abi, &v[..0], "uint8[]", "[]",       "00");
    check_cross_conversion_borrowed(abi, &v[..1], "uint8[]", "[10]",     "010a");
    check_cross_conversion_borrowed(abi, &v[..2], "uint8[]", "[10,9]",   "020a09");
    check_cross_conversion_borrowed(abi, &a[..2], "uint8[]", "[10,9]",   "020a09");
    check_cross_conversion         (abi,  v,      "uint8[]", "[10,9,8]", "030a0908");
    check_cross_conversion         (abi,  a,      "uint8[]", "[10,9,8]", "030a0908");

    Ok(())
}

#[test]
fn roundtrip_i16() -> Result<()> {
    init();

    let abi = transaction_abi();

    check_cross_conversion(abi,      0i16, "int16",      "0", "0000");
    check_cross_conversion(abi,  32767i16, "int16",  "32767", "ff7f");
    check_cross_conversion(abi, -32768i16, "int16", "-32768", "0080");
    check_cross_conversion(abi,      0u16, "uint16",     "0", "0000");
    check_cross_conversion(abi,  65535u16, "uint16", "65535", "ffff");

    check_error(|| try_decode(abi, "int16",     "01"), "stream ended");
    check_error(|| try_encode(abi, "int16",  "32768"), "integer out of range");
    check_error(|| try_encode(abi, "int16", "-32769"), "integer out of range");
    check_error(|| try_encode(abi, "uint16",    "-1"), "cannot convert given variant");
    check_error(|| try_encode(abi, "uint16", "65536"), "integer out of range");

    Ok(())
}

#[test]
fn roundtrip_i32() -> Result<()> {
    init();

    let abi = transaction_abi();

    check_cross_conversion(abi,           0i32, "int32",           "0", "00000000");
    check_cross_conversion(abi,  2147483647i32, "int32",  "2147483647", "ffffff7f");
    check_cross_conversion(abi, -2147483648i32, "int32", "-2147483648", "00000080");
    check_cross_conversion(abi,           0u32, "uint32"         , "0", "00000000");
    check_cross_conversion(abi,  4294967295u32, "uint32", "4294967295", "ffffffff");

    check_error(|| try_encode(abi, "int32",  "2147483648"), "integer out of range");
    check_error(|| try_encode(abi, "int32", "-2147483649"), "integer out of range");
    check_error(|| try_encode(abi, "uint32",         "-1"), "cannot convert given variant");
    check_error(|| try_encode(abi, "uint32", "4294967296"), "integer out of range");

    Ok(())
}

#[test]
fn roundtrip_i64() -> Result<()> {
    init();

    let abi = transaction_abi();

    check_cross_conversion(abi,                    0i64, "int64",  "0",                    "0000000000000000");
    check_cross_conversion(abi,                    1i64, "int64",  "1",                    "0100000000000000");
    check_cross_conversion(abi,                   -1i64, "int64",  "-1",                   "ffffffffffffffff");
    check_cross_conversion(abi,  9223372036854775807i64, "int64",  "9223372036854775807",  "ffffffffffffff7f");
    check_cross_conversion(abi, -9223372036854775808i64, "int64",  "-9223372036854775808", "0000000000000080");
    check_cross_conversion(abi,                    0u64, "uint64", "0",                    "0000000000000000");
    check_cross_conversion(abi, 18446744073709551615u64, "uint64", "18446744073709551615", "ffffffffffffffff");

    check_error(|| try_encode(abi, "int64",  r#""9223372036854775808""#),  "number too large to fit in target type");
    check_error(|| try_encode(abi, "int64",  r#""-9223372036854775809""#), "number too small to fit in target type");
    check_error(|| try_encode(abi, "uint64", r#""-1""#),                   "invalid digit");
    check_error(|| try_encode(abi, "uint64", r#""18446744073709551616""#), "number too large to fit in target type");

    Ok(())
}

#[test]
fn roundtrip_i128() -> Result<()> {
    init();

    let abi = transaction_abi();

    let check_i128 = |value, repr, hex| {
        check_cross_conversion!(abi, value, i128, "int128", repr, hex, repr, NO_JSON_TO_NATIVE);
    };

    let check_u128 = |value, repr, hex| {
        check_cross_conversion!(abi, value, u128, "uint128", repr, hex, repr, NO_JSON_TO_NATIVE);
    };

    check_i128(0i128, r#""0""#, "00000000000000000000000000000000");
    check_i128(1i128, r#""1""#, "01000000000000000000000000000000");
    check_i128(-1i128, r#""-1""#, "ffffffffffffffffffffffffffffffff");
    check_i128( 18446744073709551615i128,  r#""18446744073709551615""#, "ffffffffffffffff0000000000000000");
    check_i128(-18446744073709551615i128, r#""-18446744073709551615""#, "0100000000000000ffffffffffffffff");
    check_i128(170141183460469231731687303715884105727i128,
               r#""170141183460469231731687303715884105727""#, "ffffffffffffffffffffffffffffff7f");
    check_i128(-170141183460469231731687303715884105727i128,
               r#""-170141183460469231731687303715884105727""#, "01000000000000000000000000000080");
    check_i128(-170141183460469231731687303715884105728i128,
               r#""-170141183460469231731687303715884105728""#, "00000000000000000000000000000080");

    check_u128(0u128, r#""0""#, "00000000000000000000000000000000");
    check_u128(18446744073709551615u128,
               r#""18446744073709551615""#, "ffffffffffffffff0000000000000000");
    check_u128(340282366920938463463374607431768211454u128,
               r#""340282366920938463463374607431768211454""#, "feffffffffffffffffffffffffffffff");
    check_u128(340282366920938463463374607431768211455u128,
               r#""340282366920938463463374607431768211455""#, "ffffffffffffffffffffffffffffffff");

    check_error(|| try_encode(abi, "int128",  r#""170141183460469231731687303715884105728""#),  "number too large");
    check_error(|| try_encode(abi, "int128",  r#""-170141183460469231731687303715884105729""#), "number too small");
    check_error(|| try_encode(abi, "uint128", r#""-1""#),                                       "invalid integer");
    check_error(|| try_encode(abi, "uint128", r#""340282366920938463463374607431768211456""#),  "number too large");
    check_error(|| try_encode(abi, "uint128", r#""true""#),                                     "invalid integer");

    Ok(())
}

#[test]
fn roundtrip_varints() -> Result<()> {
    init();

    let abi = transaction_abi();
    let vu = VarUint32;
    let vi = VarInt32;

    check_cross_conversion(abi,           vu(0), "varuint32",          "0", "00");
    check_cross_conversion(abi,         vu(127), "varuint32",        "127", "7f");
    check_cross_conversion(abi,         vu(128), "varuint32",        "128", "8001");
    check_cross_conversion(abi,         vu(129), "varuint32",        "129", "8101");
    check_cross_conversion(abi,       vu(16383), "varuint32",      "16383", "ff7f");
    check_cross_conversion(abi,       vu(16384), "varuint32",      "16384", "808001");
    check_cross_conversion(abi,       vu(16385), "varuint32",      "16385", "818001");
    check_cross_conversion(abi,     vu(2097151), "varuint32",    "2097151", "ffff7f");
    check_cross_conversion(abi,     vu(2097152), "varuint32",    "2097152", "80808001");
    check_cross_conversion(abi,     vu(2097153), "varuint32",    "2097153", "81808001");
    check_cross_conversion(abi,   vu(268435455), "varuint32",  "268435455", "ffffff7f");
    check_cross_conversion(abi,   vu(268435456), "varuint32",  "268435456", "8080808001");
    check_cross_conversion(abi,   vu(268435457), "varuint32",  "268435457", "8180808001");
    check_cross_conversion(abi,  vu(4294967294), "varuint32", "4294967294", "feffffff0f");
    check_cross_conversion(abi,  vu(4294967295), "varuint32", "4294967295", "ffffffff0f");

    check_cross_conversion(abi,           vi(0), "varint32",           "0", "00");
    check_cross_conversion(abi,          vi(-1), "varint32",          "-1", "01");
    check_cross_conversion(abi,           vi(1), "varint32",           "1", "02");
    check_cross_conversion(abi,          vi(-2), "varint32",          "-2", "03");
    check_cross_conversion(abi,           vi(2), "varint32",           "2", "04");
    check_cross_conversion(abi, vi(-2147483647), "varint32", "-2147483647", "fdffffff0f");
    check_cross_conversion(abi,  vi(2147483647), "varint32",  "2147483647", "feffffff0f");
    check_cross_conversion(abi, vi(-2147483648), "varint32", "-2147483648", "ffffffff0f");

    check_error(|| try_encode(abi, "varuint32", "4294967296"), "integer out of range");
    check_error(|| try_encode(abi, "varuint32",         "-1"), "cannot convert given variant");
    check_error(|| try_encode(abi, "varint32",  "2147483648"), "integer out of range");
    check_error(|| try_encode(abi, "varint32", "-2147483649"), "integer out of range");

    Ok(())
}

#[test]
fn roundtrip_floats() -> Result<()> {
    init();

    let abi = transaction_abi();

    check_cross_conversion(abi,  0.0f32,   "float32",  "0",     "00000000");
    check_cross_conversion(abi,  0.125f32, "float32",  "0.125", "0000003e");
    check_cross_conversion(abi, -0.125f32, "float32", "-0.125", "000000be");
    check_cross_conversion(abi,  0.0,   "float64" , "0",     "0000000000000000");
    check_cross_conversion(abi,  0.125, "float64",  "0.125", "000000000000c03f");
    check_cross_conversion(abi, -0.125, "float64", "-0.125", "000000000000c0bf");
    check_cross_conversion2(abi, 151115727451828646838272.0, "float64",
                                "151115727451828646838272.0", "000000000000c044", "151115727451828650000000");
    check_cross_conversion2(abi, -151115727451828646838272.0, "float64",
                                "-151115727451828646838272.0", "000000000000c0c4", "-151115727451828650000000");

    Ok(())
}

#[test]
#[cfg(feature = "float128")]
fn roundtrip_float128() -> Result<()> {
    init();

    let abi = transaction_abi();

    let check_f128 = |hex: &str| {
        let repr = format!(r#""{hex}""#);
        let value = f128::from_le_bytes(Bytes::from_hex(hex).unwrap().as_ref().try_into().unwrap());
        check_cross_conversion!(abi, value, f128, "float128", &repr, hex, &repr, NO_SERDE);
    };

    check_f128("00000000000000000000000000000000");
    // NOTE: the following test is commented as it doesn't pass the Rust native test:
    //       the value it represents is NaN, and NaN != Nan (float types don't implement `Eq`)
    // check_f128("ffffffffffffffffffffffffffffffff");
    check_f128("12345678abcdef12345678abcdef1234");

    Ok(())
}

#[test]
fn roundtrip_datetimes() -> Result<()> {
    init();

    let abi = transaction_abi();

    let tps = |y, m, d, h, mm, s| { TimePointSec::new(y, m, d, h, mm, s).unwrap() };
    let check_tps = |value: TimePointSec, repr, hex| { check_cross_conversion(abi, value, "time_point_sec", repr, hex) };
    check_tps(tps(1970, 1,  1,  0,  0,  0), r#""1970-01-01T00:00:00.000""#, "00000000");
    check_tps(tps(2018, 6, 15, 19, 17, 47), r#""2018-06-15T19:17:47.000""#, "db10245b");
    check_tps(tps(2030, 6, 15, 19, 17, 47), r#""2030-06-15T19:17:47.000""#, "5b6fb671");

    let tp = |y, m, d, h, mm, s, milli| { TimePoint::new(y, m, d, h, mm, s, milli).unwrap() };
    let check_tp = |value: TimePoint, repr, hex| { check_cross_conversion(abi, value, "time_point", repr, hex) };
    check_tp(tp(1970, 1,  1,  0,  0,  0,   0), r#""1970-01-01T00:00:00.000""#, "0000000000000000");
    check_tp(tp(1970, 1,  1,  0,  0,  0,   1), r#""1970-01-01T00:00:00.001""#, "e803000000000000");
    check_tp(tp(1970, 1,  1,  0,  0,  0,   2), r#""1970-01-01T00:00:00.002""#, "d007000000000000");
    check_tp(tp(1970, 1,  1,  0,  0,  0,  10), r#""1970-01-01T00:00:00.010""#, "1027000000000000");
    check_tp(tp(1970, 1,  1,  0,  0,  0, 100), r#""1970-01-01T00:00:00.100""#, "a086010000000000");
    check_tp(tp(2018, 6, 15, 19, 17, 47,   0), r#""2018-06-15T19:17:47.000""#, "c0ac3112b36e0500");
    check_tp(tp(2018, 6, 15, 19, 17, 47, 999), r#""2018-06-15T19:17:47.999""#, "18eb4012b36e0500");
    check_tp(tp(2030, 6, 15, 19, 17, 47, 999), r#""2030-06-15T19:17:47.999""#, "188bb5fc1dc70600");
    check_cross_conversion!(abi, TimePoint::from_ymd_hms_micro(2000, 12, 31, 23, 59, 59, 999999).unwrap(),
                            TimePoint, "time_point", r#""2000-12-31T23:59:59.999999""#,
                            "ff1f23e5c3790300", r#""2000-12-31T23:59:59.999""#,
                            NO_JSON_TO_NATIVE);

    let bt = |y, m, d, h, mm, s, milli| { BlockTimestamp::new(y, m, d, h, mm, s, milli).unwrap() };
    let check_bt = |value: BlockTimestamp, repr, hex| {
        check_cross_conversion(abi, value, "block_timestamp_type", repr, hex)
    };
    check_bt(bt(2000, 1,  1,  0,  0,  0,   0), r#""2000-01-01T00:00:00.000""#, "00000000");
    check_bt(bt(2000, 1,  1,  0,  0,  0, 500), r#""2000-01-01T00:00:00.500""#, "01000000");
    check_bt(bt(2000, 1,  1,  0,  0,  1,   0), r#""2000-01-01T00:00:01.000""#, "02000000");
    check_bt(bt(2018, 6, 15, 19, 17, 47, 500), r#""2018-06-15T19:17:47.500""#, "b79a6d45");
    check_bt(bt(2018, 6, 15, 19, 17, 48,   0), r#""2018-06-15T19:17:48.000""#, "b89a6d45");

    check_error(|| try_encode(abi, "time_point_sec", "true"), "cannot convert given variant");
    check_error(|| try_encode(abi, "time_point", "true"), "cannot convert given variant");
    check_error(|| try_encode(abi, "block_timestamp_type", "true"), "cannot convert given variant");

    Ok(())
}

#[test]
fn roundtrip_names() -> Result<()> {
    init();

    let abi = transaction_abi();
    let n = |s| Name::new(s).unwrap();

    check_cross_conversion(abi, n(""),              "name", r#""""#,              "0000000000000000");
    check_cross_conversion(abi, n("1"),             "name", r#""1""#,             "0000000000000008");
    check_cross_conversion(abi, n("abcd"),          "name", r#""abcd""#,          "000000000090d031");
    check_cross_conversion(abi, n("ab.cd.ef"),      "name", r#""ab.cd.ef""#,      "0000004b8184c031");
    check_cross_conversion(abi, n("ab.cd.ef.1234"), "name", r#""ab.cd.ef.1234""#, "3444004b8184c031");
    check_cross_conversion(abi, n("zzzzzzzzzzzz"),  "name", r#""zzzzzzzzzzzz""#,  "f0ffffffffffffff");

    check_error(|| try_encode(abi, "name", "true"), "cannot convert given variant");
    check_error(|| try_encode(abi, "name", r#""..ab.cd.ef..""#), "Name not properly normalized");

    Ok(())
}

#[test]
fn roundtrip_bytes() -> Result<()> {
    init();

    let abi = transaction_abi();

    check_cross_conversion(abi, Bytes::from_hex("")?, "bytes", r#""""#, "00");
    check_cross_conversion(abi, Bytes::from_hex("00")?, "bytes", r#""00""#, "0100");
    check_cross_conversion(abi, Bytes::from_hex("aabbccddeeff00010203040506070809")?, "bytes",
                           r#""aabbccddeeff00010203040506070809""#, "10aabbccddeeff00010203040506070809");

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
    let check_string = |repr: &str, hex| {
        check_cross_conversion(abi, repr.to_owned(), "string", &format!(r#""{}""#, repr), hex);
    };

    check_string("", "00");
    check_string("z", "017a");
    check_string("This is a string.", "1154686973206973206120737472696e672e");
    check_string("' + '*'.repeat(128) + '", "1727202b20272a272e7265706561742831323829202b2027");
    check_cross_conversion_borrowed(
        abi, "\u{0000}  这是一个测试  Это тест  هذا اختبار 👍", "string",
        r#""\u0000  这是一个测试  Это тест  هذا اختبار 👍""#,
	"40002020e8bf99e698afe4b880e4b8aae6b58be8af952020d0add182d0be20d182d0b5d181d1822020d987d8b0d8a720d8a7d8aed8aad8a8d8a7d8b120f09f918d");

    check_error(|| try_decode(abi, "string", "01"), "stream ended");
    check_error(|| try_decode(abi, "string", hex::encode(b"\x11invalid utf8: \xff\xfe\xfd")), "invalid utf-8 sequence");
    check_error(|| try_encode(abi, "time_point_sec", "true"), "cannot convert given variant");

    Ok(())
}

#[test]
fn roundtrip_crypto_types() -> Result<()> {
    init();

    let abi = transaction_abi();

    let check_checksum160 = |c: &str| {
        check_cross_conversion(abi, Checksum160::from_hex(c).unwrap(), "checksum160", &format!(r#""{c}""#), c)
    };
    let check_checksum256 = |c: &str| {
        check_cross_conversion(abi, Checksum256::from_hex(c).unwrap(), "checksum256", &format!(r#""{c}""#), c)
    };
    let check_checksum512 = |c: &str| {
        check_cross_conversion(abi, Checksum512::from_hex(c).unwrap(), "checksum512", &format!(r#""{c}""#), c)
    };

    check_checksum160("0000000000000000000000000000000000000000");
    check_checksum160("123456789abcdef01234567890abcdef70123456");
    check_checksum256("0000000000000000000000000000000000000000000000000000000000000000");
    check_checksum256("0987654321abcdef0987654321ffff1234567890abcdef001234567890abcdef");
    check_checksum512("00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000");
    check_checksum512("0987654321abcdef0987654321ffff1234567890abcdef001234567890abcdef0987654321abcdef0987654321ffff1234567890abcdef001234567890abcdef");

    let check_publickey2 = |old: &str, new, hex| {
        check_cross_conversion2(abi, PublicKey::new(old).unwrap(), "public_key", &format!(r#""{old}""#), hex, &format!(r#""{new}""#));
    };

    let check_publickey = |key, hex| { check_publickey2(key, key, hex); };

    check_publickey2("EOS1111111111111111111111111111111114T1Anm",
                     "PUB_K1_11111111111111111111111111111111149Mr2R",
                     "00000000000000000000000000000000000000000000000000000000000000000000");
    check_publickey2("EOS11111111111111111111111115qCHTcgbQwptSz99m",
                     "PUB_K1_11111111111111111111111115qCHTcgbQwpvP72Uq",
                     "0000000000000000000000000000000000000000000000000000ffffffffffffffff");
    check_publickey2("EOS111111111111111114ZrjxJnU1LA5xSyrWMNuXTrYSJ57",
                     "PUB_K1_111111111111111114ZrjxJnU1LA5xSyrWMNuXTrVub2r",
                     "000000000000000000000000000000000000ffffffffffffffffffffffffffffffff");
    check_publickey2("EOS1111111113diW7pnisfdBvHTXP7wvW5k5Ky1e5DVuF23dosU",
                     "PUB_K1_1111111113diW7pnisfdBvHTXP7wvW5k5Ky1e5DVuF4PizpM",
                     "00000000000000000000ffffffffffffffffffffffffffffffffffffffffffffffff");
    check_publickey2("EOS11DsZ6Lyr1aXpm9aBqqgV4iFJpNbSw5eE9LLTwNAxqjJgmjgbT",
                     "PUB_K1_11DsZ6Lyr1aXpm9aBqqgV4iFJpNbSw5eE9LLTwNAxqjJgXSdB8",
                     "00000080ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
    check_publickey2("EOS12wkBET2rRgE8pahuaczxKbmv7ciehqsne57F9gtzf1PVYNMRa2",
                     "PUB_K1_12wkBET2rRgE8pahuaczxKbmv7ciehqsne57F9gtzf1PVb7Rf7o",
                     "0000ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
    check_publickey2("EOS1yp8ebBuKZ13orqUrZsGsP49e6K3ThVK1nLutxSyU5j9SaXz9a",
                     "PUB_K1_1yp8ebBuKZ13orqUrZsGsP49e6K3ThVK1nLutxSyU5j9Tx1r96",
                     "000080ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
    check_publickey2("EOS9adaAMuB9v8yX1mZ5PtoB6VFSCeqRGjASd8ZTM6VUkiHL7mue4K",
                     "PUB_K1_9adaAMuB9v8yX1mZ5PtoB6VFSCeqRGjASd8ZTM6VUkiHLB5XEdw",
                     "00ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
    check_publickey2("EOS69X3383RzBZj41k73CSjUNXM5MYGpnDxyPnWUKPEtYQmTBWz4D",
                     "PUB_K1_69X3383RzBZj41k73CSjUNXM5MYGpnDxyPnWUKPEtYQmVzqTY7",
                     "0002a5d2400af24411f64c29da2fe893ff2b6681a3b6ffbe980b2ee42ad10cc2e994");
    check_publickey2("EOS7yBtksm8Kkg85r4in4uCbfN77uRwe82apM8jjbhFVDgEgz3w8S",
                     "PUB_K1_7yBtksm8Kkg85r4in4uCbfN77uRwe82apM8jjbhFVDgEcarGb8",
                     "000395c2020968e922eb4319fb56eb4fb0e7543d4b84ad367d8dc1b922338eb7232b");
    check_publickey2("EOS7WnhaKwHpbSidYuh2DF1qAExTRUtPEdZCaZqt75cKcixuQUtdA",
                     "PUB_K1_7WnhaKwHpbSidYuh2DF1qAExTRUtPEdZCaZqt75cKcixtU7gEn",
                     "000359d04e6519311041b10fe9e828a226b48f3f27a52f071f8e364cd317785abebc");
    check_publickey2("EOS7Bn1YDeZ18w2N9DU4KAJxZDt6hk3L7eUwFRAc1hb5bp6xJwxNV",
                     "PUB_K1_7Bn1YDeZ18w2N9DU4KAJxZDt6hk3L7eUwFRAc1hb5bp6uEBZA8",
                     "00032ea514c6b834dbdd6520d0ac420bcf2335fe138de3d2dc5b7b2f03f9f99e9fac");
    check_publickey("PUB_K1_11111111111111111111111111111111149Mr2R",
                    "00000000000000000000000000000000000000000000000000000000000000000000");
    check_publickey("PUB_K1_11111111111111111111111115qCHTcgbQwpvP72Uq",
                    "0000000000000000000000000000000000000000000000000000ffffffffffffffff");
    check_publickey("PUB_K1_111111111111111114ZrjxJnU1LA5xSyrWMNuXTrVub2r",
                    "000000000000000000000000000000000000ffffffffffffffffffffffffffffffff");
    check_publickey("PUB_K1_1111111113diW7pnisfdBvHTXP7wvW5k5Ky1e5DVuF4PizpM",
                    "00000000000000000000ffffffffffffffffffffffffffffffffffffffffffffffff");
    check_publickey("PUB_K1_11DsZ6Lyr1aXpm9aBqqgV4iFJpNbSw5eE9LLTwNAxqjJgXSdB8",
                    "00000080ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
    check_publickey("PUB_K1_12wkBET2rRgE8pahuaczxKbmv7ciehqsne57F9gtzf1PVb7Rf7o",
                    "0000ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
    check_publickey("PUB_K1_1yp8ebBuKZ13orqUrZsGsP49e6K3ThVK1nLutxSyU5j9Tx1r96",
                    "000080ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
    check_publickey("PUB_K1_9adaAMuB9v8yX1mZ5PtoB6VFSCeqRGjASd8ZTM6VUkiHLB5XEdw",
                    "00ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
    check_publickey("PUB_K1_69X3383RzBZj41k73CSjUNXM5MYGpnDxyPnWUKPEtYQmVzqTY7",
                    "0002a5d2400af24411f64c29da2fe893ff2b6681a3b6ffbe980b2ee42ad10cc2e994");
    check_publickey("PUB_K1_7yBtksm8Kkg85r4in4uCbfN77uRwe82apM8jjbhFVDgEcarGb8",
                    "000395c2020968e922eb4319fb56eb4fb0e7543d4b84ad367d8dc1b922338eb7232b");
    check_publickey("PUB_K1_7WnhaKwHpbSidYuh2DF1qAExTRUtPEdZCaZqt75cKcixtU7gEn",
                    "000359d04e6519311041b10fe9e828a226b48f3f27a52f071f8e364cd317785abebc");
    check_publickey("PUB_K1_7Bn1YDeZ18w2N9DU4KAJxZDt6hk3L7eUwFRAc1hb5bp6uEBZA8",
                    "00032ea514c6b834dbdd6520d0ac420bcf2335fe138de3d2dc5b7b2f03f9f99e9fac");

    check_cross_conversion(abi, PrivateKey::new("PVT_R1_PtoxLPzJZURZmPS4e26pjBiAn41mkkLPrET5qHnwDvbvqFEL6")?,
                           "private_key", r#""PVT_R1_PtoxLPzJZURZmPS4e26pjBiAn41mkkLPrET5qHnwDvbvqFEL6""#,
                           "0133fb621e78d5dc78f0029b6fd714bfe3b42fe4b72bc109051591e71f204d2813");
    check_cross_conversion(abi, PrivateKey::new("PVT_R1_vbRKUuE34hjMVQiePj2FEjM8FvuG7yemzQsmzx89kPS9J8Coz")?,
                           "private_key", r#""PVT_R1_vbRKUuE34hjMVQiePj2FEjM8FvuG7yemzQsmzx89kPS9J8Coz""#,
                           "0179b0c1811bf83356f3fa2dedb76494d8d2bba188fae9c286f118e5e9f0621760");
    check_cross_conversion2(abi, PrivateKey::new("5KQwrPbwdL6PhXujxW37FSSQZ1JiwsST4cqQzDeyXtP79zkvFD3")?,
                           "private_key", r#""5KQwrPbwdL6PhXujxW37FSSQZ1JiwsST4cqQzDeyXtP79zkvFD3""#,
                            "00d2653ff7cbb2d8ff129ac27ef5781ce68b2558c41a74af1f2ddca635cbeef07d",
                            r#""PVT_K1_2bfGi9rYsXQSXXTvJbDAPhHLQUojjaNLomdm3cEJ1XTzMqUt3V""#);

    check_cross_conversion(abi, Signature::new("SIG_K1_Kg2UKjXTX48gw2wWH4zmsZmWu3yarcfC21Bd9JPj7QoDURqiAacCHmtExPk3syPb2tFLsp1R4ttXLXgr7FYgDvKPC5RCkx")?,
                           "signature", r#""SIG_K1_Kg2UKjXTX48gw2wWH4zmsZmWu3yarcfC21Bd9JPj7QoDURqiAacCHmtExPk3syPb2tFLsp1R4ttXLXgr7FYgDvKPC5RCkx""#,
                           "002056355ed1079822d2728886b449f0f4a2bbf48bf38698c0ebe8c7079768882b1c64ac07d7a4bd85cf96b8a74fdcafef1a4805f946177c609fdf31abe2463038e5");
    check_cross_conversion(abi, Signature::new("SIG_R1_Kfh19CfEcQ6pxkMBz6xe9mtqKuPooaoyatPYWtwXbtwHUHU8YLzxPGvZhkqgnp82J41e9R6r5mcpnxy1wAf1w9Vyo9wybZ")?,
                           "signature", r#""SIG_R1_Kfh19CfEcQ6pxkMBz6xe9mtqKuPooaoyatPYWtwXbtwHUHU8YLzxPGvZhkqgnp82J41e9R6r5mcpnxy1wAf1w9Vyo9wybZ""#,
                           "012053a48d3bb9a321e4ae8f079eab72efa778c8c09bc4c2f734de6d19ad9bce6a137495d877d4e51a585376aa6c1a174295dabdb25286e803bf553735cd2d31b1fc");

    check_error(|| try_encode(abi, "checksum256", r#""xy""#), "Invalid character");
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

    check_cross_conversion(abi, SymbolCode::new("A")?,   "symbol_code", r#""A""#, "4100000000000000");
    check_cross_conversion(abi, SymbolCode::new("B")?,   "symbol_code", r#""B""#, "4200000000000000");
    check_cross_conversion(abi, SymbolCode::new("SYS")?, "symbol_code", r#""SYS""#, "5359530000000000");
    check_cross_conversion(abi, Symbol::new("0,A")?,   "symbol", r#""0,A""#, "0041000000000000");
    check_cross_conversion(abi, Symbol::new("1,Z")?,   "symbol", r#""1,Z""#, "015a000000000000");
    check_cross_conversion(abi, Symbol::new("4,SYS")?, "symbol", r#""4,SYS""#, "0453595300000000");

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
    let asset = |s| { Asset::from_str(s).unwrap() };
    let check_asset = |repr: &str, hex| {
        check_cross_conversion(abi, Asset::from_str(repr).unwrap(), "asset", &format!(r#""{}""#, repr), hex)
    };

    check_asset(      "0 FOO", "000000000000000000464f4f00000000");
    check_asset(    "0.0 FOO", "000000000000000001464f4f00000000");
    check_asset(   "0.00 FOO", "000000000000000002464f4f00000000");
    check_asset(  "0.000 FOO", "000000000000000003464f4f00000000");
    check_asset( "1.2345 SYS", "39300000000000000453595300000000");
    check_asset("-1.2345 SYS", "c7cfffffffffffff0453595300000000");

    check_cross_conversion(abi, Vec::<Asset>::new(), "asset[]", r#"[]"#, "00");
    check_cross_conversion(abi, vec![asset("0 FOO")], "asset[]", r#"["0 FOO"]"#,
                           "01000000000000000000464f4f00000000");
    check_cross_conversion(abi, vec![asset("0 FOO"), asset("0.000 FOO")], "asset[]", r#"["0 FOO","0.000 FOO"]"#,
                           "02000000000000000000464f4f00000000000000000000000003464f4f00000000");
    check_cross_conversion(abi, None::<Asset>, "asset?", "null", "00");
    check_cross_conversion(abi, Some(asset("0.123456 SIX")), "asset?", r#""0.123456 SIX""#, "0140e20100000000000653495800000000");

    check_cross_conversion(abi, ExtendedAsset { quantity: "0 FOO".parse()?, contract: Name::new("bar")? },
                           "extended_asset", r#"{"quantity":"0 FOO","contract":"bar"}"#,
                           "000000000000000000464f4f00000000000000000000ae39");
    check_cross_conversion(abi, ExtendedAsset { quantity: "0.123456 SIX".parse()?, contract: Name::new("seven")? },
                           "extended_asset", r#"{"quantity":"0.123456 SIX","contract":"seven"}"#,
                           "40e201000000000006534958000000000000000080a9b6c2");

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

    let transfer = Transfer {
        from: AccountName::constant("useraaaaaaaa"),
        to: AccountName::constant("useraaaaaaab"),
        quantity: Asset::from_str("0.0001 SYS")?,
        memo: "test memo".into(),
    };

    check_cross_conversion(
        token_abi, transfer.clone(), "transfer",
        r#"{"from":"useraaaaaaaa","to":"useraaaaaaab","quantity":"0.0001 SYS","memo":"test memo"}"#,
        "608c31c6187315d6708c31c6187315d6010000000000000004535953000000000974657374206d656d6f"
    );

    check_cross_conversion2(
        token_abi, transfer, "transfer",
        r#"{"to":"useraaaaaaab","memo":"test memo","from":"useraaaaaaaa","quantity":"0.0001 SYS"}"#,
        "608c31c6187315d6708c31c6187315d6010000000000000004535953000000000974657374206d656d6f",
        r#"{"from":"useraaaaaaaa","to":"useraaaaaaab","quantity":"0.0001 SYS","memo":"test memo"}"#,
    );

    let transfer = Transfer {
        from: Name::constant("useraaaaaaaa"),
        to: Name::constant("useraaaaaaab"),
        quantity: Asset::from_str("0.0001 SYS")?,
        memo: "".into(),
    };


    let tx = Transaction {
        expiration: TimePointSec::from_str("2009-02-13T23:31:31.000")?,
        ref_block_num: 1234,
        ref_block_prefix: 5678,
        actions: vec![
            Action::new(("useraaaaaaaa", "active"), transfer.clone()),
        ],
        ..Default::default()
    };

    check_cross_conversion(
        trx_abi, tx.clone(), "transaction",
        r#"{"expiration":"2009-02-13T23:31:31.000","ref_block_num":1234,"ref_block_prefix":5678,"max_net_usage_words":0,"max_cpu_usage_ms":0,"delay_sec":0,"context_free_actions":[],"actions":[{"account":"eosio.token","name":"transfer","authorization":[{"actor":"useraaaaaaaa","permission":"active"}],"data":"608c31c6187315d6708c31c6187315d60100000000000000045359530000000000"}],"transaction_extensions":[]}"#,
        "d3029649d2042e160000000000000100a6823403ea3055000000572d3ccdcd01608c31c6187315d600000000a8ed323221608c31c6187315d6708c31c6187315d6010000000000000004535953000000000000"
    );

    check_cross_conversion2(
        trx_abi, tx, "transaction",
        r#"{"ref_block_num":1234,"ref_block_prefix":5678,"expiration":"2009-02-13T23:31:31.000","max_net_usage_words":0,"max_cpu_usage_ms":0,"delay_sec":0,"context_free_actions":[],"actions":[{"account":"eosio.token","name":"transfer","authorization":[{"actor":"useraaaaaaaa","permission":"active"}],"data":"608c31c6187315d6708c31c6187315d60100000000000000045359530000000000"}],"transaction_extensions":[]}"#,
        "d3029649d2042e160000000000000100a6823403ea3055000000572d3ccdcd01608c31c6187315d600000000a8ed323221608c31c6187315d6708c31c6187315d6010000000000000004535953000000000000",
        r#"{"expiration":"2009-02-13T23:31:31.000","ref_block_num":1234,"ref_block_prefix":5678,"max_net_usage_words":0,"max_cpu_usage_ms":0,"delay_sec":0,"context_free_actions":[],"actions":[{"account":"eosio.token","name":"transfer","authorization":[{"actor":"useraaaaaaaa","permission":"active"}],"data":"608c31c6187315d6708c31c6187315d60100000000000000045359530000000000"}],"transaction_extensions":[]}"#,
    );

    let tx = PackedTransactionV0 {
        signatures: vec![Signature::new("SIG_K1_K5PGhrkUBkThs8zdTD9mGUJZvxL4eU46UjfYJSEdZ9PXS2Cgv5jAk57yTx4xnrdSocQm6DDvTaEJZi5WLBsoZC4XYNS8b3")?],
        compression: 0,
        packed_context_free_data: Bytes::from_hex("")?,
        packed_trx: Transaction {
            expiration: "2009-02-13T23:31:31.000".try_into()?,
            ref_block_num: 1234,
            ref_block_prefix: 5678,
            actions: vec![Action::new(("useraaaaaaaa", "active"), transfer)],
            ..Default::default()
        }
    };

    check_cross_conversion(
        packed_trx_abi, tx, "packed_transaction_v0",
        r#"{"signatures":["SIG_K1_K5PGhrkUBkThs8zdTD9mGUJZvxL4eU46UjfYJSEdZ9PXS2Cgv5jAk57yTx4xnrdSocQm6DDvTaEJZi5WLBsoZC4XYNS8b3"],"compression":0,"packed_context_free_data":"","packed_trx":{"expiration":"2009-02-13T23:31:31.000","ref_block_num":1234,"ref_block_prefix":5678,"max_net_usage_words":0,"max_cpu_usage_ms":0,"delay_sec":0,"context_free_actions":[],"actions":[{"account":"eosio.token","name":"transfer","authorization":[{"actor":"useraaaaaaaa","permission":"active"}],"data":"608c31c6187315d6708c31c6187315d60100000000000000045359530000000000"}],"transaction_extensions":[]}}"#,
        "01001f4d6c791d32e38ca1a0a5f3139b8d1d521b641fe2ee675311fca4c755acdfca2d13fe4dee9953d2504fcb4382eeacbcef90e3e8034bdd32eba11f1904419df6af0000d3029649d2042e160000000000000100a6823403ea3055000000572d3ccdcd01608c31c6187315d600000000a8ed323221608c31c6187315d6708c31c6187315d6010000000000000004535953000000000000"
    );

    Ok(())
}

#[test]
#[cfg(feature = "float128")]
fn roundtrip_transaction_traces() -> Result<()> {
    use antelope::{TransactionTraceException, TransactionTraceMsg};

    init();

    let ship_abi_def = ABIDefinition::from_str(STATE_HISTORY_PLUGIN_ABI)?;
    let ship_abi = &ABI::from_definition(&ship_abi_def)?;

    let eosio = Name::from_str("eosio")?;
    let trace = TransactionTrace::V0(TransactionTraceV0 {
        id: "3098EA9476266BFA957C13FA73C26806D78753099CE8DEF2A650971F07595A69".try_into()?,
        status: 0,
        cpu_usage_us: 2000,
        net_usage_words: VarUint32(25),
        elapsed: 194,
        net_usage: 200,
        scheduled: false,
        action_traces: vec![ActionTrace::V1(ActionTraceV1 {
            action_ordinal: VarUint32(1),
            creator_action_ordinal: VarUint32(0),
            receipt: Some(ActionReceipt::V0(ActionReceiptV0 {
                receiver: eosio,
                act_digest: "F2FDEEFF77EFC899EED23EE05F9469357A096DC3083D493571CF68A422C69EFE".try_into()?,
                global_sequence: 11,
                recv_sequence: 11,
                auth_sequence: vec![AccountAuthSequence { account: eosio, sequence: 11 }],
                code_sequence: VarUint32(2),
                abi_sequence: VarUint32(0),
            })),
            receiver: eosio,
            act: Action {
                account: eosio,
                name: Name::from_str("newaccount")?,
                authorization: vec![PermissionLevel { actor: eosio, permission: PermissionName::from_str("active")? }],
                data: Bytes::from_hex("0000000000EA305500409406A888CCA501000000010002C0DED2BC1F1305FB0FAAC5E6C03EE3A1924234985427B6167CA569D13DF435CF0100000001000000010002C0DED2BC1F1305FB0FAAC5E6C03EE3A1924234985427B6167CA569D13DF435CF01000000")?,
            },
            context_free: false,
            elapsed: 83,
            console: "".into(),
            account_ram_deltas: vec![AccountDelta { account: Name::from_str("oracle.aml")?, delta: 2724 }],
            account_disk_deltas: vec![],
            except: None,
            error_code: None,
            return_value: Bytes::from_hex("")?,
        })],
        account_ram_delta: None,
        except: None,
        error_code: None,
        failed_dtrx_trace: None,
        partial: None,
    });

    check_cross_conversion(
        ship_abi, trace, "transaction_trace",
        r#"["transaction_trace_v0",{"id":"3098ea9476266bfa957c13fa73c26806d78753099ce8def2a650971f07595a69","status":0,"cpu_usage_us":2000,"net_usage_words":25,"elapsed":194,"net_usage":200,"scheduled":false,"action_traces":[["action_trace_v1",{"action_ordinal":1,"creator_action_ordinal":0,"receipt":["action_receipt_v0",{"receiver":"eosio","act_digest":"f2fdeeff77efc899eed23ee05f9469357a096dc3083d493571cf68a422c69efe","global_sequence":11,"recv_sequence":11,"auth_sequence":[{"account":"eosio","sequence":11}],"code_sequence":2,"abi_sequence":0}],"receiver":"eosio","act":{"account":"eosio","name":"newaccount","authorization":[{"actor":"eosio","permission":"active"}],"data":"0000000000ea305500409406a888cca501000000010002c0ded2bc1f1305fb0faac5e6c03ee3a1924234985427b6167ca569d13df435cf0100000001000000010002c0ded2bc1f1305fb0faac5e6c03ee3a1924234985427b6167ca569d13df435cf01000000"},"context_free":false,"elapsed":83,"console":"","account_ram_deltas":[{"account":"oracle.aml","delta":2724}],"account_disk_deltas":[],"except":null,"error_code":null,"return_value":""}]],"account_ram_delta":null,"except":null,"error_code":null,"failed_dtrx_trace":null,"partial":null}]"#,
        "003098ea9476266bfa957c13fa73c26806d78753099ce8def2a650971f07595a6900d007000019c200000000000000c800000000000000000101010001000000000000ea3055f2fdeeff77efc899eed23ee05f9469357a096dc3083d493571cf68a422c69efe0b000000000000000b00000000000000010000000000ea30550b0000000000000002000000000000ea30550000000000ea305500409e9a2264b89a010000000000ea305500000000a8ed3232660000000000ea305500409406a888cca501000000010002c0ded2bc1f1305fb0faac5e6c03ee3a1924234985427b6167ca569d13df435cf0100000001000000010002c0ded2bc1f1305fb0faac5e6c03ee3a1924234985427b6167ca569d13df435cf01000000005300000000000000000100409406a888cca5a40a000000000000000000000000000000"
    );

    let exc = TransactionTraceException { error_code: 3, error_message: "error happens".to_string() };

    check_cross_conversion(
        ship_abi, TransactionTraceMsg::Exception(exc), "transaction_trace_msg",
        r#"["transaction_trace_exception",{"error_code":3,"error_message":"error happens"}]"#,
        "0003000000000000000d6572726f722068617070656e73"
    );

    let trace = TransactionTrace::V0(TransactionTraceV0 {
        id: "b2c8d46f161e06740cfadabfc9d11f013a1c90e25337ff3e22840b195e1adc4b".try_into()?,
        status: 0,
        cpu_usage_us: 2000,
        net_usage_words: VarUint32(12),
        elapsed: 7670,
        net_usage: 96,
        scheduled: false,
        action_traces: vec![ActionTrace::V1(ActionTraceV1 {
            action_ordinal: VarUint32(1),
            creator_action_ordinal: VarUint32(0),
            receipt: Some(ActionReceipt::V0(ActionReceiptV0 {
                receiver: eosio,
                act_digest: "7670940c29ec0a4c573ef052c5a29236393f587f208222b3c1b6a9c8fea2c66a".try_into()?,
                global_sequence: 27,
                recv_sequence: 1,
                auth_sequence: vec![AccountAuthSequence { account: eosio, sequence: 2 }],
                code_sequence: VarUint32(1),
                abi_sequence: VarUint32(0),
            })),
            receiver: eosio,
            act: Action {
                account: eosio,
                name: Name::from_str("doit")?,
                authorization: vec![PermissionLevel { actor: eosio, permission: PermissionName::from_str("active")? }],
                data: Bytes::from_hex("00")?,
            },
            context_free: false,
            elapsed: 7589,
            console: "".into(),
            account_ram_deltas: vec![],
            account_disk_deltas: vec![],
            except: None,
            error_code: None,
            return_value: Bytes::from_hex("01ffffffffffffffff00")?,
        })],
        account_ram_delta: None,
        except: None,
        error_code: None,
        failed_dtrx_trace: None,
        partial: None,
    });

    check_cross_conversion(
        ship_abi, TransactionTraceMsg::Trace(trace), "transaction_trace_msg",
        r#"["transaction_trace",["transaction_trace_v0",{"id":"b2c8d46f161e06740cfadabfc9d11f013a1c90e25337ff3e22840b195e1adc4b","status":0,"cpu_usage_us":2000,"net_usage_words":12,"elapsed":7670,"net_usage":96,"scheduled":false,"action_traces":[["action_trace_v1",{"action_ordinal":1,"creator_action_ordinal":0,"receipt":["action_receipt_v0",{"receiver":"eosio","act_digest":"7670940c29ec0a4c573ef052c5a29236393f587f208222b3c1b6a9c8fea2c66a","global_sequence":27,"recv_sequence":1,"auth_sequence":[{"account":"eosio","sequence":2}],"code_sequence":1,"abi_sequence":0}],"receiver":"eosio","act":{"account":"eosio","name":"doit","authorization":[{"actor":"eosio","permission":"active"}],"data":"00"},"context_free":false,"elapsed":7589,"console":"","account_ram_deltas":[],"account_disk_deltas":[],"except":null,"error_code":null,"return_value":"01ffffffffffffffff00"}]],"account_ram_delta":null,"except":null,"error_code":null,"failed_dtrx_trace":null,"partial":null}]]"#,
        "0100b2c8d46f161e06740cfadabfc9d11f013a1c90e25337ff3e22840b195e1adc4b00d00700000cf61d0000000000006000000000000000000101010001000000000000ea30557670940c29ec0a4c573ef052c5a29236393f587f208222b3c1b6a9c8fea2c66a1b000000000000000100000000000000010000000000ea3055020000000000000001000000000000ea30550000000000ea30550000000000901d4d010000000000ea305500000000a8ed3232010000a51d00000000000000000000000a01ffffffffffffffff000000000000"
    );

    Ok(())
}
