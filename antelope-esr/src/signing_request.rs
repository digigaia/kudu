use std::io::prelude::*;
use std::rc::Rc;
use std::fmt;

use thiserror::Error;
use base64::prelude::*;
use hex;

use flate2::read::DeflateDecoder;
use serde::{Serialize, Serializer, ser::SerializeTuple, ser::SerializeStruct};

use antelope_core::{types::antelopevalue::hex_to_boxed_array, JsonValue, Name, json};
use antelope_abi::{
    ByteStream,
    abi::TypeNameRef as T,
    provider::{get_signing_request_abi, test_provider, ABIProvider, APICallABIProvider, InvalidABI, NullABIProvider, TestABIProvider},
};

use tracing::{trace, debug, warn};

pub static SIGNER_NAME: Name = Name::from_u64(1);
pub static SIGNER_PERMISSION: Name = Name::from_u64(2);



type Checksum256 = Box<[u8; 32]>;

// -----------------------------------------------------------------------------
//     ChainId enum - can be an alias or the full chain ID
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum ChainId {
    Alias(u8),
    Id(Checksum256), // AntelopeValue::Checksum256 variant assumed
}

impl Serialize for ChainId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer
    {
        let mut tup = serializer.serialize_tuple(2)?;
        match self {
            ChainId::Alias(alias) => {
                tup.serialize_element("chain_alias")?;
                tup.serialize_element(&alias)?;
            },
            ChainId::Id(id) => {
                tup.serialize_element("chain_id")?;
                tup.serialize_element(&hex::encode_upper(**id))?;
            },
        }
        tup.end()
    }
}

// -----------------------------------------------------------------------------
//     Request data enum - contains the data in the request
// -----------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum Request {
    Action(JsonValue),
    Actions(Vec<JsonValue>),
    Transaction,
    Identity,
}

impl Serialize for Request {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer
    {
        let mut tup = serializer.serialize_tuple(2)?;
        match self {
            Request::Action(action) => {
                tup.serialize_element("action")?;
                tup.serialize_element(&action)?;
            },
            Request::Actions(actions) => {
                tup.serialize_element("action[]")?;
                tup.serialize_element(&actions)?;
            },
            Request::Transaction => todo!(),
            Request::Identity => todo!(),
        }
        tup.end()
    }
}

// =============================================================================
//
//     SigningRequest main struct
//
// =============================================================================

#[derive(Clone)]
pub struct SigningRequest {
    pub chain_id: ChainId,
    // pub actions: Vec<JsonValue>,
    pub request: Request,
    pub flags: u8,
    pub callback: Option<String>,
    pub info: Vec<JsonValue>, // TODO: consider getting something more precise

    abi_provider: Rc<dyn ABIProvider>,
}

impl fmt::Debug for SigningRequest {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("SigningRequest")
           .field("chain_id", &self.chain_id)
           .field("request", &self.request)
           .field("flags", &self.flags)
           .field("callback", &self.callback)
           .field("info", &self.info)
           .finish()
    }
}

impl Default for SigningRequest {
    fn default() -> Self {
        SigningRequest {
            chain_id: ChainId::Alias(1),
            request: Request::Actions(vec![]),
            flags: 1,
            callback: None,
            info: vec![],

            abi_provider: Rc::new(NullABIProvider::new()),
        }
    }
}

// -----------------------------------------------------------------------------
//     EncodeOptions
// -----------------------------------------------------------------------------

pub struct EncodeOptions {
    pub version: u8,
    pub use_compression: bool,
    pub abi_provider: Rc<dyn ABIProvider>,
}

impl Default for EncodeOptions {
    fn default() -> Self {
        EncodeOptions {
            version: 2,
            use_compression: true,
            abi_provider: Rc::new(test_provider()),
        }
    }
}

impl EncodeOptions {
    // helper functions for commonly used options to have a more ergonomic usage
    pub fn with_abi_provider(name: &str) -> EncodeOptions {
        match name {
            "test" => EncodeOptions {
                abi_provider: Rc::new(TestABIProvider::new()),
                ..Default::default()
            },
            "jungle" => EncodeOptions {
                abi_provider: Rc::new(APICallABIProvider::new("https://jungle4.greymass.com")),
                ..Default::default()
            },
            "eos" => EncodeOptions {
                abi_provider: Rc::new(APICallABIProvider::new("https://eos.greymass.com")),
                ..Default::default()
            },
            _ => unimplemented!()
        }
    }
}

impl SigningRequest {
    // TODO: is there any case where we would want a & instead of an owned value?
    //       however we can't move stuff out of if because of shared references, so we
    //       don't really gain anything here as we're gonna have to make a copy anyway
    //       the only argument (it seems) in favor of owned value is that the API is nicer
    //       as we can construct the args with the `json!` macro without prepending it with `&`
    // FIXME: need to pass SigningRequestOptions struct as 2nd argument
    // pub fn new(request: JsonValue, opts: EncodeOptions) -> Self {
    //     // unimplemented!();
    //     assert!(request.is_object(),
    //             "request data passed when creating a SigningRequest needs to be JSON object");

    //     let request_data = &request.as_object().unwrap();
    //     let request_actions: Vec<JsonValue>;

    //     if let Some(actions) = request_data.get("actions") {
    //         // let actions = &request_data["actions"];
    //         request_actions = actions.as_array().unwrap().to_vec();
    //     }
    //     else if let Some(action) = request_data.get("action") {
    //         request_actions = vec![action.clone()];
    //     }
    //     else {
    //         panic!("could not create SigningRequest with provided data: need to specify one of:\
    //                 `action`, `actions`, `transaction`, `identity`");
    //     }

    //     // let mut result = SigningRequest {
    //     //     actions: request_actions,
    //     //     ..Default::default()
    //     // };
    //     // result.encode_actions(&opts.abi_provider);
    //     // result
    //     todo!()
    // }

    pub fn from_action(action: JsonValue, opts: EncodeOptions) -> Self {
        let mut result = SigningRequest {
            request: Request::Action(action),
            abi_provider: opts.abi_provider,
            ..Default::default()
        };
        result.encode_actions();
        result
    }

    pub fn from_actions(actions: JsonValue, opts: EncodeOptions) -> Self {
        let mut result = SigningRequest {
            request: Request::Actions(actions.as_array().unwrap().to_vec()),
            abi_provider: opts.abi_provider,
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

        if version != 2 && version != 3 {
            return Err(SigningRequestError::InvalidVersion(version));
        }

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

        trace!("uncompressed payload = {}", hex::encode_upper(&dec2));


        let abi = get_signing_request_abi();

        let mut ds = ByteStream::from(dec2);

        abi.decode_variant(&mut ds, T("signing_request"))
            .map_err(|_| SigningRequestError::Invalid(
                "cannot decode SigningRequest from JSON representation".to_owned()))
    }

    pub fn decode<T>(esr: T, opts: &EncodeOptions) -> Result<Self, SigningRequestError>
    where
        T: AsRef<[u8]>
    {
        let payload = Self::decode_payload(esr)?;
        let mut result: Result<Self, SigningRequestError> = payload.try_into();
        if let Ok(ref mut request) = result {
            request.set_abi_provider(opts.abi_provider.clone());
            request.decode_actions();
        }
        result
    }

    pub fn set_abi_provider(&mut self, abi_provider: Rc<dyn ABIProvider>) {
        self.abi_provider = abi_provider;
    }

    pub fn encode_actions(&mut self) {
        let abi_provider = &*self.abi_provider;
        match self.request {
            Request::Action(ref mut action) => {
                warn!("{}", action);
                Self::encode_action(action, abi_provider).unwrap();
            },
            Request::Actions(ref mut actions) => {
                for action in &mut actions[..] {
                    Self::encode_action(action, abi_provider).unwrap();
                }
            },
            _ => todo!(),
        }
    }

    pub fn decode_actions(&mut self) {
        let abi_provider = &*self.abi_provider;
        match self.request {
            Request::Actions(ref mut actions) => {
                for action in &mut actions[..] {
                    Self::decode_action(action, abi_provider).unwrap();
                }
            },
            _ => todo!(),
        }
    }

    fn encode_action(action: &mut JsonValue, abi_provider: &dyn ABIProvider) -> Result<(), SigningRequestError> {
        let is_action_data_encoded = action["data"].is_string();
        if is_action_data_encoded { return Ok(()); }

        let account = conv_action_field_str(action, "account")?;
        let action_name = conv_action_field_str(action, "name")?;
        let data = &action["data"];
        let mut ds = ByteStream::new();
        let abi = abi_provider.get_abi(account)?;
        abi.encode_variant(&mut ds, T(action_name), data).unwrap();
        action["data"] = JsonValue::String(ds.hex_data());
        Ok(())
    }

    fn decode_action(action: &mut JsonValue, abi_provider: &dyn ABIProvider) -> Result<(), SigningRequestError> {
        let is_action_data_encoded = action["data"].is_string();
        if !is_action_data_encoded { return Ok(()); }

        let account     = conv_action_field_str(action, "account")?;
        let action_name = conv_action_field_str(action, "name")?;
        let data        = conv_action_field_str(action, "data")?;
        let mut ds = ByteStream::from(hex::decode(data).unwrap());
        let abi = abi_provider.get_abi(account)?;
        action["data"] = abi.decode_variant(&mut ds, T(action_name)).unwrap();
        Ok(())
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut ds = ByteStream::new();
        let abi = get_signing_request_abi();

        // self.encode_actions();
        let cid = json!(self.chain_id);
        warn!("chain id = {:?}", cid);

        // let sr = json!({
        //     "chain_id": self.chain_id,
        //     "req": ["action[]", self.actions],
        //     "flags": self.flags,
        //     "callback": self.callback.clone().unwrap_or("".to_owned()),
        //     "info": self.info,
        // });
        let sr = json!(self);

        abi.encode_variant(&mut ds, T("signing_request"), &sr).unwrap(); // FIXME: remove this `unwrap`
        ds.into()
    }
}


impl Serialize for SigningRequest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer
    {
        let mut req = serializer.serialize_struct("SigningRequest", 5)?;
        req.serialize_field("chain_id", &self.chain_id)?;
        // let req_data = if self.actions.len() == 1 {
        //     json!(["action", self.actions[0]])
        // }
        // else {
        //     json!(["action[]", self.actions])
        // };
        // req.serialize_field("req", &req_data)?;
        req.serialize_field("req", &self.request)?;
        req.serialize_field("flags", &self.flags)?;
        req.serialize_field("callback", self.callback.as_ref().map_or("", |cb| cb))?;
        req.serialize_field("info", &self.info)?;
        req.end()
    }
}


// FIXME: this would be better as `serde::Deserialize`, right?
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

        let req_type = conv_str(&payload["req"][0])?;
        let req_data = &payload["req"][1];

        result.request = match req_type {
            "action" => Request::Action(req_data.clone()),
            "action[]" => {
                let actions = req_data.as_array().unwrap();
                Request::Actions(actions.to_vec())
            },
            _ => unimplemented!(),
        };

        result.flags = payload["flags"].as_u64().unwrap().try_into().unwrap();
        result.callback = match conv_str(&payload["callback"])? {
            "" => None,
            callback => Some(callback.to_owned()),
        };
        result.info = payload["info"].as_array().unwrap().to_owned();

        Ok(result)
    }
}

#[derive(Error, Debug)]
pub enum SigningRequestError {
    #[error("{0}")]
    Invalid(String),

    #[error("unsupported ESR protocol version: {0}")]
    InvalidVersion(u8),

    #[error("error decoding base64 content")]
    Base64Decode(#[from] base64::DecodeError),

    #[error("hex decoding error")]
    HexDecode(#[from] hex::FromHexError),

    #[error("ABI error")]
    ABI(#[from] InvalidABI),
}

pub fn conv_str(obj: &JsonValue) -> Result<&str, SigningRequestError> {
    obj.as_str().ok_or(SigningRequestError::Invalid(
        format!("Cannot convert object {:?} to str", obj)))
}

pub fn conv_action_field_str<'a>(action: &'a JsonValue, field: &str) -> Result<&'a str, SigningRequestError> {
    action[field].as_str().ok_or(
        SigningRequestError::Invalid(format!("Cannot convert action['{}'] to str, actual type: {:?}",
                                             field, action[field])))
}
