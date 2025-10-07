use std::fs;

use clap::Parser;
use color_eyre::{Result, eyre::WrapErr};

use kudu::{ABI, ByteStream};

#[derive(Parser)]
#[command(
    name="kudu_hex_to_json",
    version=kudu::config::VERSION,
    about="Utility to decode hex data into a corresponding JSON type according to an ABI",
    arg_required_else_help(true),
)]
struct Opts {
    #[arg(short, long)]
    abi: String,

    #[arg(short, long)]
    typename: String,

    #[arg(short='x', long)]
    hex: String,
}

pub fn main() -> Result<()> {
    color_eyre::install()?;

    let opts = Opts::parse();

    // read ABI from file
    let abi_str = fs::read_to_string(&opts.abi)
        .wrap_err_with(|| format!("Could not read file '{}'", &opts.abi))?;

    let abi = ABI::from_str(&abi_str)?;

    // create a byte stream from the given hex representation
    let mut bin = ByteStream::from_hex(opts.hex)?;

    // perform the hex->json conversion
    let v = abi.decode_variant(&mut bin, &opts.typename)?;

    // FIXME: error if data stream is not empty

    println!("{}", v);

    Ok(())
}
