use std::path::Path;

use anyhow::Result;
use argh::FromArgs;

mod handler;
mod util;

use btclib::types::Blockchain;
use dashmap::DashMap;
use static_init::dynamic;
use tokio::{net::TcpStream, sync::RwLock};

#[dynamic]
pub static BLOCKCHAIN: RwLock<Blockchain> = RwLock::new(Blockchain::new());

#[dynamic]
pub static NODES: DashMap<String, TcpStream> = DashMap::new();

#[derive(FromArgs)]
/// My toy blockchain node
struct Args {
    #[argh(option, default = "9000")]
    /// port number
    port: u16,

    #[argh(option, default = "String::from(\".blockchain.cbor\")")]
    /// blockchain file path
    blockchain_file: String,

    #[argh(positional)]
    /// addresses of initial nodes
    nodes: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Args = argh::from_env();
    let port = args.port;
    let blockchain_file = args.blockchain_file;
    let nodes = args.nodes;

    if Path::new(&blockchain_file).exists() {
        util::load_blockchain(&blockchain_file).await?;
    } else {
        println!("blockchain file is missing!");
        if nodes.is_empty() {
            println!("starting nodes are empty, starting as a seed node");
        } else {
            util::populate_connection(&nodes).await?;
            println!("total nodes: {}", NODES.len());

            if nodes.is_empty() {
                println!("starting nodes are empty, starting as a seed node");
            } else {
                let (longest_name, longest_count) = util::find_longest_chain_node().await?;
                util::dowload_blockchain(&longest_name, longest_count).await?;
                println!("blockchain downloaded from {longest_name}");

                // limit rwlock scope to within block
                // lock is released as blockchain goes out of scope
                {
                    let mut blockchain = BLOCKCHAIN.write().await;
                    let _ = blockchain.rebuild_utxos();
                }
                {
                    let mut blockchain = BLOCKCHAIN.write().await;
                    blockchain.try_adjust_target();
                }
            }
        }
    }
    Ok(())
}
