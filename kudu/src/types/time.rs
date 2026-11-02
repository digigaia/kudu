// SPDX-FileCopyrightText: 2024-2026 DigiGaia SCCL
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;
use std::ops::Add;
use std::str::FromStr;

use jiff::{civil::DateTime, tz::TimeZone, Timestamp};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{json, Value as JsonValue};
use snafu::Snafu;

use kudu_macros::with_location;

use crate::{config, impl_auto_error_conversion};


#[with_location]
#[derive(Debug, Snafu)]
pub enum InvalidTimePoint {

    // FIXME: remove unneeded errors

    #[snafu(display("Invalid year-month-day: {year}-{month}-{day}"))]
    Ymd {
        year: i32,
        month: u32,
        day: u32,
    },

    #[snafu(display("Invalid hour:minute:second: {hour}:{min}:{sec}"))]
    Hms {
        hour: u32,
        min: u32,
        sec: u32,
    },

    #[snafu(display("Invalid hour:minute:second.milli: {hour}:{min}:{sec}.{milli}"))]
    HmsMilli {
        hour: u32,
        min: u32,
        sec: u32,
        milli: u32,
    },

    #[snafu(display("Invalid hour:minute:second.micro: {hour}:{min}:{sec}.{micro}"))]
    HmsMicro {
        hour: u32,
        min: u32,
        sec: u32,
        micro: u32,
    },

    #[snafu(display("Invalid date/time"))]
    JiffError { source: jiff::Error },
}

impl_auto_error_conversion!(jiff::Error, InvalidTimePoint, JiffSnafu);


const DATE_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.f";      // accept any number of digits for millis/micros
const DATE_FORMAT_MS: &str = "%Y-%m-%dT%H:%M:%S%.3f";  // fixed 3 digits for millis
const DATE_FORMAT_NO_SECS: &str = "%Y-%m-%dT%H:%M";

/// return a date in microseconds, timezone is UTC by default
/// (we don't use naive datetimes)
fn parse_date(s: &str) -> Result<Timestamp, jiff::Error> {
    let dt = DateTime::strptime(DATE_FORMAT, s).or_else(|_| DateTime::strptime(DATE_FORMAT_NO_SECS, s))?;
    Ok(dt.to_zoned(TimeZone::UTC)?.timestamp())
}

fn timestamp_to_block_slot(dt: &Timestamp) -> u32 {
    let ms_since_epoch = dt.as_millisecond() as u64 - config::BLOCK_TIMESTAMP_EPOCH;
    let result = ms_since_epoch / (config::BLOCK_INTERVAL_MS as u64);
    result.try_into().expect("timestamp too far in the future to fit block slot in a `u32`")
}

macro_rules! impl_time_display {
    ($typ:ty) => {
        impl fmt::Display for $typ {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.to_timestamp().strftime(DATE_FORMAT_MS))
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
                let s: &str = <&str>::deserialize(deserializer)?;
                Self::from_str(s).map_err(|e| de::Error::custom(e.to_string()))
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
            type Error = jiff::Error;
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
    pub fn new(year: i16, month: i8, day: i8, hour: i8, min: i8, sec: i8, milli: i32) -> Result<Self, InvalidTimePoint> {
        let dt = DateTime::new(year, month, day, hour, min, sec, milli * 1_000_000)?;
        Ok(Self::from_timestamp(dt.to_zoned(TimeZone::UTC)?.timestamp()))
    }
    pub fn from_ymd_hms_micro(year: i16, month: i8, day: i8, hour: i8, min: i8, sec: i8, micro: i32) -> Result<Self, InvalidTimePoint> {
        let dt = DateTime::new(year, month, day, hour, min, sec, micro * 1_000)?;
        Ok(Self::from_timestamp(dt.to_zoned(TimeZone::UTC)?.timestamp()))
    }
    pub fn from_timestamp(ts: Timestamp) -> Self {
        TimePoint(ts.as_microsecond())
    }
    pub fn to_timestamp(&self) -> Timestamp {
        Timestamp::from_microsecond(self.0).unwrap()  // safe unwrap
    }
    pub fn to_json(&self) -> JsonValue {
        json!(self.to_string())
    }
}

impl_time_display!(TimePoint);
impl_serialize!(TimePoint);
impl_from!(TimePoint, i64);

impl FromStr for TimePoint {
    type Err = jiff::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(TimePoint::from_timestamp(parse_date(s)?))
    }
}


// -----------------------------------------------------------------------------
//     TimePointSec
// -----------------------------------------------------------------------------

/// TimePoint with second precision
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct TimePointSec(u32);

impl TimePointSec {
    pub fn new(year: i16, month: i8, day: i8, hour: i8, min: i8, sec: i8) -> Result<Self, InvalidTimePoint> {
        let dt = DateTime::new(year, month, day, hour, min, sec, 0)?;
        Ok(Self::from_timestamp(dt.to_zoned(TimeZone::UTC)?.timestamp()))
    }
    pub fn from_timestamp(ts: Timestamp) -> Self {
        TimePointSec(ts.as_second().try_into()
                     .expect("date too far in the future: number of seconds exceeds `u32` capacity"))
    }
    pub fn to_timestamp(&self) -> Timestamp {
        Timestamp::from_second(self.0 as i64).unwrap()  // safe unwrap
    }
    pub fn to_json(&self) -> JsonValue {
        json!(format!("{}", self.to_timestamp().strftime(DATE_FORMAT_MS)))
    }
}

impl_time_display!(TimePointSec);
impl_serialize!(TimePointSec);
impl_from!(TimePointSec, u32);

impl FromStr for TimePointSec {
    type Err = jiff::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(TimePointSec::from_timestamp(parse_date(s)?))
    }
}

impl Add<u32> for TimePointSec {
    type Output = Self;

    fn add(self, other: u32) -> Self {
        Self(self.0 + other)
    }
}

// -----------------------------------------------------------------------------
//     BlockTimestamp
// -----------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct BlockTimestamp(u32);

impl BlockTimestamp {
    pub fn new(year: i16, month: i8, day: i8, hour: i8, min: i8, sec: i8, milli: i32) -> Result<Self, InvalidTimePoint> {
        let dt = DateTime::new(year, month, day, hour, min, sec, milli * 1_000_000)?;
        Ok(Self::from_timestamp(dt.to_zoned(TimeZone::UTC)?.timestamp()))
    }
    pub fn from_timestamp(ts: Timestamp) -> Self {
        BlockTimestamp(timestamp_to_block_slot(&ts))
    }
    pub fn to_timestamp(&self) -> Timestamp {
        Timestamp::from_millisecond(
            (self.0 as i64 * config::BLOCK_INTERVAL_MS as i64) + config::BLOCK_TIMESTAMP_EPOCH as i64
        ).unwrap()  // safe unwrap
    }
    pub fn to_json(&self) -> JsonValue {
        json!(format!("{}", self.to_timestamp().strftime(DATE_FORMAT_MS)))
    }
}

impl_time_display!(BlockTimestamp);
impl_serialize!(BlockTimestamp);
impl_from!(BlockTimestamp, u32);

impl FromStr for BlockTimestamp {
    type Err = jiff::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(BlockTimestamp::from_timestamp(parse_date(s)?))
    }
}
