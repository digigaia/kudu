use std::fmt;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use snafu::{Snafu, ensure};

use antelope_macros::with_location;


#[with_location]
#[derive(Debug, Snafu)]
pub enum InvalidName {
    #[snafu(display(r#"Name is longer than 13 characters: "{name}""#))]
    TooLong { name: String },

    #[snafu(display(r#"Name not properly normalized (given name: "{given}", normalized: "{normalized}")"#))]
    InvalidNormalization {
        given: String,
        normalized: String,
    },
}


#[derive(Eq, Hash, PartialEq, Debug, Copy, Clone)]
pub struct Name {
    value: u64,
}

impl Name {
    pub fn from_str(s: &str) -> Result<Self, InvalidName> {
        ensure!(s.len() <= 13, TooLongSnafu { name: s.to_owned() });

        let result = Name {
            value: string_to_u64(s.as_bytes()),
        };

        if s == result.to_string() {
            Ok(result)
        }
        else {
            InvalidNormalizationSnafu { given: s.to_owned(), normalized: result.to_string() }.fail()
        }
    }

    pub const fn from_u64(n: u64) -> Self {
        // FIXME: do some validation?
        Self { value: n }
    }

    pub fn as_u64(&self) -> u64 { self.value }

    pub fn prefix(&self) -> Name {
        // note: antelope C++ has a more efficient implementation based on direct bit twiddling,
        //       but we're going for a simpler implementation here
        Name::from_str(self.to_string().rsplitn(2, '.').last().unwrap()).unwrap()  // both unwrap are safe here
    }
}

impl TryFrom<&str> for Name {
    type Error = InvalidName;

    fn try_from(s: &str) -> Result<Name, InvalidName> {
        Name::from_str(s)
    }
}

impl From<u64> for Name {
    fn from(n: u64) -> Name {
        Name::from_u64(n)
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


impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", u64_to_string(self.value))
    }
}


fn char_to_symbol(c: u8) -> u64 {
    match c {
        b'a'..=b'z' => (c - b'a') as u64 + 6,
        b'1'..=b'5' => (c - b'1') as u64 + 1,
        _ => 0,
    }
}


// see ref implementation in AntelopeIO/spring/libraries/chain/name.{hpp,cpp}
fn string_to_u64(s: &[u8]) -> u64 {
    let mut n: u64 = 0;
    // for i in 0..s.len().min(12) {
    for (i, &c) in s.iter().enumerate().take(12) {
        n |= char_to_symbol(c) << (64 - 5 * (i + 1));
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

fn u64_to_bytes(n: u64) -> Vec<u8> {
    let mut n = n;
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
        end_pos -= 1
    }
    s.truncate(end_pos);
    s
}

fn u64_to_string(n: u64) -> String {
    String::from_utf8(u64_to_bytes(n)).unwrap()  // safe unwrap
}


// =============================================================================
//
//     Unittests
//
// =============================================================================

#[cfg(test)]
mod tests {
    use color_eyre::eyre::Result;
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
            assert!(Name::from_str(n).is_err(), "Name \"{}\" should fail constructing but does not", n);
        }
    }

    #[test]
    fn prefix() -> Result<()> {
        assert_eq!(Name::from_str("eosio.any")?.prefix(),
                   Name::from_str("eosio")?);
        assert_eq!(Name::from_str("eosio")?.prefix(),
                   Name::from_str("eosio")?);

        Ok(())
    }

    #[test]
    fn basic_functionality() {
        let name = Name::from_str("foobar").unwrap();
        let json = r#""foobar""#;

        assert_eq!(name, Name::from_u64(6712742083569909760));
        assert_eq!(name.as_u64(), 6712742083569909760);

        assert_eq!(serde_json::from_str::<Name>(json).unwrap(), name);
        assert_eq!(serde_json::to_string(&name).unwrap(), json);
    }

}
