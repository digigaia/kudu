use std::fs;

use clap::Parser;
use color_eyre::Result;
use serde_json::Value;

use kudu::{ABI, ByteStream};

#[derive(Parser)]
struct Opts {
    #[arg(short, long)]
    abi: String,

    #[arg(short, long)]
    typename: String,

    #[arg(short, long)]
    json: String,
}


pub fn main() -> Result<()> {
    let opts = Opts::parse();

    // read ABI from file
    let abi_str = fs::read_to_string(&opts.abi)
        .unwrap_or_else(|_| panic!("{}", &format!("File {} does not exist", opts.abi)));
    let abi = ABI::from_str(&abi_str)?;

    // create a byte stream for storing the bin representation
    let mut ds = ByteStream::new();

    // perform the json->hex conversion
    let v: Value = opts.json.parse()?;
    abi.encode_variant(&mut ds, &opts.typename,  &v)?;

    println!("{}", ds.hex_data());

    Ok(())
}
