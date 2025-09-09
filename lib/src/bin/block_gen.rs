use btclib::{
    crypto::PrivateKey,
    sha256::Hash,
    types::{Block, BlockHeader, Transaction, TransactionOutput},
    util::{MerkleRoot, Saveable},
};
use chrono::Utc;
use std::{
    env::{self},
    process::exit,
};
use uuid::Uuid;

fn main() {
    let Some(path) = env::args().nth(1) else {
        eprintln!("Usage: block_gen <block_file>");
        exit(1);
    };

    let private_key = PrivateKey::new_key();
    let transactions = vec![Transaction::new(
        vec![],
        vec![TransactionOutput {
            unique_id: Uuid::new_v4(),
            value: btclib::INITIAL_REWARD * 10u64.pow(8),
            pubkey: private_key.public_key(),
        }],
    )];
    let merkle_root =
        MerkleRoot::calculate(&transactions).expect("failed to calculate merkle root");
    let block = Block::new(
        BlockHeader::new(Utc::now(), 0, Hash::zero(), merkle_root, btclib::MIN_TARGET),
        transactions,
    );

    block.save_to_file(path).expect("Failed to save block");
}
