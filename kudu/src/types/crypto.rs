use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;

use bs58;
use bytemuck::cast_ref;
use ripemd::{Digest, Ripemd160};
use secp256k1::{Message, SecretKey};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use sha2::Sha256;
use snafu::{ensure, ResultExt, Snafu};


use kudu_macros::with_location;

// NOTE: as for which library to use for computing signatures, they are a few candidates
//       - k256 + ecdsa: where the Rust crypto world seems to be going, however this doesn't offer
//         passing a custom nonce when signing, which is required to find an "EOS-canonical" signature
//       - libsecp256k1: no longer maintained, in favor or the previous one
//       - secp256k1: rust bindings to the C libsecp256k1, the one we are using here as it allows us to
//         pass a custom nonce

// TODO: investigate `hybrid_array` crate as a better way to represent our crypto data. This will also
//       give us better compatibility with the Rust crypto world as they use it as base array type
//       crypto libs used to use `generic-array` but it looks like they are all moving to `hybrid_array`

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

    pub fn with_key_type(key_type: KeyType, data: [u8; DATA_SIZE]) -> Self {
        Self { key_type, data, phantom: PhantomData }
    }

    pub fn new(s: &str) -> Result<Self, InvalidCryptoData> {
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

    pub fn to_hex(&self) -> String {
        hex::encode(self.data)
    }
}


// -----------------------------------------------------------------------------
//     `TryFrom` implementation
// -----------------------------------------------------------------------------

impl<T: CryptoDataType, const DATA_SIZE: usize> TryFrom<&str> for CryptoData<T, DATA_SIZE> {
    type Error = InvalidCryptoData;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::new(s)
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
//     `FromStr` implementation
// -----------------------------------------------------------------------------

impl<T: CryptoDataType, const DATA_SIZE: usize> FromStr for CryptoData<T, DATA_SIZE> {
    type Err = InvalidCryptoData;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
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
        self.to_string().serialize(serializer)
    }
}

impl<'de, T: CryptoDataType, const DATA_SIZE: usize> Deserialize<'de> for CryptoData<T, DATA_SIZE> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let repr: &str = <&str>::deserialize(deserializer)?;
        Self::new(repr).map_err(|e| de::Error::custom(e.to_string()))
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

/*

def _is_canonical(signature):
    canonical = all(
        [
            not (signature[1] & 0x80),
            not (signature[1] == 0 and not (signature[2] & 0x80)),
            not (signature[33] & 0x80),
            not (signature[33] == 0 and not (signature[34] & 0x80)),
        ]
    )
    return canonical

    bool public_key::is_canonical( const compact_signature& c ) {
        return !(c.data[1] & 0x80)
               && !(c.data[1] == 0 && !(c.data[2] & 0x80))
               && !(c.data[33] & 0x80)
               && !(c.data[33] == 0 && !(c.data[34] & 0x80));
    }


*/

impl Signature {
    /// Return whether this signature is an EOS-canonical signature
    pub fn is_canonical(&self) -> bool {
        let s1 = (self.data[1] & 0x80) == 0;
        let s2 = self.data[1] != 0 || (self.data[2] & 0x80 != 0);
        let s3 = self.data[33] & 0x80 == 0;
        let s4 = self.data[33] != 0 || (self.data[34] & 0x80 != 0);

        s1 && s2 && s3 && s4
    }
}

impl From<secp256k1::ecdsa::RecoverableSignature> for Signature {
    fn from(value: secp256k1::ecdsa::RecoverableSignature) -> Signature {
        let (recid, sigdata) = value.serialize_compact();
        // println!("rec id: {:?}", &recid);
        // println!("sigdata: {}", hex::encode(sigdata));
        let mut fullsig = [0u8; 65];
        fullsig[0] = 27 + 4 + (i32::from(recid) as u8);
        fullsig[1..].copy_from_slice(&sigdata);
        Signature::with_key_type(KeyType::K1, fullsig)
    }
}

impl From<&Signature> for secp256k1::ecdsa::RecoverableSignature {
    fn from(value: &Signature) -> Self {
        let recid = secp256k1::ecdsa::RecoveryId::from_u8_masked(value.data[0]);
        Self::from_compact(&value.data[1..], recid).unwrap()
    }
}

impl PrivateKey {
    pub fn sign_bytes(&self, input: &[u8]) -> Signature {
        // hash our bytes into a digest to be signed
        let digest: [u8; 32] = Sha256::digest(input).into();

        self.sign_digest(digest.into())
    }

    pub fn sign_digest(&self, digest: crate::Digest) -> Signature {
        if self.key_type == KeyType::K1 {
            // use global context
            let secp = secp256k1::global::SECP256K1;

            let secret_key = SecretKey::from_byte_array(self.data).expect("32 bytes, within curve order");
            let message = Message::from_digest(digest.0);

            // iterate over a nonce to be added to the signatures until we find a good one
            // (i.e.: EOS-canonical)

            let secp_sig = secp.sign_ecdsa_recoverable(message, &secret_key);

            let mut sig = Signature::from(secp_sig);
            let mut nonce: [u64; 4] = [0u64; 4];  // use this shape instead of [u8; 32] so we can iterate over nonce[0] more easily

            loop {
                // if sig is canonical, return it
                if sig.is_canonical() { return sig; }

                // otherwise, iterate over our nonce until we find a good signature
                nonce[0] += 1;

                let secp_sig = secp.sign_ecdsa_recoverable_with_noncedata(message, &secret_key, cast_ref::<[u64; 4], [u8; 32]>(&nonce));
                sig = Signature::from(secp_sig);
            }
        }
        else {
            unimplemented!("can only call `PrivateKey::sign_digest()` on K1 key types")
        }
    }

    pub fn to_wif(&self) -> String {
        unimplemented!("WIF key format is deprecated, use `key.to_string()` instead");
    }

}

impl PublicKey {
    pub fn from_private_key(private_key: &PrivateKey) -> Self {
        let secp = secp256k1::global::SECP256K1;
        let secret_key = SecretKey::from_byte_array(private_key.data).expect("32 bytes, within curve order");
        let public_key = secp256k1::PublicKey::from_secret_key(secp, &secret_key);
        public_key.into()
    }

    pub fn verify_signature(&self, input: &[u8], signature: &Signature) -> bool {
        let secp = secp256k1::global::SECP256K1;
        let message = Message::from_digest(Sha256::digest(input).into());
        let public_key = secp256k1::PublicKey::from_byte_array_compressed(self.data).expect("65 bytes");


        let sig = secp256k1::ecdsa::RecoverableSignature::from(signature);
        let sig = sig.to_standard();


        secp.verify_ecdsa(message, &sig, &public_key).is_ok()
    }

    pub fn to_old_format(&self) -> String {
        format!("EOS{}", &key_data_to_string(&self.data, "")[1..])
    }
}

impl From<secp256k1::PublicKey> for PublicKey {
    fn from(value: secp256k1::PublicKey) -> Self {
        PublicKey::with_key_type(KeyType::K1, value.serialize())
    }
}

impl From<PublicKey> for secp256k1::PublicKey {
    fn from(value: PublicKey) -> Self {
        secp256k1::PublicKey::from_byte_array_compressed(value.data).expect("33 bytes")
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

    // `eosio` testing key
    // priv: 5KQwrPbwdL6PhXujxW37FSSQZ1JiwsST4cqQzDeyXtP79zkvFD3
    //       D2653FF7CBB2D8FF129AC27EF5781CE68B2558C41A74AF1F2DDCA635CBEEF07D
    // pub : EOS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV
    //       02C0DED2BC1F1305FB0FAAC5E6C03EE3A1924234985427B6167CA569D13DF435


    #[test]
    fn test_keys() -> Result<()> {
        let priv_key = PrivateKey::new("5KQwrPbwdL6PhXujxW37FSSQZ1JiwsST4cqQzDeyXtP79zkvFD3")?;
        let pub_key = PublicKey::from_private_key(&priv_key);

        assert_eq!(pub_key.to_string(), "PUB_K1_6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5BoDq63");
        assert_eq!(pub_key.to_old_format(), "EOS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV");
        Ok(())
    }

    #[test]
    fn test_sign() -> Result<()> {
        let key = PrivateKey::new("5KQwrPbwdL6PhXujxW37FSSQZ1JiwsST4cqQzDeyXtP79zkvFD3")?;
        let input = b"a";
        let sig = key.sign_bytes(input);
        assert_eq!(sig.to_string(), "SIG_K1_JvyUh5EJU7xS3QJSszNKdxGTkQNoo1PUcaQUAjpGTa64Sihf7R6tyiiAjoiZVkoDcfFpEokJPMVqyKYUFmgSvW1MvcRhrM");
        assert!(sig.is_canonical());

        let public_key = PublicKey::from_private_key(&key);
        assert!(public_key.verify_signature(input, &sig));

        Ok(())
    }

}
