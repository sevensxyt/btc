use std::collections::{HashMap, HashSet};

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::io::{Error as IoError, ErrorKind as IoErrorKind};

use crate::{
    U256,
    error::{BtcError, Result},
    sha256::Hash,
    types::{Block, Transaction, TransactionOutput},
    util::{MerkleRoot, Saveable},
};

const UNEXPECTED_BUG: &str = "uh oh";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Blockchain {
    utxos: HashMap<Hash, (bool, TransactionOutput)>,
    target: U256,
    blocks: Vec<Block>,
    #[serde(default, skip_serializing)]
    // bitcoin's eviction policy is 72 hours, but we'll use 600 seconds here
    mempool: Vec<(DateTime<Utc>, Transaction)>,
}

impl Blockchain {
    pub fn new() -> Self {
        Self {
            utxos: HashMap::new(),
            blocks: vec![],
            mempool: vec![],
            target: crate::MIN_TARGET,
        }
    }

    pub fn utxos(&self) -> &HashMap<Hash, (bool, TransactionOutput)> {
        &self.utxos
    }

    pub fn target(&self) -> U256 {
        self.target
    }

    pub fn blocks(&self) -> impl Iterator<Item = &Block> {
        self.blocks.iter()
    }

    pub fn mempool(&self) -> &[(DateTime<Utc>, Transaction)] {
        &self.mempool
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

        // hard to use retain with the result type :(
        let mut new_mempool: Vec<(DateTime<Utc>, Transaction)> = vec![];
        for (datetime, transaction) in self.mempool() {
            let hash = transaction.hash()?;
            if !block_transactions.contains(&hash) {
                new_mempool.push((*datetime, transaction.clone()));
            }
        }
        self.mempool = new_mempool;

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
                    self.utxos
                        .insert(transaction.hash()?, (false, output.clone()));
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
        let target =
            BigDecimal::parse_bytes(self.target.to_string().as_bytes(), 10).expect(UNEXPECTED_BUG);

        // if time_diff is shorter than expected, mining is too fast, reduce target to make more difficult
        // and vice versa
        let new_target = target * (BigDecimal::from(time_diff) / BigDecimal::from(target_seconds));
        let new_target_str = new_target
            .to_string()
            .split(".")
            .next()
            .expect(UNEXPECTED_BUG)
            .to_string();
        let new_target = U256::from_str_radix(&new_target_str, 10).expect(UNEXPECTED_BUG);
        let new_target = new_target.clamp(self.target / 4, self.target * 4);

        self.target = new_target.min(crate::MIN_TARGET);
    }

    pub fn add_to_mempool(&mut self, transaction: Transaction) -> Result<()> {
        // validate inputs
        // input must come from a know utxo and be unique to prevent double spends
        let mut inputs = HashSet::new();
        for input in &transaction.inputs {
            if !self.utxos.contains_key(&input.prev_transaction_output_hash) {
                println!("UTXO not found");
                dbg!(&self.utxos());
                return Err(BtcError::InvalidTransaction);
            };

            if inputs.contains(&input.prev_transaction_output_hash) {
                println!("non-unique input");
                return Err(BtcError::InvalidTransaction);
            }

            inputs.insert(input.prev_transaction_output_hash);
        }

        // when more than one mempool transaction references the same utxo, let the latest one win, and evict the previous one
        for input in &transaction.inputs {
            // utxo is marked as true when it is being spent my some transaction in the mempool
            if let Some((true, _)) = self.utxos().get(&input.prev_transaction_output_hash) {
                // Find transaction that has an output matching our input's hash
                let referencing_transaction =
                    self.mempool()
                        .iter()
                        .enumerate()
                        .find(|(_, (_, transaction))| {
                            transaction.outputs.iter().any(|output| {
                                output
                                    .hash()
                                    .is_ok_and(|hash| hash == input.prev_transaction_output_hash)
                            })
                        });

                if let Some((i, (_, transaction))) = referencing_transaction {
                    // remove the earlier transaction, mark all of its utxo outputs as unused
                    // clone to resolve borrow checker :(
                    for input in transaction.inputs.clone() {
                        self.utxos
                            .entry(input.prev_transaction_output_hash)
                            .and_modify(|(marked, _)| *marked = false);
                    }
                    self.mempool.remove(i);
                } else {
                    self.utxos
                        .entry(input.prev_transaction_output_hash)
                        .and_modify(|(marked, _)| *marked = false);
                }
            }
        }

        let inputs: u64 = transaction
            .inputs
            .iter()
            .map(|input| {
                self.utxos
                    .get(&input.prev_transaction_output_hash)
                    .expect(UNEXPECTED_BUG)
                    .1
                    .value
            })
            .sum();
        let outputs: u64 = transaction.outputs.iter().map(|output| output.value).sum();

        if inputs < outputs {
            println!("inputs lower than outputs");
            return Err(BtcError::InvalidTransaction);
        }

        // mark utxos referenced by transactions as used
        for input in &transaction.inputs {
            self.utxos
                .entry(input.prev_transaction_output_hash)
                .and_modify(|(marked, _)| {
                    *marked = true;
                });
        }

        self.mempool.push((Utc::now(), transaction));
        self.mempool.sort_by_key(|(_, transaction)| {
            let inputs: u64 = transaction
                .inputs
                .iter()
                .map(|input| {
                    self.utxos
                        .get(&input.prev_transaction_output_hash)
                        .expect(UNEXPECTED_BUG)
                        .1
                        .value
                })
                .sum();

            let outputs: u64 = transaction.outputs.iter().map(|output| output.value).sum();

            #[allow(clippy::let_and_return)]
            let miner_fee = inputs - outputs;
            miner_fee
        });
        Ok(())
    }

    pub fn cleanup_mempool(&mut self) -> Result<()> {
        let now = Utc::now();
        let mut utxo_hashes_to_unmark: Vec<Hash> = vec![];

        self.mempool.retain(|(datetime, transaction)| {
            if now - datetime > chrono::Duration::seconds(crate::MAX_MEMPOOL_TRANSACTION_AGE as i64)
            {
                utxo_hashes_to_unmark.extend(
                    transaction
                        .inputs
                        .iter()
                        .map(|input| input.prev_transaction_output_hash),
                );
                false
            } else {
                true
            }
        });

        for hash in utxo_hashes_to_unmark {
            self.utxos
                .entry(hash)
                .and_modify(|(marked, _)| *marked = false);
        }

        Ok(())
    }
}

impl Default for Blockchain {
    fn default() -> Self {
        Self::new()
    }
}

impl Saveable for Blockchain {
    fn load<I: std::io::Read>(reader: I) -> std::io::Result<Self> {
        ciborium::de::from_reader(reader)
            .map_err(|_| IoError::new(IoErrorKind::InvalidData, "Failed to deserialise blockchain"))
    }
    fn save<O: std::io::Write>(&self, writer: O) -> std::io::Result<()> {
        ciborium::ser::into_writer(self, writer)
            .map_err(|_| IoError::new(IoErrorKind::InvalidData, "Failed to serialise blockchain"))
    }
}
