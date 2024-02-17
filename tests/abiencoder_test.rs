use std::fmt::Display;

use serde_json::json;
// use anyhow::Result;
use color_eyre::eyre::Result;
use chrono::NaiveDate;

use antelope::abi::*;
use antelope::{
    ABIEncoder, ByteStream,
    types::{
        AntelopeValue, Name, Symbol, Asset
    }
};

// TODO: add tests for deserialization
// TODO: check more tests at: https://github.com/wharfkit/antelope/blob/master/test/serializer.ts


#[test]
fn test_serialize_ints() {
    let i1 = AntelopeValue::Uint64(5);
    let i2 = AntelopeValue::Int64(-23);

    let abi = ABIEncoder::new();
    let mut ds = ByteStream::new();

    abi.encode(&mut ds, &i1);
    assert_eq!(ds.hex_data(), "0500000000000000");

    ds.clear();
    abi.encode(&mut ds, &i2);
    assert_eq!(ds.hex_data(), "E9FFFFFFFFFFFFFF");
}

#[test]
fn test_serialize_string() {
    let abi = ABIEncoder::new();
    let mut ds = ByteStream::new();

    abi.encode(&mut ds, &AntelopeValue::String("foo".to_owned()));
    assert_eq!(ds.hex_data(), "03666F6F");

    ds.clear();
    abi.encode(&mut ds, &AntelopeValue::String("Hello world!".to_owned()));
    assert_eq!(ds.hex_data(), "0C48656C6C6F20776F726C6421");
}

#[test]
fn test_serialize_array() {
    let a = ["foo", "bar", "baz"];
    let abi = ABIEncoder::new();
    let mut ds = ByteStream::new();

    abi.encode_variant(&mut ds, "string[]", &json!(a)).unwrap();
    assert_eq!(ds.hex_data(), "0303666F6F036261720362617A");

    ds.clear();
    let v = vec!["foo", "bar", "baz"];
    abi.encode_variant(&mut ds, "string[]", &json!(v)).unwrap();
    assert_eq!(ds.hex_data(), "0303666F6F036261720362617A");
}


#[test]
fn test_serialize_name() {
    let data = "000000005C73285D";
    let obj = Name::from_str("foobar").unwrap();
    let json = r#""foobar""#;

    let abi = ABIEncoder::new();
    let mut ds = ByteStream::new();
    abi.encode(&mut ds, &AntelopeValue::Name(obj.clone()));

    assert_eq!(obj.to_u64(), 6712742083569909760);

    assert_eq!(&ds.hex_data(), &data);

    assert_eq!(serde_json::from_str::<Name>(json).unwrap(), obj);
    assert_eq!(serde_json::to_string(&obj).unwrap(), json);
}

#[test]
fn test_serialize_symbol() {
    let data = "04464F4F00000000";
    let obj = Symbol::from_str("4,FOO").unwrap();
    let json = r#""4,FOO""#;

    let abi = ABIEncoder::new();
    let mut ds = ByteStream::new();
    abi.encode(&mut ds, &AntelopeValue::Symbol(obj.clone()));

    assert_eq!(obj.decimals(), 4);
    assert_eq!(obj.name(), "FOO");

    assert_eq!(&ds.hex_data(), &data);

    assert_eq!(serde_json::from_str::<Symbol>(json).unwrap(), obj);
    assert_eq!(serde_json::to_string(&obj).unwrap(), json);
}

#[test]
fn test_serialize_asset() {
    let data = "393000000000000004464F4F00000000";
    let obj = Asset::from_str("1.2345 FOO").unwrap();
    let json = r#""1.2345 FOO""#;

    let abi = ABIEncoder::new();
    let mut ds = ByteStream::new();
    abi.encode(&mut ds, &AntelopeValue::Asset(obj.clone()));

    assert_eq!(obj.amount(), 12345);
    assert_eq!(obj.decimals(), 4);
    assert_eq!(obj.precision(), 10000);

    assert_eq!(&ds.hex_data(), &data);

    assert_eq!(serde_json::from_str::<Asset>(json).unwrap(), obj);
    assert_eq!(serde_json::to_string(&obj).unwrap(), json);
}

#[test]
fn test_serialize_struct() {
    let abi = ABIDefinition {
        structs: vec![
            Struct {
                base: "".to_owned(),
                name: "foo".to_owned(),
                fields: vec![
                    Field { name: "one".to_owned(), type_: "string".to_owned() },
                    Field { name: "two".to_owned(), type_: "int8".to_owned() },
                ],
            },
            Struct {
                base: "foo".to_owned(),
                name: "bar".to_owned(),
                fields: vec![
                    Field { name: "three".to_owned(), type_: "name?".to_owned() },
                    Field { name: "four".to_owned(), type_: "string[]?".to_owned() },
                ],
            },
        ],
        ..Default::default()
    };

    let obj = json!({
        "one": "one",
        "two": 2,
        "three": "two",
        "four": ['f', 'o', 'u', 'r'],
    });

    let abi = ABIEncoder::from_abi(&abi);
    let mut ds = ByteStream::new();
    abi.encode_variant(&mut ds, "bar", &obj).unwrap();

    assert_eq!(&ds.hex_data(), "036F6E65020100000000000028CF01040166016F01750172");

    // FIXME: implement me!
    // let dec = abi.decode_variant(&ds, &"bar");
    // assert_eq!(dec.to_string(), r#"{"one":"one","two":2,"three":"two","four":["f","o","u","r"]}"#);
}


////////////////////////////////////////////////////////////////////////////////
//                                                                            //
// following tests come from:                                                 //
// https://github.com/FACINGS/pyntelope/blob/main/tests/unit/types_test.py    //
//                                                                            //
////////////////////////////////////////////////////////////////////////////////

fn test_serialize<T, const N: usize, const M: usize, F>(vals: [(T, &[u8; N]); M], convert: F)
where
    T: Display + Clone,
    F: Fn(T) -> AntelopeValue,
{
    let mut ds = ByteStream::new();

    for (val, repr) in vals {
        ds.clear();
        convert(val.clone()).to_bin(&mut ds);
        // val.encode(&mut ds);
        assert_eq!(ds.data(), repr, "wrong ABI serialization for: {val}");
    }
}

#[test]
fn test_bools() {
    let vals = [
        (true, b"\x01"),
        (false, b"\x00"),
    ];

    test_serialize(vals, AntelopeValue::Bool);
}

#[test]
fn test_i8() {
    let vals = [
        (-128i8, b"\x80"),
        (-127, b"\x81"),
        (-1, b"\xFF"),
        (0, b"\x00"),
        (1, b"\x01"),
        (127, b"\x7F"),

    ];

    test_serialize(vals, AntelopeValue::Int8);
}

#[test]
fn test_i16() {
    let vals = [
        (-32768i16, b"\x00\x80"),
        (-32767, b"\x01\x80"),
        (-1, b"\xFF\xFF"),
        (0, b"\x00\x00"),
        (1, b"\x01\x00"),
        (32767, b"\xFF\x7F"),

    ];

    test_serialize(vals, AntelopeValue::Int16);
}

#[test]
fn test_i32() {
    let vals = [
        (-2147483648i32, b"\x00\x00\x00\x80"),
        (-2147483647, b"\x01\x00\x00\x80"),
        (-1, b"\xFF\xFF\xFF\xFF"),
        (0, b"\x00\x00\x00\x00"),
        (1, b"\x01\x00\x00\x00"),
        (2147483647, b"\xFF\xFF\xFF\x7F"),

    ];

    test_serialize(vals, AntelopeValue::Int32);
}

#[test]
fn test_i64() {
    let vals = [
        (-9223372036854775808i64, b"\x00\x00\x00\x00\x00\x00\x00\x80"),
        (-9223372036854775807, b"\x01\x00\x00\x00\x00\x00\x00\x80"),
        (-1, b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF"),
        (0, b"\x00\x00\x00\x00\x00\x00\x00\x00"),
        (1, b"\x01\x00\x00\x00\x00\x00\x00\x00"),
        (9223372036854775807, b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x7F"),

    ];

    test_serialize(vals, AntelopeValue::Int64);
}

#[test]
fn test_f32() {
    let vals = [
        (0f32, b"\x00\x00\x00\x00"),
        (0.1, b"\xcd\xcc\xcc="),
        (0.10, b"\xcd\xcc\xcc="),
        (0.100, b"\xcd\xcc\xcc="),
        (0.00001, b"\xac\xc5'7"),
        (0.3, b"\x9a\x99\x99>"),
        (1f32, b"\x00\x00\x80?"),
        (1.0, b"\x00\x00\x80?"),
        (10f32, b"\x00\x00 A"),
        (1e15, b"\xa9_cX"),
        (1.15e15, b"h\xbd\x82X"),
        // (-0f32, b"\x00\x00\x00\x00"),  // FIXME: failing, what is the actual expected value?
        (-0.1, b"\xcd\xcc\xcc\xbd"),
        (-0.10, b"\xcd\xcc\xcc\xbd"),
        (-0.100, b"\xcd\xcc\xcc\xbd"),
        (-0.00001, b"\xac\xc5'\xb7"),
        (-0.3, b"\x9a\x99\x99\xbe"),
        (-1f32, b"\x00\x00\x80\xbf"),
        (-1.0, b"\x00\x00\x80\xbf"),
        (-10f32, b"\x00\x00 \xc1"),
        (-1e15, b"\xa9_c\xd8"),
        (-1.15e15, b"h\xbd\x82\xd8"),
    ];

    test_serialize(vals, AntelopeValue::Float32);
}

#[test]
fn test_f64() {
    let vals = [
        (0f64, b"\x00\x00\x00\x00\x00\x00\x00\x00"),
        (0.1, b"\x9a\x99\x99\x99\x99\x99\xb9?"),
        (0.10, b"\x9a\x99\x99\x99\x99\x99\xb9?"),
        (0.100, b"\x9a\x99\x99\x99\x99\x99\xb9?"),
        (0.00001, b"\xf1h\xe3\x88\xb5\xf8\xe4>"),
        (0.3, b"333333\xd3?"),
        (1f64, b"\x00\x00\x00\x00\x00\x00\xf0?"),
        (1.0, b"\x00\x00\x00\x00\x00\x00\xf0?"),
        (10f64, b"\x00\x00\x00\x00\x00\x00$@"),
        (1e15, b"\x00\x004&\xf5k\x0cC"),
        (1.15e15, b"\x00\x80\xf7\xf5\xacW\x10C"),
        // (-0f64, b"\x00\x00\x00\x00\x00\x00\x00\x00"),  // FIXME: failing, what is the actual expected value?
        (-0.1, b"\x9a\x99\x99\x99\x99\x99\xb9\xbf"),
        (-0.10, b"\x9a\x99\x99\x99\x99\x99\xb9\xbf"),
        (-0.100, b"\x9a\x99\x99\x99\x99\x99\xb9\xbf"),
        (-0.00001, b"\xf1h\xe3\x88\xb5\xf8\xe4\xbe"),
        (-0.3, b"333333\xd3\xbf"),
        (-1f64, b"\x00\x00\x00\x00\x00\x00\xf0\xbf"),
        (-1.0, b"\x00\x00\x00\x00\x00\x00\xf0\xbf"),
        (-10f64, b"\x00\x00\x00\x00\x00\x00$\xc0"),
        (-1e15, b"\x00\x004&\xf5k\x0c\xc3"),
        (-1.15e15, b"\x00\x80\xf7\xf5\xacW\x10\xc3"),
    ];

    test_serialize(vals, AntelopeValue::Float64);
}

#[test]
fn test_u8() {
    let vals = [
        (0u8, b"\x00"),
        (1, b"\x01"),
        (254, b"\xFE"),
        (255, b"\xFF"),
    ];

    test_serialize(vals, AntelopeValue::Uint8);
}

#[test]
fn test_u16() {
    let vals = [
        (0u16, b"\x00\x00"),
        (1, b"\x01\x00"),
        (65534, b"\xFE\xFF"),
        (65535, b"\xFF\xFF"),
    ];

    test_serialize(vals, AntelopeValue::Uint16);
}

#[test]
fn test_u32() {
    let vals = [
        (0u32, b"\x00\x00\x00\x00"),
        (1, b"\x01\x00\x00\x00"),
        (10800, b"0*\x00\x00"),
        (10800, b"\x30\x2a\x00\x00"),
        (123456, b"@\xe2\x01\x00"),
        (4294967294, b"\xFE\xFF\xFF\xFF"),
        (4294967295, b"\xFF\xFF\xFF\xFF"),
    ];

    test_serialize(vals, AntelopeValue::Uint32);
}

#[test]
fn test_u64() {
    let vals = [
        (0u64, b"\x00\x00\x00\x00\x00\x00\x00\x00"),
        (1, b"\x01\x00\x00\x00\x00\x00\x00\x00"),
        (18446744073709551614, b"\xFE\xFF\xFF\xFF\xFF\xFF\xFF\xFF"),
        (18446744073709551615, b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF"),
    ];

    test_serialize(vals, AntelopeValue::Uint64);
}


#[test]
fn test_var_u32() {
    let vals: &[(u32, &[u8])] = &[
        (0, b"\x00"),
        (1, b"\x01"),
        (3, b"\x03"),
        (255, b"\xFF\x01"),
        (256, b"\x80\x02"),
        (4294967295, b"\xFF\xFF\xFF\xFF\x0F"),
    ];

    // test_serialize(vals, AntelopeType::VarUint32);
    let mut ds = ByteStream::new();

    for &(val, repr) in vals {
        ds.clear();
        ds.write_var_u32(val);
        assert_eq!(ds.data(), repr, "wrong ABI serialization for: {val}");
    }
}


#[test]
fn test_name() -> Result<()> {
    let vals = [
        (Name::from_str("a")?, b"\x00\x00\x00\x00\x00\x00\x000"),
        (Name::from_str("b")?, b"\x00\x00\x00\x00\x00\x00\x008"),
        (Name::from_str("zzzzzzzzzzzzj")?, b"\xff\xff\xff\xff\xff\xff\xff\xff"),
        (Name::from_str("kacjndfvdfa")?, b"\x00\xccJ{\xa5\xf9\x90\x81"),
        (Name::from_str("user2")?, b"\x00\x00\x00\x00\x00q\x15\xd6"),
        (Name::from_str("")?, b"\x00\x00\x00\x00\x00\x00\x00\x00"),
    ];

    test_serialize(vals, AntelopeValue::Name);
    Ok(())
}

#[test]
fn test_string() {
    let vals: &[(&str, &[u8])] = &[
        ("a", b"\x01a"),
        ("A", b"\x01A"),
        ("kcjansdcd", b"\tkcjansdcd"),
        ("", b"\x00"),
    ];

    let mut ds = ByteStream::new();

    for (val, repr) in vals {
        ds.clear();
        AntelopeValue::String(val.to_string()).to_bin(&mut ds);
        assert_eq!(ds.data(), *repr);
    }
}

#[test]
fn test_time_point_sec() -> Result<()> {
    fn dt(year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32) -> u32 {
        NaiveDate::from_ymd_opt(year, month, day).unwrap().and_hms_opt(hour, min, sec).unwrap().timestamp() as u32
    }

    let vals = [
        (dt(1970, 1, 1, 0, 0, 0), b"\x00\x00\x00\x00"),
        (dt(2040, 12, 31, 23, 59, 0), b"\x44\x03\x8D\x85"),
        (dt(2021, 8, 26, 14, 1, 47), b"\xCB\x9E\x27\x61"),
        (NaiveDate::from_ymd_opt(2021, 8, 26).unwrap().and_hms_micro_opt(14, 1, 47, 184549).unwrap().timestamp() as u32,  b"\xCB\x9E\x27\x61"),
    ];

    test_serialize(vals, AntelopeValue::TimePointSec);
    Ok(())
}

#[test]
fn test_time_point() -> Result<()> {
    fn dt(year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32, micro: u32) -> i64 {
        NaiveDate::from_ymd_opt(year, month, day).unwrap().and_hms_micro_opt(hour, min, sec, micro).unwrap().timestamp_micros()
    }

    let vals = [
        (dt(1970, 1, 1, 0, 0, 0, 0), b"\x00\x00\x00\x00\x00\x00\x00\x00"),
        (dt(1970, 1, 1, 0, 0, 0, 1000), b"\xe8\x03\x00\x00\x00\x00\x00\x00"),
        (dt(1970, 1, 1, 0, 0, 0, 2000), b"\xd0\x07\x00\x00\x00\x00\x00\x00"),
        (dt(1970, 1, 1, 0, 0, 0, 3000), b"\xb8\x0b\x00\x00\x00\x00\x00\x00"),
        (dt(1970, 1, 1, 0, 0, 1, 0), b"@B\x0f\x00\x00\x00\x00\x00"),
        (dt(2040, 12, 31, 23, 59, 0, 0), b"\x00Y\x14\xef\xd2\xf5\x07\x00"),
        (dt(2021, 8, 26, 14, 1, 47, 0), b"\xc0\x08\xbd\xcev\xca\x05\x00"),
    ];

    test_serialize(vals, AntelopeValue::TimePoint);
    Ok(())
}

#[test]
fn test_symbol() -> Result<()> {
    let vals = [
        // minimum amount of characters
        (Symbol::from_str("0,W")?, b"\x00W\x00\x00\x00\x00\x00\x00"),
        // maximum amount of characters
        (Symbol::from_str("0,WAXXXXX")?, b"\x00WAXXXXX"),
        // 1 precision
        (Symbol::from_str("1,WAX")?, b"\x01WAX\x00\x00\x00\x00"),
        // max precision
        (Symbol::from_str("16,WAX")?, b"\x10WAX\x00\x00\x00\x00"),
    ];

    test_serialize(vals, AntelopeValue::Symbol);
    Ok(())
}

#[test]
fn test_asset() -> Result<()> {
    // color_eyre::install()?;
    let vals = [
        (Asset::from_str("99.9 WAX")?, b"\xe7\x03\x00\x00\x00\x00\x00\x00\x01WAX\x00\x00\x00\x00"),
        (Asset::from_str("99 WAX")?, b"c\x00\x00\x00\x00\x00\x00\x00\x00WAX\x00\x00\x00\x00"),
    ];

    test_serialize(vals, AntelopeValue::Asset);
    Ok(())
}
