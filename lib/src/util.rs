use serde::{Deserialize, Serialize};

use crate::{sha256::Hash, types::Transaction};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MerkleRoot(Hash);
impl MerkleRoot {
    pub fn calculate(transactions: &[Transaction]) -> Option<Self> {
        if transactions.is_empty() {
            return None;
        }

        let mut layer: Vec<_> = transactions.iter().map(Hash::hash).collect();

        while layer.len() > 1 {
            layer = layer
                .chunks(2)
                .map(|pair| {
                    let left = pair.first().unwrap();
                    let right = pair.get(1).unwrap_or(left);
                    Hash::hash(&[left, right])
                })
                .collect()
        }

        Some(Self(layer[0]))
    }
}
