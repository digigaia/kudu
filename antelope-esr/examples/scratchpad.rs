#![cfg_attr(feature = "float128", feature(f128))]

use std::sync::Once;

use color_eyre::eyre::Result;
use serde_json::json;
use tracing::info;
use tracing_subscriber::{
    EnvFilter,
    // fmt::format::FmtSpan,
};

use antelope::{
    ABIDefinition, TimePoint, TimePointSec, Name, Contract,
    abi::abi_schema,
    convert::{variant_to_int, variant_to_uint}
};

use antelope::{BinarySerializable, SerializeEnum, contract};


static TRACING_INIT: Once = Once::new();

fn init() {
    TRACING_INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            // .with_span_events(FmtSpan::ACTIVE)
            .init();
    });
}


#[derive(SerializeEnum)]
pub enum ChainId {
    ChainAlias(u8),
    #[serde(rename="chainid")]
    ChainId(String), // AntelopeValue::Checksum256 variant assumed
}

#[derive(Clone, BinarySerializable)]
#[contract(account="eosio.token", name="transfer")]
pub struct Transfer {
    pub from: Name,
    pub to: Name,
    pub memo: String,
}


fn main() -> Result<()> {
    init();
    info!("░▒▓ WELCOME TO SCRATCHPAD ▓▒░");

    let schema = abi_schema();
    let j = json!(schema);
    // println!("{:#}", j);

    let schema2 = ABIDefinition::from_str(&j.to_string()).unwrap();

    assert!(*schema == schema2);

    let n = u128::from_str_radix("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF", 16);
    info!("{:?}", n.unwrap() as i128);

    let n = variant_to_uint::<u128>(&json!("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF"));
    info!("{n:?}");

    let n = variant_to_int::<i128>(&json!("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF"));
    info!("{n:?}");


    let i = i16::from_str_radix("7FFF", 16);
    info!("{i:?}");

    let n = 1i32;
    info!("{}", serde_json::to_string(&n)?);
    info!("{}", antelope::json::to_string(&n)?);

    let n = json!(1);
    info!("{}", serde_json::to_string(&n)?);
    info!("{}", antelope::json::to_string(&n)?);

    let n = 1i128;
    info!("{}", serde_json::to_string(&n)?);
    info!("{}", antelope::json::to_string(&n)?);

    // let n: i64 = serde_json::from_str("170141183460469231731687303715884105727")?;

    // let n: i128 = serde_json::from_str("170141183460469231731687303715884105727")?;
    // println!("{}", n);

    let t1 = TimePointSec::new(2018, 6, 15, 19, 17, 47).unwrap();
    info!("t = {}", t1);

    let tp = |y, m, d, h, mm, s, milli| { TimePoint::new(y, m, d, h, mm, s, milli).unwrap() };
    let t2 = tp(2000, 12, 31, 23, 59, 59, 999);
    info!("t2 = {}", t2);

    let cid = ChainId::ChainAlias(23);
    info!("{}", &serde_json::to_string(&cid)?);

    let cid = ChainId::ChainId("hello".to_string());
    info!("{}", &serde_json::to_string(&cid)?);

    assert_eq!(Transfer::account().to_string(), "eosio.token");
    assert_eq!(Transfer::name().to_string(), "transfer");

    Ok(())
}
