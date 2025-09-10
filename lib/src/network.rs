use serde::{Deserialize, Serialize};
use std::io::{Error as IoError, Read, Write};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::{
    crypto::PublicKey,
    types::{Block, Transaction, TransactionOutput},
};

#[derive(Debug, Clone, Deserialize, Serialize)]
// Messages exist in three forms:
// 1. Request (1-1)
// 2. Response (1-1)
// 3. Broadcast (1-N)
pub enum Message {
    // Request: Fetch all UTXOs belonging to a public key
    FetchUTXOs(PublicKey),
    // Response: List of UTXOs belonging to the public key, true if marked
    UTXOs(Vec<(TransactionOutput, bool)>),

    // Request: Send a transaction to the network
    SubmitTransaction(Transaction),
    // Broadcast: A new transaction
    NewTransaction(Transaction),

    // Request: Node should prepate optimal block template with coinbase tx paying the public key
    FetchTemplate(PublicKey),
    // Response: Template
    Template(Block),

    // Request: Node should validate the template to prevent an invalid block from being mined
    ValidateTemplate(Block),
    // Response: Validity of the template
    TemplateValidity(Block),

    // Request: Submit a mined block to the node
    SubmitTemplate(Block),

    // Request: Ask for all nodes that a node is connected to
    DiscoverNodes,
    // Request: List of nodes
    NodeList(Vec<String>),

    // Request: Ask for difference between self and target height
    AskDifference(u32),
    // Request: Difference in height
    Difference(i32),

    // Reuest: Ask node to send a block with specific height
    FetchBlock(usize),

    // Broadcast: A new block
    NewBlock(Block),
}

impl Message {
    pub fn encode(&self) -> Result<Vec<u8>, ciborium::ser::Error<IoError>> {
        let mut bytes = Vec::new();
        ciborium::into_writer(self, &mut bytes)?;

        Ok(bytes)
    }

    pub fn decode(data: &[u8]) -> Result<Self, ciborium::de::Error<IoError>> {
        ciborium::from_reader(data)
    }

    pub fn send(&self, stream: &mut impl Write) -> Result<(), ciborium::ser::Error<IoError>> {
        let bytes = self.encode()?;
        let length = bytes.len() as u64;

        stream.write_all(&length.to_be_bytes())?;
        stream.write_all(&bytes)?;

        Ok(())
    }

    pub fn receive(&self, stream: &mut impl Read) -> Result<(), ciborium::de::Error<IoError>> {
        let mut length_bytes = [0u8; 8];
        stream.read_exact(&mut length_bytes)?;
        let length = u64::from_be_bytes(length_bytes) as usize;

        let mut data = vec![0u8; length];
        stream.read_exact(&mut data)?;

        Ok(())
    }

    pub async fn send_async(
        &self,
        stream: &mut (impl AsyncWrite + Unpin),
    ) -> Result<(), ciborium::ser::Error<IoError>> {
        let bytes = self.encode()?;
        let length = bytes.len() as u64;

        stream.write_all(&length.to_be_bytes()).await?;
        stream.write_all(&bytes).await?;

        Ok(())
    }

    pub async fn receive_async(
        &self,
        stream: &mut (impl AsyncRead + Unpin),
    ) -> Result<(), ciborium::ser::Error<IoError>> {
        let mut length_bytes = [0u8; 8];
        stream.read_exact(&mut length_bytes).await?;
        let length = u64::from_be_bytes(length_bytes) as usize;

        let mut data = vec![0u8; length];
        stream.read_exact(&mut data).await?;

        Ok(())
    }
}
