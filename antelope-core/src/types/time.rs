use std::fmt;

use chrono::{DateTime, NaiveDateTime, ParseError as ChronoParseError, TimeZone, Utc};
use serde::{Serialize, Serializer};
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
    result.try_into().expect("Timestamp too far in the future to fit in a `u32`")
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
                if serializer.is_human_readable() {
                    self.to_string().serialize(serializer)
                }
                else {
                    self.0.serialize(serializer)
                }
            }
        }
    }
}


// -----------------------------------------------------------------------------
//     TimePoint
// -----------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct TimePoint(i64);

impl TimePoint {
    pub fn from_str(s: &str) -> Result<TimePoint, ChronoParseError> {
        Ok(TimePoint(parse_date(s)?.timestamp_micros()))
    }
    pub fn to_datetime(&self) -> DateTime<Utc> {
        Utc.timestamp_micros(self.0).unwrap()  // safe unwrap
    }
    pub fn to_json(&self) -> JsonValue {
        json!(self.to_string())
    }
}

impl From<i64> for TimePoint {
    fn from(n: i64) -> TimePoint {
        TimePoint(n)
    }
}

impl From<TimePoint> for i64 {
    fn from(t: TimePoint) -> i64 {
        t.0
    }
}

impl_time_display!(TimePoint);
impl_serialize!(TimePoint);


// -----------------------------------------------------------------------------
//     TimePointSec
// -----------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct TimePointSec(u32);

impl TimePointSec {
    pub fn from_str(s: &str) -> Result<TimePointSec, ChronoParseError> {
        Ok(TimePointSec(parse_date(s)?.timestamp()
                        .try_into().expect("Date not representable as a `u32`")))
    }
    pub fn to_datetime(&self) -> DateTime<Utc> {
        Utc.timestamp_micros(self.0 as i64 * 1_000_000).unwrap()  // safe unwrap
    }
    pub fn to_json(&self) -> JsonValue {
        json!(format!("{}", self.to_datetime().format(DATE_FORMAT)))
    }
}

impl From<u32> for TimePointSec {
    fn from(n: u32) -> TimePointSec {
        TimePointSec(n)
    }
}

impl From<TimePointSec> for u32 {
    fn from(t: TimePointSec) -> u32 {
        t.0
    }
}

impl_time_display!(TimePointSec);
impl_serialize!(TimePointSec);


// -----------------------------------------------------------------------------
//     BlockTimestampType
// -----------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct BlockTimestampType(u32);

impl BlockTimestampType {
    pub fn from_str(s: &str) -> Result<BlockTimestampType, ChronoParseError> {
        Ok(BlockTimestampType(timestamp_to_block_slot(&parse_date(s)?)))
    }
    pub fn to_datetime(&self) -> DateTime<Utc> {
        Utc.timestamp_micros(
            ((self.0 as i64 * config::BLOCK_INTERVAL_MS as i64) + config::BLOCK_TIMESTAMP_EPOCH as i64) * 1000
        ).unwrap()  // safe unwrap
    }
    pub fn to_json(&self) -> JsonValue {
        json!(format!("{}", self.to_datetime().format(DATE_FORMAT)))
    }
}

impl From<u32> for BlockTimestampType {
    fn from(n: u32) -> BlockTimestampType {
        BlockTimestampType(n)
    }
}

impl From<BlockTimestampType> for u32 {
    fn from(t: BlockTimestampType) -> u32 {
        t.0
    }
}

impl_time_display!(BlockTimestampType);
impl_serialize!(BlockTimestampType);
