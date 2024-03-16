use std::fmt;
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde::de::{self, Visitor};
use thiserror::Error;
use std::num::ParseIntError;

use crate::{Symbol, InvalidSymbol};


#[derive(Error, Debug)]
pub enum InvalidAsset {
    #[error(r#"Asset's amount and symbol should be separated with space: "{0}""#)]
    MissingSpace(String),

    #[error("missing decimal fraction after decimal point")]
    MissingDecimal,

    #[error("could not parse amount for Asset")]
    ParseAmount(#[from] ParseIntError),

    #[error("amount overflow for: {0}")]
    AmountOverflow(String),

    #[error("ammount out of range, max is 2^62-1")]
    AmountOutOfRange,

    #[error("could not parse Symbol from Asset string")]
    InvalidSymbol(#[from] InvalidSymbol),
}


#[derive(PartialEq, Debug, Copy, Clone)]
pub struct Asset {
    amount: i64,
    symbol: Symbol,
}

impl Asset {
    const MAX_AMOUNT: i64 = (1 << 62) - 1;

    pub fn new(amount: i64, symbol: Symbol) -> Asset {
        Asset { amount, symbol }
    }

    fn is_amount_within_range(&self) -> bool {
        -Self::MAX_AMOUNT <= self.amount && self.amount <= Self::MAX_AMOUNT
    }

    pub fn is_valid(&self) -> bool {
        self.is_amount_within_range() && self.symbol.is_valid()
    }

    pub fn check_valid(&self) -> Result<(), InvalidAsset> {
        match self.is_amount_within_range() {
            true => Ok(()),
            false => Err(InvalidAsset::AmountOutOfRange),
        }
        // no need to check for symbol.is_valid, it has been successfully constructed
    }

    pub fn amount(&self) -> i64 { self.amount }
    pub fn symbol(&self) -> Symbol { self.symbol }
    pub fn symbol_name(&self) -> String { self.symbol.name() }
    pub fn decimals(&self) -> u8 { self.symbol.decimals() }
    pub fn precision(&self) -> i64 { self.symbol.precision() }

    pub fn to_real(&self) -> f64 {
        self.amount as f64 / self.precision() as f64
    }

    pub fn from_str(s: &str) -> Result<Self, InvalidAsset> {
        let s = s.trim();

        // find space in order to split amount and symbol
        let space_pos = s.find(' ').ok_or(InvalidAsset::MissingSpace(s.to_owned()))?;

        let amount_str = &s[..space_pos];
        let symbol_str = &s[space_pos+1..].trim();

        // parse symbol
        let dot_pos = amount_str.find('.');
        let precision;
        if let Some(dot_pos) = dot_pos {
            // Ensure that if decimal point is used (.), decimal fraction is specified
            if dot_pos == amount_str.len()-1 { return Err(InvalidAsset::MissingDecimal); }
            precision = amount_str.len() - dot_pos - 1;
        } else {
            precision = 0;
        }

        let symbol = Symbol::from_str(&format!("{},{}", precision, symbol_str))?;

        // parse amount
        let amount: i64 = match dot_pos {
            None => amount_str.parse()?,
            Some(dot_pos) => {
                let int_part: i64 = amount_str[..dot_pos].parse()?;
                let mut frac_part: i64 = amount_str[dot_pos+1..].parse()?;
                if amount_str.starts_with('-') { frac_part *= -1; }
                // check we don't overflow
                int_part
                    .checked_mul(symbol.precision()).ok_or(InvalidAsset::AmountOverflow(amount_str.to_owned()))?
                    .checked_add(frac_part).ok_or(InvalidAsset::AmountOverflow(amount_str.to_owned()))?
            },
        };

        Ok(Self {
            amount,
            symbol,
        })
    }

}


impl fmt::Display for Asset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.amount < 0 { "-" } else { "" };
        let abs_amount: i64 = self.amount.abs();
        let mut result = (abs_amount / self.precision()).to_string();
        if self.decimals() != 0 {
            let frac: i64 = abs_amount % self.precision();
            result.push('.');
            result.push_str(&(self.precision() + frac).to_string()[1..]); // ensure we have the right number of leading zeros
        }

        write!(f, "{}{} {}", sign, result, self.symbol_name())
    }
}


impl Serialize for Asset {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

struct AssetVisitor;

impl<'de> Visitor<'de> for AssetVisitor {
    type Value = Asset;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a string that is a valid EOS symbol")
    }

    fn visit_str<E>(self, s: &str) -> Result<Asset, E>
    where
        E: de::Error,
    {
        Asset::from_str(s).map_err(|e| de::Error::custom(e.to_string()))
    }
}
impl<'de> Deserialize<'de> for Asset {
    fn deserialize<D>(deserializer: D) -> Result<Asset, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(AssetVisitor)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_assets() {
        let names = [
            "99 WAXIBULGLOUBI",  // symbol name too long
            "99.2A3 WAX",        // cannot parse amount
            "1WAX",
            "1 1 WAX",
            "WAX",
            // "-1 WAX",  // negative amounts are allowed in EOS C++
            &format!("{} WAX", i128::pow(2, 64)),
            "1 WAXXXXXX",
            "99 ",
            "99",
            "99. WAXXXXXX",
            "99.",
        ];

        for n in names {
            println!("{}", n);
            assert!(Asset::from_str(n).is_err());
        }
    }

}
