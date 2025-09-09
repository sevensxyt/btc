use std::{
    fs::File,
    io::{Read, Result as IoResult, Write},
    path::Path,
};

use serde::{Deserialize, Serialize};

use crate::{
    error::{BtcError, Result},
    sha256::Hash,
    types::Transaction,
};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MerkleRoot(Hash);
impl MerkleRoot {
    pub fn calculate(transactions: &[Transaction]) -> Option<Self> {
        if transactions.is_empty() {
            return None;
        }

        let mut layer = transactions
            .iter()
            .map(Hash::hash)
            .collect::<Result<Vec<_>>>()
            .ok()?;

        while layer.len() > 1 {
            layer = layer
                .chunks(2)
                .map(|pair| {
                    let left = pair.first().ok_or(BtcError::InvalidTransaction)?;
                    let right = pair.get(1).ok_or(BtcError::InvalidTransaction)?;
                    Hash::hash(&[left, right])
                })
                .collect::<Result<Vec<Hash>>>()
                .ok()?;
        }

        Some(Self(layer[0]))
    }
}

pub trait Saveable
where
    Self: Sized,
{
    fn load<I: Read>(reader: I) -> IoResult<Self>;
    fn save<O: Write>(&self, writer: O) -> IoResult<()>;
    fn load_from_file<P: AsRef<Path>>(path: P) -> IoResult<Self> {
        Self::load(File::open(&path)?)
    }
    fn save_to_file<P: AsRef<Path>>(&self, path: P) -> IoResult<()> {
        self.save(File::create(&path)?)
    }
}
