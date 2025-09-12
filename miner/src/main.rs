use anyhow::{Result, anyhow};
use btclib::network::Message;
use std::sync::atomic::Ordering;
use std::{
    sync::{Arc, atomic::AtomicBool},
    thread,
    time::Duration,
};
use tokio::{net::TcpStream, sync::Mutex, time::interval};

use btclib::{crypto::PublicKey, types::Block, util::Saveable};
use clap::Parser;

const ATOMIC_ORDERING: Ordering = Ordering::Relaxed;

#[derive(Parser)]
#[command(author, version, about, long_about=None)]
struct Cli {
    #[arg(short, long)]
    node_address: String,
    #[arg(short, long)]
    public_key_file: String,
}

struct Miner {
    public_key: PublicKey,
    stream: Mutex<TcpStream>,
    current_template: Arc<std::sync::Mutex<Option<Block>>>,
    mining: Arc<AtomicBool>,
    mined_block_sender: flume::Sender<Block>,
    mined_block_receiver: flume::Receiver<Block>,
}

impl Miner {
    async fn new(address: String, public_key: PublicKey) -> Result<Self> {
        let stream = TcpStream::connect(&address).await?;
        let (mined_block_sender, mined_block_receiver) = flume::unbounded();

        Ok(Self {
            public_key,
            stream: Mutex::new(stream),
            current_template: Arc::new(std::sync::Mutex::new(None)),
            mining: Arc::new(AtomicBool::new(false)),
            mined_block_sender,
            mined_block_receiver,
        })
    }

    // todo: multithreaded mining
    async fn run(&self) -> Result<()> {
        let _ = self.spawn_mining_thread()?;
        let mut poll_interval = interval(Duration::from_secs(5));

        loop {
            let receiver_clone = self.mined_block_receiver.clone();

            tokio::select! {
                _ = poll_interval.tick() => self.fetch_and_validate_template().await?,
                Ok(mined_block) = receiver_clone.recv_async() => self.submit_block(mined_block).await?
            }
        }
    }

    fn spawn_mining_thread(&self) -> Result<thread::JoinHandle<()>> {
        let template = self.current_template.clone();
        let mining = self.mining.clone();
        let sender = self.mined_block_sender.clone();

        let handle = thread::spawn(move || {
            loop {
                if let Some(mut block) = template.lock().unwrap().clone() {
                    println!("Mining block with target: {}", block.header.target);

                    if block.header.mine(2_000_000).expect("Error mining block") {
                        println!(
                            "Block mined: {}",
                            block.hash().expect("Error hashing block")
                        );

                        sender.send(block).expect("Failed to send mined block");
                        mining.store(false, ATOMIC_ORDERING);
                    }
                }
            }
        });

        Ok(handle)
    }

    async fn fetch_and_validate_template(&self) -> Result<()> {
        if !self.mining.load(ATOMIC_ORDERING) {
            self.fetch_template().await?;
        } else {
            self.validate_template().await?;
        }

        Ok(())
    }

    async fn fetch_template(&self) -> Result<()> {
        println!("Fetching template");
        let message = Message::FetchTemplate(self.public_key.clone());

        // Request template from node
        let mut stream_lock = self.stream.lock().await;
        message.send_async(&mut *stream_lock).await?;
        drop(stream_lock);

        // Receive response from node
        let mut stream_lock = self.stream.lock().await;
        match Message::receive_async(&mut *stream_lock).await? {
            Message::Template(template) => {
                drop(stream_lock);

                println!("Received template with target: {}", template.header.target);

                *self.current_template.lock().unwrap() = Some(template);
                self.mining.store(true, ATOMIC_ORDERING);

                Ok(())
            }
            m => Err(anyhow!(
                "Unexpected message received when fetching template: {m:?}"
            )),
        }
    }

    async fn validate_template(&self) -> Result<()> {
        // todo: fix clippy lint
        if let Some(template) = self.current_template.lock().unwrap().clone() {
            let message = Message::ValidateTemplate(template);

            let mut stream_lock = self.stream.lock().await;
            message.send_async(&mut *stream_lock).await?;
            drop(stream_lock);

            let mut stream_lock = self.stream.lock().await;

            match Message::receive_async(&mut *stream_lock).await? {
                Message::TemplateValidity(valid) => {
                    drop(stream_lock);

                    if !valid {
                        println!("Template no longer valid");
                        self.mining.store(false, ATOMIC_ORDERING);
                    } else {
                        println!("Template is valid");
                    }

                    Ok(())
                }
                m => Err(anyhow!(
                    "Unexpected message received when validating template: {m:?}"
                )),
            }
        } else {
            Ok(())
        }
    }

    async fn submit_block(&self, block: Block) -> Result<()> {
        println!("Submitting block");
        let message = Message::SubmitTemplate(block);

        let mut stream_lock = self.stream.lock().await;
        message.send_async(&mut *stream_lock).await?;

        self.mining.store(false, ATOMIC_ORDERING);

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let public_key = PublicKey::load_from_file(&cli.public_key_file)
        .map_err(|e| anyhow!("Error reading public key: {e}"))?;
    let miner = Miner::new(cli.node_address, public_key).await?;
    miner.run().await
}
