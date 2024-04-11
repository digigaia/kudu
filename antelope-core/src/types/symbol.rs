use std::fmt;
use std::num::ParseIntError;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;


#[derive(Error, Debug)]
pub enum InvalidSymbol {
    #[error("creating Symbol from empty string")]
    Empty,

    #[error(r#"Symbol name longer than 7 characters: "{0}""#)]
    TooLong(String),

    #[error("missing comma in Symbol")]
    MissingComma,

    #[error(r#"invalid char '{1}' in Symbol "{0}""#)]
    InvalidChar(String, char),

    #[error("could not parse precision for Symbol")]
    ParsePrecisionError(#[from] ParseIntError),

    #[error("given precision {given} should be <= max precision {max}")]
    InvalidPrecision { given: u8, max: u8 },
}


#[derive(PartialEq, Debug, Copy, Clone)]
pub struct Symbol {
    value: u64,
}

impl Symbol {
    const MAX_PRECISION: u8 = 18;

    fn from_prec_and_str(precision: u8, name: &str) -> Result<Self, InvalidSymbol> {
        Ok(Self {
            value: string_to_symbol(precision, name.as_bytes())?,
        })
    }

    pub fn from_str(s: &str) -> Result<Self, InvalidSymbol> {
        let s = s.trim();
        if s.is_empty() { return Err(InvalidSymbol::Empty); }
        let pos = s.find(',').ok_or(InvalidSymbol::MissingComma)?;
        let precision: u8 = s[..pos].parse()?;
        if precision > Self::MAX_PRECISION {
            return Err(InvalidSymbol::InvalidPrecision {
                given: precision,
                max: Self::MAX_PRECISION,
            });
        }
        Self::from_prec_and_str(precision, &s[pos + 1..])
    }

    pub fn as_u64(&self) -> u64 { self.value }

    pub fn from_u64(n: u64) -> Self {
        // FIXME: do some validation here
        Self { value: n }
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

    // useless for now, this has been verified during construction with from_str
    // leaving it here though as this should be used if we provide a constructor from_u64
    pub fn is_valid_name(name: &str) -> bool {
        name.chars().all(|c| c.is_ascii_uppercase())
    }

    pub fn is_valid(&self) -> bool {
        self.decimals() <= Self::MAX_PRECISION && Self::is_valid_name(&self.name())
    }

}


impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{},{}", self.decimals(), self.name())
    }
}

impl Serialize for Symbol {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_string().serialize(serializer)
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

// see ref implementation in AntelopeIO/leap/libraries/chain/symbol.{hpp,cpp}
pub fn string_to_symbol_code(s: &[u8]) -> Result<u64, InvalidSymbol> {
    let mut result: u64 = 0;
    if s.is_empty() { return Err(InvalidSymbol::Empty); }

    let name = String::from_utf8(s.to_owned()).unwrap(); // unwrap should be safe here
    if s.len() > 7 { return Err(InvalidSymbol::TooLong(name)); }

    for (i, &c) in s.iter().enumerate() {
        if !c.is_ascii_uppercase() { return Err(InvalidSymbol::InvalidChar(name, c as char)); }
        result |= (s[i] as u64) << (8 * i);
    }
    Ok(result)
}

fn string_to_symbol(precision: u8, s: &[u8]) -> Result<u64, InvalidSymbol> {
    Ok(string_to_symbol_code(s)? << 8 | (precision as u64))
}

pub fn symbol_code_to_string(value: u64) -> String {
    let mut v: u64 = value;
    let mut result = String::new();
    while v != 0 {
        let c = (v & 0xFF) as u8;
        result.push(c as char);
        v >>= 8;
    }
    result
}


#[cfg(test)]
mod tests {
    use super::*;

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
