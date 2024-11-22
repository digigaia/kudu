use serde_json::json;
use antelope_abi::{abidefinition::abi_schema, ABIDefinition};
use antelope_core::convert::{variant_to_int, variant_to_uint};

fn main() {
    println!("hello");

    let schema = abi_schema();
    let j = json!(schema);
    // println!("{:#}", j);

    let schema2 = ABIDefinition::from_str(&j.to_string()).unwrap();

    assert!(*schema == schema2);

    let n = u128::from_str_radix("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF", 16);
    println!("{:?}", n.unwrap() as i128);

    let n = variant_to_uint::<u128>(&json!("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF"));
    println!("{n:?}");

    let n = variant_to_int::<i128>(&json!("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF"));
    println!("{n:?}");


    let i = i16::from_str_radix("7FFF", 16);
    println!("{i:?}");


}
