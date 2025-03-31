use std::fs;

use clap::Parser;
use color_eyre::Result;

use kudu::{ABI, ByteStream};

#[derive(Parser)]
struct Opts {
    #[arg(short, long)]
    abi: String,

    #[arg(short, long)]
    typename: String,

    #[arg(short='x', long)]
    hex: String,
}


pub fn main() -> Result<()> {
    let opts = Opts::parse();

    // read ABI from file
    let abi_str = fs::read_to_string(&opts.abi)
        .unwrap_or_else(|_| panic!("{}", &format!("File {} does not exist", opts.abi)));
    let abi = ABI::from_str(&abi_str)?;

    // create a byte stream from the given hex representation
    let mut bin = ByteStream::from_hex(opts.hex)?;

    // perform the hex->json conversion
    let v = abi.decode_variant(&mut bin, &opts.typename)?;

    println!("{}", v);

    Ok(())
}
