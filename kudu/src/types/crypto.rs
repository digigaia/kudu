use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;

use bs58;
use bytemuck::cast_ref;
use ripemd::{Digest, Ripemd160};
use secp256k1::{Secp256k1, Message, SecretKey};
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
    pub fn from_secp_sig(sig: secp256k1::ecdsa::RecoverableSignature) -> Signature {
        let (recid, sigdata) = sig.serialize_compact();
        // println!("rec id: {:?}", &recid);
        // println!("sigdata: {}", hex::encode(sigdata));
        let mut fullsig = [0u8; 65];
        fullsig[0] = 27 + 4 + (i32::from(recid) as u8);
        fullsig[1..].copy_from_slice(&sigdata);
        Signature::with_key_type(KeyType::K1, fullsig)
    }

    /// Return whether this signature is an EOS-canonical signature
    pub fn is_canonical(&self) -> bool {
        let s1 = (self.data[1] & 0x80) == 0;
        let s2 = self.data[1] != 0 || (self.data[2] & 0x80 != 0);
        let s3 = self.data[33] & 0x80 == 0;
        let s4 = self.data[33] != 0 || (self.data[34] & 0x80 != 0);

        s1 && s2 && s3 && s4
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
            // FIXME: use global context
            let secp = Secp256k1::new();
            let secret_key = SecretKey::from_byte_array(self.data).expect("32 bytes, within curve order");

            let message = Message::from_digest(digest.0);

            // iterate over a nonce to be added to the signatures until we find a good one
            // (i.e.: EOS-canonical)

            let secp_sig = secp.sign_ecdsa_recoverable(message, &secret_key);

            let mut sig = Signature::from_secp_sig(secp_sig);
            let mut nonce: [u64; 4] = [0u64; 4];  // use this shape instead of [u8; 32] so we can iterate over nonce[0] more easily

            loop {
                // if sig is canonical, return it
                if sig.is_canonical() { return sig; }

                // otherwise, iterate over our nonce until we find a good signature
                nonce[0] += 1;

                let secp_sig = secp.sign_ecdsa_recoverable_with_noncedata(message, &secret_key, cast_ref::<[u64; 4], [u8; 32]>(&nonce));
                sig = Signature::from_secp_sig(secp_sig);
            }
        }
        else {
            unimplemented!("can only call `PrivateKey::sign_digest()` on K1 key types")
        }
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
    fn test_sign1() -> Result<()> {
        let sig = Signature::new("SIG_K1_KvmBaAEvcU3c1YhWe6btgqd1BDGkcTdo4Pziy3tuSbHtQdNJ7mDjEawCDY5F1DQzi3H9WH7efaZspfrv2Zfza1zEktg5Dc")?;
        println!("sig0: {}", hex::encode(sig.data));
        let key = PrivateKey::new("5KQwrPbwdL6PhXujxW37FSSQZ1JiwsST4cqQzDeyXtP79zkvFD3")?;
        println!("key data: {}", hex::encode(key.data));
        let input = b"a";
        let sig = key.sign_bytes(input);
        println!("sig: {}", &sig);
        println!("sigdata: {}", hex::encode(sig.data));

        /*  expected value
sig: SIG_K1_JvyUh5EJU7xS3QJSszNKdxGTkQNoo1PUcaQUAjpGTa64Sihf7R6tyiiAjoiZVkoDcfFpEokJPMVqyKYUFmgSvW1MvcRhrM
sigdata: 1f0d23aa9b3fde14471680f9c3574c7a35f131ee183236537d5083a67cc01b1ea507fcd1610b18651e2e74fad8961c62e4cd6a1006b22a1033d02a9ea801f6a499

*/
        Ok(())
    }

}


/*

void hexdump(void* ptr, int size) {
    unsigned char* buf = (unsigned char*)ptr;
    for (int i = 0; i < size; i++) {
        printf("%02X", buf[i]);
    }
}

int main(int argc, char** argv) {
   fc::logger::get(DEFAULT_LOGGER).set_log_level(fc::log_level::debug);

   private_key priv("5KQwrPbwdL6PhXujxW37FSSQZ1JiwsST4cqQzDeyXtP79zkvFD3");
   public_key pub = priv.get_public_key();

   fc::sha256 digest("CA978112CA1BBDCAFAC231B39A23DC4DA786EFF8147C4E72B9807785AFEE48BB");

   // fc::crypto::signature sig1 = priv.sign_compact(digest, true);
   auto sig1 = priv.sign(digest, false);
   auto sig2 = priv.sign(digest, true);
   //fc::ecc::compact_signature sig2 = priv.sign(digest, true);

   cout << "==============================================================" << endl;

   cout << "priv: " << priv.to_string({}) << endl;
   cout << "      "; hexdump(&priv, 32); cout << endl;
   cout << "pub : " << pub.to_string({}) << endl;
   cout << "      "; hexdump(&pub, 32); cout << endl;
   cout << "digest: " << digest << endl;

   cout << "sig1: " << sig1.to_string() << endl;
   cout << "sig2: " << sig2.to_string() << endl;

   hexdump(&sig1, 65);
   cout << endl;


=====================================================

non canonical, no extended nonce function

0000000010584902FD7F000080574902FD7F0000FB5BA710D955000000000000
0000000010584902FD7F000080574902FD7F0000FB5BA710D955000000000000
==============================================================
priv: 5KQwrPbwdL6PhXujxW37FSSQZ1JiwsST4cqQzDeyXtP79zkvFD3
      D2653FF7CBB2D8FF129AC27EF5781CE68B2558C41A74AF1F2DDCA635CBEEF07D
pub : EOS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV
      02C0DED2BC1F1305FB0FAAC5E6C03EE3A1924234985427B6167CA569D13DF435
digest: sha256(ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb)
sig1: SIG_K1_L2mNBFkV1gLvo46wzovmv7LuzJg9w9VrXhjsu4wZC6J73uhdcBE1KTo3UpKgvzyXjF96TvZy3cH92zXBHcDQLeqLGyi3vK
sig2: SIG_K1_L2mNBFkV1gLvo46wzovmv7LuzJg9w9VrXhjsu4wZC6J73uhdcBE1KTo3UpKgvzyXjF96TvZy3cH92zXBHcDQLeqLGyi3vK
20F4BBB594CA2E33DA601AB6957C894C75BD599DC896DB8709BC5F760E0F646DE658620592CB3A11968C07E6FD3CE2959E6924B2A2ACC8A234774D5205306710FD


canonical

00000000F0C8169AFE7F000060C8169AFE7F00002B1CF6F4F355000000000000
00000000F0C8169AFE7F000060C8169AFE7F00002B1CF6F4F355000000000000
01000000F0C8169AFE7F000060C8169AFE7F00002B1CF6F4F355000000000000
02000000F0C8169AFE7F000060C8169AFE7F00002B1CF6F4F355000000000000
03000000F0C8169AFE7F000060C8169AFE7F00002B1CF6F4F355000000000000
04000000F0C8169AFE7F000060C8169AFE7F00002B1CF6F4F355000000000000
05000000F0C8169AFE7F000060C8169AFE7F00002B1CF6F4F355000000000000
06000000F0C8169AFE7F000060C8169AFE7F00002B1CF6F4F355000000000000
==============================================================
priv: 5KQwrPbwdL6PhXujxW37FSSQZ1JiwsST4cqQzDeyXtP79zkvFD3
      D2653FF7CBB2D8FF129AC27EF5781CE68B2558C41A74AF1F2DDCA635CBEEF07D
pub : EOS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV
      02C0DED2BC1F1305FB0FAAC5E6C03EE3A1924234985427B6167CA569D13DF435
digest: sha256(ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb)
sig1: SIG_K1_KvmBaAEvcU3c1YhWe6btgqd1BDGkcTdo4Pziy3tuSbHtQdNJ7mDjEawCDY5F1DQzi3H9WH7efaZspfrv2Zfza1zEktg5Dc
sig2: SIG_K1_Kixd1Axg547CcWQaQ445eFKFa8EcRDtUpt4LA7se8hYp9jutQ5sorBzEyu7YEnSKQWpUtwBW8qyyDwAtJCk5f5EmAvuj1V
20C6D8FA16FA754A8DD1415B8A450E76A5947F7259DA9E79F72AB3351931028B88202816BEC2547581BDDB8F0F2114B94B63465A1728A4E45EA00ED9018D3DC6A0
206CA1C08C104C23CD9C26232393F8BDE927F4C5499259E2ED3A7B672A40F550C579D56DAF4AA789FCCECF60F1AC1CA70DF1D12A163D9C6B115AB3A0F217CA193A
*/
