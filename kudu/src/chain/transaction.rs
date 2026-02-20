use std::sync::Arc;

use bytemuck::cast_ref;
use chrono::ParseError as ChronoParseError;
use hex::FromHexError;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use snafu::{ResultExt, Snafu, ensure};

use crate::{
    ABISerializable, APIClient, Action, ActionError, BlockId, ByteStream, Bytes, ChainId, Checksum256, Extensions, JsonValue, PrivateKey, Signature, TimePointSec, TransactionId, VarUint32, api::HttpError, bitops::endian_reverse_u32, convert::{ConversionError,  variant_to_object, variant_to_str, variant_to_uint}, impl_auto_error_conversion, json, with_location
};

// this is needed to be able to call the `ABISerializable` derive macro, which needs
// access to the `kudu` crate
extern crate self as kudu;

#[with_location]
#[derive(Debug, Snafu)]
// TODO: rename to `InvalidAction` for consistency?
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

    // FIXME!!: we need to implement the `hash` trait manually to ignore the next fields

    #[serde(skip)]
    pub chain_id: Option<ChainId>,

    #[serde(skip)]
    pub client: Option<Arc<APIClient>>,
}

// type DigestType = GenericArray<u8, U32>;
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
        let mut ds = ByteStream::new();
        self.to_bin(&mut ds);
        let hash = sha2::Sha256::digest(ds.data());
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

    pub fn sig_digest(&self, context_free_data: &[u8]) -> DigestType {
        let mut hasher = Sha256::new();
        match &self.chain_id {
            Some(chain_id) => hasher.update(chain_id),
            None => panic!("signing without a chain id!"),   // FIXME: don't panic here
        }

        let mut ds = ByteStream::new();
        self.to_bin(&mut ds);
        hasher.update(ds.data());

        if !context_free_data.is_empty() {
            hasher.update(Sha256::digest(context_free_data));
        }
        else {
            hasher.update([0u8; 32]);  // TODO: replace with Checksum256::zeros()
        }

        let r: [u8; 32] = hasher.finalize().into();
        r.into()
    }

    pub fn get_tapos_info(block: &BlockId) -> (u16, u32) {
        let hash = cast_ref::<[u8; 32], [u64; 4]>(&block.0);
        let ref_block_num = endian_reverse_u32((hash[0] & 0xFFFFFFFF) as u32) as u16;
        let ref_block_prefix = hash[1] as u32;
        (ref_block_num, ref_block_prefix)
    }

    // FIXME: pass by ref or value here?
    pub fn set_reference_block(&mut self, block: &BlockId) {
        let (ref_block_num, ref_block_prefix) = Self::get_tapos_info(block);
        self.ref_block_num = ref_block_num;
        self.ref_block_prefix = ref_block_prefix;
    }

    pub fn link(&mut self, client: Arc<APIClient>) -> Result<&mut Self, TransactionError> {
        let info = client.get("/v1/chain/get_info").context(NetworkSnafu { message: "cannot get chain info".to_string() })?;

        let block_id = info["last_irreversible_block_id"].as_str().unwrap();
        let block_id = BlockId::from_hex(block_id).unwrap();

        // set reference block info
        let (ref_block_num, ref_block_prefix) = Self::get_tapos_info(&block_id);
        self.ref_block_num = ref_block_num;
        self.ref_block_prefix = ref_block_prefix;

        // set chain id
        let chain_id = info["chain_id"].as_str().unwrap().to_owned();
        self.chain_id = Some(ChainId::from_hex(&chain_id).context(InvalidChainIdSnafu { chain_id })?);

        // set expiration time
        let expiration_delay_seconds = 120;
        let head_block_time: TimePointSec = info["head_block_time"].as_str().unwrap().parse()?;
        self.expiration = head_block_time + expiration_delay_seconds;

        // save client for sending later
        self.client = Some(client);

        Ok(self)
    }

    pub fn get_signature(&self, signing_key: &PrivateKey) -> Result<Signature, TransactionError> {
        ensure!(self.chain_id.is_some(),
                UnlinkedTransactionSnafu { message: "cannot sign transaction" });

        let context_free_data = b"";
        Ok(signing_key.sign_digest(self.sig_digest(context_free_data)))
    }

    pub fn sign(&self, signing_key: &PrivateKey) -> Result<SignedTransaction, TransactionError> {
        let sig = self.get_signature(signing_key)?;
        Ok(SignedTransaction {
            tx: self.clone(),
            signatures: vec![sig],
            compression: false,
            packed_content_free_data: Bytes::new(),
        })
    }

}

// FIXME: we implement this manually as we don't have a way yet to ignore fields using the derive macro
// FIXME: implement this, using #[serde(skip)] to decide whether to skip fields
impl kudu::ABISerializable for Transaction {
    fn to_bin(&self, s: &mut kudu::ByteStream) {
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
    fn from_bin(s: &mut kudu::ByteStream) -> ::core::result::Result<Self, kudu::SerializeError> {
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

// FIXME!! implement Serialize properly, we can't really derive it
#[derive(Eq, Hash, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct SignedTransaction {
    pub tx: Transaction,
    pub signatures: Vec<Signature>,
    pub compression: bool,
    pub packed_content_free_data: Bytes,
}

impl SignedTransaction {
    pub fn send(&self) -> Result<JsonValue, TransactionError> {
        let signed_tx = self.to_json();
        let result = self.tx.client.as_ref().unwrap()
            .call("/v1/chain/push_transaction", &signed_tx)
            .context(NetworkSnafu { message: format!("Could not push transaction: {}", &signed_tx) })?;

        Ok(result)
    }

    // FIXME: implement `Serialize` instead!!
    fn to_json(&self) -> JsonValue {
        // FIXME: this transcode is not an ergonomic API
        let tx_json = json::to_string(&self.tx).unwrap();
        let mut signed_tx: JsonValue = json::from_str(&tx_json).unwrap();
        signed_tx["signatures"] = json!(&self.signatures);
        signed_tx["compression"] = json!(self.compression);
        signed_tx["packed_content_free_data"] = json!(self.packed_content_free_data);  // FIXME! review
        let mut s = ByteStream::new();
        self.tx.to_bin(&mut s);
        signed_tx["packed_trx"] = JsonValue::String(s.hex_data());
        signed_tx
    }
}


#[cfg(test)]
mod tests {
    use color_eyre::eyre::Result;
    use crate::{json, Bytes, IntoPermissionVec, Name, PrivateKey};

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
    #[ignore]
    fn test_signing() -> Result<()> {
        let client = Arc::new(APIClient::local());
        let action = Action {
            account: Name::new("eosio.token")?,
            name: Name::new("transfer")?,
            authorization: ("alice", "active").into_permission_vec(),
            data: Bytes::new(),
        }.with_data(&json!({
            "from": "alice",
            "to": "bob",
            "quantity": "1.000 SON",
            "memo": "yep!",
        }));

        let mut tx = Transaction::new(vec![action]);
        tx.link(client.clone())?;
        println!("tx: {}", json::to_string(&tx)?);


        let signing_key = PrivateKey::new("5JEc9CzLAx48Utvn7mo4y6hhmSVj7n4zgDNJx2KNZo3gSBr8Fet")?;

        let digest = tx.sig_digest(b"");
        let sig = signing_key.sign_digest(digest);

        // FIXME: this transcode is not an ergonomic API
        let tx_json = json::to_string(&tx)?;
        let mut signed_tx: JsonValue = json::from_str(&tx_json)?;
        signed_tx["signatures"] = json!([sig]);
        signed_tx["compression"] = json!(false);
        signed_tx["packed_content_free_data"] = json!("");  // FIXME!
        let mut s = ByteStream::new();
        tx.to_bin(&mut s);
        signed_tx["packed_trx"] = JsonValue::String(s.hex_data());



        println!("signed_tx: {}", &signed_tx);

        let result = client.call("/v1/chain/push_transaction", &signed_tx)?;

        println!("result: {result}");
        assert!(result["transaction_id"].as_str().is_some());
        Ok(())
    }
}
