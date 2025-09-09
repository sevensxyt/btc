use btclib::{
    crypto::PrivateKey,
    types::{Transaction, TransactionOutput},
    util::Saveable,
};
use std::{env, process::exit};
use uuid::Uuid;

fn main() {
    let Some(path) = env::args().nth(1) else {
        println!("Usage: tx_gen <tx_file>");
        exit(1);
    };

    let private_key = PrivateKey::new_key();
    let transaction = Transaction::new(
        vec![],
        vec![TransactionOutput {
            unique_id: Uuid::new_v4(),
            value: btclib::INITIAL_REWARD * 10u64.pow(8),
            pubkey: private_key.public_key(),
        }],
    );
    transaction
        .save_to_file(path)
        .expect("Failed to save transaction");
}
