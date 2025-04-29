use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, NaiveDate, NaiveDateTime, ParseError as ChronoParseError, TimeZone, Utc};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{json, Value as JsonValue};

use crate::config;


const DATE_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.3f";
const DATE_FORMAT_NO_SECS: &str = "%Y-%m-%dT%H:%M";

/// return a date in microseconds, timezone is UTC by default
/// (we don't use naive datetimes)
fn parse_date(s: &str) -> Result<DateTime<Utc>, ChronoParseError> {
    Ok(NaiveDateTime::parse_from_str(s, DATE_FORMAT)
       .or_else(|_| NaiveDateTime::parse_from_str(s, DATE_FORMAT_NO_SECS))?
       .and_utc())
}

fn timestamp_to_block_slot(dt: &DateTime<Utc>) -> u32 {
    let ms_since_epoch = (dt.timestamp_micros() / 1000) as u64 - config::BLOCK_TIMESTAMP_EPOCH;
    let result = ms_since_epoch / (config::BLOCK_INTERVAL_MS as u64);
    result.try_into().expect("timestamp too far in the future to fit block slot in a `u32`")
}

macro_rules! impl_time_display {
    ($typ:ty) => {
        impl fmt::Display for $typ {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.to_datetime().format(DATE_FORMAT))
            }
        }
    }
}

macro_rules! impl_serialize {
    ($typ:ty) => {
        impl Serialize for $typ {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where S: Serializer
            {
                self.to_string().serialize(serializer)
            }
        }

        impl<'de> Deserialize<'de> for $typ {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                // FIXME: can we not keep &str here?
                // see: https://github.com/serde-rs/serde/issues/2065
                // see: https://github.com/serde-rs/serde/issues/1009
                // let s: &str = <&str>::deserialize(deserializer)?;
                // Self::from_str(s).map_err(|e| de::Error::custom(e.to_string()))
                let s: String = String::deserialize(deserializer)?;
                Self::from_str(&s).map_err(|e| de::Error::custom(e.to_string()))
            }
        }
    }
}

macro_rules! impl_from {
    ($typ:ty, $inner:ty) => {
        impl From<$inner> for $typ {
            fn from(n: $inner) -> $typ {
                Self(n)
            }
        }

        impl From<$typ> for $inner {
            fn from(t: $typ) -> $inner {
                t.0
            }
        }

        impl TryFrom<&str> for $typ {
            type Error = ChronoParseError;
            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Self::from_str(value)
            }
        }
    }
}


// -----------------------------------------------------------------------------
//     TimePoint
// -----------------------------------------------------------------------------

/// TimePoint with micro second precision
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct TimePoint(i64);

impl TimePoint {
    pub fn new(year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32, milli: u32) -> Option<Self> {
        Some(TimePoint::from_datetime(
            NaiveDate::from_ymd_opt(year, month, day)?
                .and_hms_milli_opt(hour, min, sec, milli)?
                .and_utc()))
    }
    pub fn from_ymd_hms_micro(year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32, micro: u32) -> Option<Self> {
        Some(TimePoint::from_datetime(
            NaiveDate::from_ymd_opt(year, month, day)?
                .and_hms_micro_opt(hour, min, sec, micro)?
                .and_utc()))
    }
    pub fn from_datetime(dt: DateTime<Utc>) -> Self {
        TimePoint(dt.timestamp_micros())
    }
    pub fn to_datetime(&self) -> DateTime<Utc> {
        Utc.timestamp_micros(self.0).unwrap()  // safe unwrap
    }
    pub fn to_json(&self) -> JsonValue {
        json!(self.to_string())
    }
}

impl_time_display!(TimePoint);
impl_serialize!(TimePoint);
impl_from!(TimePoint, i64);

impl FromStr for TimePoint {
    type Err = ChronoParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(TimePoint::from_datetime(parse_date(s)?))
    }
}


// -----------------------------------------------------------------------------
//     TimePointSec
// -----------------------------------------------------------------------------

/// TimePoint with second precision
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct TimePointSec(u32);

impl TimePointSec {
    pub fn new(year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32) -> Option<Self> {
        Some(TimePointSec::from_datetime(
            NaiveDate::from_ymd_opt(year, month, day)?
                .and_hms_opt(hour, min, sec)?
                .and_utc()))
    }
    pub fn from_datetime(dt: DateTime<Utc>) -> Self {
        TimePointSec(dt.timestamp().try_into()
                     .expect("date too far in the future: number of seconds exceeds `u32` capacity"))
    }
    pub fn to_datetime(&self) -> DateTime<Utc> {
        Utc.timestamp_millis_opt(self.0 as i64 * 1000).unwrap()  // safe unwrap
    }
    pub fn to_json(&self) -> JsonValue {
        json!(format!("{}", self.to_datetime().format(DATE_FORMAT)))
    }
}

impl_time_display!(TimePointSec);
impl_serialize!(TimePointSec);
impl_from!(TimePointSec, u32);

impl FromStr for TimePointSec {
    type Err = ChronoParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(TimePointSec::from_datetime(parse_date(s)?))
    }
}


// -----------------------------------------------------------------------------
//     BlockTimestamp
// -----------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct BlockTimestamp(u32);

impl BlockTimestamp {
    pub fn new(year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32, milli: u32) -> Option<Self> {
        Some(BlockTimestamp::from_datetime(
            NaiveDate::from_ymd_opt(year, month, day)?
                .and_hms_milli_opt(hour, min, sec, milli)?
                .and_utc()))
    }
    pub fn from_datetime(dt: DateTime<Utc>) -> Self {
        BlockTimestamp(timestamp_to_block_slot(&dt))
    }
    pub fn to_datetime(&self) -> DateTime<Utc> {
        Utc.timestamp_millis_opt(
            (self.0 as i64 * config::BLOCK_INTERVAL_MS as i64) + config::BLOCK_TIMESTAMP_EPOCH as i64
        ).unwrap()  // safe unwrap
    }
    pub fn to_json(&self) -> JsonValue {
        json!(format!("{}", self.to_datetime().format(DATE_FORMAT)))
    }
}

impl_time_display!(BlockTimestamp);
impl_serialize!(BlockTimestamp);
impl_from!(BlockTimestamp, u32);

impl FromStr for BlockTimestamp {
    type Err = ChronoParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(BlockTimestamp(timestamp_to_block_slot(&parse_date(s)?)))
    }
}
