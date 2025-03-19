use kudu_macros::with_location;
use color_eyre::Result;
use snafu::prelude::*;

#[with_location]
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum MyError {
    #[snafu(display(r#""{msg}""#))]
    Invalid {
        msg: String,
    },

    #[snafu(display("unsupported ESR protocol version: {version}"))]
    InvalidVersion {
        version: u8,
    },

    #[snafu(visibility(pub), display("{message}"))]
    Whatever {
        message: String,
    },
}

pub fn test_fail() -> Result<()> {
    InvalidSnafu { msg: "oops" }.fail()?
}

fn main() -> Result<()> {
    println!("hello world");

    WhateverSnafu { message: "yep" }.fail()?;
    test_fail()?;

    Ok(())
}
