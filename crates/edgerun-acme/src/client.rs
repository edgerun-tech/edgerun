use std::sync::Arc;

use edgerun_rt::RwLock;
use edgerun_http::{HttpClient, Method};
use edgerun_encoding::base64url_nopad_encode;
use edgerun_url::Url;

use crate::types::{Directory, DirectoryUrl, Identifier, AcmeErrorDetail, JwsHeader, SignedJws, NewAccountRequestWithNonce, NewOrderRequest, CSRRequest, RevokeCertRequest, CertificateResponse};
use crate::account::AccountKey;
use crate::order::Order;
use crate::challenge::Challenge;
use crate::http_challenge::HttpChallengeHandler;
use crate::dns_challenge::DnsChallengeManager;
use crate::AcmeError;

pub struct AcmeClient {
    config: AcmeConfig,
    directory: RwLock<Option<Directory>>,
    account_key: Arc<AccountKey>,
    account_id: RwLock<Option<String>>,
    nonce: RwLock<Option<String>>,
    http_client: HttpClient,
}

#[derive(Clone)]
pub struct AcmeConfig {
    pub directory_url: DirectoryUrl,
    pub email: Vec<String>,
    pub terms_of_service_agreed: bool,
}

impl Default for AcmeConfig {
    fn default() -> Self {
        Self {
            directory_url: DirectoryUrl::LetsEncrypt,
            email: vec![],
            terms_of_service_agreed: false,
        }
    }
}

impl AcmeClient {
    pub fn new(config: AcmeConfig, account_key: AccountKey) -> Self {
        Self {
            config,
            directory: RwLock::new(None),
            account_key: Arc::new(account_key),
            account_id: RwLock::new(None),
            nonce: RwLock::new(None),
            http_client: HttpClient::new(),
        }
    }

    pub async fn init(&self) -> Result<(), AcmeError> {
        let directory = self.fetch_directory().await?;
        *self.directory.write().await = Some(directory);
        Ok(())
    }

    async fn fetch_directory(&self) -> Result<Directory, AcmeError> {
        let url = self.config.directory_url.url();
        let res = self.http_client.get(&url.to_string())
            .await
            .map_err(|e| AcmeError::Network(e.to_string()))?;
        
        let body = res.body();
        let dir: Directory = edgerun_json::from_slice(body)
            .map_err(|e| AcmeError::Parse(e.to_string()))?;
        Ok(dir)
    }

    async fn get_nonce(&self) -> Result<String, AcmeError> {
        let nonce_opt = {
            let guard = self.nonce.read().await;
            guard.clone()
        };
        
        if let Some(nonce) = nonce_opt {
            // Clear the nonce after use
            *self.nonce.write().await = None;
            return Ok(nonce);
        }

        let dir = self.directory.read().await;
        let dir = dir.as_ref().ok_or(AcmeError::NotInitialized)?;
        
        let res = self.http_client.request(Method::HEAD, &dir.new_nonce.to_string(), None)
            .await
            .map_err(|e| AcmeError::Network(e.to_string()))?;
        
        let nonce = res.headers()
            .get("replay-nonce").map(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or(AcmeError::Protocol("missing replay-nonce".into()))?;
        
        Ok(nonce)
    }

    async fn post(&self, url: &Url, payload: Option<&[u8]>) -> Result<(u16, Vec<u8>), AcmeError> {
        let nonce = self.get_nonce().await?;
        
        let protected = JwsHeader {
            alg: "ES256".to_string(),
            jwk: Some(self.account_key.jwk()),
            url: url.to_string(),
            nonce: Some(nonce),
            key_id: self.account_id.read().await.clone(),
        };
        
        let protected_b64 = base64url_nopad_encode(&edgerun_json::to_vec(&protected).map_err(|e| AcmeError::Parse(e.to_string()))?);
        
        let payload_b64 = match payload {
            Some(p) => base64url_nopad_encode(p),
            None => String::new(),
        };
        
        let signing_input = format!("{}.{}", protected_b64, payload_b64);
        
        let signature = self.account_key.sign(signing_input.as_bytes());
        let signature_b64 = base64url_nopad_encode(&signature);
        
        let jws = SignedJws {
            protected: protected_b64,
            payload: payload_b64,
            signature: signature_b64,
        };
        
        let jws_bytes = edgerun_json::to_vec(&jws).map_err(|e| AcmeError::Parse(e.to_string()))?;
        
        let res = self.http_client.post(&url.to_string(), jws_bytes)
            .await
            .map_err(|e| AcmeError::Network(e.to_string()))?;
        
        let status = res.status().as_u16();
        let body = res.body().to_vec();
        
        let new_nonce = res.headers()
            .get("replay-nonce").map(|v| v.as_str())
            .map(|s| s.to_string());
        
        if let Some(nonce) = new_nonce {
            *self.nonce.write().await = Some(nonce);
        }
        
        if status == 200 || status == 201 {
            Ok((status, body))
        } else {
            let error_detail: Option<AcmeErrorDetail> = edgerun_json::from_slice(&body).ok();
            Err(AcmeError::Server(status, error_detail))
        }
    }

    pub async fn create_account(&self) -> Result<String, AcmeError> {
        if let Some(id) = self.account_id.read().await.clone() {
            return Ok(id);
        }

        let dir = self.directory.read().await;
        let dir = dir.as_ref().ok_or(AcmeError::NotInitialized)?;

        let payload = NewAccountRequestWithNonce {
            contact: if self.config.email.is_empty() { None } else { Some(self.config.email.clone()) },
            terms_of_service_agreed: Some(self.config.terms_of_service_agreed),
            jwk: self.account_key.jwk(),
            external_account_binding: None,
        };

        let payload_bytes = edgerun_json::to_vec(&payload).map_err(|e| AcmeError::Parse(e.to_string()))?;
        
        let (_, body) = self.post(&dir.new_account, Some(&payload_bytes)).await?;
        
        let account: crate::types::AccountResponse = edgerun_json::from_slice(&body)
            .map_err(|e| AcmeError::Parse(e.to_string()))?;
        
        let id = account.id;
        *self.account_id.write().await = Some(id.clone());
        
        Ok(id)
    }

    pub async fn create_order(&self, domains: &[String]) -> Result<Order, AcmeError> {
        let dir = self.directory.read().await;
        let dir = dir.as_ref().ok_or(AcmeError::NotInitialized)?;

        let identifiers: Vec<Identifier> = domains.iter()
            .map(|d| Identifier {
                id_type: "dns".to_string(),
                value: d.clone(),
            })
            .collect();

        let payload = NewOrderRequest {
            identifiers,
            not_before: None,
            not_after: None,
        };

        let payload_bytes = edgerun_json::to_vec(&payload).map_err(|e| AcmeError::Parse(e.to_string()))?;
        
        let (_, body) = self.post(&dir.new_order, Some(&payload_bytes)).await?;
        
        let order: crate::types::Order = edgerun_json::from_slice(&body)
            .map_err(|e| AcmeError::Parse(e.to_string()))?;
        
        Ok(Order::from_acme(order))
    }

    pub async fn get_authorization(&self, url: &Url) -> Result<crate::types::Authorization, AcmeError> {
        let (_, body) = self.post(url, None).await?;
        
        let authz: crate::types::Authorization = edgerun_json::from_slice(&body)
            .map_err(|e| AcmeError::Parse(e.to_string()))?;
        
        Ok(authz)
    }

    pub async fn get_challenge(&self, url: &Url) -> Result<Challenge, AcmeError> {
        let (_, body) = self.post(url, None).await?;
        
        let challenge: crate::types::Challenge = edgerun_json::from_slice(&body)
            .map_err(|e| AcmeError::Parse(e.to_string()))?;
        
        Ok(Challenge::from_acme(challenge))
    }

    pub async fn validate_challenge(&self, url: &Url) -> Result<Challenge, AcmeError> {
        let (_, body) = self.post(url, Some(b"{}")).await?;
        
        let challenge: crate::types::Challenge = edgerun_json::from_slice(&body)
            .map_err(|e| AcmeError::Parse(e.to_string()))?;
        
        Ok(Challenge::from_acme(challenge))
    }

    pub async fn finalize_order(&self, url: &Url, csr_der: &[u8]) -> Result<Order, AcmeError> {
        let csr_b64 = base64url_nopad_encode(csr_der);
        
        let payload = CSRRequest { csr: csr_b64 };
        let payload_bytes = edgerun_json::to_vec(&payload).map_err(|e| AcmeError::Parse(e.to_string()))?;
        
        let (_, body) = self.post(url, Some(&payload_bytes)).await?;
        
        let order: crate::types::Order = edgerun_json::from_slice(&body)
            .map_err(|e| AcmeError::Parse(e.to_string()))?;
        
        Ok(Order::from_acme(order))
    }

    pub async fn download_certificate(&self, url: &Url) -> Result<String, AcmeError> {
        let (_, body) = self.post(url, None).await?;
        
        let cert: CertificateResponse = edgerun_json::from_slice(&body)
            .map_err(|e| AcmeError::Parse(e.to_string()))?;
        
        Ok(cert.certificate)
    }

    pub async fn revoke_certificate(&self, cert_der: &[u8], reason: Option<u32>) -> Result<(), AcmeError> {
        let dir = self.directory.read().await;
        let dir = dir.as_ref().ok_or(AcmeError::NotInitialized)?;

        let cert_b64 = base64url_nopad_encode(cert_der);
        
        let payload = RevokeCertRequest {
            certificate: cert_b64,
            reason,
        };
        
        let payload_bytes = edgerun_json::to_vec(&payload).map_err(|e| AcmeError::Parse(e.to_string()))?;
        
        self.post(&dir.revoke_cert, Some(&payload_bytes)).await?;
        
        Ok(())
    }

    pub fn http_handler(&self, token: &str) -> HttpChallengeHandler {
        let key_authorization = format!("{}.{}", token, self.account_key.thumbprint_b64());
        HttpChallengeHandler {
            token: token.to_string(),
            key_authorization,
        }
    }

    pub fn dns_manager(&self) -> DnsChallengeManager {
        DnsChallengeManager::new(Arc::clone(&self.account_key))
    }

    pub fn thumbprint(&self) -> String {
        self.account_key.thumbprint_b64()
    }

    pub fn generate_key(&self) -> String {
        self.account_key.pem()
    }
}
