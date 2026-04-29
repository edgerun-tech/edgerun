use crate::prelude::v1::*;
use std::path::PathBuf;

use edgerun_secret_service::Backend;
use edgerun_tls::certificate::Certificate;

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
        let cert = Certificate::from_pem(cert_pem).map_err(AcmeError::Parse)?;
        if !cert.matches_hostname(domain) {
            return Err(AcmeError::Protocol(format!(
                "certificate does not match domain {domain}"
            )));
        }

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
                let cert = Certificate::from_pem(&cert_pem).map_err(AcmeError::Parse)?;

                Ok(Some(CertInfo {
                    domains: vec![domain.to_string()],
                    cert_pem,
                    key_pem,
                    issued_at: cert.not_before,
                    expires_at: cert.not_after,
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
        let entries = self
            .backend
            .list(&self.namespace)
            .map_err(|e| AcmeError::Storage(e.to_string()))?;

        let mut domains = Vec::new();
        for (key, _) in entries {
            let Some(domain) = key.strip_prefix("cert:") else {
                continue;
            };
            let key_key = self.key_key(domain);
            let has_key = self
                .backend
                .get(&self.namespace, &key_key)
                .map_err(|e| AcmeError::Storage(e.to_string()))?
                .is_some();
            if has_key {
                domains.push(domain.to_string());
            }
        }
        domains.sort();
        Ok(domains)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("edgerun-acme-{name}-{nanos}"))
    }

    #[test]
    fn store_load_list_delete_roundtrip() {
        let root = temp_root("cert-store");
        let mut store = CertStore::new(root.clone(), "acme").expect("store");
        let example_cert =
            edgerun_tls::generate_self_signed(&["example.com"]).expect("example cert");
        let example_cert_pem = example_cert.cert_pem();
        let example_key_pem = example_cert.key_pem().expect("example key pem");
        let www_cert = edgerun_tls::generate_self_signed(&["www.example.com"]).expect("www cert");
        let www_cert_pem = www_cert.cert_pem();
        let www_key_pem = www_cert.key_pem().expect("www key pem");

        store
            .store("example.com", &example_cert_pem, &example_key_pem)
            .expect("store cert");
        store
            .store("www.example.com", &www_cert_pem, &www_key_pem)
            .expect("store second cert");

        let loaded = store
            .load("example.com")
            .expect("load")
            .expect("stored cert");
        assert_eq!(loaded.domains, vec!["example.com".to_string()]);
        assert!(loaded.cert_pem.contains("BEGIN CERTIFICATE"));
        assert!(loaded.key_pem.contains("BEGIN PRIVATE KEY"));
        assert!(loaded.issued_at > 0);
        assert!(loaded.expires_at > loaded.issued_at);

        assert_eq!(
            store.list().expect("list"),
            vec!["example.com".to_string(), "www.example.com".to_string()]
        );

        store.delete("example.com").expect("delete");
        assert!(store.load("example.com").expect("load deleted").is_none());
        assert_eq!(
            store.list().expect("list"),
            vec!["www.example.com".to_string()]
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn store_rejects_certificate_for_different_domain() {
        let root = temp_root("cert-store-mismatch");
        let mut store = CertStore::new(root.clone(), "acme").expect("store");
        let cert = edgerun_tls::generate_self_signed(&["other.example.com"]).expect("cert");
        let cert_pem = cert.cert_pem();
        let key_pem = cert.key_pem().expect("key pem");

        let err = store
            .store("example.com", &cert_pem, &key_pem)
            .expect_err("domain mismatch should be rejected");
        assert!(err
            .to_string()
            .contains("certificate does not match domain"));

        let _ = std::fs::remove_dir_all(root);
    }
}
