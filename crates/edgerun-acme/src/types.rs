
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Directory {
    pub new_nonce: Url,
    pub new_account: Url,
    pub new_order: Url,
    pub revoke_cert: Url,
    pub key_change: Url,
    pub meta: Option<DirectoryMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryMeta {
    pub terms_of_service: Option<Url>,
    pub website: Option<Url>,
    pub caa_identities: Option<Vec<String>>,
    pub external_account_required: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DirectoryUrl {
    LetsEncrypt,
    LetsEncryptStaging,
    Custom(Url),
}

impl DirectoryUrl {
    pub fn url(&self) -> Url {
        match self {
            DirectoryUrl::LetsEncrypt => Url::parse("https://acme-v02.api.letsencrypt.org/directory").unwrap(),
            DirectoryUrl::LetsEncryptStaging => Url::parse("https://acme-staging-v02.api.letsencrypt.org/directory").unwrap(),
            DirectoryUrl::Custom(u) => u.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub key: Jwk,
    pub contact: Option<Vec<String>>,
    pub status: AccountStatus,
    pub terms_of_service_agreed: Option<bool>,
    pub orders: Url,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccountStatus {
    Valid,
    Deactivated,
    Revoked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Jwk {
    RSA { n: String, e: String },
    EC { crv: String, x: String, y: String },
}

impl Jwk {
    pub fn thumbprint(&self) -> Vec<u8> {
        use sha2::{Sha256, Digest};
        let jwk_json = serde_json::to_string(self).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(jwk_json.as_bytes());
        hasher.finalize().to_vec()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewAccountRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terms_of_service_agreed: Option<bool>,
    pub jwk: Jwk,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewAccountRequestWithNonce {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terms_of_service_agreed: Option<bool>,
    pub jwk: Jwk,
    #[serde(rename = "externalAccountBinding")]
    pub external_account_binding: Option<ExternalAccountBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalAccountBinding {
    #[serde(rename = "protected")]
    pub protected: String,
    pub payload: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountResponse {
    pub id: String,
    pub key: Jwk,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<Vec<String>>,
    #[serde(rename = "initialIP")]
    pub initial_ip: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: Option<String>,
    pub status: AccountStatus,
    #[serde(rename = "termsOfServiceAgreed")]
    pub terms_of_service_agreed: Option<bool>,
    pub orders: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewOrderRequest {
    pub identifiers: Vec<Identifier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_before: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_after: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identifier {
    #[serde(rename = "type")]
    pub id_type: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: Url,
    pub status: OrderStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_before: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_after: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifiers: Option<Vec<Identifier>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorizations: Option<Vec<Url>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finalize: Option<Url>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub certificate: Option<Url>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AcmeErrorDetail>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderStatus {
    Pending,
    Ready,
    Processing,
    Valid,
    Invalid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Authorization {
    pub id: Url,
    #[serde(rename = "identifier")]
    pub identifier: Identifier,
    pub status: AuthorizationStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub challenges: Option<Vec<Challenge>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wildcard: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthorizationStatus {
    Pending,
    Valid,
    Invalid,
    Deactivated,
    Expired,
    Revoked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Challenge {
    pub id: String,
    #[serde(rename = "type")]
    pub challenge_type: ChallengeType,
    pub url: Url,
    pub status: ChallengeStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validated: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AcmeErrorDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChallengeType {
    #[serde(rename = "http-01")]
    Http01,
    #[serde(rename = "dns-01")]
    Dns01,
    #[serde(rename = "tls-alpn-01")]
    TlsAlpn01,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChallengeStatus {
    Pending,
    Processing,
    Valid,
    Invalid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcmeErrorDetail {
    #[serde(rename = "type")]
    pub error_type: String,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subproblems: Option<Vec<AcmeErrorDetail>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CSRRequest {
    #[serde(rename = "csr")]
    pub csr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateResponse {
    pub certificate: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokeCertRequest {
    pub certificate: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedJws {
    #[serde(rename = "protected")]
    pub protected: String,
    pub payload: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwsHeader {
    pub alg: String,
    pub jwk: Option<Jwk>,
    pub url: String,
    pub nonce: Option<String>,
    #[serde(rename = "kid")]
    pub key_id: Option<String>,
}

pub fn base64url_encode(data: &[u8]) -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    URL_SAFE_NO_PAD.encode(data)
}

pub fn base64url_decode(data: &str) -> Result<Vec<u8>, &'static str> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    URL_SAFE_NO_PAD.decode(data).map_err(|_| "invalid base64url")
}
