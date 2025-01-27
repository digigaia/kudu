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

mod antelopevalue;
mod asset;
mod crypto;
mod name;
mod symbol;
mod time;
mod varint;

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

pub use varint::{VarInt32, VarUint32};

pub type Float32 = f32;
pub type Float64 = f64;


// -----------------------------------------------------------------------------
//     Bytes and String types
// -----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Bytes(pub Vec<u8>);

impl Bytes {
    pub fn new() -> Self { Bytes(vec![]) }
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

// This should probably be using Bytes::from_hex if we define it, however this
// conversion is probably prone to error so we don't define it for now unless
// a good reason comes up that we should
// impl From<&str> for Bytes {
//     fn from(s: &str) -> Bytes {
//         Bytes(s.as_bytes().to_vec())
//     }
// }

impl From<Bytes> for Vec<u8> {
    fn from(b: Bytes) -> Vec<u8> {
        b.0
    }
}

impl AsRef<[u8]> for Bytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
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
                Ok(Self(hex::decode(data)?.try_into()
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

        impl TryFrom<&str> for $typ {
            type Error = FromHexError;

            fn try_from(s: &str) -> Result<Self, Self::Error> {
                Self::from_hex(s)
            }
        }

        impl Default for $typ {
            fn default() -> Self {
                Self::from([0; $size])
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

pub type BlockId = Checksum256;
pub type Checksum = Checksum256;
pub type TransactionId = Checksum256;
pub type Digest = Checksum256;
pub type Weight = u16;
pub type BlockNum = u32;

pub type MicroSeconds = i64;


/// Extensions are prefixed with type and are a buffer that can be
/// interpreted by code that is aware and ignored by unaware code.
pub type Extensions = Vec<(u16, Bytes)>;
