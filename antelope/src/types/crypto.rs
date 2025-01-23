use std::fmt;
use std::marker::PhantomData;

use bs58;
use ripemd::{Digest, Ripemd160};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use sha2::Sha256;
use snafu::{ensure, ResultExt, Snafu};

use antelope_macros::with_location;

#[with_location]
#[derive(Debug, Snafu)]
pub enum InvalidCryptoData {
    #[snafu(display("invalid key type index: {index}"))]
    KeyTypeIndex { index: u8 },

    #[snafu(display("not crypto data: {msg}"))]
    NotCryptoData { msg: String },

    #[snafu(display("{msg}"))]
    InvalidDataSize { msg: String },

    #[snafu(display("Hashes don't match: actual: {hash} - expected: {expected}"))]
    InvalidHash { hash: String, expected: String },

    #[snafu(display("error while decoding base58 data"))]
    Base58Error { source: bs58::decode::Error },
}


#[derive(Eq, PartialEq, Hash, Debug, Copy, Clone)]
pub enum KeyType {
    K1,
    R1,
    WebAuthn,
}

impl KeyType {
    pub fn from_index(i: u8) -> Result<Self, InvalidCryptoData> {
        Ok(match i {
            0 => Self::K1,
            1 => Self::R1,
            2 => Self::WebAuthn,
            _ => KeyTypeIndexSnafu { index: i }.fail()?,
        })
    }

    pub fn index(&self) -> u8 {
        match self {
            Self::K1 => 0,
            Self::R1 => 1,
            Self::WebAuthn => 2,
        }
    }

    pub fn prefix(&self) -> &'static str {
        match self {
            Self::K1 => "K1",
            Self::R1 => "R1",
            Self::WebAuthn => "WA",
        }
    }
}

pub trait CryptoDataType {
    const DISPLAY_NAME: &'static str;
    const PREFIX: &'static str;
    // const DATA_SIZE: usize;
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct CryptoData<T: CryptoDataType, const DATA_SIZE: usize> {
    key_type: KeyType,
    data: [u8; DATA_SIZE],
    phantom: PhantomData<T>,
}

impl<T: CryptoDataType, const DATA_SIZE: usize> CryptoData<T, DATA_SIZE> {
    pub fn key_type(&self) -> KeyType { self.key_type }
    pub fn data(&self) -> &[u8; DATA_SIZE] { &self.data }

    pub fn new(key_type: KeyType, data: [u8; DATA_SIZE]) -> Self {
        Self { key_type, data, phantom: PhantomData }
    }

    pub fn from_str(s: &str) -> Result<Self, InvalidCryptoData> {
        // check legacy formats first
        if T::PREFIX == "PUB" && s.starts_with("EOS") {
            // legacy format public key
            let key_type = KeyType::K1;
            let data = string_to_key_data(&s[3..], None)?;
            Ok(Self { key_type, data: Self::vec_to_data(data)?, phantom: PhantomData })
        }
        else if T::PREFIX == "PVT" && !s.contains('_') {
            // legacy private key WIF format
            let key_type = KeyType::K1;
            let data = from_wif(s)?;
            Ok(Self { key_type, data: Self::vec_to_data(data)?, phantom: PhantomData })

        }
        else if s.starts_with(&format!("{}_K1_", T::PREFIX)) {
            let key_type = KeyType::K1;
            let data = string_to_key_data(&s[7..], Some(key_type.prefix()))?;
            Ok(Self { key_type, data: Self::vec_to_data(data)?, phantom: PhantomData })
        }
        else if s.starts_with(&format!("{}_R1_", T::PREFIX)) {
            let key_type = KeyType::R1;
            let data = string_to_key_data(&s[7..], Some(key_type.prefix()))?;
            Ok(Self { key_type, data: Self::vec_to_data(data)?, phantom: PhantomData })
            // unimplemented!()
        }
        else if s.starts_with(&format!("{}_WA_", T::PREFIX)) {
            unimplemented!()
        }
        else {
            NotCryptoDataSnafu { msg: s.to_owned() }.fail()
        }
    }

    pub fn vec_to_data(v: Vec<u8>) -> Result<[u8; DATA_SIZE], InvalidCryptoData> {
        let input_len = v.len();
        let result = v.try_into();
        ensure!(result.is_ok(), InvalidDataSizeSnafu {
            msg: format!("wrong size for {}, needs to be {} but is: {}", T::DISPLAY_NAME, DATA_SIZE, input_len)
        });
        Ok(result.unwrap())  // safe unwrap
    }
}


// -----------------------------------------------------------------------------
//     `TryFrom` implementation
// -----------------------------------------------------------------------------

impl<T: CryptoDataType, const DATA_SIZE: usize> TryFrom<&str> for CryptoData<T, DATA_SIZE> {
    type Error = InvalidCryptoData;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::from_str(s)
    }
}

// -----------------------------------------------------------------------------
//     `Display` implementation
// -----------------------------------------------------------------------------

impl<T: CryptoDataType, const DATA_SIZE: usize> fmt::Display for CryptoData<T, DATA_SIZE> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.key_type == KeyType::WebAuthn { unimplemented!("unsupported key type: {:?}", self.key_type); }
        write!(f, "{}_{}", T::PREFIX, key_data_to_string(&self.data,  self.key_type.prefix()))
   }
}


// -----------------------------------------------------------------------------
//     `Serde` traits implementation
// -----------------------------------------------------------------------------

impl<T: CryptoDataType, const DATA_SIZE: usize> Serialize for CryptoData<T, DATA_SIZE> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        if serializer.is_human_readable() {
            self.to_string().serialize(serializer)
        }
        else {
            unimplemented!()
        }
    }
}

impl<'de, T: CryptoDataType, const DATA_SIZE: usize> Deserialize<'de> for CryptoData<T, DATA_SIZE> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let repr: &str = <&str>::deserialize(deserializer)?;
        Self::from_str(repr).map_err(|e| de::Error::custom(e.to_string()))
    }
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PublicKeyType;

impl CryptoDataType for PublicKeyType {
    const DISPLAY_NAME: &'static str = "public key";
    const PREFIX: &'static str = "PUB";
}

pub type PublicKey = CryptoData<PublicKeyType, 33>;


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PrivateKeyType;

impl CryptoDataType for PrivateKeyType {
    const DISPLAY_NAME: &'static str = "private key";
    const PREFIX: &'static str = "PVT";
}

pub type PrivateKey = CryptoData<PrivateKeyType, 32>;


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SignatureType;

impl CryptoDataType for SignatureType {
    const DISPLAY_NAME: &'static str = "signature";
    const PREFIX: &'static str = "SIG";
}

pub type Signature = CryptoData<SignatureType, 65>;


fn string_to_key_data(enc_data: &str, prefix: Option<&str>) -> Result<Vec<u8>, InvalidCryptoData> {
    let data = bs58::decode(enc_data).into_vec().context(Base58Snafu)?;

    ensure!(data.len() >= 5, NotCryptoDataSnafu { msg: format!(
        "Invalid length for decoded base58 crypto data, needs to be at least 5, is {}",
        data.len())
    });

    let mut hasher = Ripemd160::new();
    hasher.update(&data[..data.len() - 4]);
    if let Some(prefix) = prefix {
        hasher.update(prefix);
    }
    let digest = hasher.finalize();

    let actual = &digest[..4];
    let expected = &data[data.len() - 4..];

    assert_eq!(actual, expected,
               "hash don't match, actual: {:?} - expected {:?}",
               hex::encode(actual), hex::encode(expected));

    Ok(data[..data.len() - 4].to_owned())
}

fn from_wif(enc_data: &str) -> Result<Vec<u8>, InvalidCryptoData> {
    let data = bs58::decode(enc_data).into_vec().context(Base58Snafu)?;

    ensure!(data.len() >= 5, NotCryptoDataSnafu { msg: format!(
        "Invalid length for decoded base58 crypto data, needs to be at least 5, is {}",
        data.len())
    });

    let digest = Sha256::digest(&data[..data.len() - 4]);
    let digest2 = Sha256::digest(digest);

    let actual = &digest[..4];
    let actual2 = &digest2[..4];
    let expected = &data[data.len() - 4..];

    ensure!(actual == expected || actual2 == expected, InvalidHashSnafu {
        hash: hex::encode(actual2),
        expected: hex::encode(expected)
    });

    Ok(data[1..data.len() - 4].to_owned())
}


fn key_data_to_string<const N: usize>(k: &[u8; N], prefix: &str) -> String {
    let mut hasher = Ripemd160::new();
    hasher.update(k);
    hasher.update(prefix);
    let digest = hasher.finalize();

    let mut data: Vec<u8> = Vec::from(*k);
    data.extend_from_slice(&digest[..4]);

    let enc_data = bs58::encode(data).into_string();

    format!("{}_{}", prefix, enc_data)
}
