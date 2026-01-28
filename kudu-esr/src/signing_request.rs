use std::backtrace::Backtrace;
use std::io::prelude::*;
use std::sync::OnceLock;
use std::fmt;

use base64::prelude::*;
use flagset::{flags, FlagSet};
use hex;
use snafu::prelude::*;

use flate2::read::DeflateDecoder;
use serde::{Serialize, Serializer, ser::SerializeStruct};

use kudu::{
    ABIDefinition, ABIError, Action, Bytes, ByteStream, Checksum256, JsonValue, Name,
    SerializeEnum, SerializeError, Transaction, ABI, PermissionLevel,
    abi::ABIProvider, json, with_location,
};

use tracing::{trace, debug, warn};

pub static SIGNER_NAME: Name = Name::from_u64(1);
pub static SIGNER_PERMISSION: Name = Name::from_u64(2);
pub static SIGNER_AUTH: PermissionLevel = PermissionLevel {
    actor: SIGNER_NAME,
    permission: SIGNER_PERMISSION
};

pub static SIGNING_REQUEST_ABI: &str = include_str!("signing_request_abi.json");

pub fn get_signing_request_abi() -> &'static ABI {
    static SR_ABI: OnceLock<ABI> = OnceLock::new();
    SR_ABI.get_or_init(|| {
        ABI::from_definition(&ABIDefinition::from_str(SIGNING_REQUEST_ABI).unwrap()).unwrap()  // safe unwrap
    })
}


// -----------------------------------------------------------------------------
//     ChainId enum - can be an alias or the full chain ID
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, SerializeEnum)]
pub enum ChainId {
    #[serde(rename="chain_alias")]
    Alias(u8),
    #[serde(rename="chain_id")]
    Id(Box<Checksum256>),
}


// -----------------------------------------------------------------------------
//     Request data enum - contains the data in the request
// -----------------------------------------------------------------------------

#[derive(Clone, Debug, SerializeEnum)]
pub enum Request {
    Action(Action),
    #[serde(rename="action[]")]
    Actions(Vec<Action>),
    Transaction(Transaction),
    Identity,
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
    pub fn from_action(action: Action) -> Self {
        SigningRequest {
            request: Request::Action(action),
            ..Default::default()
        }
    }

    pub fn from_action_json(action: &JsonValue) -> Self {
        let action = Action::from_json(action).unwrap();
        SigningRequest::from_action(action)
    }

    pub fn from_actions(actions: Vec<Action>) -> Self {
        SigningRequest {
            request: Request::Actions(actions),
            ..Default::default()
        }
    }

    pub fn from_actions_json(actions: &JsonValue) -> Self {
        let actions = Action::from_json_array(actions).unwrap();
        SigningRequest::from_actions(actions)
    }

    pub fn from_transaction_json(tx: JsonValue) -> Self {
        SigningRequest {
            request: Request::Transaction(Transaction::from_json(&tx).unwrap()),
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

        // extract flags from payload
        let compression = (dec[0] >> 7) == 1u8;
        let version = dec[0] & ((1 << 7) - 1);

        ensure!(version == 2 || version == 3, InvalidVersionSnafu { version });

        debug!(version, compression, "decoding payload");

        // if payload was compressed, decompress it now
        let mut dec2;
        if compression {
            let mut deflater = DeflateDecoder::new(&dec[1..]);
            dec2 = vec![];
            deflater.read_to_end(&mut dec2).context(DeflateSnafu)?;

        }
        else {
            dec2 = dec;
        }
        trace!("decompressed payload = {}", hex::encode(&dec2));


        let abi = get_signing_request_abi();
        let mut ds = ByteStream::from(dec2);
        abi.decode_variant(&mut ds, "signing_request").context(ABISnafu)
    }

    pub fn decode<T>(esr: T, abi_provider: Option<ABIProvider>) -> Result<Self, SigningRequestError>
    where
        T: AsRef<[u8]>
    {
        let payload = Self::decode_payload(esr)?;
        let mut result: Result<Self, SigningRequestError> = Self::try_from_json(abi_provider.as_ref(), payload);
        if let Ok(ref mut request) = result {
            if abi_provider.is_some() {
                request.set_abi_provider(abi_provider);
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

    // TODO: `SigningRequest` should be `ABISerializable` instead of having to go through
    //       its JSON representation first
    pub fn encode(&self) -> Bytes {
        let mut ds = ByteStream::new();
        let abi = get_signing_request_abi();

        // self.encode_actions();
        warn!("chain id = {:?}", json!(self.chain_id));

        let sr = json!(self);
        abi.encode_variant(&mut ds, "signing_request", &sr).unwrap(); // FIXME: remove this `unwrap`
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


impl SigningRequest {
    pub fn to_json(&self) -> JsonValue {
        // let abi_provider = self.abi_provider.as_ref()
        //     .expect("Did not set an ABI provider, required for decoding action data");
        let mut result = json!(self);
        match &self.request {
            Request::Action(action) => {
                result["req"][1]["data"] = action.decode_data().unwrap();  // FIXME: unwrap
            },
            Request::Actions(actions) => {
                for (i, action) in actions.iter().enumerate() {
                    result["req"][1][i]["data"] = action.decode_data().unwrap();
                }
            },
            Request::Transaction(_) => todo!(),
            Request::Identity => todo!(),
        }
        result
    }

    fn try_from_json(abi_provider: Option<&ABIProvider>, payload: JsonValue) -> Result<Self, SigningRequestError> {
        // FIXME: this would be better as `serde::Deserialize`, right?
        let mut result = SigningRequest::default();

        let chain_id = &payload["chain_id"];
        let chain_id_type = conv_str(&chain_id[0])?;

        result.chain_id = match chain_id_type {
            "chain_id" => {
                let data = conv_str(&chain_id[1])?;
                ChainId::Id(Box::new(Checksum256::from_hex(data).context(HexDecodeSnafu)?))
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
            "action" => Request::Action(Action::from_json(req_data).unwrap()),
            "action[]" => {
                let actions = req_data.as_array().unwrap();
                Request::Actions(actions.iter().map(|v| Action::from_json(v).unwrap()).collect())
            },
            _ => unimplemented!(),
        };

        result.flags = FlagSet::<RequestFlags>::new(payload["flags"].as_u64().unwrap().try_into().unwrap()).unwrap();
        result.callback = match conv_str(&payload["callback"])? {
            "" => None,
            callback => Some(callback.to_owned()),
        };
        // result.info = payload["info"].as_array().unwrap().to_owned();
        payload["info"].as_array().unwrap().clone_into(&mut result.info);

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
        source: ABIError,
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
