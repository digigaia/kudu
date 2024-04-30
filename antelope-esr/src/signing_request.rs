use std::io::prelude::*;
use std::fs::read_to_string;
use std::sync::OnceLock;

use base64::prelude::*;
use hex;

use flate2::{
    // Compression,
    // write::DeflateEncoder,
    read::DeflateDecoder,
};

use antelope_core::{types::antelopevalue::hex_to_boxed_array, AntelopeValue, JsonValue, Name};
use antelope_abi::{ABIDefinition, ABIEncoder, ByteStream, abi::TypeNameRef as T};

use tracing::{info, warn};

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



#[derive(Debug, Clone, PartialEq)]
pub enum ChainId {
    Alias(u8),
    Id(AntelopeValue), // AntelopeValue::Checksum256 variant assumed
}

#[derive(Debug, Clone)]
pub struct SigningRequest {
    pub chain_id: ChainId,
    pub actions: Vec<JsonValue>,
    pub flags: u64,
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
    pub fn decode_payload<T: AsRef<[u8]>>(esr: T) -> JsonValue {
        let dec = BASE64_URL_SAFE.decode(esr).unwrap();
        assert!(!dec.is_empty());

        warn!("{:?}", &dec);

        let compression = (dec[0] >> 7) == 1u8;
        let version = dec[0] & ((1 << 7) - 1);

        warn!(version, compression, "decoding payload");

        let mut dec2;
        if compression {
            let mut deflater = DeflateDecoder::new(&dec[1..]);
            dec2 = vec![];
            deflater.read_to_end(&mut dec2).unwrap();
        }
        else {
            dec2 = dec;
        }


        warn!("{:?}", &dec2);

        // let abi_str = read_to_string("src/signing_request_abi.json").unwrap();
        // let abi: ABIDefinition = serde_json::from_str(&abi_str).unwrap();
        // let encoder = ABIEncoder::from_abi(&abi);
        let encoder = signing_request_abi_parser();

        let mut ds = ByteStream::from(dec2);

        let mut payload = encoder.decode_variant(&mut ds, T("signing_request")).unwrap();

        // TODO: decode actions for all variants of `req`
        info!(%payload, "before decoding actions");
        for action in payload["req"][1].as_array_mut().unwrap() {  // note: unwrap() is not necessary here as we can iterate over an Option but we still want it to make sure we fail instead of silently skipping
            Self::decode_action(action);
        }
        info!(%payload, "after decoding actions");

        payload
    }

    pub fn decode<T: AsRef<[u8]>>(esr: T) -> Self {
        let payload = Self::decode_payload(esr);

        let mut result = SigningRequest::default();

        let chain_id = &payload["chain_id"];
        let chain_id_type = chain_id[0].as_str().unwrap();

        result.chain_id = match chain_id_type {
            "chain_id" => {
                let data = hex_to_boxed_array(chain_id[1].as_str().unwrap()).unwrap();
                ChainId::Id(AntelopeValue::Checksum256(data))
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

        result.flags = payload["flags"].as_u64().unwrap();
        result.callback = Some(payload["callback"].as_str().unwrap().to_owned());
        result.info = payload["info"].as_array().unwrap().to_owned();

        result
    }

    pub fn encode_actions(&mut self) {
        for action in &mut self.actions[..] {
            Self::encode_action(action);
        }
    }

    pub fn encode_action(action: &mut JsonValue) {
        let account = action["account"].as_str().unwrap();
        let action_name = action["name"].as_str().unwrap();
        let data = &action["data"];
        let mut ds = ByteStream::new();
        let abi = get_abi(account);
        abi.encode_variant(&mut ds, T(action_name), data).unwrap();
        action["data"] = JsonValue::String(ds.hex_data());
    }

    pub fn decode_action(action: &mut JsonValue) {
        let account = action["account"].as_str().unwrap();
        let action_name = action["name"].as_str().unwrap();
        let data = action["data"].as_str().unwrap();
        let mut ds = ByteStream::from(hex::decode(data).unwrap());
        let abi = get_abi(account);
        action["data"] = abi.decode_variant(&mut ds, T(action_name)).unwrap();
    }

    pub fn encode(&self) -> String {
        let mut ds = ByteStream::new();

        // first encode all actions
        let encoder = signing_request_abi_parser();
        encoder.encode_variant(&mut ds, T("action"), &self.actions[0]).unwrap();

        // encoder.encode_variant(&mut ds, T("signing_request"), &self.actions).unwrap(); // FIXME: remove this `unwrap`
        "".to_owned()
    }
}
