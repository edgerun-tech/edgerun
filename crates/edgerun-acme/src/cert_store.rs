use std::path::PathBuf;

use edgerun_crypto::random_p256_signing_key;
use edgerun_secret_service::Backend;

use crate::AcmeError;

#[derive(Clone, Debug)]
pub struct CertInfo {
    pub domains: Vec<String>,
    pub cert_pem: String,
    pub key_pem: String,
    pub issued_at: u64,
    pub expires_at: u64,
}

#[derive(Clone, Debug)]
pub struct StoredCert {
    pub cert_info: CertInfo,
    pub full_chain_pem: String,
}

pub struct CertStore {
    backend: Backend,
    namespace: String,
}

impl CertStore {
    pub fn new(data_root: PathBuf, namespace: &str) -> Result<Self, AcmeError> {
        let backend =
            Backend::new_noop(data_root).map_err(|e| AcmeError::Storage(e.to_string()))?;

        Ok(Self {
            backend,
            namespace: namespace.to_string(),
        })
    }

    fn cert_key(&self, domain: &str) -> String {
        format!("cert:{}", domain)
    }

    fn key_key(&self, domain: &str) -> String {
        format!("key:{}", domain)
    }

    pub fn store(&mut self, domain: &str, cert_pem: &str, key_pem: &str) -> Result<(), AcmeError> {
        let cert_key = self.cert_key(domain);
        let key_key = self.key_key(domain);

        self.backend
            .put(
                &self.namespace,
                &cert_key,
                cert_pem.as_bytes(),
                &format!("Certificate for {}", domain),
                &[],
            )
            .map_err(|e| AcmeError::Storage(e.to_string()))?;

        self.backend
            .put(
                &self.namespace,
                &key_key,
                key_pem.as_bytes(),
                &format!("Private key for {}", domain),
                &[],
            )
            .map_err(|e| AcmeError::Storage(e.to_string()))?;

        Ok(())
    }

    pub fn load(&self, domain: &str) -> Result<Option<CertInfo>, AcmeError> {
        let cert_key = self.cert_key(domain);
        let key_key = self.key_key(domain);

        let cert = self
            .backend
            .get(&self.namespace, &cert_key)
            .map_err(|e| AcmeError::Storage(e.to_string()))?;

        let key = self
            .backend
            .get(&self.namespace, &key_key)
            .map_err(|e| AcmeError::Storage(e.to_string()))?;

        match (cert, key) {
            (Some((cert_bytes, _)), Some((key_bytes, _))) => {
                let cert_pem = String::from_utf8_lossy(&cert_bytes).to_string();
                let key_pem = String::from_utf8_lossy(&key_bytes).to_string();

                Ok(Some(CertInfo {
                    domains: vec![domain.to_string()],
                    cert_pem,
                    key_pem,
                    issued_at: 0,
                    expires_at: 0,
                }))
            }
            _ => Ok(None),
        }
    }

    pub fn delete(&mut self, domain: &str) -> Result<(), AcmeError> {
        let cert_key = self.cert_key(domain);
        let key_key = self.key_key(domain);

        let _ = self.backend.delete(&self.namespace, &cert_key);
        let _ = self.backend.delete(&self.namespace, &key_key);

        Ok(())
    }

    pub fn list(&self) -> Result<Vec<String>, AcmeError> {
        Ok(vec![])
    }
}

pub fn parse_pem_cert(pem: &str) -> Result<CertInfo, AcmeError> {
    Ok(CertInfo {
        domains: vec![],
        cert_pem: pem.to_string(),
        key_pem: String::new(),
        issued_at: 0,
        expires_at: 0,
    })
}

pub fn generate_key() -> String {
    use edgerun_crypto::p256_signing_key_to_pem;
    let key = random_p256_signing_key();
    p256_signing_key_to_pem(&key).unwrap()
}
