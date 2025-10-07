use std::fs;

use clap::{Parser, Subcommand};
use color_eyre::{Result, eyre::{eyre, OptionExt, WrapErr}};
use serde_json::Value;

use kudu::{ABI, ByteStream};

#[derive(Parser)]
#[command(
    name="kuduconv",
    version=kudu::config::VERSION,
    about="Utility to convert JSON to/from hex data according to an ABI",
    arg_required_else_help(true),
)]
struct Cli {

    #[command(subcommand)]
    command: Option<Commands>,
}


#[derive(Subcommand, Debug)]
enum Commands {

    /// Convert a JSON object to its hex representation
    ToHex {
        #[arg(short, long)]
        abi: Option<String>,

        typename: String,

        json: String,
    },

    /// Parse hex data as a JSON object
    FromHex {
        #[arg(short, long)]
        abi: Option<String>,

        typename: String,

        hex: String,
    },
}

/// Return an `ABI` object given its name or filename
fn get_abi(abi_name: Option<String>) -> Result<ABI> {
    // TODO: if abi_name is not specified, try to find the corresponding typename in our preloaded ABIs
    //       we will need to pass in the typename also to be able to do this
    // TODO: if abi_name is one of the preloaded abi names, use this
    // otherwise, try to open a file with the given name

    if abi_name.is_none() {
        return Err(eyre!("Did not specify an ABI. You need to specify one to be able to perform the conversion"))
    }
    let abi = abi_name.unwrap();  // safe unwrap

    // read ABI from file
    let abi_str = fs::read_to_string(&abi)
        .wrap_err_with(|| format!("Could not read file '{}'", &abi))?;

    Ok(ABI::from_str(&abi_str)?)
}

pub fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    let cmd = cli.command.ok_or_eyre("No command given. You need to specify at least one")?;

    match cmd {
        Commands::ToHex { abi, typename, json } => {
            let abi = get_abi(abi)?;

            // create a byte stream for storing the bin representation
            let mut ds = ByteStream::new();

            // perform the json->hex conversion
            let v: Value = json.parse()?;
            abi.encode_variant(&mut ds, &typename,  &v)?;

            println!("{}", ds.hex_data());
        }

        Commands::FromHex { abi, typename, hex } => {
            let abi = get_abi(abi)?;

            // create a byte stream from the given hex representation
            let mut bin = ByteStream::from_hex(&hex)?;

            // perform the hex->json conversion
            let v = abi.decode_variant(&mut bin, &typename)?;

            if !bin.leftover().is_empty() {
                return Err(eyre!("Trailing input, {} bytes haven't been consumed. Decoded object: {:?}",
                                 bin.leftover().len(), &v));
            }

            println!("{}", v);
        }

    }

    Ok(())
}
