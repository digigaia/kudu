use strum::{Display, EnumString, VariantNames};

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

pub type Float32 = f32;
pub type Float64 = f64;

pub type Bytes = Vec<u8>;
pub type String = std::string::String;

pub struct TimePoint(pub i64);
pub struct TimePointSec(pub u32);
pub struct BlockTimestampType(pub u32);

pub type Checksum160 = Box<[u8; 20]>;
pub type Checksum256 = Box<[u8; 32]>;
pub type Checksum512 = Box<[u8; 64]>;

pub use crate::types::name::Name;
pub use crate::types::symbol::Symbol;
pub use crate::types::asset::Asset;

pub type ExtendedAsset = (Asset, Name);


impl From<i64> for TimePoint {
    fn from(n: i64) -> TimePoint {
        TimePoint(n)
    }
}

impl From<u32> for TimePointSec {
    fn from(n: u32) -> TimePointSec {
        TimePointSec(n)
    }
}

impl From<u32> for BlockTimestampType {
    fn from(n: u32) -> BlockTimestampType {
        BlockTimestampType(n)
    }
}

// see full list in: https://github.com/AntelopeIO/leap/blob/main/libraries/chain/abi_serializer.cpp#L89
#[derive(Debug, Display, EnumString, VariantNames, Clone, PartialEq)]
#[strum(serialize_all = "snake_case")]
pub enum AntelopeType {
    Bool,

    Int8,
    Int16,
    Int32,
    Int64,
    Int128,

    Uint8,
    Uint16,
    Uint32,
    Uint64,
    Uint128,

    #[strum(serialize = "varint32")]
    VarInt32,
    #[strum(serialize = "varuint32")]
    VarUint32,

    Float32,
    Float64,
    // Float128,

    Bytes, // Vec<u8>,
    String, // (String),

    TimePoint, // (i64),
    TimePointSec, // (u32),
    BlockTimestampType, // (u32),

    Checksum160, // (Box<[u8; 20]>),
    Checksum256, // (Box<[u8; 32]>),
    Checksum512, // (Box<[u8; 64]>),

    PublicKey, // (Box<PublicKey>),
    PrivateKey, // (Box<PrivateKey>),
    Signature, // (Box<Signature>),

    Name, // (Name),
    SymbolCode, // (u64),
    Symbol, // (Symbol),
    Asset, // (Asset),
    ExtendedAsset, // (Box<(Asset, Name)>),
}
