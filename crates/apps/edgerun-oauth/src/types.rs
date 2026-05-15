//! OAuth 2.0 / OIDC type definitions.

use crate::oauth_client::percent_encode;
use crate::prelude::*;
use edgerun_json::{JsonValue, to_string};
pub use edgerun_protocols::oauth::types::{GrantType, Scope};

// ---------------------------------------------------------------------------
// Client configuration
// ---------------------------------------------------------------------------

/// OAuth 2.0 client configuration for the device flow.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Base URL of the OAuth provider (e.g., `https://auth.example.com`).
    pub base_url: String,
    /// OAuth 2.0 client ID.
    pub client_id: String,
    /// Optional client secret (not needed for public clients with PKCE).
    pub client_secret: Option<String>,
    /// Requested scopes.
    pub scopes: Vec<Scope>,
    /// Device authorization endpoint path (default: `/oauth2/device/code`).
    pub device_code_path: String,
    /// Token endpoint path (default: `/oauth2/token`).
    pub token_path: String,
    /// Authz code endpoint path (default: `/oauth2/authorize`).
    pub authorize_path: String,
    /// Timeout in seconds for device flow polling (default: 300).
    pub device_flow_timeout_secs: u64,
}

impl ClientConfig {
    /// Create a device-flow config with sensible defaults.
    pub fn device_flow(base_url: &str, client_id: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client_id: client_id.to_string(),
            client_secret: None,
            scopes: vec![Scope::openid(), Scope::profile(), Scope::email()],
            device_code_path: "/oauth2/device/code".into(),
            token_path: "/oauth2/token".into(),
            authorize_path: "/oauth2/authorize".into(),
            device_flow_timeout_secs: 300,
        }
    }

    /// Create an authorization-code-flow config with sensible defaults.
    pub fn authorization_code(base_url: &str, client_id: &str, redirect_uri: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client_id: client_id.to_string(),
            client_secret: None,
            scopes: vec![Scope::openid(), Scope::profile(), Scope::email()],
            device_code_path: "/oauth2/device/code".into(),
            token_path: "/oauth2/token".into(),
            authorize_path: "/oauth2/authorize".into(),
            device_flow_timeout_secs: 300,
        }
    }

    /// Full URL for the device code endpoint.
    pub fn device_code_url(&self) -> String {
        format!("{}{}", self.base_url, self.device_code_path)
    }

    /// Full URL for the token endpoint.
    pub fn token_url(&self) -> String {
        format!("{}{}", self.base_url, self.token_path)
    }

    /// Full URL for the authorization endpoint.
    pub fn authorize_url(&self) -> String {
        format!("{}{}", self.base_url, self.authorize_path)
    }
}

// ---------------------------------------------------------------------------
// Credentials
// ---------------------------------------------------------------------------

/// OAuth 2.0 / OIDC credentials returned from the token endpoint.
#[derive(Debug, Clone)]
pub struct Credentials {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub token_type: Option<String>,
    /// Unix timestamp (seconds since epoch) when the access token expires.
    pub expiry_date: Option<u64>,
    /// Optional scope string returned by the server.
    pub scope: Option<String>,
}

impl Credentials {
    /// Check if the access token is present and not expired at `now_secs`.
    pub fn is_valid_at(&self, now_secs: u64) -> bool {
        self.access_token.is_some() && !self.is_expired_at(now_secs, 0)
    }

    /// Check if the access token is expired at `now_secs` with grace period in seconds.
    pub fn is_expired_at(&self, now_secs: u64, grace_secs: u64) -> bool {
        match self.expiry_date {
            Some(expiry) => now_secs + grace_secs >= expiry,
            None => true,
        }
    }

    /// Get the bearer token.
    pub fn bearer_token(&self) -> Option<&str> {
        self.access_token.as_deref()
    }
}

// ---------------------------------------------------------------------------
// Token request / response
// ---------------------------------------------------------------------------

/// A token request to the OAuth server.
pub struct TokenRequest {
    pub grant_type: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub device_code: Option<String>,
    pub code: Option<String>,
    pub redirect_uri: Option<String>,
    pub code_verifier: Option<String>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}

impl TokenRequest {
    /// Encode as `application/x-www-form-urlencoded` body.
    pub fn to_form_body(&self) -> String {
        let mut parts = Vec::new();
        parts.push(url_encode("grant_type", &self.grant_type));
        parts.push(url_encode("client_id", &self.client_id));
        if let Some(ref secret) = self.client_secret {
            parts.push(url_encode("client_secret", secret));
        }
        if let Some(ref dc) = self.device_code {
            parts.push(url_encode("device_code", dc));
        }
        if let Some(ref code) = self.code {
            parts.push(url_encode("code", code));
        }
        if let Some(ref uri) = self.redirect_uri {
            parts.push(url_encode("redirect_uri", uri));
        }
        if let Some(ref cv) = self.code_verifier {
            parts.push(url_encode("code_verifier", cv));
        }
        if let Some(ref rt) = self.refresh_token {
            parts.push(url_encode("refresh_token", rt));
        }
        if let Some(ref scope) = self.scope {
            parts.push(url_encode("scope", scope));
        }
        parts.join("&")
    }
}

/// A token response from the OAuth server.
pub struct TokenResponse {
    pub access_token: Option<String>,
    pub token_type: Option<String>,
    pub expires_in: Option<u64>,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub scope: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

impl TokenResponse {
    /// Parse from a JSON string.
    pub fn from_json(json_str: &str) -> Result<Self, String> {
        let tape = edgerun_json::parse_json_tape(json_str)
            .map_err(|e| format!("JSON parse error: {e}"))?;
        let value = tape
            .root(json_str)
            .ok_or_else(|| "JSON parse error: missing root value".to_string())?;
        Ok(Self {
            access_token: value
                .get("access_token")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            token_type: value
                .get("token_type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            expires_in: value.get("expires_in").and_then(|v| v.as_u64()),
            refresh_token: value
                .get("refresh_token")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            id_token: value
                .get("id_token")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            scope: value
                .get("scope")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            error: value
                .get("error")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            error_description: value
                .get("error_description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        })
    }

    /// Serialize to JSON.
    pub fn to_json(&self) -> String {
        let mut obj: Vec<(String, JsonValue)> = Vec::new();
        if let Some(ref v) = self.access_token {
            obj.push(("access_token".into(), JsonValue::String(v.clone())));
        }
        if let Some(ref v) = self.token_type {
            obj.push(("token_type".into(), JsonValue::String(v.clone())));
        }
        if let Some(v) = self.expires_in {
            obj.push(("expires_in".into(), JsonValue::Number(v.into())));
        }
        if let Some(ref v) = self.refresh_token {
            obj.push(("refresh_token".into(), JsonValue::String(v.clone())));
        }
        if let Some(ref v) = self.id_token {
            obj.push(("id_token".into(), JsonValue::String(v.clone())));
        }
        if let Some(ref v) = self.scope {
            obj.push(("scope".into(), JsonValue::String(v.clone())));
        }
        if let Some(ref v) = self.error {
            obj.push(("error".into(), JsonValue::String(v.clone())));
        }
        if let Some(ref v) = self.error_description {
            obj.push(("error_description".into(), JsonValue::String(v.clone())));
        }
        let val = JsonValue::Object(edgerun_json::Map::from_iter(obj));
        to_string(&val).unwrap_or_else(|_| "{}".into())
    }

    /// Convert to `Credentials` using runtime-provided `now_secs`.
    pub fn into_credentials_at(self, now_secs: u64) -> Credentials {
        Credentials {
            access_token: self.access_token,
            refresh_token: self.refresh_token,
            id_token: self.id_token,
            token_type: self.token_type,
            expiry_date: self.expires_in.map(|e| now_secs + e),
            scope: self.scope,
        }
    }
}

// ---------------------------------------------------------------------------
// Device flow types
// ---------------------------------------------------------------------------

/// Device authorization request parameters.
pub struct DeviceAuthorizationRequest {
    pub client_id: String,
    pub scope: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
}

impl DeviceAuthorizationRequest {
    /// Encode as `application/x-www-form-urlencoded` body.
    pub fn to_form_body(&self) -> String {
        let mut parts = Vec::new();
        parts.push(url_encode("client_id", &self.client_id));
        parts.push(url_encode("scope", &self.scope));
        parts.push(url_encode("code_challenge", &self.code_challenge));
        parts.push(url_encode(
            "code_challenge_method",
            &self.code_challenge_method,
        ));
        parts.join("&")
    }
}

/// Device authorization response from the server.
pub struct DeviceAuthorizationResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_in: u64,
    pub interval: u64,
}

impl DeviceAuthorizationResponse {
    /// Parse from JSON string.
    pub fn from_json(json_str: &str) -> Result<Self, String> {
        let tape = edgerun_json::parse_json_tape(json_str)
            .map_err(|e| format!("JSON parse error: {e}"))?;
        let value = tape
            .root(json_str)
            .ok_or_else(|| "JSON parse error: missing root value".to_string())?;
        let device_code = value
            .get("device_code")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'device_code'")?
            .to_string();
        let user_code = value
            .get("user_code")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'user_code'")?
            .to_string();
        let verification_uri = value
            .get("verification_uri")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'verification_uri'")?
            .to_string();
        let verification_uri_complete = value
            .get("verification_uri_complete")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'verification_uri_complete'")?
            .to_string();
        let expires_in = value
            .get("expires_in")
            .and_then(|v| v.as_u64())
            .unwrap_or(600);
        let interval = value.get("interval").and_then(|v| v.as_u64()).unwrap_or(5);
        Ok(Self {
            device_code,
            user_code,
            verification_uri,
            verification_uri_complete,
            expires_in,
            interval,
        })
    }
}

// ---------------------------------------------------------------------------
// Auth code flow types
// ---------------------------------------------------------------------------

/// An authorization code callback.
pub trait OAuthCallback: Send + Sync {
    /// Called with the authorization URL the user should visit.
    fn display_authorization_url(&self, url: &str);
}

/// An OAuth request (generic).
pub struct OAuthRequest {
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: String,
    pub state: String,
    pub code_challenge: String,
}

impl OAuthRequest {
    pub fn to_authorization_url(&self, authorize_endpoint: &str) -> String {
        let mut parts = Vec::new();
        parts.push(url_encode("response_type", "code"));
        parts.push(url_encode("client_id", &self.client_id));
        parts.push(url_encode("redirect_uri", &self.redirect_uri));
        parts.push(url_encode("scope", &self.scope));
        parts.push(url_encode("state", &self.state));
        parts.push(url_encode("code_challenge", &self.code_challenge));
        parts.push(url_encode("code_challenge_method", "S256"));
        format!("{authorize_endpoint}?{}", parts.join("&"))
    }
}

/// An OAuth response from the redirect.
pub struct OAuthResponse {
    pub code: String,
    pub state: String,
}

// ---------------------------------------------------------------------------
// URL encoding helper
// ---------------------------------------------------------------------------

fn url_encode(key: &str, value: &str) -> String {
    edgerun_encoding::percent::url_encode_pair(key, value)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_parse_list() {
        let scopes = Scope::parse_list("openid profile email offline_access");
        assert_eq!(scopes.len(), 4);
        assert_eq!(scopes[0].0, "openid");
        assert_eq!(scopes[3].0, "offline_access");
    }

    #[test]
    fn test_scope_format_list() {
        let scopes = vec![Scope::openid(), Scope::profile()];
        assert_eq!(Scope::format_list(&scopes), "openid profile");
    }

    #[test]
    fn test_scope_presets() {
        assert_eq!(Scope::openid().0, "openid");
        assert_eq!(Scope::email().0, "email");
        assert_eq!(Scope::profile().0, "profile");
        assert_eq!(Scope::offline_access().0, "offline_access");
    }

    #[test]
    fn test_credentials_validity() {
        let now = 1_000;

        let valid = Credentials {
            access_token: Some("tok".into()),
            refresh_token: Some("ref".into()),
            id_token: None,
            token_type: Some("Bearer".into()),
            expiry_date: Some(now + 3600),
            scope: None,
        };
        assert!(valid.is_valid_at(now));
        assert!(!valid.is_expired_at(now, 0));
        assert_eq!(valid.bearer_token(), Some("tok"));
    }

    #[test]
    fn test_credentials_expired() {
        let now = 1_000;
        let expired = Credentials {
            access_token: Some("tok".into()),
            refresh_token: None,
            id_token: None,
            token_type: None,
            expiry_date: Some(now - 1),
            scope: None,
        };
        assert!(!expired.is_valid_at(now));
        assert!(expired.is_expired_at(now, 0));
    }

    #[test]
    fn test_credentials_no_access_token() {
        let no_token = Credentials {
            access_token: None,
            refresh_token: Some("ref".into()),
            id_token: None,
            token_type: None,
            expiry_date: Some(9999999999),
            scope: None,
        };
        assert!(!no_token.is_valid_at(0));
    }

    #[test]
    fn test_token_response_parse() {
        let json = r#"{"access_token":"abc","token_type":"Bearer","expires_in":3600,"refresh_token":"xyz"}"#;
        let resp = TokenResponse::from_json(json).unwrap();
        assert_eq!(resp.access_token, Some("abc".into()));
        assert_eq!(resp.token_type, Some("Bearer".into()));
        assert_eq!(resp.expires_in, Some(3600));
        assert_eq!(resp.refresh_token, Some("xyz".into()));
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_token_response_error() {
        let json = r#"{"error":"invalid_grant","error_description":"bad code"}"#;
        let resp = TokenResponse::from_json(json).unwrap();
        assert_eq!(resp.error, Some("invalid_grant".into()));
        assert_eq!(resp.error_description, Some("bad code".into()));
    }

    #[test]
    fn test_token_response_roundtrip() {
        let resp = TokenResponse {
            access_token: Some("tok".into()),
            token_type: Some("Bearer".into()),
            expires_in: Some(3600),
            refresh_token: Some("ref".into()),
            id_token: Some("jwt".into()),
            scope: Some("openid".into()),
            error: None,
            error_description: None,
        };
        let json = resp.to_json();
        let parsed = TokenResponse::from_json(&json).unwrap();
        assert_eq!(parsed.access_token, resp.access_token);
        assert_eq!(parsed.refresh_token, resp.refresh_token);
    }

    #[test]
    fn test_device_response_parse() {
        let json = r#"{"device_code":"dc","user_code":"XKCD-ABCD","verification_uri":"https://auth.example.com/verify","verification_uri_complete":"https://auth.example.com/verify?user_code=XKCD-ABCD","expires_in":600,"interval":5}"#;
        let resp = DeviceAuthorizationResponse::from_json(json).unwrap();
        assert_eq!(resp.device_code, "dc");
        assert_eq!(resp.user_code, "XKCD-ABCD");
        assert_eq!(resp.expires_in, 600);
        assert_eq!(resp.interval, 5);
    }

    #[test]
    fn test_device_response_defaults() {
        let json = r#"{"device_code":"dc","user_code":"UC","verification_uri":"https://a","verification_uri_complete":"https://b"}"#;
        let resp = DeviceAuthorizationResponse::from_json(json).unwrap();
        assert_eq!(resp.expires_in, 600);
        assert_eq!(resp.interval, 5);
    }

    #[test]
    fn test_token_request_form_body() {
        let req = TokenRequest {
            grant_type: "authorization_code".into(),
            client_id: "my-client".into(),
            client_secret: Some("secret".into()),
            device_code: None,
            code: Some("code123".into()),
            redirect_uri: Some("https://app.example.com/callback".into()),
            code_verifier: Some("verifier123".into()),
            refresh_token: None,
            scope: None,
        };
        let body = req.to_form_body();
        assert!(body.contains("grant_type=authorization_code"));
        assert!(body.contains("client_id=my-client"));
        assert!(body.contains("client_secret=secret"));
        assert!(body.contains("code=code123"));
        assert!(body.contains("code_verifier=verifier123"));
    }

    #[test]
    fn test_device_request_form_body() {
        let req = DeviceAuthorizationRequest {
            client_id: "my-client".into(),
            scope: "openid profile".into(),
            code_challenge: "challenge123".into(),
            code_challenge_method: "S256".into(),
        };
        let body = req.to_form_body();
        assert!(body.contains("client_id=my-client"));
        // Scope is percent-encoded: spaces become + or %20
        assert!(body.contains("scope=openid+profile") || body.contains("scope=openid%20profile"));
        assert!(body.contains("code_challenge=challenge123"));
        assert!(body.contains("code_challenge_method=S256"));
    }

    #[test]
    fn test_url_encoding() {
        assert_eq!(percent_encode("hello world"), "hello%20world");
        assert_eq!(percent_encode("a=b&c=d"), "a%3Db%26c%3Dd");
        assert_eq!(percent_encode("safe-._~"), "safe-._~");
    }

    #[test]
    fn test_oauth_request_to_url() {
        let req = OAuthRequest {
            client_id: "my-client".into(),
            redirect_uri: "https://app.example.com/callback".into(),
            scope: "openid profile".into(),
            state: "xyz".into(),
            code_challenge: "abc123".into(),
        };
        let url = req.to_authorization_url("https://auth.example.com/oauth2/authorize");
        assert!(url.starts_with("https://auth.example.com/oauth2/authorize?"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=my-client"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("state=xyz"));
    }
}
