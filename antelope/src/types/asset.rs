use std::fmt;
use std::num::ParseIntError;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use snafu::{ensure, Snafu, OptionExt, ResultExt};

use antelope_macros::with_location;
use crate::{InvalidSymbol, Symbol};


#[with_location]
#[derive(Debug, Snafu)]
pub enum InvalidAsset {
    #[snafu(display("asset amount and symbol should be separated with space"))]
    MissingSpace,

    #[snafu(display("missing decimal fraction after decimal point"))]
    MissingDecimal,

    #[snafu(display("could not parse amount for asset"))]
    ParseAmount { source: ParseIntError },

    #[snafu(display("amount overflow for: {amount}"))]
    AmountOverflow { amount: String },

    #[snafu(display("ammount out of range, max is 2^62-1"))]
    AmountOutOfRange,

    #[snafu(display("could not parse symbol from asset string"))]
    InvalidSymbol { source: InvalidSymbol },
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
        ensure!(self.is_amount_within_range(), AmountOutOfRangeSnafu);
        // no need to check for symbol.is_valid, it has been successfully constructed
        Ok(())
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
        let space_pos = s.find(' ').context(MissingSpaceSnafu)?;

        let amount_str = &s[..space_pos];
        let symbol_str = &s[space_pos + 1..].trim();

        // parse symbol
        let dot_pos = amount_str.find('.');
        let precision;
        if let Some(dot_pos) = dot_pos {
            // Ensure that if decimal point is used (.), decimal fraction is specified
            ensure!(dot_pos != amount_str.len() - 1, MissingDecimalSnafu);
            precision = amount_str.len() - dot_pos - 1;
        }
        else {
            precision = 0;
        }

        let symbol = Symbol::from_str(&format!("{},{}", precision, symbol_str))
            .context(InvalidSymbolSnafu)?;

        // parse amount
        let amount: i64 = match dot_pos {
            None => amount_str.parse().context(ParseAmountSnafu)?,
            Some(dot_pos) => {
                let int_part: i64 = amount_str[..dot_pos].parse().context(ParseAmountSnafu)?;
                let mut frac_part: i64 = amount_str[dot_pos+1..].parse().context(ParseAmountSnafu)?;
                if amount_str.starts_with('-') { frac_part *= -1; }
                // check we don't overflow
                int_part
                    .checked_mul(symbol.precision())
                    .context(AmountOverflowSnafu { amount: amount_str.to_owned() })?
                    .checked_add(frac_part)
                    .context(AmountOverflowSnafu { amount: amount_str.to_owned() })?
            },
        };

        Ok(Self { amount, symbol })
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

    #[test]
    fn serialize_json() {
        let obj = Asset::from_str("1.2345 FOO").unwrap();
        let json = r#""1.2345 FOO""#;

        assert_eq!(obj.amount(), 12345);
        assert_eq!(obj.decimals(), 4);
        assert_eq!(obj.precision(), 10000);

        assert_eq!(serde_json::from_str::<Asset>(json).unwrap(), obj);
        assert_eq!(serde_json::to_string(&obj).unwrap(), json);
    }
}
