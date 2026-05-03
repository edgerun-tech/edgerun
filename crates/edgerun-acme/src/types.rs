use crate::prelude::v1::*;
use edgerun_encoding::{base64url_decode as decode_base64url, base64url_nopad_encode};
use edgerun_json::{FromJson, JsonValue, JsonValueError, Map, ToJson};
use edgerun_url::Url;

#[derive(Debug, Clone)]
pub struct Directory {
    pub new_nonce: Url,
    pub new_account: Url,
    pub new_order: Url,
    pub revoke_cert: Url,
    pub key_change: Url,
    pub meta: Option<DirectoryMeta>,
}

#[derive(Debug, Clone)]
pub struct DirectoryMeta {
    pub terms_of_service: Option<Url>,
    pub website: Option<Url>,
    pub caa_identities: Option<Vec<String>>,
    pub external_account_required: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectoryUrl {
    LetsEncrypt,
    LetsEncryptStaging,
    Custom(Url),
}

impl DirectoryUrl {
    pub fn url(&self) -> Url {
        match self {
            DirectoryUrl::LetsEncrypt => Url::parse("https://acme-v02.api.letsencrypt.org/directory")
                .expect("built-in Let's Encrypt directory URL is valid"),
            DirectoryUrl::LetsEncryptStaging => Url::parse("https://acme-staging-v02.api.letsencrypt.org/directory")
                .expect("built-in Let's Encrypt staging directory URL is valid"),
            DirectoryUrl::Custom(u) => u.clone(),
        }
    }

    pub fn validate_custom(url: &Url) -> Result<(), JsonValueError> {
        if url.scheme() != "https" {
            return Err(expected("ACME directory URL must use https"));
        }
        if url.host().is_empty() {
            return Err(expected("ACME directory URL must include a host"));
        }
        if url.fragment().is_some() {
            return Err(expected("ACME directory URL must not contain a fragment"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Account {
    pub id: String,
    pub key: Jwk,
    pub contact: Option<Vec<String>>,
    pub status: AccountStatus,
    pub terms_of_service_agreed: Option<bool>,
    pub orders: Url,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountStatus {
    Valid,
    Deactivated,
    Revoked,
}

#[derive(Debug, Clone)]
pub enum Jwk {
    RSA { n: String, e: String },
    EC { crv: String, x: String, y: String },
}

impl Jwk {
    pub fn thumbprint(&self) -> Vec<u8> {
        use sha2::{Digest, Sha256};
        let jwk_json = match self {
            Jwk::RSA { n, e } => format!(r#"{{"e":"{e}","kty":"RSA","n":"{n}"}}"#),
            Jwk::EC { crv, x, y } => format!(r#"{{"crv":"{crv}","kty":"EC","x":"{x}","y":"{y}"}}"#),
        };
        let mut hasher = Sha256::new();
        hasher.update(jwk_json.as_bytes());
        hasher.finalize().to_vec()
    }
}

#[derive(Debug, Clone)]
pub struct NewAccountRequest {
    pub contact: Option<Vec<String>>,
    pub terms_of_service_agreed: Option<bool>,
    pub jwk: Jwk,
}

#[derive(Debug, Clone)]
pub struct NewAccountRequestWithNonce {
    pub contact: Option<Vec<String>>,
    pub terms_of_service_agreed: Option<bool>,
    pub external_account_binding: Option<ExternalAccountBinding>,
}

#[derive(Debug, Clone)]
pub struct ExternalAccountBinding {
    pub protected: String,
    pub payload: String,
    pub signature: String,
}

#[derive(Debug, Clone)]
pub struct AccountResponse {
    pub id: String,
    pub key: Jwk,
    pub contact: Option<Vec<String>>,
    pub initial_ip: Option<String>,
    pub created_at: Option<String>,
    pub status: AccountStatus,
    pub terms_of_service_agreed: Option<bool>,
    pub orders: String,
}

#[derive(Debug, Clone)]
pub struct NewOrderRequest {
    pub identifiers: Vec<Identifier>,
    pub not_before: Option<String>,
    pub not_after: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Identifier {
    pub id_type: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct Order {
    pub id: Url,
    pub status: OrderStatus,
    pub expires: Option<String>,
    pub not_before: Option<String>,
    pub not_after: Option<String>,
    pub identifiers: Option<Vec<Identifier>>,
    pub authorizations: Option<Vec<Url>>,
    pub finalize: Option<Url>,
    pub certificate: Option<Url>,
    pub error: Option<AcmeErrorDetail>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderStatus {
    Pending,
    Ready,
    Processing,
    Valid,
    Invalid,
}

#[derive(Debug, Clone)]
pub struct Authorization {
    pub id: Url,
    pub identifier: Identifier,
    pub status: AuthorizationStatus,
    pub expires: Option<String>,
    pub challenges: Option<Vec<Challenge>>,
    pub wildcard: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorizationStatus {
    Pending,
    Valid,
    Invalid,
    Deactivated,
    Expired,
    Revoked,
}

#[derive(Debug, Clone)]
pub struct Challenge {
    pub id: Option<String>,
    pub challenge_type: ChallengeType,
    pub url: Url,
    pub status: ChallengeStatus,
    pub validated: Option<String>,
    pub error: Option<AcmeErrorDetail>,
    pub token: Option<String>,
    pub authorization: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChallengeType {
    Http01,
    Dns01,
    TlsAlpn01,
    Other(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChallengeStatus {
    Pending,
    Processing,
    Valid,
    Invalid,
}

#[derive(Debug, Clone)]
pub struct AcmeErrorDetail {
    pub error_type: String,
    pub detail: String,
    pub subproblems: Option<Vec<AcmeErrorDetail>>,
}

#[derive(Debug, Clone)]
pub struct CSRRequest {
    pub csr: String,
}

#[derive(Debug, Clone)]
pub struct CertificateResponse {
    pub certificate: String,
}

#[derive(Debug, Clone)]
pub struct RevokeCertRequest {
    pub certificate: String,
    pub reason: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct SignedJws {
    pub protected: String,
    pub payload: String,
    pub signature: String,
}

#[derive(Debug, Clone)]
pub struct JwsHeader {
    pub alg: String,
    pub jwk: Option<Jwk>,
    pub url: String,
    pub nonce: Option<String>,
    pub key_id: Option<String>,
}

fn expected(message: impl Into<String>) -> JsonValueError {
    JsonValueError::WrongType(message.into())
}

fn object_from_json(value: JsonValue) -> Result<Map, JsonValueError> {
    match value {
        JsonValue::Object(object) => Ok(object),
        _ => Err(expected("expected JSON object")),
    }
}

fn required<T: FromJson>(object: &mut Map, key: &str) -> Result<T, JsonValueError> {
    T::from_json(
        object
            .remove(key)
            .ok_or_else(|| expected(format!("missing required JSON field `{key}`")))?,
    )
}

edgerun_json::impl_json_struct! {
    Directory {
        required {
            new_nonce: "newNonce" => Url,
            new_account: "newAccount" => Url,
            new_order: "newOrder" => Url,
            revoke_cert: "revokeCert" => Url,
            key_change: "keyChange" => Url
        }
        optional {
            meta: "meta" => DirectoryMeta
        }
    }
}

edgerun_json::impl_json_struct! {
    DirectoryMeta {
        required {}
        optional {
            terms_of_service: "termsOfService" => Url,
            website: "website" => Url,
            caa_identities: "caaIdentities" => Vec<String>,
            external_account_required: "externalAccountRequired" => bool
        }
    }
}

impl ToJson for DirectoryUrl {
    fn to_json(&self) -> JsonValue {
        match self {
            DirectoryUrl::LetsEncrypt => JsonValue::String("letsencrypt".to_string()),
            DirectoryUrl::LetsEncryptStaging => JsonValue::String("letsencryptstaging".to_string()),
            DirectoryUrl::Custom(url) => url.to_json(),
        }
    }
}

impl FromJson for DirectoryUrl {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let value = String::from_json(value)?;
        match value.as_str() {
            "letsencrypt" => Ok(Self::LetsEncrypt),
            "letsencryptstaging" => Ok(Self::LetsEncryptStaging),
            _ => {
                let url = Url::parse(&value).map_err(|_| expected("invalid directory URL"))?;
                Self::validate_custom(&url)?;
                Ok(Self::Custom(url))
            }
        }
    }
}

edgerun_json::impl_json_struct! {
    Account {
        required {
            id: "id" => String,
            key: "key" => Jwk,
            status: "status" => AccountStatus,
            orders: "orders" => Url
        }
        optional {
            contact: "contact" => Vec<String>,
            terms_of_service_agreed: "termsOfServiceAgreed" => bool
        }
    }
}

edgerun_json::impl_json_string_enum! {
    AccountStatus {
        Valid => "valid",
        Deactivated => "deactivated",
        Revoked => "revoked",
    }
}

impl ToJson for Jwk {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::new();
        match self {
            Jwk::RSA { n, e } => {
                object.push_field("kty", "RSA");
                object.push_field("n", n.to_json());
                object.push_field("e", e.to_json());
            }
            Jwk::EC { crv, x, y } => {
                object.push_field("kty", "EC");
                object.push_field("crv", crv.to_json());
                object.push_field("x", x.to_json());
                object.push_field("y", y.to_json());
            }
        }
        object.into()
    }
}

impl FromJson for Jwk {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object_from_json(value)?;
        if object.contains_key("n") || object.contains_key("e") {
            Ok(Jwk::RSA {
                n: required(&mut object, "n")?,
                e: required(&mut object, "e")?,
            })
        } else {
            Ok(Jwk::EC {
                crv: required(&mut object, "crv")?,
                x: required(&mut object, "x")?,
                y: required(&mut object, "y")?,
            })
        }
    }
}

edgerun_json::impl_json_struct! {
    NewAccountRequest {
        required {
            jwk: "jwk" => Jwk
        }
        optional {
            contact: "contact" => Vec<String>,
            terms_of_service_agreed: "termsOfServiceAgreed" => bool
        }
    }
}

edgerun_json::impl_json_struct! {
    NewAccountRequestWithNonce {
        required {}
        optional {
            contact: "contact" => Vec<String>,
            terms_of_service_agreed: "termsOfServiceAgreed" => bool,
            external_account_binding: "externalAccountBinding" => ExternalAccountBinding
        }
    }
}

edgerun_json::impl_json_struct! {
    ExternalAccountBinding {
        required {
            protected: "protected" => String,
            payload: "payload" => String,
            signature: "signature" => String
        }
        optional {}
    }
}

edgerun_json::impl_json_struct! {
    AccountResponse {
        required {
            id: "id" => String,
            key: "key" => Jwk,
            status: "status" => AccountStatus,
            orders: "orders" => String
        }
        optional {
            contact: "contact" => Vec<String>,
            initial_ip: "initialIP" => String,
            created_at: "createdAt" => String,
            terms_of_service_agreed: "termsOfServiceAgreed" => bool
        }
    }
}

edgerun_json::impl_json_struct! {
    NewOrderRequest {
        required {
            identifiers: "identifiers" => Vec<Identifier>
        }
        optional {
            not_before: "notBefore" => String,
            not_after: "notAfter" => String
        }
    }
}

edgerun_json::impl_json_struct! {
    Identifier {
        required {
            id_type: "type" => String,
            value: "value" => String
        }
        optional {}
    }
}

edgerun_json::impl_json_struct! {
    Order {
        required {
            id: "id" => Url,
            status: "status" => OrderStatus
        }
        optional {
            expires: "expires" => String,
            not_before: "notBefore" => String,
            not_after: "notAfter" => String,
            identifiers: "identifiers" => Vec<Identifier>,
            authorizations: "authorizations" => Vec<Url>,
            finalize: "finalize" => Url,
            certificate: "certificate" => Url,
            error: "error" => AcmeErrorDetail
        }
    }
}

edgerun_json::impl_json_string_enum! {
    OrderStatus {
        Pending => "pending",
        Ready => "ready",
        Processing => "processing",
        Valid => "valid",
        Invalid => "invalid",
    }
}

edgerun_json::impl_json_struct! {
    Authorization {
        required {
            id: "id" => Url,
            identifier: "identifier" => Identifier,
            status: "status" => AuthorizationStatus
        }
        optional {
            expires: "expires" => String,
            challenges: "challenges" => Vec<Challenge>,
            wildcard: "wildcard" => bool
        }
    }
}

edgerun_json::impl_json_string_enum! {
    AuthorizationStatus {
        Pending => "pending",
        Valid => "valid",
        Invalid => "invalid",
        Deactivated => "deactivated",
        Expired => "expired",
        Revoked => "revoked",
    }
}

edgerun_json::impl_json_struct! {
    Challenge {
        required {
            challenge_type: "type" => ChallengeType,
            url: "url" => Url,
            status: "status" => ChallengeStatus
        }
        optional {
            id: "id" => String,
            validated: "validated" => String,
            error: "error" => AcmeErrorDetail,
            token: "token" => String,
            authorization: "authorization" => String
        }
    }
}

impl ToJson for ChallengeType {
    fn to_json(&self) -> JsonValue {
        JsonValue::String(match self {
            ChallengeType::Http01 => "http-01".to_string(),
            ChallengeType::Dns01 => "dns-01".to_string(),
            ChallengeType::TlsAlpn01 => "tls-alpn-01".to_string(),
            ChallengeType::Other(value) => value.clone(),
        })
    }
}

impl FromJson for ChallengeType {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let value = <String as FromJson>::from_json(value)?;
        Ok(match value.as_str() {
            "http-01" => ChallengeType::Http01,
            "dns-01" => ChallengeType::Dns01,
            "tls-alpn-01" => ChallengeType::TlsAlpn01,
            _ => ChallengeType::Other(value),
        })
    }
}

edgerun_json::impl_json_string_enum! {
    ChallengeStatus {
        Pending => "pending",
        Processing => "processing",
        Valid => "valid",
        Invalid => "invalid",
    }
}

edgerun_json::impl_json_struct! {
    AcmeErrorDetail {
        required {
            error_type: "type" => String,
            detail: "detail" => String
        }
        optional {
            subproblems: "subproblems" => Vec<AcmeErrorDetail>
        }
    }
}

edgerun_json::impl_json_struct! {
    CSRRequest {
        required {
            csr: "csr" => String
        }
        optional {}
    }
}

edgerun_json::impl_json_struct! {
    CertificateResponse {
        required {
            certificate: "certificate" => String
        }
        optional {}
    }
}

edgerun_json::impl_json_struct! {
    RevokeCertRequest {
        required {
            certificate: "certificate" => String
        }
        optional {
            reason: "reason" => u32
        }
    }
}

edgerun_json::impl_json_struct! {
    SignedJws {
        required {
            protected: "protected" => String,
            payload: "payload" => String,
            signature: "signature" => String
        }
        optional {}
    }
}

edgerun_json::impl_json_struct! {
    JwsHeader {
        required {
            alg: "alg" => String,
            url: "url" => String
        }
        optional {
            jwk: "jwk" => Jwk,
            nonce: "nonce" => String,
            key_id: "kid" => String
        }
    }
}

pub fn base64url_encode(data: &[u8]) -> String {
    base64url_nopad_encode(data)
}

pub fn base64url_decode(data: &str) -> Result<Vec<u8>, &'static str> {
    decode_base64url(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directory_uses_acme_camel_case_fields_without_serde() {
        let directory: Directory = edgerun_json::from_json_str(
            r#"{
                "newNonce":"https://example.test/nonce",
                "newAccount":"https://example.test/account",
                "newOrder":"https://example.test/order",
                "revokeCert":"https://example.test/revoke",
                "keyChange":"https://example.test/key-change",
                "meta":{"termsOfService":"https://example.test/tos","externalAccountRequired":true}
            }"#,
        )
        .unwrap();

        assert_eq!(
            directory.new_nonce.to_string(),
            "https://example.test/nonce"
        );
        assert_eq!(
            directory
                .meta
                .as_ref()
                .and_then(|meta| meta.external_account_required),
            Some(true)
        );
        let encoded = edgerun_json::to_json_string(&directory).unwrap();
        assert!(encoded.contains(r#""newNonce":"https://example.test/nonce""#));
        assert!(encoded.contains(r#""externalAccountRequired":true"#));
    }

    #[test]
    fn jws_header_omits_absent_optional_fields() {
        let header = JwsHeader {
            alg: "ES256".to_string(),
            jwk: None,
            url: "https://example.test/order".to_string(),
            nonce: Some("nonce".to_string()),
            key_id: None,
        };

        let encoded = edgerun_json::to_json_string(&header).unwrap();
        assert!(encoded.contains(r#""nonce":"nonce""#));
        assert!(!encoded.contains("jwk"));
        assert!(!encoded.contains("kid"));
    }

    #[test]
    fn authorization_accepts_letsencrypt_challenges_without_ids() {
        let authorization: Authorization = edgerun_json::from_json_str(
            r#"{
                "id":"https://acme-staging-v02.api.letsencrypt.org/acme/authz/1",
                "identifier":{"type":"dns","value":"mail.edgerun.tech"},
                "status":"pending",
                "challenges":[{
                    "type":"dns-01",
                    "url":"https://acme-staging-v02.api.letsencrypt.org/acme/chall/1/2",
                    "status":"pending",
                    "token":"test-token"
                },{
                    "type":"dns-persist-01",
                    "url":"https://acme-staging-v02.api.letsencrypt.org/acme/chall/1/3",
                    "status":"pending"
                }]
            }"#,
        )
        .unwrap();

        let challenges = authorization.challenges.unwrap();
        let challenge = &challenges[0];
        assert_eq!(challenge.id, None);
        assert_eq!(challenge.challenge_type, ChallengeType::Dns01);
        assert_eq!(challenge.token.as_deref(), Some("test-token"));
        assert_eq!(
            challenges[1].challenge_type,
            ChallengeType::Other("dns-persist-01".to_string())
        );
    }

    #[test]
    fn jwk_thumbprint_uses_rfc7638_member_order() {
        let jwk = Jwk::EC {
            crv: "P-256".to_string(),
            x: "x-coordinate".to_string(),
            y: "y-coordinate".to_string(),
        };
        let expected_json = r#"{"crv":"P-256","kty":"EC","x":"x-coordinate","y":"y-coordinate"}"#;
        let expected = {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(expected_json.as_bytes());
            hasher.finalize().to_vec()
        };

        assert_eq!(jwk.thumbprint(), expected);
    }

    #[test]
    fn custom_directory_url_requires_https() {
        assert!(DirectoryUrl::from_json(JsonValue::String("http://example.test/directory".to_string())).is_err());
        assert!(DirectoryUrl::from_json(JsonValue::String("https://example.test/directory".to_string())).is_ok());
    }
}
