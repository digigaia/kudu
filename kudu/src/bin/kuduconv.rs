use std::fs;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use color_eyre::{Result, eyre::{eyre, OptionExt, WrapErr}};
use serde_json::Value;

use kudu::{abi, ByteStream, ABI};


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
        /// the name of a preloaded ABI or the filename of an ABI to load.
        /// If not specified, will try to automatically find a matching ABI for the given typename
        #[arg(short, long)]
        abi: Option<String>,

        /// the typename of the object to convert
        typename: String,

        /// a JSON representation of the object to convert
        json: String,
    },

    /// Decode hex data as a JSON object
    FromHex {
        /// the name of a preloaded ABI or a filename of an ABI to load
        #[arg(short, long)]
        abi: Option<String>,

        /// the typename of the object to convert
        typename: String,

        /// an hex representation of the object we want to decode
        hex: String,
    },
}

/// Return an `ABI` object given its name or filename
fn get_abi(abi_name: Option<String>, typename: &str) -> Result<Arc<ABI>> {
    // TODO: if abi_name is not specified,
    //       we will need to pass in the typename also to be able to do this
    // TODO: if abi_name is one of the preloaded abi names, use this
    // otherwise, try to open a file with the given name

    // if abi_name is not specified, try to find the corresponding typename in our preloaded ABIs
    if abi_name.is_none() {
        // we didn't specify an ABI file, try to look into our registry if we have an ABI
        // that knows about the type we want to convert
        return abi::registry::find_abi_for(typename).
            wrap_err("Did not specify an ABI, nor is there one preloaded that matches the given typename");
    }
    let abi_name = abi_name.unwrap();  // safe unwrap

    // if abi_name is the name of a preloaded ABI, use it
    if let Ok(abi) = abi::registry::get_abi(&abi_name) {
        return Ok(abi);
    }

    // otherwise, read ABI from file
    let abi_str = fs::read_to_string(&abi_name)
        .wrap_err_with(|| format!("Could not read file '{}'", &abi_name))?;

    Ok(Arc::new(ABI::from_str(&abi_str)?))
}

pub fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    let cmd = cli.command.ok_or_eyre("No command given. You need to specify at least one")?;

    match cmd {
        Commands::ToHex { abi, typename, json } => {
            let abi = get_abi(abi, &typename)?;

            // create a byte stream for storing the bin representation
            let mut ds = ByteStream::new();

            // perform the json->hex conversion
            let v: Value = json.parse()?;
            abi.encode_variant(&mut ds, &typename,  &v)?;

            println!("{}", ds.hex_data());
        }

        Commands::FromHex { abi, typename, hex } => {
            let abi = get_abi(abi, &typename)?;

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
