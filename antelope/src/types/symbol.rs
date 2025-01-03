use std::fmt;
use std::num::ParseIntError;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use snafu::{ensure, Snafu, ResultExt, OptionExt};


#[derive(Debug, Snafu)]
pub enum InvalidSymbol {
    #[snafu(display("creating symbol from empty string"))]
    Empty,

    #[snafu(display(r#"symbol name longer than 7 characters: "{name}""#))]
    TooLong { name: String },

    #[snafu(display("missing comma in symbol"))]
    MissingComma,

    #[snafu(display(r#"invalid char '{c}' in symbol "{symbol}""#))]
    CharError { symbol: String, c: char },

    #[snafu(display("could not parse precision for symbol"))]
    ParsePrecisionError { source: ParseIntError },

    #[snafu(display("given precision {given} should be <= max precision {max}"))]
    PrecisionError { given: u8, max: u8 },

    #[snafu(display("invalid u64 representation: {value} cannot be turned into a valid symbol"))]
    InvalidU64Representation { value: u64 },
}


#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SymbolCode(u64);

impl SymbolCode {
    pub fn from_u64(n: u64) -> SymbolCode {
        SymbolCode(n)
    }

    pub fn as_u64(&self) -> u64 { self.0 }

    pub fn from_str(s: &str) -> Result<SymbolCode, InvalidSymbol> {
        string_to_symbol_code(s).map(SymbolCode)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Symbol {
    value: u64,
}

impl Symbol {
    const MAX_PRECISION: u8 = 18;

    fn from_prec_and_str(precision: u8, name: &str) -> Result<Self, InvalidSymbol> {
        Ok(Self {
            value: string_to_symbol(precision, name)?,
        })
    }

    pub fn from_str(s: &str) -> Result<Self, InvalidSymbol> {
        let s = s.trim();
        ensure!(!s.is_empty(), EmptySnafu);
        let pos = s.find(',').context(MissingCommaSnafu)?;
        let precision: u8 = s[..pos].parse().context(ParsePrecisionSnafu)?;
        ensure!(precision <= Self::MAX_PRECISION,
                PrecisionSnafu { given: precision, max: Self::MAX_PRECISION });
        Self::from_prec_and_str(precision, &s[pos + 1..])
    }

    pub fn as_u64(&self) -> u64 { self.value }

    pub fn from_u64(n: u64) -> Result<Self, InvalidSymbol> {
        let result =  Self { value: n };
        ensure!(result.is_valid(), InvalidU64RepresentationSnafu { value: n });
        Ok(result)
    }

    pub fn decimals(&self) -> u8 {
        (self.value & 0xFF) as u8
    }

    pub fn precision(&self) -> i64 {
        let decimals = self.decimals();
        assert!(decimals <= Self::MAX_PRECISION,
                "precision {} should be <= {}", decimals, Self::MAX_PRECISION);
        let mut p10: i64 = 1;
        let mut p = decimals as i64;
        while p > 0 {
            p10 *= 10;
            p -= 1;
        }
        p10
    }

    pub fn name(&self) -> String {
        symbol_code_to_string(self.value >> 8)
    }

    pub fn is_valid(&self) -> bool {
        self.decimals() <= Self::MAX_PRECISION && is_valid_symbol_name(&self.name())
    }

}

// -----------------------------------------------------------------------------
//     helper functions
// -----------------------------------------------------------------------------

// see ref implementation in AntelopeIO/spring/libraries/chain/symbol.{hpp,cpp}


// FIXME: inline these into `SymbolCode` methods
fn string_to_symbol_code(s: &str) -> Result<u64, InvalidSymbol> {
    let mut result: u64 = 0;
    ensure!(!s.is_empty(), EmptySnafu);

    // let name = std::str::from_utf8(s).unwrap(); // unwrap should be safe here
    let name = s;
    ensure!(s.len() <= 7, TooLongSnafu { name });

    for (i, &c) in s.as_bytes().iter().enumerate() {
        ensure!(c.is_ascii_uppercase(), CharSnafu { symbol: name, c: c as char });
        result |= (c as u64) << (8 * i);
    }
    Ok(result)
}

fn symbol_code_to_string(value: u64) -> String {
    let mut v: u64 = value;
    let mut result = String::with_capacity(7);
    while v != 0 {
        let c = (v & 0xFF) as u8;
        result.push(c as char);
        v >>= 8;
    }
    result
}

fn string_to_symbol(precision: u8, s: &str) -> Result<u64, InvalidSymbol> {
    ensure!(precision <= Symbol::MAX_PRECISION,
            PrecisionSnafu { given: precision, max: Symbol::MAX_PRECISION });
    Ok(string_to_symbol_code(s)? << 8 | (precision as u64))
}

fn is_valid_symbol_name(name: &str) -> bool {
    name.chars().all(|c| c.is_ascii_uppercase())
}


// -----------------------------------------------------------------------------
//     `Display` implementation
// -----------------------------------------------------------------------------

impl fmt::Display for SymbolCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", symbol_code_to_string(self.0))
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{},{}", self.decimals(), self.name())
    }
}

// -----------------------------------------------------------------------------
//     `Serde` traits implementation
// -----------------------------------------------------------------------------

impl Serialize for SymbolCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        if serializer.is_human_readable() {
            self.to_string().serialize(serializer)
        }
        else {
            self.as_u64().serialize(serializer)
        }
    }
}

impl Serialize for Symbol {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            self.to_string().serialize(serializer)
        }
        else {
            self.as_u64().serialize(serializer)
        }
    }
}

struct SymbolVisitor;

impl<'de> Visitor<'de> for SymbolVisitor {
    type Value = Symbol;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a string that is a valid EOS symbol")
    }

    fn visit_str<E>(self, s: &str) -> Result<Symbol, E>
    where
        E: de::Error,
    {
        Symbol::from_str(s).map_err(|e| de::Error::custom(e.to_string()))
    }
}
impl<'de> Deserialize<'de> for Symbol {
    fn deserialize<D>(deserializer: D) -> Result<Symbol, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(SymbolVisitor)
    }
}


// =============================================================================
//
//     Unittests
//
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_functionality() {
        let obj = Symbol::from_str("4,FOO").unwrap();
        let json = r#""4,FOO""#;

        assert_eq!(obj.decimals(), 4);
        assert_eq!(obj.name(), "FOO");

        assert_eq!(serde_json::from_str::<Symbol>(json).unwrap(), obj);
        assert_eq!(serde_json::to_string(&obj).unwrap(), json);
    }

    #[test]
    fn invalid_symbols() {
        let symbols = [
            "0,WAXXXXXX",
            "0,",
            "0, ",
            ",",
            "19,WAX",
            "-1,WAX",
        ];

        for s in symbols {
            assert!(Symbol::from_str(s).is_err());
        }
    }
}
