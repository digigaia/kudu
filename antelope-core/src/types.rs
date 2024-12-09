//! `Antelope` built-in types and other base types
//!
//! For the [built-in types][1] there are:
//!  - type aliases when the Rust native type matches the Antelope type
//!    (e.g.: [`Int32`] is the same as `i32`)
//!  - thin wrappers when the Antelope type adds some more functionality over
//!    a base Rust type (e.g.: [`VarInt32`] wraps an `i32`)
//!  - separate structs when the behavior is more complicated, eg: [`Asset`] or
//!    [`Symbol`]
//!
//! Apart from the built-in types, there is [`AntelopeValue`] which is
//! an enum that can contain any of the built-in types and
//! [`AntelopeType`] which contains the list of its discriminants (i.e.: the
//! list of all built-in types).
//!
//! Other useful types include [`Action`], [`PermissionLevel`].
//!
//! [1]: <https://github.com/AntelopeIO/spring/blob/main/libraries/chain/abi_serializer.cpp#L90>

pub mod action;
pub mod antelopevalue;
pub mod asset;
pub mod crypto;
pub mod name;
pub mod symbol;
pub mod time;

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

/// Newtype wrapper around an `i32` that has a different serialization implementation
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct VarInt32(pub i32);

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct VarUint32(pub u32);

impl From<i32> for VarInt32 {
    fn from(n: i32) -> VarInt32 { VarInt32(n) }
}

impl From<VarInt32> for i32 {
    fn from(n: VarInt32) -> i32 { n.0 }
}

impl From<u32> for VarUint32 {
    fn from(n: u32) -> VarUint32 { VarUint32(n) }
}

impl From<VarUint32> for u32 {
    fn from(n: VarUint32) -> u32 { n.0 }
}

impl From<usize> for VarUint32 {
    fn from(n: usize) -> VarUint32 {
        let n: u32 = n.try_into().expect("number too large to fit in a `u32`");
        VarUint32(n)
    }
}

impl From<VarUint32> for usize {
    fn from(n: VarUint32) -> usize {
        n.0 as usize
    }
}

pub type Float32 = f32;
pub type Float64 = f64;


// -----------------------------------------------------------------------------
//     Bytes and String types
// -----------------------------------------------------------------------------

pub type Bytes = std::vec::Vec<u8>;
pub type String = std::string::String;


// -----------------------------------------------------------------------------
//     Time-related types
// -----------------------------------------------------------------------------

pub use crate::types::time::{TimePoint, TimePointSec, BlockTimestampType};


// -----------------------------------------------------------------------------
//     Crypto types
// -----------------------------------------------------------------------------

pub type Checksum160 = [u8; 20];
pub type Checksum256 = [u8; 32];
pub type Checksum512 = [u8; 64];

pub use crate::types::crypto::{PrivateKey, PublicKey, Signature, InvalidCryptoData};


// -----------------------------------------------------------------------------
//     Other builtin Antelope types
// -----------------------------------------------------------------------------

pub use name::{Name, InvalidName};
pub use symbol::{Symbol, InvalidSymbol, SymbolCode};
pub use asset::{Asset, InvalidAsset};

pub type ExtendedAsset = (Asset, Name);


// -----------------------------------------------------------------------------
//     Other base Antelope types
// -----------------------------------------------------------------------------

pub use action::{PermissionLevel, Action};
pub use antelopevalue::{AntelopeType, AntelopeValue, InvalidValue};

// from https://github.com/AntelopeIO/spring/blob/main/libraries/chain/include/eosio/chain/types.hpp#L119-L123
pub type ActionName = Name;
pub type ScopeName = Name;
pub type AccountName = Name;
pub type PermissionName = Name;
pub type TableName = Name;
