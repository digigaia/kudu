use annotated_error::with_location;
use color_eyre::Result;
use snafu::prelude::*;

#[with_location]
#[derive(Debug, Snafu)]
pub enum MyError {
    #[snafu(display("{msg}"))]
    Invalid {
        msg: String,
    },

    #[snafu(display("unsupported ESR protocol version: {version}"))]
    InvalidVersion {
        version: u8,
    },
}

pub fn test_fail() -> Result<()> {
    InvalidSnafu { msg: "oops" }.fail()?
}

fn main() -> Result<()> {
    println!("hello world");

    test_fail()?;

    Ok(())
}
