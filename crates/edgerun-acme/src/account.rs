use crate::prelude::v1::*;
use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashSigner;
use edgerun_crypto::p256::ecdsa::Signature;
use edgerun_crypto::{
    p256_signing_key_from_pem, p256_signing_key_to_pem, random_p256_signing_key, SigningKey,
};
use edgerun_encoding::base64::base64url_nopad_encode;

use crate::types::Jwk;
use crate::AcmeError;

pub struct AccountKey {
    key: SigningKey,
    pem: String,
}

impl AccountKey {
    pub fn generate() -> Self {
        let key = random_p256_signing_key();
        let pem = p256_signing_key_to_pem(&key);
        Self { key, pem }
    }

    pub fn from_pem(pem: &str) -> Result<Self, AcmeError> {
        let key = p256_signing_key_from_pem(pem)
            .ok_or_else(|| AcmeError::Crypto("invalid P-256 signing key".to_string()))?;
        Ok(Self {
            key,
            pem: pem.to_string(),
        })
    }

    pub fn jwk(&self) -> Jwk {
        let pk = self.key.verifying_key();
        let encoded = pk.to_encoded_point(false);
        let bytes = encoded.as_bytes();

        let x = &bytes[1..33];
        let y = &bytes[33..65];

        Jwk::EC {
            crv: "P-256".to_string(),
            x: base64url_nopad_encode(x),
            y: base64url_nopad_encode(y),
        }
    }

    pub fn thumbprint_b64(&self) -> String {
        base64url_nopad_encode(&self.jwk().thumbprint())
    }

    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        use edgerun_crypto::digest::Digest;
        use edgerun_crypto::Sha256;
        let mut hasher = <Sha256 as Digest>::new();
        hasher.update(message);
        let digest = hasher.finalize();
        let sig: Signature = self.key.sign_prehash(&digest).expect("sign failed");
        sig.to_bytes().to_vec()
    }

    pub fn pem(&self) -> String {
        self.pem.clone()
    }
}

impl Clone for AccountKey {
    fn clone(&self) -> Self {
        Self::from_pem(&self.pem).expect("AccountKey PEM should always be valid")
    }
}
