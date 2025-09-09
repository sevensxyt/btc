use btclib::{types::Transaction, util::Saveable};
use std::{env, fs::File, process::exit};

fn main() {
    let Some(path) = env::args().nth(1) else {
        println!("Usage: tx_print <tx_file>");
        exit(1);
    };

    if let Ok(file) = File::open(path) {
        let transaction = Transaction::load(file).expect("failed to load transaction from file");
        println!("{transaction:#?}");
        exit(1);
    };
}
