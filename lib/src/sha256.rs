use std::fmt;

use serde::{Deserialize, Serialize};
use sha256::digest;

use crate::{
    U256,
    error::{BtcError, Result},
};

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
pub struct Hash(U256);
impl Hash {
    pub fn hash<T: serde::Serialize>(data: &T) -> Result<Self> {
        let mut serialized: Vec<u8> = vec![];

        if let Err(e) = ciborium::into_writer(data, &mut serialized) {
            panic!("Failed to serialise data: {e:?}")
        }

        let hash = digest(&serialized);
        let Ok(hash_bytes) = hex::decode(hash) else {
            return Err(BtcError::InvalidHash);
        };

        let hash_array: [u8; 32] = hash_bytes
            .as_slice()
            .try_into()
            .map_err(|_| BtcError::InvalidHash)?;

        Ok(Hash(U256::from_little_endian(&hash_array)))
    }

    pub fn matches_target(&self, target: U256) -> bool {
        self.0 <= target
    }

    pub fn zero() -> Self {
        Hash(U256::zero())
    }

    pub fn as_bytes(&self) -> [u8; 32] {
        self.0.to_little_endian()
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:x}", self.0)
    }
}
