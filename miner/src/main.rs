use std::{env, process::exit};

use btclib::{types::Block, util::Saveable};

fn main() {
    let (Some(path), Some(steps)) = (env::args().nth(1), env::args().nth(2)) else {
        exit(1);
    };

    let Ok(steps) = steps.parse::<usize>() else {
        eprint!("<steps> should be a positive integer");
        exit(1);
    };

    let original_block = Block::load_from_file(path).expect("Failed to load block");
    let mut block = original_block.clone();

    while !block.header.mine(steps).expect("Failed to mine block") {
        println!("minining...");
    }

    println!("original: {original_block:#?}");
    println!(
        "hash: {}",
        original_block
            .hash()
            .expect("Failed to hash original block")
    );

    println!("final: {block:#?}");
    println!(
        "hash: {}",
        block.hash().expect("Failed to hash original block")
    );
}
