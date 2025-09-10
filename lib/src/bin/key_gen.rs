use btclib::{crypto::PrivateKey, util::Saveable};
use std::env;

fn main() {
    let name = env::args().nth(1).expect("Please provide a name");

    let private_key = PrivateKey::new_key();
    let public_key = private_key.public_key();

    let public_key_file = name.clone() + ".pub.pem";
    let private_key_file = name + ".priv.cbor";

    private_key
        .save_to_file(&private_key_file)
        .expect("Error saving private key file");
    public_key
        .save_to_file(&public_key_file)
        .expect("Error saving public key file");
}
