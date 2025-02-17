use std::fmt;
use std::num::ParseIntError;
use std::str::FromStr;

use serde::{de, Serialize, Serializer, Deserialize, Deserializer};
use snafu::{ensure, Snafu, OptionExt, ResultExt};

use antelope_macros::with_location;
use crate::{InvalidSymbol, Name, Symbol, impl_auto_error_conversion};


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

impl_auto_error_conversion!(ParseIntError, InvalidAsset, ParseAmountSnafu);
impl_auto_error_conversion!(InvalidSymbol, InvalidAsset, InvalidSymbolSnafu);


/// `Asset` includes amount and currency symbol.
///
/// ## Example
/// ```
/// # use antelope::{Asset, InvalidAsset, Symbol};
/// # use snafu::Whatever;
/// let asset: Asset = "10.0000 CUR".parse()?;
/// assert_eq!(asset.to_real(), 10.0);
/// assert_eq!(asset.symbol(), "4,CUR".parse::<Symbol>()?);
/// # Ok::<(), InvalidAsset>(())
/// ```
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub struct Asset {
    amount: i64,
    symbol: Symbol,
}

impl Asset {
    const MAX_AMOUNT: i64 = (1 << 62) - 1;

    pub fn new(amount: i64, symbol: Symbol) -> Result<Asset, InvalidAsset> {
        ensure!((-Self::MAX_AMOUNT..Self::MAX_AMOUNT).contains(&amount), AmountOutOfRangeSnafu);
        // no need to check for `symbol.is_valid()` as it has been successfully
        // constructed so must be valid already
        Ok(Asset { amount, symbol })
    }

    pub fn amount(&self) -> i64 { self.amount }
    pub fn symbol(&self) -> Symbol { self.symbol }
    pub fn symbol_name(&self) -> String { self.symbol.name() }
    pub fn decimals(&self) -> u8 { self.symbol.decimals() }
    pub fn precision(&self) -> i64 { self.symbol.precision() }

    pub fn to_real(&self) -> f64 {
        self.amount as f64 / self.precision() as f64
    }

}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtendedAsset {
    pub quantity: Asset,
    pub contract: Name,
}


// -----------------------------------------------------------------------------
//     Conversion traits
// -----------------------------------------------------------------------------

impl TryFrom<&str> for Asset {
    type Error = InvalidAsset;

    fn try_from(s: &str) -> Result<Asset, InvalidAsset> {
        Asset::from_str(s)
    }
}


// -----------------------------------------------------------------------------
//     `Display` implementation
// -----------------------------------------------------------------------------

// FIXME: this could be made more efficient
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


// -----------------------------------------------------------------------------
//     `FromStr` implementation
// -----------------------------------------------------------------------------

impl FromStr for Asset {
    type Err = InvalidAsset;

    fn from_str(s: &str) -> Result<Self, InvalidAsset> {
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

        let symbol = Symbol::new(&format!("{},{}", precision, symbol_str))
            .context(InvalidSymbolSnafu)?;

        // parse amount
        let amount: i64 = match dot_pos {
            None => amount_str.parse().context(ParseAmountSnafu)?,
            Some(dot_pos) => {
                let int_part: i64 = amount_str[..dot_pos].parse().context(ParseAmountSnafu)?;
                let mut frac_part: i64 = amount_str[dot_pos+1..].parse().context(ParseAmountSnafu)?;
                if amount_str.starts_with('-') { frac_part *= -1; }
                // check that we don't overflow
                int_part
                    .checked_mul(symbol.precision())
                    .with_context(|| AmountOverflowSnafu { amount: amount_str })?
                    .checked_add(frac_part)
                    .with_context(|| AmountOverflowSnafu { amount: amount_str })?
            },
        };

        Asset::new(amount, symbol)
    }
}


// -----------------------------------------------------------------------------
//     `Serde` traits implementation
// -----------------------------------------------------------------------------

impl Serialize for Asset {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_string().serialize(serializer)
    }
}


impl<'de> Deserialize<'de> for Asset {
    fn deserialize<D>(deserializer: D) -> Result<Asset, D::Error>
    where
        D: Deserializer<'de>,
    {
        let asset: &str = <&str>::deserialize(deserializer)?;
        Asset::from_str(asset).map_err(|e| de::Error::custom(e.to_string()))
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
            assert!(Asset::from_str(n).is_err());
        }
    }

    #[test]
    fn basic_functionality() {
        let obj: Asset = "1.2345 FOO".parse().unwrap();
        let json = r#""1.2345 FOO""#;

        assert_eq!(obj.amount(), 12345);
        assert_eq!(obj.decimals(), 4);
        assert_eq!(obj.precision(), 10000);

        assert_eq!(serde_json::from_str::<Asset>(json).unwrap(), obj);
        assert_eq!(serde_json::to_string(&obj).unwrap(), json);
    }
}
