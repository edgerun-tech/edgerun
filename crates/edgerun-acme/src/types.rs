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
            DirectoryUrl::LetsEncrypt => {
                Url::parse("https://acme-v02.api.letsencrypt.org/directory").unwrap()
            }
            DirectoryUrl::LetsEncryptStaging => {
                Url::parse("https://acme-staging-v02.api.letsencrypt.org/directory").unwrap()
            }
            DirectoryUrl::Custom(u) => u.clone(),
        }
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
        let jwk_json = edgerun_json::to_json_string(self).unwrap_or_default();
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
    pub jwk: Jwk,
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
    pub id: String,
    pub challenge_type: ChallengeType,
    pub url: Url,
    pub status: ChallengeStatus,
    pub validated: Option<String>,
    pub error: Option<AcmeErrorDetail>,
    pub token: Option<String>,
    pub authorization: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChallengeType {
    Http01,
    Dns01,
    TlsAlpn01,
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

fn optional<T: FromJson>(object: &mut Map, key: &str) -> Result<Option<T>, JsonValueError> {
    match object.remove(key) {
        Some(JsonValue::Null) | None => Ok(None),
        Some(value) => T::from_json(value).map(Some),
    }
}

macro_rules! impl_acme_struct_json {
    (
        $ty:ty {
            required { $($required_field:ident : $required_key:expr => $required_ty:ty),* $(,)? }
            optional { $($optional_field:ident : $optional_key:expr => $optional_ty:ty),* $(,)? }
        }
    ) => {
        impl ToJson for $ty {
            fn to_json(&self) -> JsonValue {
                let mut object = Map::new();
                $(
                    object.push_field($required_key, self.$required_field.to_json());
                )*
                $(
                    object.push_opt_field(
                        $optional_key,
                        self.$optional_field.as_ref().map(ToJson::to_json),
                    );
                )*
                object.into()
            }
        }

        impl FromJson for $ty {
            fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
                let mut object = object_from_json(value)?;
                Ok(Self {
                    $($required_field: required::<$required_ty>(&mut object, $required_key)?,)*
                    $($optional_field: optional::<$optional_ty>(&mut object, $optional_key)?,)*
                })
            }
        }
    };
}

macro_rules! impl_string_enum_json {
    ($ty:ty { $($variant:ident => $value:expr),* $(,)? }) => {
        impl ToJson for $ty {
            fn to_json(&self) -> JsonValue {
                JsonValue::String(match self {
                    $(Self::$variant => $value,)*
                }.to_string())
            }
        }

        impl FromJson for $ty {
            fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
                let value = String::from_json(value)?;
                match value.as_str() {
                    $($value => Ok(Self::$variant),)*
                    _ => Err(expected(format!("unknown {} value `{value}`", stringify!($ty)))),
                }
            }
        }
    };
}

impl_acme_struct_json! {
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

impl_acme_struct_json! {
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
            _ => Url::parse(&value)
                .map(Self::Custom)
                .map_err(|_| expected("invalid directory URL")),
        }
    }
}

impl_acme_struct_json! {
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

impl_string_enum_json! {
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
                object.push_field("n", n.to_json());
                object.push_field("e", e.to_json());
            }
            Jwk::EC { crv, x, y } => {
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

impl_acme_struct_json! {
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

impl_acme_struct_json! {
    NewAccountRequestWithNonce {
        required {
            jwk: "jwk" => Jwk
        }
        optional {
            contact: "contact" => Vec<String>,
            terms_of_service_agreed: "termsOfServiceAgreed" => bool,
            external_account_binding: "externalAccountBinding" => ExternalAccountBinding
        }
    }
}

impl_acme_struct_json! {
    ExternalAccountBinding {
        required {
            protected: "protected" => String,
            payload: "payload" => String,
            signature: "signature" => String
        }
        optional {}
    }
}

impl_acme_struct_json! {
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

impl_acme_struct_json! {
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

impl_acme_struct_json! {
    Identifier {
        required {
            id_type: "type" => String,
            value: "value" => String
        }
        optional {}
    }
}

impl_acme_struct_json! {
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

impl_string_enum_json! {
    OrderStatus {
        Pending => "pending",
        Ready => "ready",
        Processing => "processing",
        Valid => "valid",
        Invalid => "invalid",
    }
}

impl_acme_struct_json! {
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

impl_string_enum_json! {
    AuthorizationStatus {
        Pending => "pending",
        Valid => "valid",
        Invalid => "invalid",
        Deactivated => "deactivated",
        Expired => "expired",
        Revoked => "revoked",
    }
}

impl_acme_struct_json! {
    Challenge {
        required {
            id: "id" => String,
            challenge_type: "type" => ChallengeType,
            url: "url" => Url,
            status: "status" => ChallengeStatus
        }
        optional {
            validated: "validated" => String,
            error: "error" => AcmeErrorDetail,
            token: "token" => String,
            authorization: "authorization" => String
        }
    }
}

impl_string_enum_json! {
    ChallengeType {
        Http01 => "http-01",
        Dns01 => "dns-01",
        TlsAlpn01 => "tls-alpn-01",
    }
}

impl_string_enum_json! {
    ChallengeStatus {
        Pending => "pending",
        Processing => "processing",
        Valid => "valid",
        Invalid => "invalid",
    }
}

impl_acme_struct_json! {
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

impl_acme_struct_json! {
    CSRRequest {
        required {
            csr: "csr" => String
        }
        optional {}
    }
}

impl_acme_struct_json! {
    CertificateResponse {
        required {
            certificate: "certificate" => String
        }
        optional {}
    }
}

impl_acme_struct_json! {
    RevokeCertRequest {
        required {
            certificate: "certificate" => String
        }
        optional {
            reason: "reason" => u32
        }
    }
}

impl_acme_struct_json! {
    SignedJws {
        required {
            protected: "protected" => String,
            payload: "payload" => String,
            signature: "signature" => String
        }
        optional {}
    }
}

impl_acme_struct_json! {
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
}
