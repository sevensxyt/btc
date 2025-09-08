use std::collections::{HashMap, HashSet};

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    U256,
    crypto::{PublicKey, Signature},
    error::{BtcError, Result},
    sha256::Hash,
    util::MerkleRoot,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Blockchain {
    pub utxos: HashMap<Hash, TransactionOutput>,
    pub target: U256,
    pub blocks: Vec<Block>,
}

impl Blockchain {
    pub fn new() -> Self {
        Self {
            utxos: HashMap::new(),
            blocks: vec![],
            target: crate::MIN_TARGET,
        }
    }

    pub fn block_height(&self) -> u64 {
        self.blocks.len() as u64
    }

    pub fn add_block(&mut self, block: Block) -> Result<()> {
        if self.blocks.is_empty() {
            if block.header.prev_block_hash != Hash::zero() {
                println!("zero hash");
                return Err(BtcError::InvalidBlock);
            }
        } else {
            let prev_block = self.blocks.last().ok_or(BtcError::InvalidBlock)?;
            if block.header.prev_block_hash != prev_block.hash()? {
                println!("prev hash does not match");
                return Err(BtcError::InvalidHash);
            }

            if !block.header.hash()?.matches_target(block.header.target) {
                println!("target does not match");
                return Err(BtcError::InvalidBlock);
            }

            let merkle_root =
                MerkleRoot::calculate(&block.transactions).ok_or(BtcError::InvalidMerkleRoot)?;
            if merkle_root != block.header.merkle_root {
                println!("invalid merkle root");
                return Err(BtcError::InvalidMerkleRoot);
            }

            if block.header.timestamp <= prev_block.header.timestamp {
                return Err(BtcError::InvalidBlock);
            }

            block.verify_transactions(self.block_height(), &self.utxos)?;
        }

        let block_transactions: HashSet<_> = block
            .transactions
            .iter()
            .map(|transaction| transaction.hash())
            .collect::<Result<HashSet<_>>>()?;

        // self.mempool.retain(|(_, transaction)| {
        //     !block_transactions.contains(&tx.hash())
        // });
        self.try_adjust_target();
        self.blocks.push(block);

        Ok(())
    }

    pub fn rebuild_utxos(&mut self) -> Result<()> {
        for block in &self.blocks {
            for transaction in &block.transactions {
                // old utxos have been spent
                for input in &transaction.inputs {
                    self.utxos.remove(&input.prev_transaction_output_hash);
                }

                // create new utxos
                for output in &transaction.outputs {
                    self.utxos.insert(transaction.hash()?, output.clone());
                }
            }
        }
        Ok(())
    }

    fn try_adjust_target(&mut self) {
        // let N = block count interval to update difficulty
        // return early if N blocks have not passed
        if self.blocks.is_empty() {
            return;
        }

        if self.blocks.len() % crate::DIFFICULTY_UPDATE_INTERVAL as usize != 0 {
            return;
        }

        let start_time = self.blocks
            [self.blocks.len() - crate::DIFFICULTY_UPDATE_INTERVAL as usize]
            .header
            .timestamp;
        let end_time = self.blocks.last().unwrap().header.timestamp;
        let time_diff = (end_time - start_time).num_seconds();

        // target_seconds represents the ideal duration to mine N blocks
        let target_seconds = crate::IDEAL_BLOCK_TIME * crate::DIFFICULTY_UPDATE_INTERVAL;
        let target = BigDecimal::parse_bytes(self.target.to_string().as_bytes(), 10)
            .expect("This shouldn't happen");
        // if time_diff is shorter than expected, mining is too fast, reduce target to make more difficult
        // and vice versa
        let new_target = target * (BigDecimal::from(time_diff) / BigDecimal::from(target_seconds));
        let new_target_str = new_target
            .to_string()
            .split(".")
            .next()
            .expect("uh oh")
            .to_string();
        let new_target = U256::from_str_radix(&new_target_str, 10).expect("uh oh");
        let new_target = new_target.clamp(self.target / 4, self.target * 4);

        self.target = new_target.min(crate::MIN_TARGET);
    }
}

impl Default for Blockchain {
    fn default() -> Self {
        Self::new()
    }
}

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

    fn verify_transactions(
        &self,
        predicted_block_height: u64,
        utxos: &HashMap<Hash, TransactionOutput>,
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
                        .verify(&input.prev_transaction_output_hash, &prev_output.pubkey)
                    {
                        return Err(BtcError::InvalidSignature);
                    }

                    inputs.insert(input.prev_transaction_output_hash, prev_output.clone());
                    Ok(prev_output.value)
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
        utxos: &HashMap<Hash, TransactionOutput>,
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

    fn calculate_miner_fees(&self, utxos: &HashMap<Hash, TransactionOutput>) -> Result<u64> {
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

                inputs.insert(input.prev_transaction_output_hash, prev_output.clone());
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
