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
//! [1]: <https://github.com/AntelopeIO/spring/blob/main/libraries/chain/abi_serializer.cpp#L90>

pub mod antelopevalue;
pub mod asset;
mod crypto;
mod name;
mod symbol;
mod time;

use hex::FromHexError;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

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

/// Newtype wrapper around a `i32` that has a different serialization implementation
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct VarInt32(pub i32);

impl From<i32> for VarInt32 {
    fn from(n: i32) -> VarInt32 { VarInt32(n) }
}

impl From<VarInt32> for i32 {
    fn from(n: VarInt32) -> i32 { n.0 }
}

impl Serialize for VarInt32 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer
    {
        if serializer.is_human_readable() {
            self.0.serialize(serializer)
        }
        else {
            let mut n = ((self.0 as u32) << 1) ^ ((self.0 >> 31) as u32);
            let mut buf = [0u8; 5];
            let mut size = 0;
            loop {
                if n >> 7 != 0 {
                    buf[size] = (0x80 | (n & 0x7f)) as u8;
                    size += 1;
                    n >>= 7;
                }
                else {
                    buf[size] = n as u8;
                    size += 1;
                    break;
                }
            }
            serializer.serialize_bytes(&buf[..size])
        }
    }
}

impl<'de> Deserialize<'de> for VarInt32 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let n = i32::deserialize(deserializer)?;
        Ok(n.into())
    }

}

/// Newtype wrapper around a `u32` that has a different serialization implementation
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct VarUint32(pub u32);

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

impl Serialize for VarUint32 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer
    {
        if serializer.is_human_readable() {
            self.0.serialize(serializer)
        }
        else {
            // FIXME: deprecated?
            let mut n = self.0;
            let mut buf = [0u8; 5];
            let mut size = 0;
            loop {
                if n >> 7 != 0 {
                    buf[size] = (0x80 | (n & 0x7f)) as u8;
                    size += 1;
                    n >>= 7;
                }
                else {
                    buf[size] = n as u8;
                    size += 1;
                    break;
                }
            }
            serializer.serialize_bytes(&buf[..size])
        }
    }
}

impl<'de> Deserialize<'de> for VarUint32 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let n = u32::deserialize(deserializer)?;
        Ok(n.into())
    }
}

pub type Float32 = f32;
pub type Float64 = f64;


// -----------------------------------------------------------------------------
//     Bytes and String types
// -----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Bytes(pub Vec<u8>);

impl Bytes {
    pub fn from_hex<T: AsRef<[u8]>>(data: T) -> Result<Bytes, FromHexError> {
        Ok(Bytes(hex::decode(data)?))
    }
    pub fn to_hex(&self) -> String {
        hex::encode(&self.0)
    }
}

impl From<Vec<u8>> for Bytes {
    fn from(v: Vec<u8>) -> Bytes {
        Bytes(v)
    }
}

impl From<&[u8]> for Bytes {
    fn from(s: &[u8]) -> Bytes {
        Bytes(s.to_vec())
    }
}

impl From<Bytes> for Vec<u8> {
    fn from(b: Bytes) -> Vec<u8> {
        b.0
    }
}

impl Serialize for Bytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        if serializer.is_human_readable() {
            self.to_hex().serialize(serializer)
        }
        else {
            // FIXME: deprecated?
            unimplemented!();
            // self.0.serialize(serializer)
        }
    }
}

impl<'de> Deserialize<'de> for Bytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let hex_repr: &str = <&str>::deserialize(deserializer)?;
        Bytes::from_hex(hex_repr).map_err(|e| de::Error::custom(e.to_string()))
    }
}

pub type String = std::string::String;


// -----------------------------------------------------------------------------
//     Time-related types
// -----------------------------------------------------------------------------

pub use crate::types::time::{TimePoint, TimePointSec, BlockTimestampType};


// -----------------------------------------------------------------------------
//     Crypto types
// -----------------------------------------------------------------------------

macro_rules! impl_checksum {
    ($typ:ident, $size:literal) => {
        #[derive(Clone, Debug, PartialEq, Eq, Hash)]
        pub struct $typ(pub [u8; $size]);

        impl $typ {
            pub fn from_hex<T: AsRef<[u8]>>(data: T) -> Result<$typ, FromHexError> {
                Ok($typ(hex::decode(data)?.try_into()
                        .map_err(|_| FromHexError::InvalidStringLength)?))
            }
            pub fn to_hex(&self) -> String {
                hex::encode(self.0)
            }
        }

        impl From<[u8; $size]> for $typ {
            fn from(v: [u8; $size]) -> Self {
                Self(v)
            }
        }

        impl Default for $typ {
            fn default() -> Self {
                $typ::from([0; $size])
            }
        }

        impl Serialize for $typ {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer
            {
                if serializer.is_human_readable() {
                    self.to_hex().serialize(serializer)
                }
                else {
                    self.0.serialize(serializer)
                }
            }
        }

        impl<'de> Deserialize<'de> for $typ {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let hex_repr: &str = <&str>::deserialize(deserializer)?;
                Self::from_hex(hex_repr).map_err(|e| de::Error::custom(e.to_string()))
            }
        }
    }
}

impl_checksum!(Checksum160, 20);
impl_checksum!(Checksum256, 32);
impl_checksum!(Checksum512, 64);


pub use crate::types::crypto::{
    CryptoData, CryptoDataType, InvalidCryptoData,
    KeyType, PrivateKey, PublicKey, Signature,
};


// -----------------------------------------------------------------------------
//     Other builtin Antelope types
// -----------------------------------------------------------------------------

pub use name::{Name, InvalidName};
pub use symbol::{Symbol, InvalidSymbol, SymbolCode};
pub use asset::{Asset, InvalidAsset, ExtendedAsset};


// -----------------------------------------------------------------------------
//     Other base Antelope types
// -----------------------------------------------------------------------------

pub use antelopevalue::{AntelopeType, AntelopeValue, InvalidValue};

// from https://github.com/AntelopeIO/spring/blob/main/libraries/chain/include/eosio/chain/types.hpp
pub type ActionName = Name;
pub type ScopeName = Name;
pub type AccountName = Name;
pub type PermissionName = Name;
pub type TableName = Name;

pub type BlockID = Checksum256;
pub type Checksum = Checksum256;
pub type TransactionID = Checksum256;
pub type Digest = Checksum256;
pub type Weight = u16;
pub type BlockNum = u32;

pub type MicroSeconds = i64;


/// Extensions are prefixed with type and are a buffer that can be
/// interpreted by code that is aware and ignored by unaware code.
pub type Extensions = Vec<(u16, Bytes)>;
