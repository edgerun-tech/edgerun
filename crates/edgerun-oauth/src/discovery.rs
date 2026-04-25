//! OIDC Discovery (RFC 8414) and JWKS document types.

use crate::errors::OAuthError;
use crate::types::Scope;
use edgerun_json::{from_str, JsonValue};

/// OIDC Discovery document (`.well-known/openid-configuration`).
#[derive(Debug, Clone)]
pub struct OidcDiscoveryDocument {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub userinfo_endpoint: Option<String>,
    pub jwks_uri: String,
    pub device_authorization_endpoint: Option<String>,
    pub introspection_endpoint: Option<String>,
    pub revocation_endpoint: Option<String>,
    pub response_types_supported: Vec<String>,
    pub grant_types_supported: Vec<String>,
    pub scopes_supported: Vec<Scope>,
    pub subject_types_supported: Vec<String>,
    pub id_token_signing_alg_values_supported: Vec<String>,
    pub code_challenge_methods_supported: Vec<String>,
    pub token_endpoint_auth_methods_supported: Vec<String>,
}

impl OidcDiscoveryDocument {
    /// Parse from JSON string.
    pub fn from_json(json_str: &str) -> Result<Self, String> {
        let v: JsonValue = from_str(json_str).map_err(|e| format!("JSON parse: {e}"))?;

        let str_field = |key: &str| v.get(key).and_then(|x| x.as_str()).map(|s| s.to_string());
        let str_array = |key: &str| -> Vec<String> {
            v.get(key)
                .and_then(|x| x.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str())
                        .map(|s| s.to_string())
                        .collect()
                })
                .unwrap_or_default()
        };

        let scopes_supported = str_array("scopes_supported")
            .into_iter()
            .map(Scope)
            .collect();

        Ok(Self {
            issuer: str_field("issuer").unwrap_or_default(),
            authorization_endpoint: str_field("authorization_endpoint").unwrap_or_default(),
            token_endpoint: str_field("token_endpoint").unwrap_or_default(),
            userinfo_endpoint: str_field("userinfo_endpoint"),
            jwks_uri: str_field("jwks_uri").unwrap_or_default(),
            device_authorization_endpoint: str_field("device_authorization_endpoint"),
            introspection_endpoint: str_field("introspection_endpoint"),
            revocation_endpoint: str_field("revocation_endpoint"),
            response_types_supported: str_array("response_types_supported"),
            grant_types_supported: str_array("grant_types_supported"),
            scopes_supported,
            subject_types_supported: str_array("subject_types_supported"),
            id_token_signing_alg_values_supported: str_array(
                "id_token_signing_alg_values_supported",
            ),
            code_challenge_methods_supported: str_array("code_challenge_methods_supported"),
            token_endpoint_auth_methods_supported: str_array(
                "token_endpoint_auth_methods_supported",
            ),
        })
    }

    /// Serialize to JSON string.
    pub fn to_json(&self) -> String {
        use edgerun_json::{to_string, JsonValue, Map};
        let mut obj = Vec::new();
        obj.push(("issuer".into(), JsonValue::String(self.issuer.clone())));
        obj.push((
            "authorization_endpoint".into(),
            JsonValue::String(self.authorization_endpoint.clone()),
        ));
        obj.push((
            "token_endpoint".into(),
            JsonValue::String(self.token_endpoint.clone()),
        ));
        if let Some(ref v) = self.userinfo_endpoint {
            obj.push(("userinfo_endpoint".into(), JsonValue::String(v.clone())));
        }
        obj.push(("jwks_uri".into(), JsonValue::String(self.jwks_uri.clone())));
        if let Some(ref v) = self.device_authorization_endpoint {
            obj.push((
                "device_authorization_endpoint".into(),
                JsonValue::String(v.clone()),
            ));
        }
        if let Some(ref v) = self.introspection_endpoint {
            obj.push((
                "introspection_endpoint".into(),
                JsonValue::String(v.clone()),
            ));
        }
        if let Some(ref v) = self.revocation_endpoint {
            obj.push(("revocation_endpoint".into(), JsonValue::String(v.clone())));
        }
        obj.push((
            "response_types_supported".into(),
            str_array(&self.response_types_supported),
        ));
        obj.push((
            "grant_types_supported".into(),
            str_array(&self.grant_types_supported),
        ));
        obj.push((
            "scopes_supported".into(),
            str_array(
                &self
                    .scopes_supported
                    .iter()
                    .map(|s| s.0.clone())
                    .collect::<Vec<_>>(),
            ),
        ));
        obj.push((
            "subject_types_supported".into(),
            str_array(&self.subject_types_supported),
        ));
        obj.push((
            "id_token_signing_alg_values_supported".into(),
            str_array(&self.id_token_signing_alg_values_supported),
        ));
        obj.push((
            "code_challenge_methods_supported".into(),
            str_array(&self.code_challenge_methods_supported),
        ));
        obj.push((
            "token_endpoint_auth_methods_supported".into(),
            str_array(&self.token_endpoint_auth_methods_supported),
        ));
        let val = JsonValue::Object(Map::from_iter(obj));
        to_string(&val).unwrap_or_else(|_| "{}".into())
    }

    /// Discover from a base URL by fetching the well-known endpoint.
    pub async fn discover(base_url: &str) -> Result<Self, OAuthError> {
        let url = format!(
            "{}/.well-known/openid-configuration",
            base_url.trim_end_matches('/')
        );
        let client = edgerun_http::HttpClient::new();
        let resp = client
            .get(&url)
            .await
            .map_err(|e| OAuthError::HttpError(e.to_string()))?;
        if resp.status().as_u16() != 200 {
            return Err(OAuthError::HttpError(format!(
                "HTTP {}",
                resp.status().as_u16()
            )));
        }
        let body = String::from_utf8_lossy(resp.body()).to_string();
        Self::from_json(&body).map_err(OAuthError::JsonError)
    }
}

/// JWKS document (`.well-known/jwks.json`).
#[derive(Debug, Clone)]
pub struct JwksDocument {
    pub keys: Vec<Jwk>,
}

/// JSON Web Key (RFC 7517).
#[derive(Debug, Clone)]
pub struct Jwk {
    pub kid: Option<String>,
    pub kty: String,
    pub alg: Option<String>,
    pub use_: Option<String>,
    /// For EC keys
    pub crv: Option<String>,
    pub x: Option<String>,
    pub y: Option<String>,
    /// For RSA keys
    pub n: Option<String>,
    pub e: Option<String>,
    /// For symmetric keys
    pub k: Option<String>,
    /// Raw JSON for extensibility
    pub raw: JsonValue,
}

impl JwksDocument {
    /// Parse from JSON string.
    pub fn from_json(json_str: &str) -> Result<Self, String> {
        let v: JsonValue = from_str(json_str).map_err(|e| format!("JSON parse: {e}"))?;
        let keys_array = v
            .get("keys")
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default();
        let mut keys = Vec::new();

        for key_json in keys_array {
            let str_field = |k: &str| {
                key_json
                    .get(k)
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string())
            };

            keys.push(Jwk {
                kid: str_field("kid"),
                kty: str_field("kty").unwrap_or_default(),
                alg: str_field("alg"),
                use_: str_field("use"),
                crv: str_field("crv"),
                x: str_field("x"),
                y: str_field("y"),
                n: str_field("n"),
                e: str_field("e"),
                k: str_field("k"),
                raw: key_json,
            });
        }

        Ok(Self { keys })
    }

    /// Serialize to JSON string.
    pub fn to_json(&self) -> String {
        use edgerun_json::{to_string, JsonValue, Map};
        let keys_arr: Vec<JsonValue> = self.keys.iter().map(|k| k.raw.clone()).collect();
        let obj = vec![("keys".into(), JsonValue::Array(keys_arr))];
        let val = JsonValue::Object(Map::from_iter(obj));
        to_string(&val).unwrap_or_else(|_| "{}".into())
    }

    /// Fetch JWKS from a URL.
    pub async fn fetch(url: &str) -> Result<Self, OAuthError> {
        let client = edgerun_http::HttpClient::new();
        let resp = client
            .get(url)
            .await
            .map_err(|e| OAuthError::HttpError(e.to_string()))?;
        if resp.status().as_u16() != 200 {
            return Err(OAuthError::HttpError(format!(
                "HTTP {}",
                resp.status().as_u16()
            )));
        }
        let body = String::from_utf8_lossy(resp.body()).to_string();
        Self::from_json(&body).map_err(OAuthError::JsonError)
    }

    /// Find a key by kid.
    pub fn find_by_kid(&self, kid: &str) -> Option<&Jwk> {
        self.keys.iter().find(|k| k.kid.as_deref() == Some(kid))
    }
}

fn str_array(strings: &[String]) -> JsonValue {
    use edgerun_json::JsonValue;
    JsonValue::Array(
        strings
            .iter()
            .map(|s| JsonValue::String(s.clone()))
            .collect(),
    )
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovery_parse_minimal() {
        let json = r#"{
            "issuer": "https://auth.example.com",
            "authorization_endpoint": "https://auth.example.com/authorize",
            "token_endpoint": "https://auth.example.com/token",
            "jwks_uri": "https://auth.example.com/jwks"
        }"#;
        let doc = OidcDiscoveryDocument::from_json(json).unwrap();
        assert_eq!(doc.issuer, "https://auth.example.com");
        assert_eq!(
            doc.authorization_endpoint,
            "https://auth.example.com/authorize"
        );
        assert_eq!(doc.token_endpoint, "https://auth.example.com/token");
        assert_eq!(doc.jwks_uri, "https://auth.example.com/jwks");
    }

    #[test]
    fn test_discovery_parse_full() {
        let json = r#"{
            "issuer": "https://auth.example.com",
            "authorization_endpoint": "https://auth.example.com/authorize",
            "token_endpoint": "https://auth.example.com/token",
            "userinfo_endpoint": "https://auth.example.com/userinfo",
            "jwks_uri": "https://auth.example.com/jwks",
            "device_authorization_endpoint": "https://auth.example.com/device/code",
            "introspection_endpoint": "https://auth.example.com/introspect",
            "response_types_supported": ["code"],
            "grant_types_supported": ["authorization_code", "refresh_token"],
            "scopes_supported": ["openid", "profile", "email"],
            "subject_types_supported": ["public"],
            "id_token_signing_alg_values_supported": ["ES256"],
            "code_challenge_methods_supported": ["S256"]
        }"#;
        let doc = OidcDiscoveryDocument::from_json(json).unwrap();
        assert_eq!(
            doc.userinfo_endpoint,
            Some("https://auth.example.com/userinfo".into())
        );
        assert_eq!(
            doc.device_authorization_endpoint,
            Some("https://auth.example.com/device/code".into())
        );
        assert_eq!(doc.response_types_supported, vec!["code"]);
        assert_eq!(doc.scopes_supported.len(), 3);
    }

    #[test]
    fn test_discovery_serialization_roundtrip() {
        let json = r#"{
            "issuer": "https://auth.example.com",
            "authorization_endpoint": "https://auth.example.com/authorize",
            "token_endpoint": "https://auth.example.com/token",
            "jwks_uri": "https://auth.example.com/jwks",
            "response_types_supported": ["code"],
            "grant_types_supported": ["authorization_code"],
            "scopes_supported": ["openid"],
            "subject_types_supported": ["public"],
            "id_token_signing_alg_values_supported": ["ES256"],
            "code_challenge_methods_supported": ["S256"],
            "token_endpoint_auth_methods_supported": ["client_secret_post"]
        }"#;
        let doc = OidcDiscoveryDocument::from_json(json).unwrap();
        let serialized = doc.to_json();
        let parsed = OidcDiscoveryDocument::from_json(&serialized).unwrap();
        assert_eq!(parsed.issuer, doc.issuer);
        assert_eq!(parsed.token_endpoint, doc.token_endpoint);
    }

    #[test]
    fn test_jwks_parse() {
        let json = r#"{
            "keys": [
                {"kty": "EC", "crv": "P-256", "x": "abc", "y": "def", "alg": "ES256", "use": "sig", "kid": "key1"}
            ]
        }"#;
        let doc = JwksDocument::from_json(json).unwrap();
        assert_eq!(doc.keys.len(), 1);
        let key = &doc.keys[0];
        assert_eq!(key.kty, "EC");
        assert_eq!(key.crv.as_deref(), Some("P-256"));
        assert_eq!(key.kid.as_deref(), Some("key1"));
    }

    #[test]
    fn test_jwks_find_by_kid() {
        let json = r#"{
            "keys": [
                {"kty": "EC", "crv": "P-256", "x": "abc", "y": "def", "kid": "key1"},
                {"kty": "EC", "crv": "P-256", "x": "ghi", "y": "jkl", "kid": "key2"}
            ]
        }"#;
        let doc = JwksDocument::from_json(json).unwrap();
        assert!(doc.find_by_kid("key1").is_some());
        assert!(doc.find_by_kid("key2").is_some());
        assert!(doc.find_by_kid("key3").is_none());
    }
}
