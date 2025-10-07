use std::fs;

use clap::Parser;
use color_eyre::{Result, eyre::WrapErr};
use serde_json::Value;

use kudu::{ABI, ByteStream};

// TODO: rethink clap args: abi can be optional (and then we try to be smart), can also
//       have a few preloaded ones (so we don't have to specify a file)
//       typename and json are both required args (instead of options)

#[derive(Parser)]
#[command(
    name="kudu_json_to_hex",
    version=kudu::config::VERSION,
    about="Utility to encode a JSON type into hex according to an ABI",
    arg_required_else_help(true),
)]
struct Opts {
    #[arg(short, long)]
    abi: String,

    #[arg(short, long)]
    typename: String,

    #[arg(short, long)]
    json: String,
}


pub fn main() -> Result<()> {
    color_eyre::install()?;

    let opts = Opts::parse();

    // read ABI from file
    let abi_str = fs::read_to_string(&opts.abi)
        .wrap_err_with(|| format!("Could not read file '{}'", &opts.abi))?;

    let abi = ABI::from_str(&abi_str)?;

    // create a byte stream for storing the bin representation
    let mut ds = ByteStream::new();

    // perform the json->hex conversion
    let v: Value = opts.json.parse()?;
    abi.encode_variant(&mut ds, &opts.typename,  &v)?;

    println!("{}", ds.hex_data());

    Ok(())
}
