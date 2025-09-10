use std::{
    env::{self},
    process::exit,
};

use btclib::{crypto::PublicKey, network::Message, types::Block, util::Saveable};
use clap::Parser;
use tokio::net::TcpStream;

#[derive(Parser)]
#[command(author, version, about, long_about=None)]
struct Cli {
    #[arg(short, long)]
    address: String,
    #[arg(short, long)]
    public_key_file: String,
}

struct Miner;
impl Miner {
    async fn new(address: String, public_key: PublicKey) -> ! {
        unimplemented!()
    }

    async fn run(&self) -> ! {
        unimplemented!()
    }

    fn spawn_mining_thread(&self) -> ! {
        unimplemented!()
    }

    async fn fetch_and_validate_template(&self) -> ! {
        unimplemented!()
    }

    async fn fetch_template(&self) -> ! {
        unimplemented!()
    }

    async fn validate_template(&self) -> ! {
        unimplemented!()
    }

    async fn submit_block(&self, block: Block) -> ! {
        unimplemented!()
    }
}

fn usage() -> ! {
    eprintln!(
        "Usage: {} <address> <public_key_file>",
        env::args().next().unwrap()
    );
    exit(1)
}

#[tokio::main]
async fn main() {
    let address = env::args().nth(1).unwrap_or_else(|| usage());
    let public_key_file = env::args().nth(2).unwrap_or_else(|| usage());
    let Ok(public_key) = PublicKey::load_from_file(&public_key_file) else {
        eprint!("Error reading public key from file {public_key_file}");
        exit(1);
    };

    println!("Connecting to {address} to mine with {public_key:?}");
    let Ok(mut stream) = TcpStream::connect(&address).await else {
        eprintln!("Failed to connect to server at {address}");
        exit(1);
    };

    println!("requesting work from {address}");
    let message = Message::FetchTemplate(public_key);
    // message.send(&mut stream);
    // todo: asynd send and receive
}
