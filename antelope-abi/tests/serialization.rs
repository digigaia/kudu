use std::fmt::Debug;
use std::str::FromStr;

use color_eyre::eyre::Result;
use chrono::{NaiveDate, TimeZone, Utc};

use antelope_abi::{
    ByteStream, BinarySerializable,
    abiserializable::ABISerializable,
};
use antelope_core::{
    AntelopeValue, AntelopeType, VarInt32, VarUint32,
    Name, Symbol, SymbolCode, Asset, TimePoint, TimePointSec,
};

// =============================================================================
//
//     NOTES:
//      - tests have been sourced from:
//        - https://github.com/FACINGS/pyntelope/blob/main/tests/unit/types_test.py
//        - https://github.com/AntelopeIO/abieos/blob/main/src/test.cpp#L577
//
//     TODO:
//      - missing tests for: Bytes, BlockTimestampType, checksum types,
//        crypto types, ExtendedAsset
//      - check more tests at: https://github.com/wharfkit/antelope/blob/master/test/serializer.ts
//
// =============================================================================


// -----------------------------------------------------------------------------
//     Utility test functions
// -----------------------------------------------------------------------------

#[track_caller]
fn test_encode<T>(obj: T, repr: &str)
where
    T: BinarySerializable + Debug + PartialEq,
{
    let mut stream = ByteStream::new();

    // abi.encode(&mut stream, &obj);
    obj.encode(&mut stream);
    assert_eq!(stream.hex_data(), repr,
               "wrong ABI serialization for: {obj:?}");
}

#[track_caller]
fn test_roundtrip<T>(obj: T, repr: &str)
where
    T: BinarySerializable + Debug + PartialEq,
{
    let mut stream = ByteStream::new();

    // abi.encode(&mut stream, &obj);
    obj.encode(&mut stream);
    assert_eq!(stream.hex_data(), repr,
               "wrong serialization for: {obj:?}");

    let decoded = T::decode(&mut stream).unwrap();
    assert_eq!(decoded, obj,
               "deserialized object `{:?}` is not the same as original one `{:?}`",
               decoded, obj);
}

#[track_caller]
fn test_roundtrip_variant(obj: AntelopeValue, repr: &str) {
    let mut stream = ByteStream::new();

    obj.to_bin(&mut stream);
    assert_eq!(stream.hex_data(), repr, "wrong serialization for: {obj:?}");

    let typename: AntelopeType = AntelopeType::from_str(obj.as_ref()).unwrap();
    let decoded = AntelopeValue::from_bin(typename, &mut stream).unwrap();
    assert_eq!(decoded, obj,
               "deserialized object `{:?}` is not the same as original one `{:?}`",
               decoded, obj);
}

#[track_caller]
fn check_round_trip<T, const N: usize, F>(vals: [(T, &str); N], convert: F)
where
    T: BinarySerializable + Debug + Clone + PartialEq,
    F: Fn(T) -> AntelopeValue,
{
    for (val, repr) in vals {
        // test serialization of the type itself
        test_roundtrip(val.clone(), repr);

        // test serialization of the type wrapped in an `AntelopeValue`
        test_roundtrip_variant(convert(val.clone()), repr);
    }
}

#[track_caller]
fn check_round_trip_map_type<T, MT, const N: usize, MF, F>(
    vals: [(T, &str); N],
    map_input: MF,
    convert: F
) where
    MF: Fn(T) -> MT,
    MT: BinarySerializable + Debug + Clone + PartialEq,
    F: Fn(MT) -> AntelopeValue,
{
    for (val, repr) in vals {
        let val = map_input(val);

        // test serialization of the type itself
        test_roundtrip(val.clone(), repr);

        // test serialization of the type wrapped in an `AntelopeValue`
        test_roundtrip_variant(convert(val.clone()), repr);
    }
}


// -----------------------------------------------------------------------------
//     Bool tests
// -----------------------------------------------------------------------------

#[test]
fn test_bools() {
    let vals = [
        (true,  "01"),
        (false, "00"),
    ];

    check_round_trip(vals, AntelopeValue::Bool);
}


// -----------------------------------------------------------------------------
//     Signed int tests
// -----------------------------------------------------------------------------

#[test]
fn test_i8() {
    let vals = [
        (-128i8, "80"),
        (  -127, "81"),
        (    -1, "ff"),
        (     0, "00"),
        (     1, "01"),
        (   127, "7f"),
    ];

    check_round_trip(vals, AntelopeValue::Int8);
}

#[test]
fn test_i16() {
    let vals = [
        (-32768i16, "0080"),
        (   -32767, "0180"),
        (       -1, "ffff"),
        (        0, "0000"),
        (        1, "0100"),
        (    32767, "ff7f"),
    ];

    check_round_trip(vals, AntelopeValue::Int16);
}

#[test]
fn test_i32() {
    let vals = [
        (-2147483648i32, "00000080"),
        (   -2147483647, "01000080"),
        (            -1, "ffffffff"),
        (             0, "00000000"),
        (             1, "01000000"),
        (    2147483647, "ffffff7f"),
    ];

    check_round_trip(vals, AntelopeValue::Int32);
}

#[test]
fn test_i64() {
    let vals = [
        (-9223372036854775808i64, "0000000000000080"),
        (   -9223372036854775807, "0100000000000080"),
        (                    -23, "e9ffffffffffffff"),
        (                     -1, "ffffffffffffffff"),
        (                      0, "0000000000000000"),
        (                      1, "0100000000000000"),
        (    9223372036854775807, "ffffffffffffff7f"),
    ];

    check_round_trip(vals, AntelopeValue::Int64);
}

#[test]
fn test_i128() {
    let vals = [
        (                                  0_i128, "00000000000000000000000000000000"),
        (                                       1, "01000000000000000000000000000000"),
        (                                      -1, "ffffffffffffffffffffffffffffffff"),
        (                    18446744073709551615, "ffffffffffffffff0000000000000000"),
        (                   -18446744073709551615, "0100000000000000ffffffffffffffff"),
        ( 170141183460469231731687303715884105727, "ffffffffffffffffffffffffffffff7f"),
        (-170141183460469231731687303715884105727, "01000000000000000000000000000080"),
        (-170141183460469231731687303715884105728, "00000000000000000000000000000080"),
    ];

    check_round_trip(vals, AntelopeValue::Int128);
}

#[test]
fn test_var_i32() {
    let vals = [
        (      0_i32, "00"),
        (         -1, "01"),
        (          1, "02"),
        (         -2, "03"),
        (          2, "04"),
        (-2147483647, "fdffffff0f"),
        ( 2147483647, "feffffff0f"),
        (-2147483648, "ffffffff0f"),
    ];

    check_round_trip_map_type(vals, VarInt32::from, AntelopeValue::VarInt32);
}


// -----------------------------------------------------------------------------
//     Unsigned int tests
// -----------------------------------------------------------------------------

#[test]
fn test_u8() {
    let vals = [
        (0u8, "00"),
        (  1, "01"),
        (254, "fe"),
        (255, "ff"),
    ];

    check_round_trip(vals, AntelopeValue::Uint8);
}

#[test]
fn test_u16() {
    let vals = [
        ( 0u16, "0000"),
        (    1, "0100"),
        (65534, "feff"),
        (65535, "ffff"),
    ];

    check_round_trip(vals, AntelopeValue::Uint16);
}

#[test]
fn test_u32() {
    let vals = [
        (      0u32, "00000000"),
        (         1, "01000000"),
        (     10800, "302a0000"),
        (    123456, "40e20100"),
        (4294967294, "feffffff"),
        (4294967295, "ffffffff"),
    ];

    check_round_trip(vals, AntelopeValue::Uint32);
}

#[test]
fn test_u64() {
    let vals = [
        (                0u64, "0000000000000000"),
        (                   1, "0100000000000000"),
        (                   5, "0500000000000000"),
        (18446744073709551614, "feffffffffffffff"),
        (18446744073709551615, "ffffffffffffffff"),
    ];

    check_round_trip(vals, AntelopeValue::Uint64);
}

#[test]
fn test_u128() {
    let vals = [
        (                                 0_u128, "00000000000000000000000000000000"),
        (                                      1, "01000000000000000000000000000000"),
        (                   18446744073709551615, "ffffffffffffffff0000000000000000"),
        (170141183460469231731687303715884105727, "ffffffffffffffffffffffffffffff7f"),
        (340282366920938463463374607431768211454, "feffffffffffffffffffffffffffffff"),
        (340282366920938463463374607431768211455, "ffffffffffffffffffffffffffffffff"),
    ];

    check_round_trip(vals, AntelopeValue::Uint128);
}

#[test]
fn test_var_u32() {
    let vals = [
        (     0_u32, "00"),
        (         1, "01"),
        (         3, "03"),
        (       127, "7f"),
        (       128, "8001"),
        (       129, "8101"),
        (       255, "ff01"),
        (       256, "8002"),
        (     16383, "ff7f"),
        (     16384, "808001"),
        (     16385, "818001"),
        (   2097151, "ffff7f"),
        (   2097152, "80808001"),
        (   2097153, "81808001"),
        ( 268435455, "ffffff7f"),
        ( 268435456, "8080808001"),
        ( 268435457, "8180808001"),
        (4294967294, "feffffff0f"),
        (4294967295, "ffffffff0f"),
    ];

    check_round_trip_map_type(vals, VarUint32::from, AntelopeValue::VarUint32);
}


// -----------------------------------------------------------------------------
//     Floating point tests
// -----------------------------------------------------------------------------

#[test]
fn test_f32() {
    let vals = [
        (    0f32, "00000000"),
        (     0.1, "cdcccc3d"),
        (    0.10, "cdcccc3d"),
        (   0.100, "cdcccc3d"),
        ( 0.00001, "acc52737"),
        (     0.3, "9a99993e"),
        (    1f32, "0000803f"),
        (     1.0, "0000803f"),
        (   10f32, "00002041"),
        (    1e15, "a95f6358"),
        ( 1.15e15, "68bd8258"),
        // (-0f32, "00000000"),  // FIXME: failing, what is the actual expected value?
        (    -0.1, "cdccccbd"),
        (   -0.10, "cdccccbd"),
        (  -0.100, "cdccccbd"),
        (-0.00001, "acc527b7"),
        (    -0.3, "9a9999be"),
        (   -1f32, "000080bf"),
        (    -1.0, "000080bf"),
        (  -10f32, "000020c1"),
        (   -1e15, "a95f63d8"),
        (-1.15e15, "68bd82d8"),
    ];

    check_round_trip(vals, AntelopeValue::Float32);
}

#[test]
fn test_f64() {
    let vals = [
        (    0f64, "0000000000000000"),
        (     0.1, "9a9999999999b93f"),
        (    0.10, "9a9999999999b93f"),
        (   0.100, "9a9999999999b93f"),
        ( 0.00001, "f168e388b5f8e43e"),
        (     0.3, "333333333333d33f"),
        (    1f64, "000000000000f03f"),
        (     1.0, "000000000000f03f"),
        (   10f64, "0000000000002440"),
        (    1e15, "00003426f56b0c43"),
        ( 1.15e15, "0080f7f5ac571043"),
        // (-0f64, "0000000000000000"),  // FIXME: failing, what is the actual expected value?
        (    -0.1, "9a9999999999b9bf"),
        (   -0.10, "9a9999999999b9bf"),
        (  -0.100, "9a9999999999b9bf"),
        (-0.00001, "f168e388b5f8e4be"),
        (    -0.3, "333333333333d3bf"),
        (   -1f64, "000000000000f0bf"),
        (    -1.0, "000000000000f0bf"),
        (  -10f64, "00000000000024c0"),
        (   -1e15, "00003426f56b0cc3"),
        (-1.15e15, "0080f7f5ac5710c3"),
    ];

    check_round_trip(vals, AntelopeValue::Float64);
}


// -----------------------------------------------------------------------------
//     String and Bytes tests
// -----------------------------------------------------------------------------


#[test]
fn test_string() {
    let vals = [
        ("", "00"),
        ("a", "0161"),
        ("A", "0141"),
        ("Hello world!", "0c48656c6c6f20776f726c6421"),
    ];
    check_round_trip_map_type(vals, |s| s.to_owned(), AntelopeValue::String);

    test_encode("foo", "03666f6f");  // can't decode to &str due to lifetime issues
}


// -----------------------------------------------------------------------------
//     Time-related types tests
// -----------------------------------------------------------------------------


#[test]
fn test_time_point_sec() -> Result<()> {
    fn dt(year: i32, month: u32, day: u32,
          hour: u32, min: u32, sec: u32) -> TimePointSec
    {
        TimePointSec::from(Utc.with_ymd_and_hms(year, month, day, hour, min, sec)
                           .unwrap().timestamp() as u32)
    }

    let vals = [
        (dt(1970,  1,  1,  0,  0,  0), "00000000"),
        (dt(2040, 12, 31, 23, 59,  0), "44038d85"),
        (dt(2021,  8, 26, 14,  1, 47), "cb9e2761"),
        // this next constructor is a bit verbose but there's no Utc.with_ymd_and_hms_and_micros...
        (TimePointSec::from(NaiveDate::from_ymd_opt(2021, 8, 26,).unwrap()
                            .and_hms_micro_opt(14, 1, 47, 184549).unwrap()
                            .and_utc().timestamp() as u32), "cb9e2761"),
    ];

    check_round_trip(vals, AntelopeValue::TimePointSec);
    Ok(())
}

#[test]
fn test_time_point() -> Result<()> {
    fn dt(year: i32, month: u32, day: u32,
          hour: u32, min: u32, sec: u32, micro: u32) -> TimePoint
    {
        TimePoint::from(NaiveDate::from_ymd_opt(year, month, day).unwrap()
                        .and_hms_micro_opt(hour, min, sec, micro).unwrap()
                        .and_utc().timestamp_micros())
    }

    let vals = [
        (dt(1970,  1,  1,  0,  0,  0,    0), "0000000000000000"),
        (dt(1970,  1,  1,  0,  0,  0, 1000), "e803000000000000"),
        (dt(1970,  1,  1,  0,  0,  0, 2000), "d007000000000000"),
        (dt(1970,  1,  1,  0,  0,  0, 3000), "b80b000000000000"),
        (dt(1970,  1,  1,  0,  0,  1,    0), "40420f0000000000"),
        (dt(2040, 12, 31, 23, 59,  0,    0), "005914efd2f50700"),
        (dt(2021,  8, 26, 14,  1, 47,    0), "c008bdce76ca0500"),
    ];

    check_round_trip(vals, AntelopeValue::TimePoint);
    Ok(())
}


// -----------------------------------------------------------------------------
//     Other builtin Antelope types tests
// -----------------------------------------------------------------------------

#[test]
fn test_name() -> Result<()> {
    let vals = [
        ("a",             "0000000000000030"),
        ("b",             "0000000000000038"),
        ("foobar",        "000000005c73285d"),
        ("zzzzzzzzzzzzj", "ffffffffffffffff"),
        ("kacjndfvdfa",   "00cc4a7ba5f99081"),
        ("user2",         "00000000007115d6"),
        ("",              "0000000000000000"),
    ];

    check_round_trip_map_type(vals,
                              |s| Name::from_str(s).unwrap(),
                              AntelopeValue::Name);
    Ok(())
}

#[test]
fn test_symbol_code() -> Result<()> {
    let vals = [
        ("A",   "4100000000000000"),
        ("B",   "4200000000000000"),
        ("SYS", "5359530000000000"),
    ];

    check_round_trip_map_type(vals,
                              |s| SymbolCode::from_str(s).unwrap(),
                              AntelopeValue::SymbolCode);
    Ok(())
}

#[test]
fn test_symbol() -> Result<()> {
    let vals = [
        ("0,W",       "0057000000000000"),  // minimum amount of characters
        ("0,WAXXXXX", "0057415858585858"),  // maximum amount of characters
        ("1,WAX",     "0157415800000000"),  // 1 precision
        ("16,WAX",    "1057415800000000"),  // max precision
        ("4,FOO",     "04464f4f00000000"),
    ];

    check_round_trip_map_type(vals,
                              |s| Symbol::from_str(s).unwrap(),
                              AntelopeValue::Symbol);
    Ok(())
}

#[test]
fn test_asset() -> Result<()> {
    let vals = [
        ("99.9 WAX",   "e7030000000000000157415800000000"),
        ("99 WAX",     "63000000000000000057415800000000"),
        ("1.2345 FOO", "393000000000000004464f4f00000000"),
    ];

    check_round_trip_map_type(vals,
                              |s| Asset::from_str(s).unwrap(),
                              AntelopeValue::Asset);
    Ok(())
}
