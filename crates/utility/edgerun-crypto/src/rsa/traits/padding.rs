//! Supported padding schemes.

use alloc::vec::Vec;

use crate::rand_core::CryptoRngCore;

use crate::rsa::errors::Result;
use crate::rsa::key::{RsaPrivateKey, RsaPublicKey};

/// Digital signature scheme.
pub trait SignatureScheme {
    /// Sign the given digest.
    fn sign<Rng: CryptoRngCore>(
        self,
        rng: Option<&mut Rng>,
        priv_key: &RsaPrivateKey,
        hashed: &[u8],
    ) -> Result<Vec<u8>>;

    /// Verify a signed message.
    ///
    /// `hashed` must be the result of hashing the input using the hashing function
    /// passed in through `hash`.
    ///
    /// If the message is valid `Ok(())` is returned, otherwise an `Err` indicating failure.
    fn verify(self, pub_key: &RsaPublicKey, hashed: &[u8], sig: &[u8]) -> Result<()>;
}
