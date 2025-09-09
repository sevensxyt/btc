// mining reward in bitcoins
pub const INITIAL_REWARD: u64 = 50;
// halving interval in blocks
pub const HALVING_INTERVAL: u64 = 210;
// ideal time to mine block in seconds
pub const IDEAL_BLOCK_TIME: u64 = 10;
// hash target to mine
pub const MIN_TARGET: U256 = U256([
    0xFFFF_FFFF_FFFF_FFFF,
    0xFFFF_FFFF_FFFF_FFFF,
    0xFFFF_FFFF_FFFF_FFFF,
    0x0000_FFFF_FFFF_FFFF,
]);
// block interval count to update difficulty
pub const DIFFICULTY_UPDATE_INTERVAL: u64 = 50;
// maximum mempool transaction age in seconds
pub const MAX_MEMPOOL_TRANSACTION_AGE: u64 = 600;

pub mod crypto;
pub mod error;
pub mod sha256;
pub mod types;
pub mod util;

use serde::{Deserialize, Serialize};
use uint::construct_uint;

construct_uint! {
    #[derive(Serialize, Deserialize)]
    pub struct U256(4);
}
