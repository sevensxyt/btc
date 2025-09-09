use btclib::{types::Block, util::Saveable};
use std::{
    env::{self},
    fs::File,
};

fn main() {
    let path = if let Some(arg) = env::args().nth(1) {
        arg
    } else {
        println!("Usage: block_print <block_file>");
        std::process::exit(1);
    };

    if let Ok(file) = File::open(path) {
        let block = Block::load(file).expect("failed to load block from file");
        println!("{block:#?}");
    };
}
