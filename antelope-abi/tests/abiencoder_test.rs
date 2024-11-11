use serde_json::json;

use antelope_abi::abidefinition::*;
use antelope_abi::{
    ABI, ByteStream,
};

// TODO: add tests for deserialization
// TODO: check more tests at: https://github.com/wharfkit/antelope/blob/master/test/serializer.ts



#[test]
fn test_serialize_array() {
    let a = ["foo", "bar", "baz"];
    let abi = ABI::new();
    let mut ds = ByteStream::new();

    abi.encode_variant(&mut ds, "string[]", &json!(a)).unwrap();
    assert_eq!(ds.hex_data().to_uppercase(), "0303666F6F036261720362617A");

    ds.clear();
    let v = vec!["foo", "bar", "baz"];
    abi.encode_variant(&mut ds, "string[]", &json!(v)).unwrap();
    assert_eq!(ds.hex_data().to_uppercase(), "0303666F6F036261720362617A");
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

    let abi = ABI::from_abi(&abi);
    let mut ds = ByteStream::new();
    abi.encode_variant(&mut ds, "bar", &obj).unwrap();

    assert_eq!(&ds.hex_data().to_uppercase(), "036F6E65020100000000000028CF01040166016F01750172");

    // FIXME: implement me!
    // let dec = abi.decode_variant(&ds, &"bar");
    // assert_eq!(dec.to_string(), r#"{"one":"one","two":2,"three":"two","four":["f","o","u","r"]}"#);
}
