pub const INITIAL_REWARD: u64 = 50;
pub const HALVING_INTERVAL: u64 = 210;
pub const IDEAL_BLOCK_TIME: u64 = 10;
pub const MIN_TARGET: U256 = U256([
    0xFFFF_FFFF_FFFF_FFFF,
    0xFFFF_FFFF_FFFF_FFFF,
    0xFFFF_FFFF_FFFF_FFFF,
    0x0000_FFFF_FFFF_FFFF,
]);
pub const DIFFICULTY_UPDATE_INTERVAL: u64 = 50;

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
