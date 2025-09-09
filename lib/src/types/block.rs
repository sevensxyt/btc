use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    U256,
    error::{BtcError, Result},
    sha256::Hash,
    types::transaction::{Transaction, TransactionOutput},
    util::MerkleRoot,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
}

impl Block {
    pub fn new(header: BlockHeader, transactions: Vec<Transaction>) -> Self {
        Self {
            header,
            transactions,
        }
    }

    pub fn hash(&self) -> Result<Hash> {
        Hash::hash(self)
    }

    pub fn verify_transactions(
        &self,
        predicted_block_height: u64,
        utxos: &HashMap<Hash, (bool, TransactionOutput)>,
    ) -> Result<()> {
        if self.transactions.is_empty() {
            return Err(BtcError::InvalidTransaction);
        }

        let _ = self.verify_coinbase_transaction(predicted_block_height, utxos);

        let mut inputs: HashMap<Hash, TransactionOutput> = HashMap::new();

        for transaction in self.transactions.iter().skip(1) {
            let input_value: u64 = transaction
                .inputs
                .iter()
                .map(|input| {
                    // error if input does not come from some previous utxo
                    let Some(prev_output) = utxos.get(&input.prev_transaction_output_hash) else {
                        return Err(BtcError::InvalidTransaction);
                    };

                    // error on double spend
                    if inputs.contains_key(&input.prev_transaction_output_hash) {
                        return Err(BtcError::InvalidTransaction);
                    }

                    if !input
                        .signature
                        .verify(&input.prev_transaction_output_hash, &prev_output.1.pubkey)
                    {
                        return Err(BtcError::InvalidSignature);
                    }

                    inputs.insert(input.prev_transaction_output_hash, prev_output.1.clone());
                    Ok(prev_output.1.value)
                })
                .collect::<Result<Vec<_>>>()?
                .iter()
                .sum();

            let output_value = transaction.outputs.iter().map(|output| output.value).sum();

            if input_value < output_value {
                return Err(BtcError::InvalidTransaction);
            }
        }

        Ok(())
    }

    fn verify_coinbase_transaction(
        &self,
        predicted_block_height: u64,
        utxos: &HashMap<Hash, (bool, TransactionOutput)>,
    ) -> Result<()> {
        let Some(coinbase_transaction) = self.transactions.first() else {
            return Err(BtcError::InvalidBlock);
        };

        if coinbase_transaction.inputs.is_empty() || coinbase_transaction.outputs.is_empty() {
            return Err(BtcError::InvalidTransaction);
        }

        let miner_fees = self.calculate_miner_fees(utxos)?;
        let block_reward = self.calcualte_block_reward(predicted_block_height);
        let total_coinbase_outputs: u64 = coinbase_transaction
            .outputs
            .iter()
            .map(|output| output.value)
            .sum();

        if total_coinbase_outputs != block_reward + miner_fees {
            return Err(BtcError::InvalidTransaction);
        }

        Ok(())
    }

    fn calcualte_block_reward(&self, predicted_block_height: u64) -> u64 {
        // * 10 ^ 8 converts BTC to satoshies
        crate::INITIAL_REWARD * 10u64.pow(8)
        // block rewards halve on every halving interval
            / 2u64.pow((predicted_block_height / crate::HALVING_INTERVAL) as u32)
    }

    fn calculate_miner_fees(
        &self,
        utxos: &HashMap<Hash, (bool, TransactionOutput)>,
    ) -> Result<u64> {
        let mut inputs: HashMap<Hash, TransactionOutput> = HashMap::new();
        let mut outputs: HashMap<Hash, TransactionOutput> = HashMap::new();

        for transction in self.transactions.iter().skip(1) {
            for input in &transction.inputs {
                let Some(prev_output) = utxos.get(&input.prev_transaction_output_hash) else {
                    return Err(BtcError::InvalidTransaction);
                };

                if inputs.contains_key(&input.prev_transaction_output_hash) {
                    return Err(BtcError::InvalidTransaction);
                }

                inputs.insert(input.prev_transaction_output_hash, prev_output.1.clone());
            }

            for output in &transction.outputs {
                let hash = output.hash()?;
                if outputs.contains_key(&hash) {
                    return Err(BtcError::InvalidTransaction);
                }

                outputs.insert(hash, output.clone());
            }
        }

        let input_value: u64 = inputs.values().map(|input| input.value).sum();
        let output_value: u64 = outputs.values().map(|output| output.value).sum();

        Ok(input_value - output_value)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BlockHeader {
    pub timestamp: DateTime<Utc>,
    pub nonce: u64,
    pub prev_block_hash: Hash,
    pub merkle_root: MerkleRoot,
    pub target: U256,
}

impl BlockHeader {
    pub fn new(
        timestamp: DateTime<Utc>,
        nonce: u64,
        prev_block_hash: Hash,
        merkle_root: MerkleRoot,
        target: U256,
    ) -> Self {
        Self {
            timestamp,
            nonce,
            prev_block_hash,
            merkle_root,
            target,
        }
    }

    pub fn hash(&self) -> Result<Hash> {
        Hash::hash(self)
    }

    pub fn mine(&mut self, steps: usize) -> Result<bool> {
        if self.hash()?.matches_target(self.target) {
            return Ok(true);
        }

        for _ in 0..steps {
            if let Some(nonce) = self.nonce.checked_add(1) {
                self.nonce = nonce
            } else {
                // reset nonce and timestamp if nonce overflows, also causes hash of header ot change
                self.nonce = 0;
                self.timestamp = Utc::now()
            }

            if self.hash()?.matches_target(self.target) {
                return Ok(true);
            }
        }

        Ok(true)
    }
}
