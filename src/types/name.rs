use std::fmt;
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde::de::{self, Visitor};
use anyhow::Result;
use thiserror::Error;

use crate::{AntelopeType, ByteStream};

#[derive(Error, Debug)]
pub enum InvalidName {
    #[error("Name is longer than 13 characters: \"{0}\"")]
    TooLong(String),

    #[error(r#"Name not properly normalized (given name: "{0}", normalized: "{1}")"#)]
    InvalidNormalization(String, String),
}


#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct Name {
    value: u64,
}

impl Name {
    pub fn from_str(s: &str) -> Result<Self, InvalidName> {
        if s.len() > 13 { return Err(InvalidName::TooLong(s.to_owned())); }

        let result = Name {
            value: string_to_u64(s.as_bytes()),
        };

        if s == result.to_string() {
            Ok(result)
        } else {
            Err(InvalidName::InvalidNormalization(s.to_owned(), result.to_string()))
        }
    }

    pub fn from_u64(n: u64) -> Self {
        Self {
            value: n,
        }
    }

    pub fn to_u64(&self) -> u64 { self.value }

    pub fn encode(&self, stream: &mut ByteStream) {
        AntelopeType::Uint64(self.value).to_bin(stream);
    }
}


impl Serialize for Name {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

struct NameVisitor;

impl<'de> Visitor<'de> for NameVisitor {
    type Value = Name;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a string that is a valid EOS name")
    }

    fn visit_str<E>(self, s: &str) -> Result<Name, E>
    where
        E: de::Error,
    {
        Name::from_str(s).map_err(|e| de::Error::custom(e.to_string()))
    }
}
impl<'de> Deserialize<'de> for Name {
    fn deserialize<D>(deserializer: D) -> Result<Name, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(NameVisitor)
    }
}


/*
impl ABISerializable for Name {
    fn encode(&self, encoder: &mut ByteStream) {
        encoder.write_u64(self.value);
    }
    fn decode(_stream: &mut ByteStream) -> Self {
        todo!();
    }
}
*/

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", String::from_utf8(u64_to_string(self.value)).unwrap())
    }
}


fn char_to_symbol(c: u8) -> u64 {
    match c {
        b'a'..=b'z' => (c - b'a') as u64 + 6,
        b'1'..=b'5' => (c - b'1') as u64 + 1,
        _ => 0
    }
}


// see ref implementation in AntelopeIO/leap/libraries/chain/name.{hpp,cpp}
fn string_to_u64(s: &[u8]) -> u64 {
    let mut n: u64 = 0;
    for i in 0..s.len().min(12) {
        n = n | (char_to_symbol(s[i]) << (64 - 5 * (i + 1)));
    }

    // The for-loop encoded up to 60 high bits into uint64 'name' variable,
    // if (strlen(str) > 12) then encode str[12] into the low (remaining)
    // 4 bits of 'name'
    if s.len() >= 13 {
        n |= char_to_symbol(s[12]) & 0x0F;
    }

    n
}

const CHARMAP: &[u8] = b".12345abcdefghijklmnopqrstuvwxyz";

fn u64_to_string(n: u64) -> Vec<u8> {
    let mut n = n.clone();
    let mut s: Vec<u8> = vec![b'.'; 13];
    for i in 0..=12 {
        let c: u8 = CHARMAP[n as usize & match i { 0 => 0x0F, _ => 0x1F }];
        s[12-i] = c;
        n >>= match i { 0 => 4, _ => 5 };
    }

    // truncate string with unused trailing symbols
    let mut end_pos = 13;
    loop {
        if end_pos == 0 { break; }
        if s[end_pos - 1] != b'.' {
            break;
        }
        end_pos = end_pos - 1;
    }
    s.truncate(end_pos);
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_names() -> Result<()> {
        let n = Name::from_str("nico")?;
        assert_eq!(n.to_string(), "nico");

        let n2 = Name::from_str("eosio.token")?;
        assert_eq!(n2.to_string(), "eosio.token");

        let n3 = Name::from_str("a.b.c.d.e")?;
        assert_eq!(n3.to_string(), "a.b.c.d.e");

        assert_eq!(Name::from_str("")?,
                   Name::from_u64(0));

        assert_eq!(Name::from_str("foobar")?,
                   Name::from_u64(6712742083569909760));

        Ok(())
    }

    #[test]
    fn invalid_names() {
        let names = [
            "yepthatstoolong", // too long
            "abcDef",          // invalid chars
            "a.",              // not normalized
            "A",
            "zzzzzzzzzzzzzz",
            "á",
            ".",
            "....",
            "zzzzzzzzzzzzz",
            "aaaaaaaaaaaaz",
            "............z",

        ];

        for n in names {
            assert!(Name::from_str(n).is_err());
        }
    }

}
