use anyhow::Result;
use btclib::{network::Message, types::Blockchain, util::Saveable};
use tokio::net::TcpStream;

pub async fn load_blockchain(blockchain_file: &str) -> Result<()> {
    println!("loading blockchain from file.. (questionable, I know)");
    let new_blockchain = Blockchain::load_from_file(blockchain_file)?;
    println!("blockchain loaded!");

    let mut blockchain = crate::BLOCKCHAIN.write().await;
    *blockchain = new_blockchain;

    println!("rebuilding utxos...");
    blockchain.rebuild_utxos()?;
    println!("utxos rebuild");

    println!("adjusting target...");
    println!("current target: {}", blockchain.target());
    blockchain.try_adjust_target();
    println!("new target: {}", blockchain.target());

    println!("blockchain initialisation complete!");
    Ok(())
}

pub async fn populate_connection(nodes: &[String]) -> Result<()> {
    println!("connecting to other nodes...");

    for node in nodes {
        let mut stream = TcpStream::connect(node).await?;

        let message = Message::DiscoverNodes;
        message.send_async(&mut stream).await?;
        println!("sent discover nodes message to {node}");

        let message = Message::receive_async(&mut stream).await?;
        match message {
            Message::NodeList(neighours) => {
                println!("received node list from {node}");

                for neighbour in neighours {
                    println!("adding node {neighbour}");
                    let stream = TcpStream::connect(&neighbour).await?;
                    crate::NODES.insert(neighbour, stream);
                }
            }
            m => println!("unexpected message from {node}: {m:?}"),
        }

        crate::NODES.insert(node.clone(), stream);
    }
    Ok(())
}

pub async fn find_longest_chain_node() -> Result<(String, u32)> {
    Ok((String::new(), 0))
}

pub async fn dowload_blockchain(longest_name: &str, longest_count: u32) -> Result<()> {
    Ok(())
}
