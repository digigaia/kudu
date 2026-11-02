// SPDX-FileCopyrightText: 2024-2026 DigiGaia SCCL
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;
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
    #[snafu(display("Invalid date/time"))]
    JiffError { source: jiff::Error },
}

impl_auto_error_conversion!(jiff::Error, InvalidTimePoint, JiffSnafu);


const DATE_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.f";      // accept any number of digits for millis/micros
const DATE_FORMAT_MS: &str = "%Y-%m-%dT%H:%M:%S%.3f";  // fixed 3 digits for millis
const DATE_FORMAT_NO_SECS: &str = "%Y-%m-%dT%H:%M";

/// return a jiff::Timestamp (nanosecond precision), timezone is UTC by default
fn parse_date(s: &str) -> Result<Timestamp, jiff::Error> {
    let dt = DateTime::strptime(DATE_FORMAT, s).or_else(|_| DateTime::strptime(DATE_FORMAT_NO_SECS, s))?;
    Ok(dt.to_zoned(TimeZone::UTC)?.timestamp())
}

fn timestamp_to_block_slot(dt: &Timestamp) -> u32 {
    let ms_since_epoch = dt.as_millisecond() as u64 - config::BLOCK_TIMESTAMP_EPOCH;
    let result = ms_since_epoch / (config::BLOCK_INTERVAL_MS as u64);
    result.try_into().expect("timestamp too far in the future to fit block slot in a `u32`")
}

macro_rules! impl_ctor {
    ($typ:ty) => {
        impl $typ {
            pub fn new_with_micro(year: i16, month: i8, day: i8, hour: i8, min: i8, sec: i8, micro: i32) -> Result<Self, InvalidTimePoint> {
                let dt = DateTime::new(year, month, day, hour, min, sec, micro * 1_000)?;
                Ok(Self::from_timestamp(dt.to_zoned(TimeZone::UTC)?.timestamp()))
            }
        }
    }
}

macro_rules! impl_time_display {
    ($typ:ty) => {
        impl fmt::Display for $typ {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.to_timestamp().strftime(DATE_FORMAT_MS))
            }
        }

        impl $typ {
            pub fn to_json(&self) -> JsonValue {
                json!(self.to_string())
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

        impl FromStr for $typ {
            type Err = jiff::Error;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(<$typ>::from_timestamp(parse_date(s)?))
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
        Self::new_with_micro(year, month, day, hour, min, sec, milli * 1_000)
    }
    pub fn from_timestamp(ts: Timestamp) -> Self {
        TimePoint(ts.as_microsecond())
    }
    pub fn to_timestamp(&self) -> Timestamp {
        Timestamp::from_microsecond(self.0).unwrap()  // safe unwrap
    }
}

impl_ctor!(TimePoint);
impl_time_display!(TimePoint);
impl_serialize!(TimePoint);
impl_from!(TimePoint, i64);


// -----------------------------------------------------------------------------
//     TimePointSec
// -----------------------------------------------------------------------------

/// TimePoint with second precision
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct TimePointSec(u32);

impl TimePointSec {
    pub fn new(year: i16, month: i8, day: i8, hour: i8, min: i8, sec: i8) -> Result<Self, InvalidTimePoint> {
        Self::new_with_micro(year, month, day, hour, min, sec, 0)
    }
    pub fn from_timestamp(ts: Timestamp) -> Self {
        TimePointSec(ts.as_second().try_into()
                     .expect("date too far in the future: number of seconds exceeds `u32` capacity"))
    }
    pub fn to_timestamp(&self) -> Timestamp {
        Timestamp::from_second(self.0 as i64).unwrap()  // safe unwrap
    }
    pub fn add_seconds(&self, sec: u32) -> TimePointSec {
        TimePointSec(self.0.saturating_add(sec))
    }
}

impl_ctor!(TimePointSec);
impl_time_display!(TimePointSec);
impl_serialize!(TimePointSec);
impl_from!(TimePointSec, u32);


// -----------------------------------------------------------------------------
//     BlockTimestamp
// -----------------------------------------------------------------------------

/// Index of block slot since blockchain epoch
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct BlockTimestamp(u32);

impl BlockTimestamp {
    pub fn new(year: i16, month: i8, day: i8, hour: i8, min: i8, sec: i8, milli: i32) -> Result<Self, InvalidTimePoint> {
        Self::new_with_micro(year, month, day, hour, min, sec, milli * 1_000)
    }
    pub fn from_timestamp(ts: Timestamp) -> Self {
        BlockTimestamp(timestamp_to_block_slot(&ts))
    }
    pub fn to_timestamp(&self) -> Timestamp {
        Timestamp::from_millisecond(
            (self.0 as i64 * config::BLOCK_INTERVAL_MS as i64) + config::BLOCK_TIMESTAMP_EPOCH as i64
        ).unwrap()  // safe unwrap
    }
}

impl_ctor!(BlockTimestamp);
impl_time_display!(BlockTimestamp);
impl_serialize!(BlockTimestamp);
impl_from!(BlockTimestamp, u32);
