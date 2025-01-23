use std::io;

use serde::Serialize;

use serde_json::Result;
use serde_json::ser::{Formatter, CompactFormatter, Serializer};

pub struct EOSFormatter {
    base: CompactFormatter,
}

/// JSON formatter with the following difference to `serde_json::Formatter`:
///  - `u128` and `i128` are implemented and are represented as strings (ie: double-quoted)
///  - although Antelope also quotes `u64` and `i64` types, we do not follow the same rule
///    here as otherwise all `serde_json::Value::Number` variants will also be quoted as they
///    are internally represented as `i64` (even though they might be used to represent
///    smaller sized types such as `i8`, `i16`, etc.)
///  - `f32` and `f64` never use scientific notation, and floats that have a fractional
///    part do not have a trailing ".0" (contrary to Antelope types)
impl EOSFormatter {
    fn new() -> Self {
        EOSFormatter { base: CompactFormatter {} }
    }
}

impl Formatter for EOSFormatter {
    // #[inline]
    // fn write_u64<W>(&mut self, writer: &mut W, value: u64) -> io::Result<()>
    // where
    //     W: ?Sized + io::Write,
    // {
    //     writer.write_all(b"\"")?;
    //     self.base.write_u64(writer, value)?;
    //     writer.write_all(b"\"")
    // }

    // #[inline]
    // fn write_i64<W>(&mut self, writer: &mut W, value: i64) -> io::Result<()>
    // where
    //     W: ?Sized + io::Write,
    // {
    //     writer.write_all(b"\"")?;
    //     self.base.write_i64(writer, value)?;
    //     writer.write_all(b"\"")
    // }

    #[inline]
    fn write_u128<W>(&mut self, writer: &mut W, value: u128) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b"\"")?;
        self.base.write_u128(writer, value)?;
        writer.write_all(b"\"")
    }

    #[inline]
    fn write_i128<W>(&mut self, writer: &mut W, value: i128) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b"\"")?;
        self.base.write_i128(writer, value)?;
        writer.write_all(b"\"")
    }

    #[inline]
    fn write_f32<W>(&mut self, writer: &mut W, value: f32) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        // use this instead of the default impl that uses Ryu in order to ensure
        // that we never use scientific notation
        write!(writer, "{}", value)
    }

    #[inline]
    fn write_f64<W>(&mut self, writer: &mut W, value: f64) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        // use this instead of the default impl that uses Ryu in order to ensure
        // that we never use scientific notation
        write!(writer, "{}", value)
    }

}


pub fn to_string<T>(value: &T) -> Result<String>
where
    T: ?Sized + Serialize,
{
    let fmt = EOSFormatter::new();
    let mut vec = Vec::with_capacity(128);
    let mut ser = Serializer::with_formatter(&mut vec, fmt);
    value.serialize(&mut ser)?;
    let string = unsafe {
        // We do not emit invalid UTF-8.
        String::from_utf8_unchecked(vec)
    };
    Ok(string)
}

pub use serde_json::from_str;
