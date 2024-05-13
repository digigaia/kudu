use std::fmt;
use std::marker::PhantomData;

use bs58;
use ripemd::{Digest, Ripemd160};
use sha2::Sha256;
use thiserror::Error;


#[derive(Eq, PartialEq, Hash, Debug, Copy, Clone)]
pub enum KeyType {
    K1,
    R1,
    WebAuthn,
}

impl KeyType {
    pub fn from_index(i: u8) -> Self {
        match i {
            0 => Self::K1,
            1 => Self::R1,
            2 => Self::WebAuthn,
            _ => panic!("invalid key type index: {}", i),
        }
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


#[derive(Error, Debug)]
pub enum InvalidCryptoData {
    #[error("not crypto data: {0}")]
    NotCryptoData(String),

    #[error("error while decoding base58 data")]
    Base58Error(#[from] bs58::decode::Error),

    #[error("invalid checksum for crypto data")]
    InvalidChecksum,
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
            Ok(Self { key_type, data: Self::vec_to_data(data), phantom: PhantomData })
        }
        else if T::PREFIX == "PVT" && !s.contains('_') {
            // legacy private key WIF format
            let key_type = KeyType::K1;
            let data = from_wif(s)?;
            Ok(Self { key_type, data: Self::vec_to_data(data), phantom: PhantomData })

        }
        else if s.starts_with(&format!("{}_K1_", T::PREFIX)) {
            let key_type = KeyType::K1;
            let data = string_to_key_data(&s[7..], Some(key_type.prefix()))?;
            Ok(Self { key_type, data: Self::vec_to_data(data), phantom: PhantomData })
        }
        else if s.starts_with(&format!("{}_R1_", T::PREFIX)) {
            let key_type = KeyType::R1;
            let data = string_to_key_data(&s[7..], Some(key_type.prefix()))?;
            Ok(Self { key_type, data: Self::vec_to_data(data), phantom: PhantomData })
            // unimplemented!()
        }
        else if s.starts_with(&format!("{}_WA_", T::PREFIX)) {
            unimplemented!()
        }
        else {
            Err(InvalidCryptoData::NotCryptoData(s.to_owned()))
        }
    }

    pub fn vec_to_data(v: Vec<u8>) -> [u8; DATA_SIZE] {
        v.try_into().unwrap_or_else(|v: Vec<u8>| panic!("wrong size for {}, needs to be {} but is: {}", T::DISPLAY_NAME, DATA_SIZE, v.len()))
    }

}


impl<T: CryptoDataType, const DATA_SIZE: usize> fmt::Display for CryptoData<T, DATA_SIZE> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.key_type == KeyType::WebAuthn { panic!("unsupported key type: {:?}", self.key_type); }
        write!(f, "{}_{}", T::PREFIX, key_data_to_string(&self.data,  self.key_type.prefix()))
   }
}


#[derive(Debug, Clone, PartialEq)]
pub struct PublicKeyType;

impl CryptoDataType for PublicKeyType {
    const DISPLAY_NAME: &'static str = "public key";
    const PREFIX: &'static str = "PUB";
}

pub type PublicKey = CryptoData<PublicKeyType, 33>;


#[derive(Debug, Clone, PartialEq)]
pub struct PrivateKeyType;

impl CryptoDataType for PrivateKeyType {
    const DISPLAY_NAME: &'static str = "private key";
    const PREFIX: &'static str = "PVT";
}

pub type PrivateKey = CryptoData<PrivateKeyType, 32>;


#[derive(Debug, Clone, PartialEq)]
pub struct SignatureType;

impl CryptoDataType for SignatureType {
    const DISPLAY_NAME: &'static str = "signature";
    const PREFIX: &'static str = "SIG";
}

pub type Signature = CryptoData<SignatureType, 65>;


fn string_to_key_data(enc_data: &str, prefix: Option<&str>) -> Result<Vec<u8>, InvalidCryptoData> {
    let data = bs58::decode(enc_data).into_vec()?;
    if data.len() < 5 {
        return Err(InvalidCryptoData::NotCryptoData(format!(
            "Invalid length for decoded base58 crypto data, needs to be at least 5, is {}",
            data.len()
        )));
    }

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
               hex::encode_upper(actual), hex::encode_upper(expected));

    Ok(data[..data.len() - 4].to_owned())
}

fn from_wif(enc_data: &str) -> Result<Vec<u8>, InvalidCryptoData> {
    let data = bs58::decode(enc_data).into_vec()?;
    if data.len() < 5 {
        return Err(InvalidCryptoData::NotCryptoData(format!(
            "Invalid length for decoded base58 crypto data, needs to be at least 5, is {}",
            data.len()
        )));
    }

    let digest = Sha256::digest(&data[..data.len() - 4]);
    let digest2 = Sha256::digest(digest);

    let actual = &digest[..4];
    let actual2 = &digest2[..4];
    let expected = &data[data.len() - 4..];

    assert!(actual == expected || actual2 == expected,
            "hash don't match, actual: {:?} - expected {:?}",
            hex::encode_upper(actual2), hex::encode_upper(expected));

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
