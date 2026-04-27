use crate::prelude::*;
use edgerun_error::Error;

#[derive(Error, Debug)]
pub enum SigningError {
    Failed(String),
}

pub trait Signer {
    fn sign(&self, message: &[u8]) -> Result<[u8; 64], SigningError>;
    fn pubkey(&self) -> [u8; 32];
}

pub struct Ed25519Signer {
    key: edgerun_crypto::Ed25519SigningKey,
}

impl Ed25519Signer {
    pub fn from_bytes(key: &[u8; 32]) -> Self {
        Self {
            key: edgerun_crypto::Ed25519SigningKey::from_bytes(key),
        }
    }
}

impl Signer for Ed25519Signer {
    fn sign(&self, message: &[u8]) -> Result<[u8; 64], SigningError> {
        use edgerun_crypto::Signer;
        let sig = self.key.sign(message);
        let bytes: [u8; 64] = sig.to_bytes();
        Ok(bytes)
    }

    fn pubkey(&self) -> [u8; 32] {
        self.key.verifying_key().to_bytes()
    }
}
