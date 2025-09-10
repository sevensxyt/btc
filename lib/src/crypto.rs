use crate::{sha256::Hash, util::Saveable};
use ecdsa::{
    Signature as ECDSASignature, SigningKey, VerifyingKey,
    signature::{SignerMut, Verifier, rand_core::OsRng},
};
use k256::Secp256k1;
use serde::{Deserialize, Serialize};
use spki::{DecodePublicKey, EncodePublicKey};
use std::io::{Error as IoError, ErrorKind as IoErrorKind, Result as IoResult};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Signature(pub ECDSASignature<Secp256k1>);
impl Signature {
    pub fn sign_output(output_hash: &Hash, private_key: &mut PrivateKey) -> Self {
        let signature = private_key.0.sign(&output_hash.as_bytes());
        Signature(signature)
    }

    pub fn verify(&self, output_hash: &Hash, public_key: &PublicKey) -> bool {
        public_key
            .0
            .verify(&output_hash.as_bytes(), &self.0)
            .is_ok()
    }
}
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct PublicKey(pub VerifyingKey<Secp256k1>);
impl Saveable for PublicKey {
    fn load<I: std::io::Read>(mut reader: I) -> IoResult<Self> {
        let mut buf = String::new();
        reader.read_to_string(&mut buf)?;
        let public_key: VerifyingKey<Secp256k1> = VerifyingKey::from_public_key_pem(&buf)
            .map_err(|_| IoError::new(IoErrorKind::InvalidData, "Failed to parse public key"))?;
        Ok(PublicKey(public_key))
    }

    fn save<O: std::io::Write>(&self, mut writer: O) -> IoResult<()> {
        let s = self.0.to_public_key_pem(Default::default()).map_err(|_| {
            IoError::new(IoErrorKind::InvalidData, "Failed to serialise public key")
        })?;
        writer.write_all(s.as_bytes())?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PrivateKey(#[serde(with = "signkey_serde")] pub SigningKey<Secp256k1>);
impl PrivateKey {
    pub fn new_key() -> Self {
        Self(SigningKey::random(&mut OsRng))
    }

    pub fn public_key(&self) -> PublicKey {
        PublicKey(*self.0.verifying_key())
    }
}

impl Default for PrivateKey {
    fn default() -> Self {
        Self::new_key()
    }
}

impl Saveable for PrivateKey {
    fn load<I: std::io::Read>(reader: I) -> IoResult<Self> {
        ciborium::de::from_reader(reader).map_err(|_| {
            IoError::new(
                IoErrorKind::InvalidData,
                "Failed to deserialise private key",
            )
        })
    }

    fn save<O: std::io::Write>(&self, writer: O) -> IoResult<()> {
        ciborium::ser::into_writer(self, writer).map_err(|_| {
            IoError::new(
                IoErrorKind::InvalidData,
                "Failed to deserialise private key",
            )
        })
    }
}

mod signkey_serde {
    use serde::Deserialize;

    pub fn serialize<S>(
        key: &super::SigningKey<super::Secp256k1>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&key.to_bytes())
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<super::SigningKey<super::Secp256k1>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes = Vec::<u8>::deserialize(deserializer)?;
        Ok(super::SigningKey::from_slice(&bytes).unwrap())
    }
}
