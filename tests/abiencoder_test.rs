use std::fmt::Display;

use serde_json::json;
// use anyhow::Result;
use color_eyre::eyre::Result;

use antelope::abi::*;
use antelope::{
    ABIEncoder, ByteStream,
    types::{
        AntelopeType, Name, Symbol, Asset
    }
};

// TODO: add tests for deserialization
// TODO: check more tests at: https://github.com/wharfkit/antelope/blob/master/test/serializer.ts


#[test]
fn test_serialize_ints() {
    let i1 = AntelopeType::Uint64(5);
    let i2 = AntelopeType::Int64(-23);

    let abi = ABIEncoder::new();
    let mut ds = ByteStream::new();

    abi.encode(&mut ds, &i1);
    assert_eq!(ds.hex_data(), "0500000000000000");

    ds.clear();
    abi.encode(&mut ds, &i2);
    assert_eq!(ds.hex_data(), "e9ffffffffffffff");
}

#[test]
fn test_serialize_string() {
    let abi = ABIEncoder::new();
    let mut ds = ByteStream::new();

    abi.encode(&mut ds, &AntelopeType::String("foo".to_owned()));
    assert_eq!(ds.hex_data(), "03666f6f");

    ds.clear();
    abi.encode(&mut ds, &AntelopeType::String("Hello world!".to_owned()));
    assert_eq!(ds.hex_data(), "0c48656c6c6f20776f726c6421");
}

#[test]
fn test_serialize_array() {
    let a = ["foo", "bar", "baz"];
    let abi = ABIEncoder::new();
    let mut ds = ByteStream::new();

    abi.encode_variant(&mut ds, "string[]", &json!(a)).unwrap();
    assert_eq!(ds.hex_data(), "0303666f6f036261720362617a");

    ds.clear();
    let v = vec!["foo", "bar", "baz"];
    abi.encode_variant(&mut ds, "string[]", &json!(v)).unwrap();
    assert_eq!(ds.hex_data(), "0303666f6f036261720362617a");
}


#[test]
fn test_serialize_name() {
    let data = "000000005c73285d";
    let obj = Name::from_str("foobar").unwrap();
    let json = r#""foobar""#;

    let abi = ABIEncoder::new();
    let mut ds = ByteStream::new();
    abi.encode(&mut ds, &AntelopeType::Name(obj.clone()));

    assert_eq!(obj.to_u64(), 6712742083569909760);

    assert_eq!(&ds.hex_data(), &data);

    assert_eq!(serde_json::from_str::<Name>(&json).unwrap(), obj);
    assert_eq!(serde_json::to_string(&obj).unwrap(), json);
}

#[test]
fn test_serialize_symbol() {
    let data = "04464f4f00000000";
    let obj = Symbol::from_str("4,FOO").unwrap();
    let json = r#""4,FOO""#;

    let abi = ABIEncoder::new();
    let mut ds = ByteStream::new();
    abi.encode(&mut ds, &AntelopeType::Symbol(obj.clone()));

    assert_eq!(obj.decimals(), 4);
    assert_eq!(obj.name(), "FOO");

    assert_eq!(&ds.hex_data(), &data);

    assert_eq!(serde_json::from_str::<Symbol>(&json).unwrap(), obj);
    assert_eq!(serde_json::to_string(&obj).unwrap(), json);
}

#[test]
fn test_serialize_asset() {
    let data = "393000000000000004464f4f00000000";
    let obj = Asset::from_str("1.2345 FOO").unwrap();
    let json = r#""1.2345 FOO""#;

    let abi = ABIEncoder::new();
    let mut ds = ByteStream::new();
    abi.encode(&mut ds, &AntelopeType::Asset(obj.clone()));

    assert_eq!(obj.amount(), 12345);
    assert_eq!(obj.decimals(), 4);
    assert_eq!(obj.precision(), 10000);

    assert_eq!(&ds.hex_data(), &data);

    assert_eq!(serde_json::from_str::<Asset>(&json).unwrap(), obj);
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
    abi.encode_variant(&mut ds, &"bar", &obj).unwrap();

    assert_eq!(&ds.hex_data(), "036f6e65020100000000000028cf01040166016f01750172");

    // FIXME: implement me!
    // let dec = abi.decode_variant(&ds, &"bar");
    // assert_eq!(dec.to_string(), r#"{"one":"one","two":2,"three":"two","four":["f","o","u","r"]}"#);
}


fn test_serialize<T, const N: usize, const M: usize, F>(vals: [(T, &[u8; N]); M], convert: F)
where
    T: Display + Clone,
    F: Fn(T) -> AntelopeType,
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

    test_serialize(vals, AntelopeType::Bool);
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

    test_serialize(vals, AntelopeType::Int8);
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

    test_serialize(vals, AntelopeType::Int16);
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

    test_serialize(vals, AntelopeType::Int32);
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

    test_serialize(vals, AntelopeType::Int64);
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

    test_serialize(vals, AntelopeType::Float32);
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

    test_serialize(vals, AntelopeType::Float64);
}

#[test]
fn test_u8() {
    let vals = [
        (0u8, b"\x00"),
        (1, b"\x01"),
        (254, b"\xFE"),
        (255, b"\xFF"),
    ];

    test_serialize(vals, AntelopeType::Uint8);
}

#[test]
fn test_u16() {
    let vals = [
        (0u16, b"\x00\x00"),
        (1, b"\x01\x00"),
        (65534, b"\xFE\xFF"),
        (65535, b"\xFF\xFF"),
    ];

    test_serialize(vals, AntelopeType::Uint16);
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

    test_serialize(vals, AntelopeType::Uint32);
}

#[test]
fn test_u64() {
    let vals = [
        (0u64, b"\x00\x00\x00\x00\x00\x00\x00\x00"),
        (1, b"\x01\x00\x00\x00\x00\x00\x00\x00"),
        (18446744073709551614, b"\xFE\xFF\xFF\xFF\xFF\xFF\xFF\xFF"),
        (18446744073709551615, b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF"),
    ];

    test_serialize(vals, AntelopeType::Uint64);
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

    test_serialize(vals, AntelopeType::Name);
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
        AntelopeType::String(val.to_string()).to_bin(&mut ds);
        assert_eq!(ds.data(), *repr);
    }
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

    test_serialize(vals, AntelopeType::Symbol);
    Ok(())
}

#[test]
fn test_asset() -> Result<()> {
    // color_eyre::install()?;
    let vals = [
        (Asset::from_str("99.9 WAX")?, b"\xe7\x03\x00\x00\x00\x00\x00\x00\x01WAX\x00\x00\x00\x00"),
        (Asset::from_str("99 WAX")?, b"c\x00\x00\x00\x00\x00\x00\x00\x00WAX\x00\x00\x00\x00"),
    ];

    test_serialize(vals, AntelopeType::Asset);
    Ok(())
}

// TODO: do the other tests from here: https://github.com/FACINGS/pyntelope/blob/main/tests/unit/types_test.py
// missing for now:
//  - UnixTimestamp
//  - TimePoint


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

static _TOKEN_HEX_ABI: &str = concat!(
    "0e656f73696f3a3a6162692f312e30  01 0c6163636f756e745f6e616d65 046e61",
    "6d65 05 087472616e73666572 00 04 0466726f6d 0c6163636f756e745f6e616d65",
    "02746f 0c6163636f756e745f6e616d65 087175616e74697479 056173736574 04",
    "6d656d6f 06737472696e67 06637265617465 00 02 06697373756572 0c6163636f",
    "756e745f6e616d65 0e6d6178696d756d5f737570706c79 056173736574 056973",
    "737565 00 03 02746f 0c6163636f756e745f6e616d65 087175616e74697479 0561",
    "73736574 046d656d6f 06737472696e67076163636f756e7400010762616c616e",
    "63650561737365740e63757272656e63795f7374617473000306737570706c79",
    "0561737365740a6d61785f737570706c79056173736574066973737565720c61",
    "63636f756e745f6e616d6503000000572d3ccdcd087472616e73666572000000",
    "000000a531760569737375650000000000a86cd4450663726561746500020000",
    "00384f4d113203693634010863757272656e6379010675696e74363407616363",
    "6f756e740000000000904dc603693634010863757272656e6379010675696e74",
    "36340e63757272656e63795f7374617473000000");

static _TRANSACTION_ABI: &str = r#"{
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

static _PACKED_TRANSACTION_ABI: &str = r#"{
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
    let l = env_logger::builder().is_test(true).try_init();
    eprintln!("{:?}", l);
}

#[test]
fn integration_test() -> Result<()> {
    init();

    let test_abi_def = ABIDefinition::from_str(TEST_ABI);
    let test_abi = ABIEncoder::from_abi(&test_abi_def);

    let token_abi = ABIEncoder::from_hex_abi(TOKEN_HEX_ABI);


    assert!(false);
    Ok(())
}
