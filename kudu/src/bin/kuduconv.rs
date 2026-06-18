// SPDX-FileCopyrightText: 2025, 2026 DigiGaia SCCL
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fs;
use std::path::Path;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use color_eyre::{Result, eyre::{eyre, OptionExt, WrapErr}};
use serde_json::Value;

use kudu::{abi, tracing_init, Bytes, ABI};


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

/// Return an `ABI` object given its name or filename. If none is given, try to find one that
/// can handle the given typename in the ABIs preloaded in the registry.
fn get_abi(abi_name: Option<String>, typename: &str) -> Result<Arc<ABI>> {
    if let Some(abi_name) = abi_name {
        // if abi_name is an existing file, load it
        // do this first to avoid pre-loading ABIs in the registry if that is not needed
        if Path::new(&abi_name).is_file() {
            let abi_str = fs::read_to_string(&abi_name).unwrap();  // safe unwrap
            return Ok(Arc::new(ABI::from_str(&abi_str)?))
        }

        // if it isn't a file, try to look for a pre-loaded ABI in our registry with that name
        if let Ok(abi) = abi::registry::get_abi(&abi_name) {
            return Ok(abi);
        }

        Err(eyre!("Could not find file or ABI with name: {}", abi_name))
    }
    else {
        // we didn't specify an ABI file, try to look in our registry if we have an ABI
        // that knows about the type we want to convert
        abi::registry::find_abi_for(typename).
            wrap_err("Did not specify an ABI and there is none preloaded that matches the given typename")
    }
}

pub fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_init();

    let cli = Cli::parse();

    let cmd = cli.command.ok_or_eyre("No command given. You need to specify at least one")?;

    match cmd {
        Commands::ToHex { abi, typename, json } => {
            let abi = get_abi(abi, &typename)?;

            // create a byte stream for storing the bin representation
            let mut ds = Bytes::new();

            // perform the json->hex conversion
            let v: Value = json.parse()?;
            abi.encode_variant(&mut ds, &typename,  &v)?;

            println!("{}", ds.to_hex());
        }

        Commands::FromHex { abi, typename, hex } => {
            let abi = get_abi(abi, &typename)?;

            // create a byte stream from the given hex representation
            let bin = Bytes::from_hex(&hex)?;
            let mut view = bin.view();

            // perform the hex->json conversion
            let v = abi.decode_variant(&mut view, &typename)?;

            if !view.leftover().is_empty() {
                return Err(eyre!("Trailing input, {} bytes haven't been consumed. Decoded object: {:?}",
                                 view.leftover().len(), &v));
            }

            println!("{}", v);
        }

    }

    Ok(())
}
