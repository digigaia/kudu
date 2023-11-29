use std::fmt;
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde::de::{self, Visitor};
use anyhow::Result;
use thiserror::Error;
use bs58;
use ripemd::{Ripemd160, Digest};

use crate::{AntelopeType, ByteStream, InvalidValue};

#[derive(Error, Debug)]
pub enum InvalidSignature {
    #[error("not a signature: {0}")]
    NotASignature(String),

    #[error("error while decoding base58 data")]
    Base58Error(#[from] bs58::decode::Error),

    #[error("invalid checksum for signature")]
    InvalidChecksum,
}


#[derive(Eq, PartialEq, Hash, Debug, Copy, Clone)]
pub enum KeyType {
    K1,
    R1,
    WebAuthn,
}

impl KeyType {
    pub fn index(&self) -> u8 {
        match self {
            Self::K1 => 0,
            Self::R1 => 1,
            Self::WebAuthn => 2,
        }
    }

    pub fn suffix(&self) -> &'static str {
        match self {
            Self::K1 => "K1",
            Self::R1 => "R1",
            Self::WebAuthn => "WA",
        }
    }
}

type ECCSignature = [u8; 65];

fn to_sig(v: Vec<u8>) -> ECCSignature {
    v.try_into().unwrap_or_else(|v: Vec<u8>| panic!("wrong size for ECC signature, needs to be 65 but is: {}", v.len()))
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct Signature {
    key_type: KeyType,
    data: ECCSignature,
}

impl Signature {
    pub fn from_str(s: &str) -> Result<Self, InvalidSignature> {
        if s.starts_with("SIG_K1_") {
            let key_type = KeyType::K1;
            let data = string_to_key_data(&s[7..], &key_type)?;
            Ok(Signature { key_type, data: to_sig(data) })
        }
        else if s.starts_with("SIG_R1_") {
            unimplemented!()
        }
        else if s.starts_with("SIG_WA_") {
            unimplemented!()
        }
        else {
            Err(InvalidSignature::NotASignature(s.to_owned()))
        }
    }

    pub fn encode(&self, stream: &mut ByteStream) {
        todo!()
    }

    pub fn decode(stream: &mut ByteStream) -> Result<Self, InvalidValue> {
        todo!()
        // let n: usize = AntelopeType::from_bin("uint64", stream)?.try_into()?;
        // Ok(Name::from_u64(n as u64))
    }
}


fn string_to_key_data(enc_data: &str, key_type: &KeyType) -> Result<Vec<u8>, InvalidSignature> {
    let data = bs58::decode(enc_data).into_vec()?;

    let mut hasher = Ripemd160::new();
    hasher.update(&data[..data.len()-4]);
    hasher.update(key_type.suffix());
    let digest = hasher.finalize();

    assert_eq!(&digest[..4], &data[data.len()-4..]);

    Ok(data[..data.len()-4].to_owned())
}

fn key_data_to_string(k: &ECCSignature, key_type: KeyType) -> String {
    if key_type != KeyType::K1 { panic!("unsupported key type: {:?}", key_type); }

    let mut hasher = Ripemd160::new();
    hasher.update(k);
    hasher.update(key_type.suffix());
    let digest = hasher.finalize();

    let mut data: Vec<u8> = Vec::from(k.clone());
    data.extend_from_slice(&digest[..4]);

    let enc_data = bs58::encode(data).into_string();

    format!("SIG_{}_{}", key_type.suffix(), enc_data)
}

impl Serialize for Signature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

struct SignatureVisitor;

impl<'de> Visitor<'de> for SignatureVisitor {
    type Value = Signature;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a string that is a valid EOS signature")
    }

    fn visit_str<E>(self, s: &str) -> Result<Signature, E>
    where
        E: de::Error,
    {
        Signature::from_str(s).map_err(|e| de::Error::custom(e.to_string()))
    }
}
impl<'de> Deserialize<'de> for Signature {
    fn deserialize<D>(deserializer: D) -> Result<Signature, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(SignatureVisitor)
    }
}


impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.key_type != KeyType::K1 { panic!("unsupported key type: {:?}", self.key_type); }
        write!(f, "{}", key_data_to_string(&self.data, self.key_type))
   }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_signatures() -> Result<()> {
        // let n = Name::from_str("nico")?;
        // assert_eq!(n.to_string(), "nico");

        // let n2 = Name::from_str("eosio.token")?;
        // assert_eq!(n2.to_string(), "eosio.token");

        // let n3 = Name::from_str("a.b.c.d.e")?;
        // assert_eq!(n3.to_string(), "a.b.c.d.e");

        // assert_eq!(Name::from_str("")?,
        //            Name::from_u64(0));

        // assert_eq!(Name::from_str("foobar")?,
        //            Name::from_u64(6712742083569909760));

        Ok(())
    }

    #[test]
    fn invalid_signatures() {
        // let names = [
        //     "yepthatstoolong", // too long
        //     "abcDef",          // invalid chars
        //     "a.",              // not normalized
        //     "A",
        //     "zzzzzzzzzzzzzz",
        //     "á",
        //     ".",
        //     "....",
        //     "zzzzzzzzzzzzz",
        //     "aaaaaaaaaaaaz",
        //     "............z",

        // ];

        // for n in names {
        //     assert!(Name::from_str(n).is_err());
        // }
    }

}
