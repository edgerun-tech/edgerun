use crate::prelude::v1::*;
use alloc::sync::Arc;
use edgerun_encoding::base64url_nopad_encode;
use edgerun_http::{HttpClient, HttpVersion, Method, Request};
use edgerun_json::{FromJson, JsonValue};
use edgerun_rt::RwLock;
use edgerun_url::Url;

use crate::account::AccountKey;
use crate::challenge::Challenge;
use crate::order::Order;
use crate::types::{
    AcmeErrorDetail, CSRRequest, Directory, DirectoryUrl, Identifier, JwsHeader,
    NewAccountRequestWithNonce, NewOrderRequest, RevokeCertRequest, SignedJws,
};
use crate::AcmeError;
use edgerun_protocols::acme::dns01::Dns01Challenge;
use edgerun_protocols::acme::http01::Http01Challenge;

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
            // ACME endpoints sit behind strict HTTP front ends. Keep the CA
            // client on HTTP/1.1 until the Edgerun HTTP/2 client has broader
            // interoperability coverage with public CDNs and CA ingress tiers.
            http_client: HttpClient::new().version(HttpVersion::Http1),
        }
    }

    pub async fn init(&self) -> Result<(), AcmeError> {
        let directory = self.fetch_directory().await?;
        *self.directory.write() = Some(directory);
        Ok(())
    }

    async fn fetch_directory(&self) -> Result<Directory, AcmeError> {
        let url = self.config.directory_url.url();
        let res = self
            .http_client
            .get(&url.to_string())
            .await
            .map_err(|e| AcmeError::Network(e.to_string()))?;

        let body = res.body();
        let dir: Directory =
            edgerun_json::from_json_slice(body).map_err(|e| AcmeError::Parse(e.to_string()))?;
        Ok(dir)
    }

    async fn get_nonce(&self) -> Result<String, AcmeError> {
        let nonce_opt = {
            let guard = self.nonce.read();
            guard.clone()
        };

        if let Some(nonce) = nonce_opt {
            *self.nonce.write() = None;
            return Ok(nonce);
        }

        let new_nonce_url = {
            let dir = self.directory.read();
            dir.as_ref()
                .map(|dir| dir.new_nonce.clone())
                .ok_or(AcmeError::NotInitialized)?
        };

        let res = self
            .http_client
            .request(Method::HEAD, &new_nonce_url.to_string(), None)
            .await
            .map_err(|e| AcmeError::Network(e.to_string()))?;

        let nonce = res
            .headers()
            .get("replay-nonce")
            .map(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or(AcmeError::Protocol("missing replay-nonce".into()))?;

        Ok(nonce)
    }

    async fn post(&self, url: &Url, payload: Option<&[u8]>) -> Result<(u16, Vec<u8>), AcmeError> {
        let (status, body, _) = self.post_with_location(url, payload).await?;
        Ok((status, body))
    }

    async fn post_with_location(
        &self,
        url: &Url,
        payload: Option<&[u8]>,
    ) -> Result<(u16, Vec<u8>, Option<String>), AcmeError> {
        let nonce = self.get_nonce().await?;
        let account_id = self.account_id.read().clone();

        let protected = JwsHeader {
            alg: "ES256".to_string(),
            jwk: if account_id.is_some() {
                None
            } else {
                Some(self.account_key.jwk())
            },
            url: url.to_string(),
            nonce: Some(nonce),
            key_id: account_id,
        };

        let protected_b64 = base64url_nopad_encode(
            &edgerun_json::to_json_vec(&protected).map_err(|e| AcmeError::Parse(e.to_string()))?,
        );

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

        let jws_bytes =
            edgerun_json::to_json_vec(&jws).map_err(|e| AcmeError::Parse(e.to_string()))?;

        let request = Request::builder()
            .method(Method::POST)
            .uri(url.to_string())
            .header("Content-Type", "application/jose+json")
            .body(jws_bytes)
            .build()
            .map_err(|e| AcmeError::Network(e.to_string()))?;
        let res = self
            .http_client
            .execute(&request)
            .await
            .map_err(|e| AcmeError::Network(e.to_string()))?;

        let status = res.status().as_u16();
        let body = res.body().to_vec();
        let location = res
            .headers()
            .get("location")
            .map(|value| value.as_str().to_string());

        let new_nonce = res
            .headers()
            .get("replay-nonce")
            .map(|v| v.as_str())
            .map(|s| s.to_string());

        if let Some(nonce) = new_nonce {
            *self.nonce.write() = Some(nonce);
        }

        if status == 200 || status == 201 || status == 202 {
            Ok((status, body, location))
        } else if let Ok(error_detail) = edgerun_json::from_json_slice::<AcmeErrorDetail>(&body) {
            Err(AcmeError::Server(status, Some(error_detail)))
        } else {
            let body = String::from_utf8_lossy(&body);
            Err(AcmeError::Protocol(format!(
                "ACME server error {status}: {body}"
            )))
        }
    }

    pub async fn create_account(&self) -> Result<String, AcmeError> {
        if let Some(id) = self.account_id.read().clone() {
            return Ok(id);
        }

        let new_account_url = {
            let dir = self.directory.read();
            dir.as_ref()
                .map(|dir| dir.new_account.clone())
                .ok_or(AcmeError::NotInitialized)?
        };

        let payload = NewAccountRequestWithNonce {
            contact: if self.config.email.is_empty() {
                None
            } else {
                Some(self.config.email.clone())
            },
            terms_of_service_agreed: Some(self.config.terms_of_service_agreed),
            external_account_binding: None,
        };

        let payload_bytes =
            edgerun_json::to_json_vec(&payload).map_err(|e| AcmeError::Parse(e.to_string()))?;

        let (_, _body, location) = self
            .post_with_location(&new_account_url, Some(&payload_bytes))
            .await?;

        let id = location.ok_or_else(|| AcmeError::Protocol("missing account location".into()))?;
        *self.account_id.write() = Some(id.clone());

        Ok(id)
    }

    pub async fn create_order(&self, domains: &[String]) -> Result<Order, AcmeError> {
        if domains.is_empty() {
            return Err(AcmeError::Protocol(
                "ACME order must include at least one domain".into(),
            ));
        }
        for domain in domains {
            validate_acme_dns_identifier(domain)?;
        }

        let new_order_url = {
            let dir = self.directory.read();
            dir.as_ref()
                .map(|dir| dir.new_order.clone())
                .ok_or(AcmeError::NotInitialized)?
        };

        let identifiers: Vec<Identifier> = domains
            .iter()
            .map(|d| Identifier {
                id_type: "dns".to_string(),
                value: d.trim_end_matches('.').to_ascii_lowercase(),
            })
            .collect();

        let payload = NewOrderRequest {
            identifiers,
            not_before: None,
            not_after: None,
        };

        let payload_bytes =
            edgerun_json::to_json_vec(&payload).map_err(|e| AcmeError::Parse(e.to_string()))?;

        let (_, body, location) = self
            .post_with_location(&new_order_url, Some(&payload_bytes))
            .await?;

        let order: crate::types::Order = parse_acme_object_with_id(
            &body,
            location.as_deref().unwrap_or(&new_order_url.to_string()),
        )?;

        Ok(Order::from_acme(order))
    }

    pub async fn get_order(&self, url: &Url) -> Result<Order, AcmeError> {
        let (_, body, _) = self.post_with_location(url, None).await?;
        let order: crate::types::Order = parse_acme_object_with_id(&body, &url.to_string())?;
        Ok(Order::from_acme(order))
    }

    pub async fn get_authorization(
        &self,
        url: &Url,
    ) -> Result<crate::types::Authorization, AcmeError> {
        let (_, body, _) = self.post_with_location(url, None).await?;

        let authz: crate::types::Authorization =
            parse_acme_object_with_id(&body, &url.to_string())?;

        Ok(authz)
    }

    pub async fn get_challenge(&self, url: &Url) -> Result<Challenge, AcmeError> {
        let (_, body, _) = self.post_with_location(url, None).await?;

        let challenge: crate::types::Challenge =
            parse_acme_object_with_id(&body, &url.to_string())?;

        Ok(Challenge::from_acme(challenge))
    }

    pub async fn validate_challenge(&self, url: &Url) -> Result<Challenge, AcmeError> {
        let (_, body, _) = self.post_with_location(url, Some(b"{}")).await?;

        let challenge: crate::types::Challenge =
            parse_acme_object_with_id(&body, &url.to_string())?;

        Ok(Challenge::from_acme(challenge))
    }

    pub async fn finalize_order(&self, url: &Url, csr_der: &[u8]) -> Result<Order, AcmeError> {
        let csr_b64 = base64url_nopad_encode(csr_der);

        let payload = CSRRequest { csr: csr_b64 };
        let payload_bytes =
            edgerun_json::to_json_vec(&payload).map_err(|e| AcmeError::Parse(e.to_string()))?;

        let (_, body, _) = self.post_with_location(url, Some(&payload_bytes)).await?;

        let order: crate::types::Order = parse_acme_object_with_id(&body, &url.to_string())?;

        Ok(Order::from_acme(order))
    }

    pub async fn download_certificate(&self, url: &Url) -> Result<String, AcmeError> {
        let (_, body) = self.post(url, None).await?;
        String::from_utf8(body).map_err(|e| AcmeError::Parse(e.to_string()))
    }

    pub async fn revoke_certificate(
        &self,
        cert_der: &[u8],
        reason: Option<u32>,
    ) -> Result<(), AcmeError> {
        let revoke_cert_url = {
            let dir = self.directory.read();
            dir.as_ref()
                .map(|dir| dir.revoke_cert.clone())
                .ok_or(AcmeError::NotInitialized)?
        };

        let cert_b64 = base64url_nopad_encode(cert_der);

        let payload = RevokeCertRequest {
            certificate: cert_b64,
            reason,
        };

        let payload_bytes =
            edgerun_json::to_json_vec(&payload).map_err(|e| AcmeError::Parse(e.to_string()))?;

        self.post(&revoke_cert_url, Some(&payload_bytes)).await?;

        Ok(())
    }

    pub fn http_01_challenge(&self, token: &str) -> Http01Challenge {
        Http01Challenge::from_thumbprint(token, &self.account_key.thumbprint_b64())
    }

    pub fn dns_01_challenge(&self, domain: &str, token: &str) -> Dns01Challenge {
        Dns01Challenge::new(domain, token, &self.account_key.thumbprint_b64())
    }

    pub fn thumbprint(&self) -> String {
        self.account_key.thumbprint_b64()
    }

    pub fn generate_key(&self) -> String {
        self.account_key.pem()
    }
}

fn validate_acme_dns_identifier(domain: &str) -> Result<(), AcmeError> {
    let domain = domain.trim().trim_end_matches('.');
    if domain.is_empty() {
        return Err(AcmeError::Protocol("empty ACME DNS identifier".into()));
    }
    if domain.len() > 253 {
        return Err(AcmeError::Protocol("ACME DNS identifier too long".into()));
    }
    if domain.starts_with("*.") && domain[2..].contains('*') {
        return Err(AcmeError::Protocol(
            "invalid wildcard ACME DNS identifier".into(),
        ));
    }
    if !domain.starts_with("*.") && domain.contains('*') {
        return Err(AcmeError::Protocol(
            "invalid wildcard ACME DNS identifier".into(),
        ));
    }

    let labels = if let Some(rest) = domain.strip_prefix("*.") {
        rest.split('.')
    } else {
        domain.split('.')
    };

    let mut label_count = 0usize;
    for label in labels {
        label_count += 1;
        if label.is_empty() || label.len() > 63 {
            return Err(AcmeError::Protocol("invalid ACME DNS label length".into()));
        }
        if label.starts_with('-') || label.ends_with('-') {
            return Err(AcmeError::Protocol(
                "ACME DNS label cannot start or end with '-'".into(),
            ));
        }
        if !label
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return Err(AcmeError::Protocol(
                "ACME DNS label contains invalid characters".into(),
            ));
        }
    }

    if label_count < 2 {
        return Err(AcmeError::Protocol(
            "ACME DNS identifier must be fully qualified".into(),
        ));
    }

    Ok(())
}

fn parse_acme_object_with_id<T: FromJson>(body: &[u8], id: &str) -> Result<T, AcmeError> {
    let mut value: JsonValue =
        edgerun_json::from_json_slice(body).map_err(|e| AcmeError::Parse(e.to_string()))?;
    if let JsonValue::Object(_) = value {
        value.push_field("id", id.to_string());
    }
    T::from_json(value).map_err(|e| AcmeError::Parse(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_acme_dns_identifier_rejects_bad_names() {
        assert!(validate_acme_dns_identifier("").is_err());
        assert!(validate_acme_dns_identifier("localhost").is_err());
        assert!(validate_acme_dns_identifier("-bad.example").is_err());
        assert!(validate_acme_dns_identifier("bad-.example").is_err());
        assert!(validate_acme_dns_identifier("bad_*example.com").is_err());
        assert!(validate_acme_dns_identifier("*.*.example.com").is_err());
    }

    #[test]
    fn validate_acme_dns_identifier_accepts_normal_and_wildcard_names() {
        assert!(validate_acme_dns_identifier("example.com").is_ok());
        assert!(validate_acme_dns_identifier("www.example.com.").is_ok());
        assert!(validate_acme_dns_identifier("*.example.com").is_ok());
    }
}
