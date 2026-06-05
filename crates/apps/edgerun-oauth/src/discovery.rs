//! OIDC Discovery (RFC 8414) and JWKS document types.

use crate::errors::OAuthError;
use crate::prelude::*;
use crate::types::Scope;

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
        let root = crate::json_fixed::parse_object(json_str)?;
        let scopes_supported = root
            .string_array_field("scopes_supported")
            .into_iter()
            .map(Scope)
            .collect();

        Ok(Self {
            issuer: root.str_field("issuer").unwrap_or_default(),
            authorization_endpoint: root.str_field("authorization_endpoint").unwrap_or_default(),
            token_endpoint: root.str_field("token_endpoint").unwrap_or_default(),
            userinfo_endpoint: root.str_field("userinfo_endpoint"),
            jwks_uri: root.str_field("jwks_uri").unwrap_or_default(),
            device_authorization_endpoint: root.str_field("device_authorization_endpoint"),
            introspection_endpoint: root.str_field("introspection_endpoint"),
            revocation_endpoint: root.str_field("revocation_endpoint"),
            response_types_supported: root.string_array_field("response_types_supported"),
            grant_types_supported: root.string_array_field("grant_types_supported"),
            scopes_supported,
            subject_types_supported: root.string_array_field("subject_types_supported"),
            id_token_signing_alg_values_supported: root
                .string_array_field("id_token_signing_alg_values_supported"),
            code_challenge_methods_supported: root
                .string_array_field("code_challenge_methods_supported"),
            token_endpoint_auth_methods_supported: root
                .string_array_field("token_endpoint_auth_methods_supported"),
        })
    }

    /// Serialize to JSON string.
    pub fn to_json(&self) -> String {
        let mut out = String::from("{");
        let mut first = true;
        crate::json_fixed::write_string_field(&mut out, &mut first, "issuer", &self.issuer);
        crate::json_fixed::write_string_field(
            &mut out,
            &mut first,
            "authorization_endpoint",
            &self.authorization_endpoint,
        );
        crate::json_fixed::write_string_field(
            &mut out,
            &mut first,
            "token_endpoint",
            &self.token_endpoint,
        );
        if let Some(ref v) = self.userinfo_endpoint {
            crate::json_fixed::write_string_field(&mut out, &mut first, "userinfo_endpoint", v);
        }
        crate::json_fixed::write_string_field(&mut out, &mut first, "jwks_uri", &self.jwks_uri);
        if let Some(ref v) = self.device_authorization_endpoint {
            crate::json_fixed::write_string_field(
                &mut out,
                &mut first,
                "device_authorization_endpoint",
                v,
            );
        }
        if let Some(ref v) = self.introspection_endpoint {
            crate::json_fixed::write_string_field(
                &mut out,
                &mut first,
                "introspection_endpoint",
                v,
            );
        }
        if let Some(ref v) = self.revocation_endpoint {
            crate::json_fixed::write_string_field(&mut out, &mut first, "revocation_endpoint", v);
        }
        crate::json_fixed::write_string_array_field(
            &mut out,
            &mut first,
            "response_types_supported",
            &self.response_types_supported,
        );
        crate::json_fixed::write_string_array_field(
            &mut out,
            &mut first,
            "grant_types_supported",
            &self.grant_types_supported,
        );
        let scopes_supported = self
            .scopes_supported
            .iter()
            .map(|s| s.0.clone())
            .collect::<Vec<_>>();
        crate::json_fixed::write_string_array_field(
            &mut out,
            &mut first,
            "scopes_supported",
            &scopes_supported,
        );
        crate::json_fixed::write_string_array_field(
            &mut out,
            &mut first,
            "subject_types_supported",
            &self.subject_types_supported,
        );
        crate::json_fixed::write_string_array_field(
            &mut out,
            &mut first,
            "id_token_signing_alg_values_supported",
            &self.id_token_signing_alg_values_supported,
        );
        crate::json_fixed::write_string_array_field(
            &mut out,
            &mut first,
            "code_challenge_methods_supported",
            &self.code_challenge_methods_supported,
        );
        crate::json_fixed::write_string_array_field(
            &mut out,
            &mut first,
            "token_endpoint_auth_methods_supported",
            &self.token_endpoint_auth_methods_supported,
        );
        out.push('}');
        out
    }

    /// Discover from a base URL by fetching the well-known endpoint.
    pub async fn discover(base_url: &str) -> Result<Self, OAuthError> {
        let url = format!(
            "{}/.well-known/openid-configuration",
            base_url.trim_end_matches('/')
        );
        let client = edgerun_node::http::HttpClient::new();
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
    /// Raw object span for audit and future extension projection.
    pub raw_json: String,
}

impl JwksDocument {
    /// Parse from JSON string.
    pub fn from_json(json_str: &str) -> Result<Self, String> {
        let root = crate::json_fixed::parse_object(json_str)?;
        let keys_array = root.object_array_field("keys");
        let mut keys = Vec::new();

        for key_json in keys_array {
            let key = crate::json_fixed::parse_object(key_json)?;
            keys.push(Jwk {
                kid: key.str_field("kid"),
                kty: key.str_field("kty").unwrap_or_default(),
                alg: key.str_field("alg"),
                use_: key.str_field("use"),
                crv: key.str_field("crv"),
                x: key.str_field("x"),
                y: key.str_field("y"),
                n: key.str_field("n"),
                e: key.str_field("e"),
                k: key.str_field("k"),
                raw_json: key_json.to_string(),
            });
        }

        Ok(Self { keys })
    }

    /// Serialize to JSON string.
    pub fn to_json(&self) -> String {
        let mut out = String::from("{\"keys\":[");
        for (idx, key) in self.keys.iter().enumerate() {
            if idx != 0 {
                out.push(',');
            }
            out.push_str(&key.to_json());
        }
        out.push_str("]}");
        out
    }

    /// Fetch JWKS from a URL.
    pub async fn fetch(url: &str) -> Result<Self, OAuthError> {
        let client = edgerun_node::http::HttpClient::new();
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

impl Jwk {
    pub fn to_json(&self) -> String {
        let mut out = String::from("{");
        let mut first = true;
        if let Some(ref v) = self.kid {
            crate::json_fixed::write_string_field(&mut out, &mut first, "kid", v);
        }
        crate::json_fixed::write_string_field(&mut out, &mut first, "kty", &self.kty);
        if let Some(ref v) = self.alg {
            crate::json_fixed::write_string_field(&mut out, &mut first, "alg", v);
        }
        if let Some(ref v) = self.use_ {
            crate::json_fixed::write_string_field(&mut out, &mut first, "use", v);
        }
        if let Some(ref v) = self.crv {
            crate::json_fixed::write_string_field(&mut out, &mut first, "crv", v);
        }
        if let Some(ref v) = self.x {
            crate::json_fixed::write_string_field(&mut out, &mut first, "x", v);
        }
        if let Some(ref v) = self.y {
            crate::json_fixed::write_string_field(&mut out, &mut first, "y", v);
        }
        if let Some(ref v) = self.n {
            crate::json_fixed::write_string_field(&mut out, &mut first, "n", v);
        }
        if let Some(ref v) = self.e {
            crate::json_fixed::write_string_field(&mut out, &mut first, "e", v);
        }
        if let Some(ref v) = self.k {
            crate::json_fixed::write_string_field(&mut out, &mut first, "k", v);
        }
        out.push('}');
        out
    }
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
