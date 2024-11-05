//! This contains all the `Antelope` builtin types
//!
//! There are:
//!  - type synonyms when the Rust native type matches the Antelope type
//!  - thin wrappers when the Antelope type adds some more functionality over
//!    a base Rust type, eg: `VarInt32` wraps an `i32`
//!  - separate structs when the behavior is more complicated, eg: `Asset` or `ExtendedAsset`
//!


use std::fmt;

use chrono::{DateTime, NaiveDateTime, ParseError as ChronoParseError, TimeZone, Utc};
use serde_json::{json, Value as JsonValue};

use crate::config;

// -----------------------------------------------------------------------------
//     Native POD and varint types
// -----------------------------------------------------------------------------

pub type Bool = bool;

pub type Int8 = i8;
pub type Int16 = i16;
pub type Int32 = i32;
pub type Int64 = i64;
pub type Int128 = i128;

pub type Uint8 = u8;
pub type Uint16 = u16;
pub type Uint32 = u32;
pub type Uint64 = u64;
pub type Uint128 = u128;

pub struct VarInt32(pub i32);
pub struct VarUint32(pub u32);

impl From<usize> for VarUint32 {
    fn from(n: usize) -> VarUint32 {
        let n: u32 = n.try_into().expect("number too large to fit in u32");
        VarUint32(n)
    }
}

pub type Float32 = f32;
pub type Float64 = f64;


// -----------------------------------------------------------------------------
//     Bytes and String types
// -----------------------------------------------------------------------------

pub type Bytes = Vec<u8>;
pub type String = std::string::String;


// -----------------------------------------------------------------------------
//     Time-related types
// -----------------------------------------------------------------------------

// FIXME: remove pub inner type

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct TimePoint(pub i64);

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct TimePointSec(pub u32);

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct BlockTimestampType(pub u32);

const DATE_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.3f";

/// return a date in microseconds, timezone is UTC by default
/// (we don't use naive datetimes)
fn parse_date(s: &str) -> Result<DateTime<Utc>, ChronoParseError> {
    Ok(NaiveDateTime::parse_from_str(s, DATE_FORMAT)?.and_utc())
}

fn timestamp_to_block_slot(dt: &DateTime<Utc>) -> u32 {
    let ms_since_epoch = (dt.timestamp_micros() / 1000) as u64 - config::BLOCK_TIMESTAMP_EPOCH;
    let result = ms_since_epoch / (config::BLOCK_INTERVAL_MS as u64);
    result as u32
}


impl TimePoint {
    pub fn from_str(s: &str) -> Result<TimePoint, ChronoParseError> {
        Ok(TimePoint(parse_date(s)?.timestamp_micros()))
    }
    pub fn to_datetime(&self) -> DateTime<Utc> {
        Utc.timestamp_micros(self.0).unwrap()
    }
    pub fn to_json(&self) -> JsonValue {
        json!(format!("{}", self.to_datetime().format(DATE_FORMAT)))
    }
}

impl From<i64> for TimePoint {
    fn from(n: i64) -> TimePoint {
        TimePoint(n)
    }
}

impl fmt::Display for TimePoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_datetime().format(DATE_FORMAT))
    }
}

impl TimePointSec {
    pub fn from_str(s: &str) -> Result<TimePointSec, ChronoParseError> {
        Ok(TimePointSec(parse_date(s)?.timestamp()
                        .try_into().expect("Date not representable as a `u32`")))
    }
    pub fn to_datetime(&self) -> DateTime<Utc> {
        Utc.timestamp_micros(self.0 as i64 * 1_000_000).unwrap()
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

impl fmt::Display for TimePointSec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_datetime().format(DATE_FORMAT))
    }
}

impl BlockTimestampType {
    pub fn from_str(s: &str) -> Result<BlockTimestampType, ChronoParseError> {
        Ok(BlockTimestampType(timestamp_to_block_slot(&parse_date(s)?)))
    }
    pub fn to_datetime(&self) -> DateTime<Utc> {
        Utc.timestamp_micros(
            ((self.0 as i64 * config::BLOCK_INTERVAL_MS as i64) + config::BLOCK_TIMESTAMP_EPOCH as i64) * 1000
        ).unwrap()
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

impl fmt::Display for BlockTimestampType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_datetime().format(DATE_FORMAT))
    }
}


pub type Checksum160 = Box<[u8; 20]>;
pub type Checksum256 = Box<[u8; 32]>;
pub type Checksum512 = Box<[u8; 64]>;

pub use crate::types::name::Name;
pub use crate::types::symbol::{Symbol, SymbolCode};
pub use crate::types::asset::Asset;

pub type ExtendedAsset = (Asset, Name);

// // see full list in: https://github.com/AntelopeIO/leap/blob/main/libraries/chain/abi_serializer.cpp#L89
// #[derive(Debug, Display, EnumString, VariantNames, Clone, PartialEq)]
// #[strum(serialize_all = "snake_case")]
// pub enum AntelopeType {
//     Bool,

//     Int8,
//     Int16,
//     Int32,
//     Int64,
//     Int128,

//     Uint8,
//     Uint16,
//     Uint32,
//     Uint64,
//     Uint128,

//     #[strum(serialize = "varint32")]
//     VarInt32,
//     #[strum(serialize = "varuint32")]
//     VarUint32,

//     Float32,
//     Float64,
//     // Float128,

//     Bytes, // Vec<u8>,
//     String, // (String),

//     TimePoint, // (i64),
//     TimePointSec, // (u32),
//     BlockTimestampType, // (u32),

//     Checksum160, // (Box<[u8; 20]>),
//     Checksum256, // (Box<[u8; 32]>),
//     Checksum512, // (Box<[u8; 64]>),

//     PublicKey, // (Box<PublicKey>),
//     PrivateKey, // (Box<PrivateKey>),
//     Signature, // (Box<Signature>),

//     Name, // (Name),
//     SymbolCode, // (u64),
//     Symbol, // (Symbol),
//     Asset, // (Asset),
//     ExtendedAsset, // (Box<(Asset, Name)>),
// }
