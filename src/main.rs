// pub mod abi;
// pub mod base64u;
// pub mod types;

// use abi;
use antelope::types::*;
use antelope::base64u;

fn main() {
    println!("Hello, world! {}", base64u::encode(b""));

    let t = AntelopeType::Bool(true);

    println!("this is a bool: {:?}", t.to_variant());

    let t2 = AntelopeType::Int16(23);
    println!("this is a number: {:?}", t2.to_variant());

    /*
    let bool_type = abi::Type {
        new_type_name: "bool".into(),
        type_: "bool".into(),
    };

    let abi = abi::ABI {
        version: "1.1".into(),
        types: vec![bool_type],
        ..
    };

    let j = serde_json::to_string(&abi).unwrap();
    println!("{}", j);
    */
}
