use std::fmt;
use std::str::FromStr;

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use snafu::{Snafu, ensure};

use kudu_macros::with_location;


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

/// Represent an immutable name in the Antelope data model and is encoded as a `uint64`.
#[derive(Eq, Hash, PartialEq, Ord, PartialOrd, Debug, Copy, Clone, Default)]
pub struct Name {
    value: u64,
}

impl Name {
    /// Build a `Name` from its string representation.
    ///
    /// ## Example
    /// ```
    /// # use kudu::{Name, InvalidName};
    /// assert!(Name::new("nico").is_ok());
    /// assert_eq!(Name::new("eosio.token")?.to_string(), "eosio.token");
    /// assert_eq!(Name::new("a.b.c.d.e")?.to_string(), "a.b.c.d.e");
    /// assert_eq!(Name::new("")?.as_u64(), 0);
    /// # Ok::<(), InvalidName>(())
    /// ```
    pub fn new(s: &str) -> Result<Self, InvalidName> {
        ensure!(s.len() <= 13, TooLongSnafu { name: s.to_owned() });

        let result = Name {
            value: string_to_u64(s.as_bytes()),
        };

        if is_normalized(s.as_bytes(), result.value) {
            Ok(result)
        }
        else {
            InvalidNormalizationSnafu { given: s.to_owned(), normalized: result.to_string() }.fail()
        }
    }

    pub const fn constant(s: &str) -> Self {
        if s.len() > 13 { panic!("Name too long! Max is 13 chars"); }
        let value = string_to_u64(s.as_bytes());
        if !is_normalized(s.as_bytes(), value) { panic!("Invalid normalization"); }
        Name { value  }
    }

    /// Build a `Name` from its `u64` representation.
    #[inline]
    pub const fn from_u64(n: u64) -> Self {
        // NOTE: no validation here, all u64 are valid names
        Self { value: n }
    }

    /// Return the name `u64` representation.
    #[inline]
    pub fn as_u64(&self) -> u64 { self.value }

    /// Return the prefix.
    ///
    /// # Example
    /// ```
    /// # use kudu::{Name, InvalidName};
    /// assert_eq!(Name::new("eosio.any")?.prefix(), Name::new("eosio")?);
    /// assert_eq!(Name::new("eosio")?.prefix(), Name::new("eosio")?);
    /// # Ok::<(), InvalidName>(())
    /// ```
    pub fn prefix(&self) -> Name {
        // note: antelope C++ has a more efficient implementation based on direct bit twiddling,
        //       but we're going for a simpler implementation here
        Name::new(self.to_string().rsplitn(2, '.').last().unwrap()).unwrap()  // both unwrap are safe here
    }
}


// -----------------------------------------------------------------------------
//     Helper functions
// -----------------------------------------------------------------------------

const fn char_to_symbol(c: u8) -> u64 {
    match c {
        b'a'..=b'z' => (c - b'a') as u64 + 6,
        b'1'..=b'5' => (c - b'1') as u64 + 1,
        _ => 0,
    }
}

// see ref implementation in AntelopeIO/spring/libraries/chain/name.{hpp,cpp}
const fn string_to_u64(s: &[u8]) -> u64 {
    let mut n: u64 = 0;
    let maxlen = if s.len() < 12 { s.len() } else { 12 };
    let mut i = 0;
    while i < maxlen {
        n |= char_to_symbol(s[i]) << (64 - 5 * (i + 1));
        i += 1;
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

const fn is_normalized(s: &[u8], encoded: u64) -> bool {
    let mut n = encoded;
    let mut s2 = [b'.'; 13];
    let mut i = 0;
    while i < 13 {
        let c: u8 = CHARMAP[n as usize & match i { 0 => 0x0F, _ => 0x1F }];
        s2[12-i] = c;
        n >>= match i { 0 => 4, _ => 5 };
        i += 1;
    }

    // truncate string with unused trailing symbols
    let mut end_pos = 13;
    loop {
        if end_pos == 0 { break; }
        if s2[end_pos - 1] != b'.' {
            break;
        }
        end_pos -= 1
    }

    // compare original string with normalized one
    if s.len() != end_pos { return false; }
    i = 0;
    while i < end_pos {
        if s[i] != s2[i] { return false; }
        i += 1;
    }

    true
}

fn u64_to_string(n: u64) -> String {
    String::from_utf8(u64_to_bytes(n)).unwrap()  // safe unwrap
}


// -----------------------------------------------------------------------------
//     Conversion traits
// -----------------------------------------------------------------------------

impl TryFrom<&str> for Name {
    type Error = InvalidName;

    fn try_from(s: &str) -> Result<Name, InvalidName> {
        Name::new(s)
    }
}

impl From<u64> for Name {
    fn from(n: u64) -> Name {
        Name::from_u64(n)
    }
}


// -----------------------------------------------------------------------------
//     `Display` implementation
// -----------------------------------------------------------------------------

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", u64_to_string(self.value))
    }
}


// -----------------------------------------------------------------------------
//     `FromStr` implementation
// -----------------------------------------------------------------------------

impl FromStr for Name {
    type Err = InvalidName;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Name::new(s)
    }
}


// -----------------------------------------------------------------------------
//     `Serde` traits implementation
// -----------------------------------------------------------------------------

impl Serialize for Name {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Name {
    fn deserialize<D>(deserializer: D) -> Result<Name, D::Error>
    where
        D: Deserializer<'de>,
    {
        let name: &str = <&str>::deserialize(deserializer)?;
        Name::new(name).map_err(|e| de::Error::custom(e.to_string()))
    }
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
        let n = Name::new("nico")?;
        assert_eq!(n.to_string(), "nico");

        let n2 = Name::new("eosio.token")?;
        assert_eq!(n2.to_string(), "eosio.token");

        let n3 = Name::new("a.b.c.d.e")?;
        assert_eq!(n3.to_string(), "a.b.c.d.e");

        assert_eq!(Name::new("")?,
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
            assert!(Name::new(n).is_err(), "Name \"{}\" should fail constructing but does not", n);
        }
    }

    #[test]
    fn basic_functionality() {
        let name = Name::new("foobar").unwrap();
        let json = r#""foobar""#;

        assert_eq!(name, Name::from_u64(6712742083569909760));
        assert_eq!(name.as_u64(), 6712742083569909760);

        assert_eq!(serde_json::from_str::<Name>(json).unwrap(), name);
        assert_eq!(serde_json::to_string(&name).unwrap(), json);
    }

}
