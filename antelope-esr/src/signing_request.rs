use std::backtrace::Backtrace;
use std::io::prelude::*;
use std::fmt;

use base64::prelude::*;
use flagset::{flags, FlagSet};
use hex;
use snafu::prelude::*;

use flate2::read::DeflateDecoder;
use serde::{Serialize, Serializer, ser::SerializeTuple, ser::SerializeStruct};

use antelope_macros::with_location;
use antelope_core::{convert::hex_to_boxed_array, JsonValue, Name, json};
use antelope_abi::{
    ByteStream, SerializeError,
    abidefinition::TypeNameRef as T,
    provider::{get_signing_request_abi, ABIProvider, InvalidABI},
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
    Transaction(JsonValue),
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
            Request::Transaction(tx) => {
                tup.serialize_element("transaction")?;
                tup.serialize_element(tx)?;
            },
            Request::Identity => todo!(),
        }
        tup.end()
    }
}

// -----------------------------------------------------------------------------
//     Request flags definition
// -----------------------------------------------------------------------------

flags! {
    pub enum RequestFlags : u8 {
        Broadcast,
        Background,
    }
}

// =============================================================================
//
//     SigningRequest main struct
//
// =============================================================================

// FIXME: do we need to derive Clone? do we want to?
// #[derive(Clone)]
pub struct SigningRequest {
    pub chain_id: ChainId,
    pub request: Request,
    pub flags: FlagSet<RequestFlags>,
    pub callback: Option<String>,
    pub info: Vec<JsonValue>, // TODO: consider getting something more precise

    abi_provider: Option<ABIProvider>,
}

impl fmt::Debug for SigningRequest {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("SigningRequest")
           .field("chain_id", &self.chain_id)
           .field("request", &self.request)
           .field("flags", &self.flags.bits())
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
            flags: RequestFlags::Broadcast.into(),
            callback: None,
            info: vec![],

            abi_provider: None,
        }
    }
}

// -----------------------------------------------------------------------------
//     EncodeOptions
// -----------------------------------------------------------------------------

// TODO: use builder pattern to create instances of this struct
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
    // TODO: is there any case where we would want a & instead of an owned value?
    //       however we can't move stuff out of if because of shared references, so we
    //       don't really gain anything here as we're gonna have to make a copy anyway
    //       the only argument (it seems) in favor of owned value is that the API is nicer
    //       as we can construct the args with the `json!` macro without prepending it with `&`

     pub fn from_action(action: JsonValue) -> Self {
        SigningRequest {
            request: Request::Action(action),
            ..Default::default()
        }
    }

    pub fn from_actions(actions: JsonValue) -> Self {
        SigningRequest {
            request: Request::Actions(actions.as_array().unwrap().to_vec()),
            ..Default::default()
        }
    }

    pub fn from_transaction(tx: JsonValue) -> Self {
        // set default values if missing
        let mut tx = tx.as_object().unwrap().clone();
        tx.entry("expiration")            .or_insert("1970-01-01T00:00:00.000".into());
        tx.entry("ref_block_num")         .or_insert(json!(0));
        tx.entry("ref_block_prefix")      .or_insert(json!(0));
        tx.entry("max_cpu_usage_ms")      .or_insert(json!(0));
        tx.entry("max_net_usage_words")   .or_insert(json!(0));
        tx.entry("delay_sec")             .or_insert(json!(0));
        tx.entry("context_free_actions")  .or_insert(json!([]));
        tx.entry("actions")               .or_insert(json!([]));
        tx.entry("transaction_extensions").or_insert(json!([]));
        tx.entry("context_free_data")     .or_insert(json!([]));  // FIXME: needed? wanted?

        SigningRequest {
            request: Request::Transaction(JsonValue::Object(tx)),
            ..Default::default()
        }
    }

    pub fn from_uri(uri: &str) -> Result<Self, SigningRequestError> {
        ensure!(uri.starts_with("esr://"), InvalidURISnafu { uri });
        let payload = &uri[6..];
        warn!("payload: {}", payload);
        Self::decode(payload, None)
    }

    pub fn decode_payload<T: AsRef<[u8]>>(esr: T) -> Result<JsonValue, SigningRequestError> {
        let content = String::from_utf8(esr.as_ref().to_vec()).unwrap();
        let dec = BASE64_URL_SAFE_NO_PAD.decode(esr).context(Base64DecodeSnafu { content: content.clone() })?;
        ensure!(!dec.is_empty(), InvalidSnafu { msg: format!("base64-decoded payload {content} is empty") });

        let compression = (dec[0] >> 7) == 1u8;
        let version = dec[0] & ((1 << 7) - 1);

        ensure!(version == 2 || version == 3, InvalidVersionSnafu { version });

        debug!(version, compression, "decoding payload");

        let mut dec2;
        if compression {
            let mut deflater = DeflateDecoder::new(&dec[1..]);
            dec2 = vec![];
            deflater.read_to_end(&mut dec2).context(DeflateSnafu)?;

        }
        else {
            dec2 = dec;
        }

        trace!("uncompressed payload = {}", hex::encode_upper(&dec2));


        let abi = get_signing_request_abi();

        let mut ds = ByteStream::from(dec2);

        abi.decode_variant(&mut ds, T("signing_request")).context(JsonDecodeSnafu)
    }

    pub fn decode<T>(esr: T, abi_provider: Option<ABIProvider>) -> Result<Self, SigningRequestError>
    where
        T: AsRef<[u8]>
    {
        let payload = Self::decode_payload(esr)?;
        let mut result: Result<Self, SigningRequestError> = payload.try_into();
        if let Ok(ref mut request) = result {
            if abi_provider.is_some() {
                request.set_abi_provider(abi_provider);
                request.decode_actions();
            }
        }
        result
    }

    pub fn set_abi_provider(&mut self, abi_provider: Option<ABIProvider>) {
        self.abi_provider = abi_provider;
    }

    pub fn with_abi_provider(self, abi_provider: ABIProvider) -> Self {
        let mut req = self;
        req.set_abi_provider(Some(abi_provider));
        req
    }

    pub fn with_callback(self, callback: &str, background: bool) -> Self {
        let mut req = self;
        req.callback = Some(callback.to_owned());
        if background {
            req.flags |= RequestFlags::Background;
        }
        else {
            req.flags -= RequestFlags::Background;
        }
        req
    }

    pub fn with_broadcast(self, broadcast: bool) -> Self {
        let mut req = self;
        if broadcast { req.flags |= RequestFlags::Broadcast }
        else         { req.flags -= RequestFlags::Broadcast }
        req
    }

    pub fn encode_actions(&mut self) {
        let abi_provider = self.abi_provider.as_ref(); // do not unwrap the Option here, only do it when needed
        const ERROR_MSG: &str = "No ABIProvider has been set for the signing request, cannot encode actions";

        match self.request {
            Request::Action(ref mut action) => {
                if !Self::is_action_encoded(action) {
                    Self::encode_action(action, abi_provider.expect(ERROR_MSG)).unwrap();
                }
            },
            Request::Actions(ref mut actions) => {
                for action in &mut actions[..] {
                    if !Self::is_action_encoded(action) {
                        Self::encode_action(action, abi_provider.expect(ERROR_MSG)).unwrap();
                    }
                }
            },
            Request::Transaction(ref mut tx) => {
                for action in tx["actions"].as_array_mut().unwrap() {
                    if !Self::is_action_encoded(action) {
                        Self::encode_action(action, abi_provider.expect(ERROR_MSG)).unwrap();
                    }
                }
            },
            _ => todo!(),
        }
    }

    pub fn decode_actions(&mut self) {
        let abi_provider = self.abi_provider.as_ref();
        const ERROR_MSG: &str = "No ABIProvider has been set for the signing request, cannot decode actions";

        match self.request {
            Request::Actions(ref mut actions) => {
                for action in &mut actions[..] {
                    Self::decode_action(action, abi_provider.expect(ERROR_MSG)).unwrap();
                }
            },
            Request::Action(ref mut action) => {
                Self::decode_action(action, abi_provider.expect(ERROR_MSG)).unwrap();
            },
            _ => todo!(),
        }
    }

    fn is_action_encoded(action: &JsonValue) -> bool {
        action["data"].is_string()
    }

    fn encode_action(action: &mut JsonValue, abi_provider: &ABIProvider) -> Result<(), SigningRequestError> {
        if Self::is_action_encoded(action) { return Ok(()); }

        let account = conv_action_field_str(action, "account")?;
        let action_name = conv_action_field_str(action, "name")?;
        let data = &action["data"];
        let mut ds = ByteStream::new();
        let abi = abi_provider.get_abi(account).context(ABISnafu)?;
        abi.encode_variant(&mut ds, T(action_name), data).unwrap();
        action["data"] = JsonValue::String(ds.hex_data());
        Ok(())
    }

    fn decode_action(action: &mut JsonValue, abi_provider: &ABIProvider) -> Result<(), SigningRequestError> {
        if !Self::is_action_encoded(action) { return Ok(()); }

        let account     = conv_action_field_str(action, "account")?;
        let action_name = conv_action_field_str(action, "name")?;
        let data        = conv_action_field_str(action, "data")?;
        let mut ds = ByteStream::from(hex::decode(data).unwrap());
        let abi = abi_provider.get_abi(account).context(ABISnafu)?;
        action["data"] = abi.decode_variant(&mut ds, T(action_name)).unwrap();
        Ok(())
    }

    pub fn encode(&mut self) -> Vec<u8> {
        let mut ds = ByteStream::new();
        let abi = get_signing_request_abi();

        self.encode_actions();
        let cid = json!(self.chain_id);
        warn!("chain id = {:?}", cid);

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
        req.serialize_field("req", &self.request)?;
        req.serialize_field("flags", &self.flags.bits())?;
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
                ChainId::Id(hex_to_boxed_array(data).context(HexDecodeSnafu)?)
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

        result.flags = FlagSet::<RequestFlags>::new(payload["flags"].as_u64().unwrap().try_into().unwrap()).unwrap();
        result.callback = match conv_str(&payload["callback"])? {
            "" => None,
            callback => Some(callback.to_owned()),
        };
        result.info = payload["info"].as_array().unwrap().to_owned();

        Ok(result)
    }
}


#[with_location]
#[derive(Debug, Snafu)]
pub enum SigningRequestError {
    #[snafu(display("{msg}"))]
    Invalid {
        msg: String,
        backtrace: Backtrace,
    },

    #[snafu(display("unsupported ESR protocol version: {version}"))]
    InvalidVersion {
        version: u8,
        backtrace: Backtrace,
    },

    #[snafu(display("not a valid ESR URI: {uri}"))]
    InvalidURI {
        uri: String,
    },

    #[snafu(display("can not decompress (deflate) payload data"))]
    Deflate {
        source: std::io::Error,
    },

    #[snafu(display("error decoding base64 content: {content}"))]
    Base64Decode {
        content: String,
        source: base64::DecodeError,
    },

    #[snafu(display("cannot decode SigningRequest from JSON representation"))]
    JsonDecode {
        source: SerializeError,
    },

    #[snafu(display("hex decoding error"))]
    HexDecode {
        source: hex::FromHexError,
    },

    #[snafu(display("ABI error"))]
    ABI {
        source: InvalidABI,
    },

}

pub fn conv_str(obj: &JsonValue) -> Result<&str, SigningRequestError> {
    obj.as_str().context(InvalidSnafu {
        msg: format!("Cannot convert object {:?} to str", obj)
    })
}

pub fn conv_action_field_str<'a>(action: &'a JsonValue, field: &str) -> Result<&'a str, SigningRequestError> {
    action[field].as_str().context(InvalidSnafu {
        msg: format!("Cannot convert action['{}'] to str, actual type: {:?}", field, action[field])
    })
}
