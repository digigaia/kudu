use std::sync::Once;

use color_eyre::eyre::Result;
use serde_json::json;
use tracing::info;
use tracing_subscriber::{
    EnvFilter,
    // fmt::format::FmtSpan,
};

use antelope::{abidefinition::abi_schema, ABIDefinition};
use antelope::convert::{variant_to_int, variant_to_uint};

static TRACING_INIT: Once = Once::new();

fn init() {
    TRACING_INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            // .with_span_events(FmtSpan::ACTIVE)
            .init();
    });
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

    Ok(())
}
