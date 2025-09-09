use serde::{Deserialize, Serialize};
use std::io::{Error as IoError, ErrorKind as IoErrorKind};
use uuid::Uuid;

use crate::{
    crypto::{PublicKey, Signature},
    error::Result,
    sha256::Hash,
    util::Saveable,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Transaction {
    pub inputs: Vec<TransactionInput>,
    pub outputs: Vec<TransactionOutput>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionInput {
    pub prev_transaction_output_hash: Hash,
    pub signature: Signature,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionOutput {
    pub value: u64,
    pub unique_id: Uuid,
    pub pubkey: PublicKey,
}

impl TransactionOutput {
    pub fn hash(&self) -> Result<Hash> {
        Hash::hash(self)
    }
}

impl Transaction {
    pub fn new(inputs: Vec<TransactionInput>, outputs: Vec<TransactionOutput>) -> Self {
        Self { inputs, outputs }
    }

    pub fn hash(&self) -> Result<Hash> {
        Hash::hash(self)
    }
}

impl Saveable for Transaction {
    fn load<I: std::io::Read>(reader: I) -> std::io::Result<Self> {
        ciborium::de::from_reader(reader).map_err(|_| {
            IoError::new(
                IoErrorKind::InvalidData,
                "Failed to deserialise transaction",
            )
        })
    }
    fn save<O: std::io::Write>(&self, writer: O) -> std::io::Result<()> {
        ciborium::ser::into_writer(self, writer)
            .map_err(|_| IoError::new(IoErrorKind::InvalidData, "Failed to serialise transaction"))
    }
}
