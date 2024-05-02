use std::io::prelude::*;
use std::fs::read_to_string;
use std::sync::OnceLock;

use thiserror::Error;
use base64::prelude::*;
use hex;

use flate2::{
    // Compression,
    // write::DeflateEncoder,
    read::DeflateDecoder,
};

use antelope_core::{types::antelopevalue::hex_to_boxed_array, JsonValue, Name, json};
use antelope_abi::{ABIDefinition, ABIEncoder, ByteStream, abi::TypeNameRef as T};

use tracing::debug;

// static SIGNER_NAME: Name = Name::from_str("............1").unwrap();  // == Name::from_u64(1)  TODO: make this a unittest
pub static SIGNER_NAME: Name = Name::from_u64(1);
pub static SIGNER_PERMISSION: Name = Name::from_u64(2);



pub fn signing_request_abi_schema() -> &'static ABIDefinition {
    static SIGNING_REQUEST_ABI_SCHEMA: OnceLock<ABIDefinition> = OnceLock::new();
    SIGNING_REQUEST_ABI_SCHEMA.get_or_init(|| {
        let abi_str = read_to_string("src/signing_request_abi.json").unwrap();
        let abi: ABIDefinition = serde_json::from_str(&abi_str).unwrap();
        abi
    })
}

pub fn signing_request_abi_parser() -> &'static ABIEncoder {
    static SIGNING_REQUEST_ABI_PARSER: OnceLock<ABIEncoder> = OnceLock::new();
    SIGNING_REQUEST_ABI_PARSER.get_or_init(|| {
        ABIEncoder::with_abi(signing_request_abi_schema())
    })
}

static EOSIO_ABI: &str  = r#"{
    "version": "eosio::abi/1.2",
    "structs": [
        {
            "name": "voteproducer",
            "base": "",
            "fields": [
                { "name": "voter", "type": "name" },
                { "name": "proxy", "type": "name" },
                { "name": "producers", "type": "name[]" }
            ]
        }
    ]
}
"#;

pub fn get_abi(abi_name: &str) -> ABIEncoder {
    match abi_name {
        "eosio" => ABIEncoder::from_abi(&ABIDefinition::from_str(EOSIO_ABI).unwrap()),
        "signing_request" => signing_request_abi_parser().clone(),
        _ => panic!("no abi with name {}", abi_name),
    }

}


type Checksum256 = Box<[u8; 32]>;

#[derive(Debug, Clone, PartialEq)]
pub enum ChainId {
    Alias(u8),
    Id(Checksum256), // AntelopeValue::Checksum256 variant assumed
}

impl From<ChainId> for JsonValue {
    fn from(cid: ChainId) -> JsonValue {
        match cid {
            ChainId::Alias(alias) => json!(["chain_alias", alias]),
            // ChainId::Id(id) => json!(["chain_id", id.to_string()]),  // FIXME: check we get hex encoded repr here
            _ => unimplemented!(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SigningRequest {
    pub chain_id: ChainId,
    pub actions: Vec<JsonValue>,
    pub flags: u8,
    pub callback: Option<String>,
    pub info: Vec<JsonValue>, // TODO: consider getting something more precise
}

impl Default for SigningRequest {
    fn default() -> Self {
        SigningRequest {
            chain_id: ChainId::Alias(1),
            actions: vec![],
            flags: 1,
            callback: None,
            info: vec![],

        }
    }
}

pub struct EncodeOptions {
    pub version: u8,
    pub use_compression: bool,
}

impl Default for EncodeOptions {
    fn default() -> Self {
        EncodeOptions {
            version: 2,
            use_compression: true,
        }
    }
}

impl SigningRequest {
    pub fn new(actions: JsonValue) -> Self {
        let actions = actions.as_array().unwrap();
        let mut result = SigningRequest {
            actions: actions.to_vec(),
            ..Default::default()
        };
        result.encode_actions();
        result
    }

    // FIXME: return Result<JsonValue, InvalidPayload>
    pub fn decode_payload<T: AsRef<[u8]>>(esr: T) -> Result<JsonValue, SigningRequestError> {
        let dec = BASE64_URL_SAFE.decode(esr)?;
        assert!(!dec.is_empty());

        let compression = (dec[0] >> 7) == 1u8;
        let version = dec[0] & ((1 << 7) - 1);

        debug!(version, compression, "decoding payload");

        let mut dec2;
        if compression {
            let mut deflater = DeflateDecoder::new(&dec[1..]);
            dec2 = vec![];
            deflater.read_to_end(&mut dec2).map_err(
                |_| SigningRequestError::Invalid("can not decompress payload data".to_owned()))?;

        }
        else {
            dec2 = dec;
        }


        let abi = signing_request_abi_parser();

        let mut ds = ByteStream::from(dec2);

        abi.decode_variant(&mut ds, T("signing_request"))
            .map_err(|_| SigningRequestError::Invalid(
                "cannot decode SigningRequest from JSON representation".to_owned()))
    }

    pub fn decode<T>(esr: T) -> Result<Self, SigningRequestError>
    where
        T: AsRef<[u8]>
    {
        let payload = Self::decode_payload(esr)?;
        payload.try_into()

    }

    pub fn encode_actions(&mut self) {
        for action in &mut self.actions[..] {
            Self::encode_action(action).unwrap();
        }
    }

    pub fn decode_actions(&mut self) {
        for action in &mut self.actions[..] {
            Self::decode_action(action).unwrap();
        }
    }

    pub fn encode_action(action: &mut JsonValue) -> Result<(), SigningRequestError> {
        let is_action_data_encoded = action["data"].is_string();
        if is_action_data_encoded { return Ok(()); }

        let account = conv_action_field_str(action, "account")?;
        let action_name = conv_action_field_str(action, "name")?;
        let data = &action["data"];
        let mut ds = ByteStream::new();
        let abi = get_abi(account);
        abi.encode_variant(&mut ds, T(action_name), data).unwrap();
        action["data"] = JsonValue::String(ds.hex_data());
        Ok(())
    }

    pub fn decode_action(action: &mut JsonValue) -> Result<(), SigningRequestError> {
        let is_action_data_encoded = action["data"].is_string();
        if !is_action_data_encoded { return Ok(()); }

        let account     = conv_action_field_str(action, "account")?;
        let action_name = conv_action_field_str(action, "name")?;
        let data        = conv_action_field_str(action, "data")?;
        let mut ds = ByteStream::from(hex::decode(data).unwrap());
        let abi = get_abi(account);
        action["data"] = abi.decode_variant(&mut ds, T(action_name)).unwrap();
        Ok(())
    }

    pub fn encode(&self) -> String {
        let mut ds = ByteStream::new();
        let abi = signing_request_abi_parser();

        // self.encode_actions();
        let sr = json!({
            "chain_id": JsonValue::from(self.chain_id.clone()),
        });

        abi.encode_variant(&mut ds, T("signing_request"), &sr).unwrap(); // FIXME: remove this `unwrap`
        ds.hex_data()
    }
}



impl TryFrom<JsonValue> for SigningRequest {
    type Error = SigningRequestError;

    fn try_from(payload: JsonValue) -> Result<Self, Self::Error> {
        let mut result = SigningRequest::default();

        let chain_id = &payload["chain_id"];
        let chain_id_type = conv_str(&chain_id[0])?;

        result.chain_id = match chain_id_type {
            "chain_id" => {
                let data = conv_str(&chain_id[1])?;
                ChainId::Id(hex_to_boxed_array(data)?)
            },
            "chain_alias" => {
                let alias = chain_id[1].as_u64().unwrap();
                let alias = u8::try_from(alias).unwrap();
                ChainId::Alias(alias)
            },
            _ => unimplemented!(),
        };

        let req_type = payload["req"][0].as_str().unwrap();
        let req_data = &payload["req"][1];

        result.actions = match req_type {
            "action" => vec![req_data.clone()],
            "action[]" => {
                let actions = req_data.as_array().unwrap();
                actions.to_vec()
            },
            _ => unimplemented!(),
        };

        result.flags = payload["flags"].as_u64().unwrap().try_into().unwrap();
        result.callback = Some(payload["callback"].as_str().unwrap().to_owned());
        result.info = payload["info"].as_array().unwrap().to_owned();

        result.decode_actions();

        Ok(result)
    }
}

#[derive(Error, Debug)]
pub enum SigningRequestError {
    #[error("{0}")]
    Invalid(String),

    #[error("error decoding base64 content")]
    Base64Decode(#[from] base64::DecodeError),

    #[error("hex decoding error")]
    HexDecode(#[from] hex::FromHexError),

    // #[error(r#"cannot convert given variant {1} to Antelope type "{0}""#)]
    // IncompatibleVariantTypes(String, JsonValue),

    // #[error("invalid bool")]
    // Bool(#[from] ParseBoolError),
}

pub fn conv_str<'a>(obj: &'a JsonValue) -> Result<&'a str, SigningRequestError> {
    obj.as_str().ok_or(SigningRequestError::Invalid(
        format!("Cannot convert object {:?} to str", obj)))
}

pub fn conv_action_field_str<'a>(action: &'a JsonValue, field: &str) -> Result<&'a str, SigningRequestError> {
    action[field].as_str().ok_or(
        SigningRequestError::Invalid(format!("Cannot convert action['{}'] to str, actual type: {:?}",
                                             field, action[field])))
}
