use anyhow::{Context, Result};
use edgerun_crypto::p256::ecdsa::{Signature, SigningKey, VerifyingKey};
use edgerun_crypto::p256::elliptic_curve::sec1::ToEncodedPoint;
use edgerun_crypto::sha2;
use edgerun_crypto::sha2::Digest;
use edgerun_crypto::signature::Verifier;
use edgerun_crypto::Signer;
use edgerun_proto::edgerun::v0::stream::{AppIntent, AppPrincipal};

#[derive(Debug)]
pub struct AppKeyPair {
    pub app_id: Vec<u8>,
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
}

impl AppKeyPair {
    pub fn generate(app_id: Vec<u8>) -> Result<Self> {
        let signing_key = SigningKey::random(&mut edgerun_crypto::OsRng);
        let verifying_key = VerifyingKey::from(&signing_key);
        Ok(Self {
            app_id,
            signing_key,
            verifying_key,
        })
    }

    pub fn principal(&self) -> AppPrincipal {
        AppPrincipal {
            app_id: self.app_id.clone(),
            public_key: self
                .verifying_key
                .to_encoded_point(false)
                .as_bytes()
                .to_vec(),
            metadata_object: None,
        }
    }

    pub fn sign_intent(&self, payload: &[u8]) -> Result<AppIntent> {
        let mut hasher = sha2::Sha256::new();
        hasher.update(&self.app_id);
        hasher.update(payload);
        let message_hash = hasher.finalize();

        let signature: Signature = self.signing_key.sign(&message_hash);

        Ok(AppIntent {
            app_id: self.app_id.clone(),
            payload: payload.to_vec(),
            signature: signature.to_der().as_bytes().to_vec(),
        })
    }
}

pub fn compute_app_id(app_package_bytes: &[u8]) -> Vec<u8> {
    let mut hasher = sha2::Sha256::new();
    hasher.update(b"edgerun:v0:app");
    hasher.update(app_package_bytes);
    hasher.finalize().to_vec()
}

pub fn verify_app_intent(intent: &AppIntent, public_key: &[u8]) -> Result<()> {
    let verifying_key = VerifyingKey::from_sec1_bytes(public_key)
        .map_err(|e| anyhow::anyhow!("Invalid app public key: {:?}", e))?;

    let mut hasher = sha2::Sha256::new();
    hasher.update(&intent.app_id);
    hasher.update(&intent.payload);
    let message_hash = hasher.finalize();

    let signature = Signature::from_der(&intent.signature)
        .map_err(|e| anyhow::anyhow!("Invalid DER signature: {:?}", e))?;

    verifying_key
        .verify(&message_hash, &signature)
        .map_err(|e| anyhow::anyhow!("App intent signature verification failed: {:?}", e))
}
