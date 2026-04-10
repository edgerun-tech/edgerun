//! ECDH key exchange using P-256 (secp256r1) from the workspace p256 crate.
//! All randomness flows through edgerun-core::crypto::fill_random.

use edgerun_crypto::p256::ecdh::EphemeralSecret;
use edgerun_crypto::rand_core::{CryptoRng, RngCore};
use edgerun_crypto::p256::EncodedPoint;

/// ECDH key pair using P-256
pub struct EcdhKeyPair {
    secret: EphemeralSecret,
    public: EncodedPoint,
}

impl EcdhKeyPair {
    /// Generate a new P-256 ECDH key pair
    pub fn generate() -> Result<Self, String> {
        let secret = EphemeralSecret::random(&mut CoreRng);
        let public = EncodedPoint::from(secret.public_key());
        Ok(EcdhKeyPair { secret, public })
    }

    /// Raw public key bytes (uncompressed SEC1: 0x04 || x || y, 65 bytes)
    pub fn public_key_bytes(&self) -> Vec<u8> {
        self.public.as_bytes().to_vec()
    }

    /// Compute the shared secret with the server's public key
    pub fn exchange(&self, server_pk: &[u8]) -> Result<Vec<u8>, String> {
        let server_pk =
            edgerun_crypto::p256::PublicKey::from_sec1_bytes(server_pk).map_err(|e| format!("Invalid server public key: {:?}", e))?;

        let shared = self.secret.diffie_hellman(&server_pk);
        Ok(shared.raw_secret_bytes().to_vec())
    }
}

/// RNG adapter — bridges edgerun_core::crypto::fill_random to p256's rand_core.
/// All randomness comes from /dev/urandom via edgerun-core's fill_random.
struct CoreRng;

impl RngCore for CoreRng {
    fn next_u32(&mut self) -> u32 {
        let mut buf = [0u8; 4];
        self.fill_bytes(&mut buf);
        u32::from_le_bytes(buf)
    }

    fn next_u64(&mut self) -> u64 {
        let mut buf = [0u8; 8];
        self.fill_bytes(&mut buf);
        u64::from_le_bytes(buf)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.try_fill_bytes(dest).expect("fill_random failed");
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), edgerun_crypto::p256::elliptic_curve::rand_core::Error> {
        edgerun_crypto::rand_core::OsRng.fill_bytes(dest);
        Ok(())
    }
}

impl CryptoRng for CoreRng {}
