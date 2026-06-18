// SPDX-FileCopyrightText: 2025, 2026 DigiGaia SCCL
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use bytemuck::cast_ref;
use chrono::ParseError as ChronoParseError;
use hex::FromHexError;
use serde::{
    Deserialize, Serialize,
    ser::{Serializer, SerializeMap}
};
use sha2::{Sha256, Digest};
use snafu::{OptionExt, ResultExt, Snafu, ensure};

use crate::{
    ABISerializable, APIClient, Action, ActionError, BlockId, Bytes, ChainId,
    Checksum256, Extensions, JsonValue, PrivateKey, Signature, TimePointSec, TransactionId,
    VarUint32,
    api::HttpError,
    bitops::endian_reverse_u32,
    convert::{ConversionError,  variant_to_object, variant_to_str, variant_to_uint},
    impl_auto_error_conversion, json, with_location
};

// this is needed to be able to call the `ABISerializable` derive macro, which needs
// access to the `kudu` crate
extern crate self as kudu;

#[with_location]
#[derive(Debug, Snafu)]
pub enum TransactionError {
    #[snafu(display("unknown field: '{field}'"))]
    UnknownField { field: String },

    #[snafu(display("cannot convert field to required target type"))]
    Conversion { source: ConversionError },

    #[snafu(display("cannot parse date/time"))]
    DateTimeParse { source: ChronoParseError },

    #[snafu(display("invalid action"))]
    InvalidAction { source: ActionError },

    #[snafu(display("unlinked transaction: {message}"))]
    UnlinkedTransaction { message: String },

    #[snafu(display("network error: {message}\ndetails: {source}"))]
    NetworkError { message: String, source: HttpError  },

    #[snafu(display("invalid chain id: {chain_id}"))]
    InvalidChainId { chain_id: String, source: FromHexError },

    #[snafu(display("could not match JSON object to transaction"))]
    FromJson { source: serde_json::Error },

    #[snafu(display("Nodeos error: {message}"))]
    NodeosError { message: String },
}

impl_auto_error_conversion!(ChronoParseError, TransactionError, DateTimeParseSnafu);
impl_auto_error_conversion!(ConversionError, TransactionError, ConversionSnafu);
impl_auto_error_conversion!(ActionError, TransactionError, InvalidActionSnafu);
impl_auto_error_conversion!(serde_json::Error, TransactionError, FromJsonSnafu);


#[derive(Eq, Hash, PartialEq, Debug, Clone, Default, Serialize, Deserialize)]
pub struct Transaction {
    // -----------------------------------------------------------------------------
    //     TransactionHeader fields
    // -----------------------------------------------------------------------------

    /// The time at which a transaction expires.
    pub expiration: TimePointSec,
    /// Specifies a block num in the last 2^16 blocks.
    pub ref_block_num: u16,
    /// Specifies the lower 32 bits of the block id.
    pub ref_block_prefix: u32,
    /// Upper limit on total network bandwidth (in 8 byte words) billed for this transaction.
    pub max_net_usage_words: VarUint32,
    /// Upper limit on the total CPU time billed for this transaction.
    pub max_cpu_usage_ms: u8,
    /// Number of seconds to delay this transaction for during which it may be canceled.
    pub delay_sec: VarUint32,

    // -----------------------------------------------------------------------------
    //     Transaction fields
    // -----------------------------------------------------------------------------

    pub context_free_actions: Vec<Action>,
    pub actions: Vec<Action>,
    pub transaction_extensions: Extensions,

    // -----------------------------------------------------------------------------
    //     Optional, convenience fields
    //       these do not get serialized
    // -----------------------------------------------------------------------------

    #[serde(skip)]
    pub chain_id: Option<ChainId>,

    #[serde(skip)]
    pub client: Option<Arc<APIClient>>,
}

type DigestType = Checksum256;


impl Transaction {
    pub fn new(actions: Vec<Action>) -> Transaction {
        Transaction {
            expiration: 0.into(),
            ref_block_num: 0,
            ref_block_prefix: 0,
            max_net_usage_words: VarUint32(0),
            max_cpu_usage_ms: 0,
            delay_sec: VarUint32(0),
            context_free_actions: vec![],
            actions,
            transaction_extensions: vec![],
            chain_id: None,
            client: None
        }
    }
    pub fn id(&self) -> TransactionId {
        let mut data = Bytes::new();
        self.to_bin(&mut data);
        let hash = sha2::Sha256::digest(&data);
        let r: [u8; 32] = hash.into();
        r.into()
    }

    /// Create a new `Transaction` from a JSON value containing the non-default fields.
    /// You should make sure that the necessary ABIs are properly loaded in the registry
    /// if the data fields for the `Actions` are not encoded yet, it is unnecessary otherwise.
    pub fn from_json(tx: &JsonValue) -> Result<Transaction, TransactionError> {
        let mut result = Transaction::default();
        for (field, value) in variant_to_object(tx)?.iter() {
            match field.as_str() {
                "expiration"           => result.expiration           = variant_to_str(value)?.parse()?,
                "ref_block_num"        => result.ref_block_num        = variant_to_uint(value)?,
                "ref_block_prefix"     => result.ref_block_prefix     = variant_to_uint(value)?,
                "max_cpu_usage_ms"     => result.max_cpu_usage_ms     = variant_to_uint(value)?,
                "max_net_usage_words"  => result.max_net_usage_words  = variant_to_uint::<u32>(value)?.into(),
                "delay_sec"            => result.delay_sec            = variant_to_uint::<u32>(value)?.into(),
                "context_free_actions" => result.context_free_actions = Action::from_json_array(value)?,
                "actions"              => result.actions              = Action::from_json_array(value)?,
                "transaction_extensions" => result.transaction_extensions = serde_json::from_value(value.clone())?,
                other => UnknownFieldSnafu { field: other }.fail()?,
            }
        }
        Ok(result)
    }

    pub fn sig_digest(&self, context_free_data: &[u8]) -> Result<DigestType, TransactionError> {
        let mut hasher = Sha256::new();
        match &self.chain_id {
            Some(chain_id) => hasher.update(chain_id),
            None => UnlinkedTransactionSnafu { message: "you need a chain ID to be set to compute the tx digest".to_string() }.fail()?,
        }

        let mut ds = Bytes::new();
        self.to_bin(&mut ds);
        hasher.update(&ds);

        if !context_free_data.is_empty() {
            hasher.update(Sha256::digest(context_free_data));
        }
        else {
            hasher.update([0u8; 32]);  // TODO: replace with Checksum256::zeros()
        }

        let r: [u8; 32] = hasher.finalize().into();
        Ok(r.into())
    }

    fn get_tapos_info(block: &BlockId) -> (u16, u32) {
        let hash = cast_ref::<[u8; 32], [u64; 4]>(&block.0);
        let ref_block_num = endian_reverse_u32((hash[0] & 0xFFFFFFFF) as u32) as u16;
        let ref_block_prefix = hash[1] as u32;
        (ref_block_num, ref_block_prefix)
    }

    pub fn link(&mut self, client: Arc<APIClient>) -> Result<&mut Self, TransactionError> {
        let info = client.get("/v1/chain/get_info").context(NetworkSnafu { message: "cannot get chain info".to_string() })?;

        let block_id = info["last_irreversible_block_id"].as_str()
            .context(NodeosSnafu { message: "chain info 'last_irreversible_block_id' is not a string" })?;
        let block_id = BlockId::from_hex(block_id)
            .map_err(|_| NodeosSnafu { message: "chain info 'last_irreversible_block_id' is not an hex value" }.build())?;

        // set reference block info
        let (ref_block_num, ref_block_prefix) = Self::get_tapos_info(&block_id);
        self.ref_block_num = ref_block_num;
        self.ref_block_prefix = ref_block_prefix;

        // set chain id
        let chain_id = info["chain_id"].as_str()
            .context(NodeosSnafu { message:  "chain info 'chain_id' is not a string" })?;
        self.chain_id = Some(ChainId::from_hex(chain_id).context(InvalidChainIdSnafu { chain_id })?);

        // set expiration time
        let expiration_delay_seconds = 120;
        let head_block_time: TimePointSec = info["head_block_time"].as_str()
            .context(NodeosSnafu { message:  "chain info 'head_block_time' is not a string" })?
            .parse()?;
        self.expiration = head_block_time + expiration_delay_seconds;

        // save client for sending later
        self.client = Some(client);

        Ok(self)
    }

    fn get_signature(&self, signing_key: &PrivateKey) -> Result<Signature, TransactionError> {
        let context_free_data = b"";  // TODO: support this
        Ok(signing_key.sign_digest(self.sig_digest(context_free_data)?))
    }

    pub fn sign(&self, signing_key: &PrivateKey) -> Result<SignedTransaction, TransactionError> {
        ensure!(self.chain_id.is_some(),
                UnlinkedTransactionSnafu { message: "cannot sign transaction" });

        let sig = self.get_signature(signing_key)?;
        Ok(SignedTransaction {
            tx: self.clone(),
            signatures: vec![sig],
            compression: false,
            packed_content_free_data: Bytes::new(),
        })
    }

}

// TODO: we implement this manually as we don't have a way yet to ignore fields using the derive macro
// TODO: implement this, using #[serde(skip)] to decide whether to skip fields
impl kudu::ABISerializable for Transaction {
    fn to_bin(&self, s: &mut kudu::Bytes) {
        // transaction header
        self.expiration.to_bin(s);
        self.ref_block_num.to_bin(s);
        self.ref_block_prefix.to_bin(s);
        self.max_net_usage_words.to_bin(s);
        self.max_cpu_usage_ms.to_bin(s);
        self.delay_sec.to_bin(s);
        // transaction body
        self.context_free_actions.to_bin(s);
        self.actions.to_bin(s);
        self.transaction_extensions.to_bin(s);
    }
    fn from_bin(s: &mut kudu::ByteStreamView) -> ::core::result::Result<Self, kudu::SerializeError> {
        Ok(Self {
            // transaction header
            expiration: TimePointSec::from_bin(s)?,
            ref_block_num: u16::from_bin(s)?,
            ref_block_prefix: u32::from_bin(s)?,
            max_net_usage_words: VarUint32::from_bin(s)?,
            max_cpu_usage_ms: u8::from_bin(s)?,
            delay_sec: VarUint32::from_bin(s)?,
            // transaction body
            context_free_actions: Vec::<Action>::from_bin(s)?,
            actions: Vec::<Action>::from_bin(s)?,
            transaction_extensions: Extensions::from_bin(s)?,
            // optional
            // FIXME!!: we need to give proper values here
            chain_id: None,
            client: None,
        })
    }
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct SignedTransaction {
    pub tx: Transaction,
    pub signatures: Vec<Signature>,
    pub compression: bool,
    pub packed_content_free_data: Bytes,
}

impl SignedTransaction {
    pub fn send(&self) -> Result<JsonValue, TransactionError> {
        let signed_tx = json!(self);
        let result = self.tx.client.as_ref().unwrap()  // safe unwrap: a SignedTransaction is necessarily linked
            .call("/v1/chain/push_transaction", &signed_tx)
            .context(NetworkSnafu { message: format!("Could not push transaction: {}", &signed_tx) })?;

        Ok(result)
    }

    pub fn send_unchecked(&self) -> Result<JsonValue, TransactionError> {
        let signed_tx = json!(self);
        let result = self.tx.client.as_ref().unwrap()  // safe unwrap: a SignedTransaction is necessarily linked
            .call_unchecked("/v1/chain/push_transaction", &signed_tx)
            .context(NetworkSnafu { message: format!("Could not push transaction: {}", &signed_tx) })?;

        Ok(result)
    }
}

// NOTE: we implement `Serialize` manually but we can't implement `Deserialize` as we
//       haven't serialized the `chain_id` field and so can't restore it.
impl Serialize for SignedTransaction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        let mut map = serializer.serialize_map(Some(9 + 4))?;
        // transaction header
        map.serialize_entry("signatures", &self.signatures)?;
        map.serialize_entry("compression", &self.compression)?;
        map.serialize_entry("packed_content_free_data", &self.packed_content_free_data)?;

        let mut s = Bytes::new();
        self.tx.to_bin(&mut s);
        map.serialize_entry("packed_trx", &s.to_hex())?;

        map.end()
    }
}


#[cfg(test)]
mod tests {
    use color_eyre::eyre::Result;

    use crate::{chain::Transfer, Name};
    use super::*;



    #[test]
    fn test_tapos() -> Result<()> {
        let block_id = Checksum256::from_hex("0eeb31a70905138203051bf848fc7176336a0eb41d078338460af949d8cf2abd")?;
        let (ref_block_num, ref_block_prefix) = Transaction::get_tapos_info(&block_id);
        assert_eq!(ref_block_num, 12711);
        assert_eq!(ref_block_prefix, 4162520323);
        Ok(())
    }

    #[test]
    fn test_sign_transaction() -> Result<()> {

        let transfer = Transfer {
            from: Name::new("useraaaaaaaa")?,
            to: Name::new("useraaaaaaab")?,
            quantity: "0.0001 SYS".try_into()?,
            memo: "".into(),
        };

        let mut tx = Transaction {
            expiration: "2009-02-13T23:31:31.000".parse()?,
            ref_block_num: 1234,
            ref_block_prefix: 5678,
            actions: vec![
                Action::new(("useraaaaaaaa", "active"), &transfer),
            ],
            ..Default::default()
        };

        assert_eq!(json!(tx), json!({
            "ref_block_num": 1234,
            "ref_block_prefix": 5678,
            "expiration": "2009-02-13T23:31:31.000",
            "max_net_usage_words": 0,
            "max_cpu_usage_ms": 0,
            "delay_sec": 0,
            "context_free_actions": [],
            "actions": [{
                "account": "eosio.token",
                "name": "transfer",
                "authorization": [{"actor":"useraaaaaaaa","permission":"active"}],
                "data": "608c31c6187315d6708c31c6187315d60100000000000000045359530000000000"
            }],
            "transaction_extensions": []
        }));

        let signing_key = PrivateKey::eosio_dev();
        tx.chain_id = Some(Checksum256::from_hex(crate::config::JUNGLE_CHAIN_ID)?);
        let sig = tx.get_signature(&signing_key)?;

        let signed_tx = SignedTransaction {
            tx: tx.clone(),
            signatures: vec![sig],
            compression: false,
            packed_content_free_data: Bytes::new(),
        };

        assert_eq!(json!(signed_tx), json!({
            "signatures": ["SIG_K1_K18qEA2qTqVj153ZKriMnnRwHpLuENX7bp9UYs5AJsRWhgD6diPgMeoebwRRFQuvyicDsgwVYTt3g4GsG5FxCXM3WNZVN7"],
            "compression": false,
            "packed_content_free_data": "",
            "packed_trx": "d3029649d2042e160000000000000100a6823403ea3055000000572d3ccdcd01608c31c6187315d600000000a8ed323221608c31c6187315d6708c31c6187315d6010000000000000004535953000000000000",
        }));

        Ok(())
    }
}
