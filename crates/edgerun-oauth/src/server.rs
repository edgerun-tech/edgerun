//! OAuth 2.0 + OIDC server implementation.
//!
//! Provides request handlers for:
//! - OIDC Discovery document (`/.well-known/openid-configuration`)
//! - JWKS document (`/.well-known/jwks.json`)
//! - Device authorization endpoint (`POST /oauth2/device/code`)
//! - Token endpoint (`POST /oauth2/token`)
//! - Token introspection (`POST /oauth2/introspect`)
//! - Userinfo endpoint (`GET /oauth2/userinfo`)
//!
//! `OAuthServer` implements `edgerun_http::Handler` so it can be mounted
//! directly on any `HttpServer`.

use crate::device_state::{DeviceGrantStore, PendingDeviceGrant};
use crate::discovery::{Jwk, JwksDocument};
use crate::errors::{DeviceError, OAuthError, OAuthResult};
use crate::jwt::{IdToken, JwtVerifier};
use crate::pkce::PkcePair;
use crate::prelude::*;
use crate::types::{Credentials, TokenResponse};
use edgerun_crypto::ecdsa::Signature;
use edgerun_crypto::fill_random;
use edgerun_crypto::p256::ecdsa::signature::Signer;
use edgerun_crypto::p256::ecdsa::SigningKey;
use edgerun_crypto::sha256;
use edgerun_encoding::base64::base64url_nopad_encode;
use edgerun_encoding::percent::{parse_form_urlencoded as parse_form_btree, percent_decode};
use edgerun_http::{Handler, Middleware, Next, Request, Response, StatusCode};
use edgerun_json::{from_str, to_string, JsonNumber, JsonValue, Map};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::{Duration, Instant};

// ===========================================================================
// Poison-safe RwLock helpers
// ===========================================================================

fn read_lock<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    #[cfg(target_os = "none")]
    {
        lock.read().unwrap()
    }
    #[cfg(not(target_os = "none"))]
    lock.read().unwrap_or_else(|e| e.into_inner())
}
fn write_lock<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    #[cfg(target_os = "none")]
    {
        lock.write().unwrap()
    }
    #[cfg(not(target_os = "none"))]
    lock.write().unwrap_or_else(|e| e.into_inner())
}

// ===========================================================================
// Server configuration
// ===========================================================================

/// Server configuration.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// The issuer URL (e.g., `https://auth.example.com`).
    pub issuer_url: String,
    /// Issuer identifier (usually the same as issuer URL).
    pub issuer: String,
    /// Device authorization endpoint path.
    pub device_code_path: String,
    /// Token endpoint path.
    pub token_path: String,
    /// Authorization endpoint path.
    pub authorize_path: String,
    /// Userinfo endpoint path.
    pub userinfo_path: String,
    /// Introspection endpoint path.
    pub introspect_path: String,
    /// JWKS URI path.
    pub jwks_path: String,
    /// Discovery document path.
    pub discovery_path: String,
    /// Device code expiry in seconds.
    pub device_code_expiry_secs: u64,
    /// Device code polling interval in seconds.
    pub device_code_interval_secs: u64,
    /// Access token TTL in seconds.
    pub access_token_ttl_secs: u64,
    /// Refresh token TTL in seconds.
    pub refresh_token_ttl_secs: u64,
}

impl ServerConfig {
    pub fn new(issuer_url: &str, issuer: &str) -> Self {
        let base = issuer_url.trim_end_matches('/').to_string();
        Self {
            device_code_path: "/oauth2/device/code".into(),
            token_path: "/oauth2/token".into(),
            authorize_path: "/oauth2/authorize".into(),
            userinfo_path: "/oauth2/userinfo".into(),
            introspect_path: "/oauth2/introspect".into(),
            jwks_path: "/.well-known/jwks.json".into(),
            discovery_path: "/.well-known/openid-configuration".into(),
            issuer_url: base.clone(),
            issuer: issuer.to_string(),
            device_code_expiry_secs: 600,
            device_code_interval_secs: 5,
            access_token_ttl_secs: 3600,
            refresh_token_ttl_secs: 86400 * 30,
        }
    }

    /// Check if a request path matches one of the OAuth endpoints.
    pub fn matches(&self, path: &str) -> bool {
        path == self.device_code_path
            || path == self.token_path
            || path == self.authorize_path
            || path == self.userinfo_path
            || path == self.introspect_path
            || path == self.jwks_path
            || path == self.discovery_path
    }
}

// ===========================================================================
// Client registration
// ===========================================================================

/// A registered client on the server.
#[derive(Debug, Clone)]
pub struct ClientRegistration {
    pub client_id: String,
    pub client_name: String,
    pub client_secret: Option<String>,
    pub redirect_uris: Vec<String>,
    pub grant_types: Vec<String>,
    pub scopes: Vec<String>,
    pub created_at: Instant,
}

// ===========================================================================
// Claims for request enrichment
// ===========================================================================

/// OIDC claims extracted from a validated access token.
///
/// Store this in `Request` extensions via `BearerAuthMiddleware` so
/// downstream handlers can access the authenticated user context.
#[derive(Debug, Clone)]
pub struct Claims {
    pub sub: String,
    pub client_id: String,
    pub scopes: Vec<String>,
    pub token_type: String,
    pub exp: u64,
    pub iat: u64,
}

// ===========================================================================
// Token entry for introspection
// ===========================================================================

#[derive(Debug, Clone)]
struct TokenEntry {
    pub client_id: String,
    pub scopes: Vec<String>,
    pub active: bool,
    pub token_type: String,
    pub exp: u64,
    pub iat: u64,
    pub sub: Option<String>,
    /// Raw access_token (stored as-is for bearer lookup).
    pub raw_token: String,
}

// ===========================================================================
// OAuthServer
// ===========================================================================

/// The OAuth 2.0 + OIDC server.
///
/// Implements `edgerun_http::Handler` so it can be mounted directly on any
/// `HttpServer`. Routes internally by request path.
///
/// # Example
/// ```no_run
/// use edgerun_oauth::{OAuthServer, ServerConfig};
///
/// # edgerun_rt::block_on(async {
/// let config = ServerConfig::new("https://auth.example.com", "my-issuer");
/// let oauth = OAuthServer::new(config).unwrap();
///
/// // Register a client
/// let client = oauth.register_client("my-app", vec!["openid".into(), "email".into()]);
/// println!("client_id: {}", client.client_id);
///
/// // `oauth` implements `edgerun_http::Handler` and can be mounted by
/// // the HTTP server when the server feature is enabled.
/// # });
/// ```
pub struct OAuthServer {
    config: ServerConfig,
    /// Registered clients.
    clients: RwLock<HashMap<String, ClientRegistration>>,
    /// Pending device grants.
    device_store: Arc<DeviceGrantStore>,
    /// Issued tokens (access_token -> TokenEntry mapping for introspection/userinfo).
    tokens: RwLock<HashMap<String, TokenEntry>>,
    /// P-256 key pair for signing ID tokens (ES256).
    signing_key: SigningKey,
    /// The kid for our signing key.
    signing_key_id: String,
}

impl OAuthServer {
    /// Create a new OAuth server with an auto-generated P-256 signing key.
    pub fn new(config: ServerConfig) -> OAuthResult<Self> {
        let signing_key = edgerun_crypto::random_p256_signing_key();
        let key_bytes = base64url_nopad_encode(&sha256(b"server-signing-key"));
        let signing_key_id = key_bytes.chars().take(16).collect();

        Ok(Self {
            config,
            clients: RwLock::new(HashMap::new()),
            device_store: Arc::new(DeviceGrantStore::new()),
            tokens: RwLock::new(HashMap::new()),
            signing_key,
            signing_key_id,
        })
    }

    /// Create from a reference-counted signing key (for key rotation / HSM).
    pub fn with_signing_key(config: ServerConfig, signing_key: SigningKey) -> Self {
        let key_bytes = base64url_nopad_encode(&sha256(b"server-signing-key"));
        let signing_key_id = key_bytes.chars().take(16).collect();
        Self {
            config,
            clients: RwLock::new(HashMap::new()),
            device_store: Arc::new(DeviceGrantStore::new()),
            tokens: RwLock::new(HashMap::new()),
            signing_key,
            signing_key_id,
        }
    }

    /// Register a new client.
    pub fn register_client(&self, name: &str, scopes: Vec<String>) -> ClientRegistration {
        let client_id = generate_token(32);
        let client_secret = generate_token(48);

        let registration = ClientRegistration {
            client_id,
            client_name: name.to_string(),
            client_secret: Some(client_secret),
            redirect_uris: vec![],
            grant_types: vec![
                "urn:ietf:params:oauth:grant-type:device_code".into(),
                "authorization_code".into(),
                "refresh_token".into(),
            ],
            scopes,
            created_at: Instant::now(),
        };

        let id = registration.client_id.clone();
        write_lock(&self.clients).insert(id, registration.clone());
        registration
    }

    /// Register a client with explicit ID and secret.
    pub fn register_client_with_id(
        &self,
        client_id: &str,
        client_secret: &str,
        scopes: Vec<String>,
    ) -> ClientRegistration {
        let registration = ClientRegistration {
            client_id: client_id.to_string(),
            client_name: client_id.to_string(),
            client_secret: Some(client_secret.to_string()),
            redirect_uris: vec![],
            grant_types: vec![
                "urn:ietf:params:oauth:grant-type:device_code".into(),
                "authorization_code".into(),
                "refresh_token".into(),
            ],
            scopes,
            created_at: Instant::now(),
        };

        write_lock(&self.clients).insert(client_id.to_string(), registration.clone());
        registration
    }

    /// Look up a client by ID.
    pub fn get_client(&self, client_id: &str) -> Option<ClientRegistration> {
        read_lock(&self.clients).get(client_id).cloned()
    }

    // -----------------------------------------------------------------------
    // Handler: OIDC Discovery
    // -----------------------------------------------------------------------

    /// Generate the OIDC Discovery document as an HTTP response.
    pub fn discovery_response(&self) -> Response {
        let mut scopes = std::collections::HashSet::new();
        scopes.insert("openid");
        scopes.insert("profile");
        scopes.insert("email");
        let clients = read_lock(&self.clients);
        for client in clients.values() {
            for s in &client.scopes {
                scopes.insert(s.as_str());
            }
        }
        let scopes_list: Vec<String> = scopes.into_iter().map(|s| s.to_string()).collect();

        let doc = crate::discovery::OidcDiscoveryDocument {
            issuer: self.config.issuer.clone(),
            authorization_endpoint: format!(
                "{}{}",
                self.config.issuer_url, self.config.authorize_path
            ),
            token_endpoint: format!("{}{}", self.config.issuer_url, self.config.token_path),
            userinfo_endpoint: Some(format!(
                "{}{}",
                self.config.issuer_url, self.config.userinfo_path
            )),
            jwks_uri: format!("{}{}", self.config.issuer_url, self.config.jwks_path),
            device_authorization_endpoint: Some(format!(
                "{}{}",
                self.config.issuer_url, self.config.device_code_path
            )),
            introspection_endpoint: Some(format!(
                "{}{}",
                self.config.issuer_url, self.config.introspect_path
            )),
            revocation_endpoint: None,
            response_types_supported: vec!["code".into()],
            grant_types_supported: vec![
                "urn:ietf:params:oauth:grant-type:device_code".into(),
                "authorization_code".into(),
                "refresh_token".into(),
            ],
            scopes_supported: scopes_list.into_iter().map(crate::types::Scope).collect(),
            subject_types_supported: vec!["public".into()],
            id_token_signing_alg_values_supported: vec!["ES256".into()],
            code_challenge_methods_supported: vec!["S256".into()],
            token_endpoint_auth_methods_supported: vec!["client_secret_post".into()],
        };

        Response::json(StatusCode::new(200).unwrap(), &doc.to_json())
    }

    // -----------------------------------------------------------------------
    // Handler: JWKS
    // -----------------------------------------------------------------------

    /// Generate the JWKS document as an HTTP response.
    pub fn jwks_response(&self) -> Response {
        use edgerun_crypto::elliptic_curve::sec1::ToEncodedPoint;
        use edgerun_crypto::p256::ecdsa::VerifyingKey;

        let vk = VerifyingKey::from(&self.signing_key);
        let point = vk.to_encoded_point(false);
        let raw = point.as_bytes();

        let mut fields: Vec<(String, JsonValue)> = Vec::new();
        fields.push(("kty".into(), JsonValue::String("EC".into())));
        fields.push(("crv".into(), JsonValue::String("P-256".into())));
        fields.push(("alg".into(), JsonValue::String("ES256".into())));
        fields.push(("use".into(), JsonValue::String("sig".into())));
        fields.push(("kid".into(), JsonValue::String(self.signing_key_id.clone())));
        fields.push((
            "x".into(),
            JsonValue::String(base64url_nopad_encode(&raw[1..33])),
        ));
        fields.push((
            "y".into(),
            JsonValue::String(base64url_nopad_encode(&raw[33..65])),
        ));

        let jwk_json =
            to_string(&JsonValue::Object(Map::from_iter(fields))).unwrap_or_else(|_| "{}".into());
        let jwk_raw: JsonValue = from_str(&jwk_json).unwrap_or(JsonValue::Object(Map::new()));

        let keys_arr = vec![jwk_raw];
        let mut obj: Vec<(String, JsonValue)> = Vec::new();
        obj.push(("keys".into(), JsonValue::Array(keys_arr)));
        let doc_json =
            to_string(&JsonValue::Object(Map::from_iter(obj))).unwrap_or_else(|_| "{}".into());

        Response::json(StatusCode::new(200).unwrap(), &doc_json)
    }

    /// Get the JWT verifier for the server's signing key.
    pub fn jwt_verifier(&self) -> JwtVerifier {
        use edgerun_crypto::p256::ecdsa::VerifyingKey;
        JwtVerifier::Es256 {
            verifying_key: VerifyingKey::from(&self.signing_key),
        }
    }

    // -----------------------------------------------------------------------
    // Handler: Device Authorization (POST /oauth2/device/code)
    // -----------------------------------------------------------------------

    /// Handle a device authorization request.
    pub fn handle_device_code(&self, form_body: &str) -> Response {
        let params = parse_form_urlencoded(form_body);

        let client_id = match params.get("client_id") {
            Some(id) => id,
            None => return self.oauth_error("invalid_request", "missing client_id", 400),
        };
        let code_challenge = match params.get("code_challenge") {
            Some(cc) => cc,
            None => return self.oauth_error("invalid_request", "missing code_challenge", 400),
        };
        let cc_method = match params.get("code_challenge_method") {
            Some(m) => m,
            None => {
                return self.oauth_error("invalid_request", "missing code_challenge_method", 400)
            }
        };

        if cc_method != "S256" {
            return self.oauth_error(
                "invalid_request",
                "only S256 code_challenge_method is supported",
                400,
            );
        }

        // Verify client exists
        if read_lock(&self.clients).get(client_id).is_none() {
            return self.oauth_error("invalid_client", "unknown client_id", 401);
        }

        let scopes: Vec<String> = params
            .get("scope")
            .map(|s| s.split_whitespace().map(|s| s.to_string()).collect())
            .unwrap_or_else(|| vec!["openid".into()]);

        let base_uri = format!("{}{}", self.config.issuer_url, self.config.authorize_path);
        let grant = match PendingDeviceGrant::generate(client_id, scopes, code_challenge, &base_uri)
        {
            Ok(g) => g,
            Err(e) => return Response::text(StatusCode::new(500).unwrap(), &e.to_string()),
        };

        let device_code = grant.device_code.clone();
        let user_code = grant.user_code.clone();
        let verification_uri = grant.verification_uri.clone();
        let verification_uri_complete = grant.verification_uri_complete.clone();
        let expires_in = self.config.device_code_expiry_secs;
        let interval = self.config.device_code_interval_secs;

        self.device_store.store(grant);

        let mut fields: Vec<(String, JsonValue)> = Vec::new();
        fields.push(("device_code".into(), JsonValue::String(device_code)));
        fields.push(("user_code".into(), JsonValue::String(user_code)));
        fields.push((
            "verification_uri".into(),
            JsonValue::String(verification_uri),
        ));
        fields.push((
            "verification_uri_complete".into(),
            JsonValue::String(verification_uri_complete),
        ));
        fields.push(("expires_in".into(), JsonValue::from(expires_in)));
        fields.push(("interval".into(), JsonValue::from(interval)));

        let val = JsonValue::Object(Map::from_iter(fields));
        let json = to_string(&val).unwrap_or_else(|_| "{}".into());
        Response::json(StatusCode::new(200).unwrap(), &json)
    }

    // -----------------------------------------------------------------------
    // Handler: Authorize device grant (user visits verification URI)
    // -----------------------------------------------------------------------

    /// Mark a device grant as authorized.
    pub fn authorize_device_grant(&self, device_code: &str) -> bool {
        self.device_store.authorize_by_device_code(device_code)
    }

    /// Look up a device grant by user code.
    pub fn lookup_device_grant_by_user_code(
        &self,
        user_code: &str,
    ) -> Option<(String, Vec<String>)> {
        let grant = self.device_store.lookup_by_user_code(user_code)?;
        if grant.is_expired() {
            return None;
        }
        Some((grant.client_id, grant.scopes))
    }

    // -----------------------------------------------------------------------
    // Handler: Token Endpoint (POST /oauth2/token)
    // -----------------------------------------------------------------------

    /// Handle a token request.
    pub fn handle_token(&self, form_body: &str) -> Response {
        let params = parse_form_urlencoded(form_body);

        let grant_type = match params.get("grant_type") {
            Some(gt) => gt,
            None => return self.oauth_error("invalid_request", "missing grant_type", 400),
        };

        match grant_type.as_str() {
            "urn:ietf:params:oauth:grant-type:device_code" => self.handle_device_token(&params),
            "authorization_code" => self.handle_auth_code_token(&params),
            "refresh_token" => self.handle_refresh_token(&params),
            _ => self.oauth_error(
                "unsupported_grant_type",
                &format!("unsupported grant_type: {grant_type}"),
                400,
            ),
        }
    }

    fn handle_device_token(&self, params: &HashMap<String, String>) -> Response {
        let client_id = match params.get("client_id") {
            Some(id) => id,
            None => return self.oauth_error("invalid_request", "missing client_id", 400),
        };
        let device_code = match params.get("device_code") {
            Some(dc) => dc,
            None => return self.oauth_error("invalid_request", "missing device_code", 400),
        };
        let code_verifier = match params.get("code_verifier") {
            Some(cv) => cv,
            None => return self.oauth_error("invalid_request", "missing code_verifier", 400),
        };

        // Verify client exists
        let clients = read_lock(&self.clients);
        let client = match clients.get(client_id) {
            Some(c) => c,
            None => return self.oauth_error("invalid_client", "unknown client", 401),
        };

        // Verify client_secret if set
        if let Some(ref secret) = client.client_secret {
            if let Some(provided) = params.get("client_secret") {
                if provided != secret {
                    return self.oauth_error("invalid_client", "invalid client_secret", 401);
                }
            } else {
                return self.oauth_error("invalid_client", "client_secret required", 401);
            }
        }

        let grant = match self.device_store.lookup_by_device_code(device_code) {
            Some(g) => g,
            None => return self.oauth_error("invalid_grant", "unknown device code", 400),
        };

        if grant.is_expired() {
            self.device_store.remove(device_code);
            return self.oauth_error("expired_token", "device code expired", 400);
        }

        if !grant.authorized {
            return self.oauth_error("authorization_pending", "user has not yet authorized", 400);
        }

        // Verify PKCE
        let computed_challenge = base64url_nopad_encode(&sha256(code_verifier.as_bytes()));
        if computed_challenge != grant.code_challenge {
            return self.oauth_error("invalid_grant", "PKCE code_verifier does not match", 400);
        }

        // Validate requested scopes against registered client scopes
        let granted_scopes = self.validate_scopes(&grant.scopes, &client.scopes);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let access_token = generate_token(48);
        let refresh_token = generate_token(48);

        // Sign ID token with at_hash
        let id_token =
            match self.sign_id_token_with_at_hash(&grant.client_id, &granted_scopes, &access_token)
            {
                Ok(t) => t,
                Err(e) => return Response::text(StatusCode::new(500).unwrap(), &e.to_string()),
            };

        // Store token for introspection
        write_lock(&self.tokens).insert(
            access_token.clone(),
            TokenEntry {
                client_id: client_id.clone(),
                scopes: granted_scopes.clone(),
                active: true,
                token_type: "Bearer".into(),
                exp: now + self.config.access_token_ttl_secs,
                iat: now,
                sub: Some(grant.client_id.clone()),
                raw_token: access_token.clone(),
            },
        );

        self.device_store.remove(device_code);

        self.token_response(
            &access_token,
            &refresh_token,
            Some(&id_token),
            self.config.access_token_ttl_secs,
        )
    }

    fn handle_auth_code_token(&self, params: &HashMap<String, String>) -> Response {
        let client_id = match params.get("client_id") {
            Some(id) => id,
            None => return self.oauth_error("invalid_request", "missing client_id", 400),
        };
        let code_verifier = match params.get("code_verifier") {
            Some(cv) => cv,
            None => return self.oauth_error("invalid_request", "missing code_verifier", 400),
        };

        let clients = read_lock(&self.clients);
        let client = match clients.get(client_id) {
            Some(c) => c,
            None => return self.oauth_error("invalid_client", "unknown client", 401),
        };

        // Verify client_secret
        if let Some(ref secret) = client.client_secret {
            if let Some(provided) = params.get("client_secret") {
                if provided != secret {
                    return self.oauth_error("invalid_client", "invalid client_secret", 401);
                }
            } else {
                return self.oauth_error("invalid_client", "client_secret required", 401);
            }
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let access_token = generate_token(48);
        let refresh_token = generate_token(48);
        let granted_scopes = client.scopes.clone();

        let id_token =
            match self.sign_id_token_with_at_hash(client_id, &granted_scopes, &access_token) {
                Ok(t) => t,
                Err(e) => return Response::text(StatusCode::new(500).unwrap(), &e.to_string()),
            };

        write_lock(&self.tokens).insert(
            access_token.clone(),
            TokenEntry {
                client_id: client_id.clone(),
                scopes: granted_scopes.clone(),
                active: true,
                token_type: "Bearer".into(),
                exp: now + self.config.access_token_ttl_secs,
                iat: now,
                sub: Some(client_id.clone()),
                raw_token: access_token.clone(),
            },
        );

        self.token_response(
            &access_token,
            &refresh_token,
            Some(&id_token),
            self.config.access_token_ttl_secs,
        )
    }

    fn handle_refresh_token(&self, params: &HashMap<String, String>) -> Response {
        let client_id = match params.get("client_id") {
            Some(id) => id,
            None => return self.oauth_error("invalid_request", "missing client_id", 400),
        };
        let refresh_token = match params.get("refresh_token") {
            Some(rt) => rt,
            None => return self.oauth_error("invalid_request", "missing refresh_token", 400),
        };

        let clients = read_lock(&self.clients);
        let client = match clients.get(client_id) {
            Some(c) => c,
            None => return self.oauth_error("invalid_client", "unknown client", 401),
        };

        // Verify client_secret
        if let Some(ref secret) = client.client_secret {
            if let Some(provided) = params.get("client_secret") {
                if provided != secret {
                    return self.oauth_error("invalid_client", "invalid client_secret", 401);
                }
            } else {
                return self.oauth_error("invalid_client", "client_secret required", 401);
            }
        }

        // Find the original token to get scopes
        let original_scopes = {
            let tokens = read_lock(&self.tokens);
            tokens
                .values()
                .find(|t| t.client_id == *client_id)
                .map(|t| t.scopes.clone())
                .unwrap_or_else(|| vec!["openid".into()])
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let access_token = generate_token(48);

        write_lock(&self.tokens).insert(
            access_token.clone(),
            TokenEntry {
                client_id: client_id.clone(),
                scopes: original_scopes.clone(),
                active: true,
                token_type: "Bearer".into(),
                exp: now + self.config.access_token_ttl_secs,
                iat: now,
                sub: Some(client_id.clone()),
                raw_token: access_token.clone(),
            },
        );

        self.token_response(
            &access_token,
            refresh_token,
            None,
            self.config.access_token_ttl_secs,
        )
    }

    // -----------------------------------------------------------------------
    // Handler: Userinfo (GET /oauth2/userinfo) — OIDC Core 1.0 §5.3
    // -----------------------------------------------------------------------

    /// Handle a userinfo request. Expects `Authorization: Bearer <token>`.
    pub fn handle_userinfo(&self, auth_header: Option<&str>) -> Response {
        let token = match auth_header {
            Some(h) => h
                .strip_prefix("Bearer ")
                .or_else(|| h.strip_prefix("bearer ")),
            None => None,
        };

        let token = match token {
            Some(t) => t,
            None => {
                return Response::new(StatusCode::new(401).unwrap())
                    .with_header("WWW-Authenticate", r#"Bearer error="invalid_token", error_description="missing authorization header""#);
            }
        };

        let entry = match read_lock(&self.tokens).get(token) {
            Some(e) => e.clone(),
            None => {
                return Response::new(StatusCode::new(401).unwrap()).with_header(
                    "WWW-Authenticate",
                    r#"Bearer error="invalid_token", error_description="token not found""#,
                );
            }
        };

        if !entry.active {
            return Response::new(StatusCode::new(401).unwrap()).with_header(
                "WWW-Authenticate",
                r#"Bearer error="invalid_token", error_description="token revoked""#,
            );
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        if entry.exp < now {
            return Response::new(StatusCode::new(401).unwrap()).with_header(
                "WWW-Authenticate",
                r#"Bearer error="expired_token", error_description="token expired""#,
            );
        }

        let mut fields: Vec<(String, JsonValue)> = Vec::new();
        fields.push((
            "sub".into(),
            JsonValue::String(entry.sub.clone().unwrap_or(entry.client_id.clone())),
        ));
        fields.push((
            "client_id".into(),
            JsonValue::String(entry.client_id.clone()),
        ));
        if !entry.scopes.is_empty() {
            fields.push(("scope".into(), JsonValue::String(entry.scopes.join(" "))));
        }

        // If openid scope was requested, include OIDC claims
        if entry.scopes.iter().any(|s| s == "openid") {
            fields.push(("iss".into(), JsonValue::String(self.config.issuer.clone())));
            fields.push(("aud".into(), JsonValue::String(entry.client_id.clone())));
            fields.push(("iat".into(), JsonValue::from(entry.iat)));
        }

        let val = JsonValue::Object(Map::from_iter(fields));
        let json = to_string(&val).unwrap_or_else(|_| "{}".into());
        Response::json(StatusCode::new(200).unwrap(), &json)
    }

    // -----------------------------------------------------------------------
    // Handler: Token Introspection (RFC 7662)
    // -----------------------------------------------------------------------

    /// Handle a token introspection request.
    pub fn handle_introspect(&self, form_body: &str) -> Response {
        let params = parse_form_urlencoded(form_body);
        let token = match params.get("token") {
            Some(t) => t,
            None => return self.oauth_error("invalid_request", "missing token", 400),
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let entry = read_lock(&self.tokens).get(token).cloned();

        let mut fields: Vec<(String, JsonValue)> = Vec::new();
        if let Some(entry) = entry {
            if entry.exp < now {
                fields.push(("active".into(), JsonValue::Bool(false)));
            } else {
                fields.push(("active".into(), JsonValue::Bool(true)));
                fields.push(("client_id".into(), JsonValue::String(entry.client_id)));
                fields.push(("token_type".into(), JsonValue::String(entry.token_type)));
                fields.push(("exp".into(), JsonValue::from(entry.exp)));
                fields.push(("iat".into(), JsonValue::from(entry.iat)));
                if let Some(ref sub) = entry.sub {
                    fields.push(("sub".into(), JsonValue::String(sub.clone())));
                }
                if !entry.scopes.is_empty() {
                    fields.push(("scope".into(), JsonValue::String(entry.scopes.join(" "))));
                }
            }
        } else {
            fields.push(("active".into(), JsonValue::Bool(false)));
        }

        let val = JsonValue::Object(Map::from_iter(fields));
        let json = to_string(&val).unwrap_or_else(|_| "{}".into());
        Response::json(StatusCode::new(200).unwrap(), &json)
    }

    // -----------------------------------------------------------------------
    // Token verification for BearerAuthMiddleware
    // -----------------------------------------------------------------------

    /// Verify a bearer token and return claims. Returns `None` if invalid/expired.
    pub fn verify_bearer_token(&self, token: &str) -> Option<Claims> {
        let entry = read_lock(&self.tokens).get(token).cloned()?;

        if !entry.active {
            return None;
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        if entry.exp < now {
            return None;
        }

        Some(Claims {
            sub: entry.sub.unwrap_or(entry.client_id.clone()),
            client_id: entry.client_id,
            scopes: entry.scopes,
            token_type: entry.token_type,
            exp: entry.exp,
            iat: entry.iat,
        })
    }

    // -----------------------------------------------------------------------
    // ID Token signing with at_hash
    // -----------------------------------------------------------------------

    /// Sign an OIDC ID token (JWT) using ES256, including `at_hash` claim.
    ///
    /// `at_hash` (access token hash) is REQUIRED for code flow per OIDC Core 1.0 §3.1.3.6.
    /// It is the left half of the SHA-256 hash of the ASCII representation of the access_token.
    fn sign_id_token_with_at_hash(
        &self,
        client_id: &str,
        scopes: &[String],
        access_token: &str,
    ) -> OAuthResult<String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // Compute at_hash: left half of SHA-256(access_token), base64url encoded
        let at_hash = {
            let hash = sha256(access_token.as_bytes());
            base64url_nopad_encode(&hash[..16]) // left half (16 bytes)
        };

        let header_json = format!(
            r#"{{"alg":"ES256","typ":"JWT","kid":"{}"}}"#,
            self.signing_key_id
        );
        let header_b64 = base64url_nopad_encode(header_json.as_bytes());

        let mut payload_fields: Vec<(String, JsonValue)> = Vec::new();
        payload_fields.push(("iss".into(), JsonValue::String(self.config.issuer.clone())));
        payload_fields.push(("sub".into(), JsonValue::String(client_id.to_string())));
        payload_fields.push(("aud".into(), JsonValue::String(client_id.to_string())));
        payload_fields.push((
            "exp".into(),
            JsonValue::from(now + self.config.access_token_ttl_secs),
        ));
        payload_fields.push(("iat".into(), JsonValue::from(now)));
        payload_fields.push(("scope".into(), JsonValue::String(scopes.join(" "))));
        payload_fields.push(("at_hash".into(), JsonValue::String(at_hash)));

        let payload_json = to_string(&JsonValue::Object(Map::from_iter(payload_fields)))
            .map_err(|e| OAuthError::JsonError(e.to_string()))?;
        let payload_b64 = base64url_nopad_encode(payload_json.as_bytes());

        let signing_input = format!("{header_b64}.{payload_b64}");
        let signature: Signature<edgerun_crypto::p256::NistP256> =
            self.signing_key.sign(signing_input.as_bytes());
        let sig_der = signature.to_der().as_bytes().to_vec();
        let sig_b64 = base64url_nopad_encode(&sig_der);

        Ok(format!("{header_b64}.{payload_b64}.{sig_b64}"))
    }

    // -----------------------------------------------------------------------
    // Scope validation
    // -----------------------------------------------------------------------

    /// Validate that requested scopes are a subset of the client's registered scopes.
    /// Returns the intersection (granted scopes).
    fn validate_scopes(&self, requested: &[String], registered: &[String]) -> Vec<String> {
        let registered_set: std::collections::HashSet<&str> =
            registered.iter().map(|s| s.as_str()).collect();
        requested
            .iter()
            .filter(|s| registered_set.contains(s.as_str()))
            .cloned()
            .collect()
    }

    // -----------------------------------------------------------------------
    // Response helpers
    // -----------------------------------------------------------------------

    fn token_response(
        &self,
        access_token: &str,
        refresh_token: &str,
        id_token: Option<&str>,
        expires_in: u64,
    ) -> Response {
        let mut fields: Vec<(String, JsonValue)> = Vec::new();
        fields.push((
            "access_token".into(),
            JsonValue::String(access_token.to_string()),
        ));
        fields.push(("token_type".into(), JsonValue::String("Bearer".into())));
        fields.push(("expires_in".into(), JsonValue::from(expires_in)));
        fields.push((
            "refresh_token".into(),
            JsonValue::String(refresh_token.to_string()),
        ));
        if let Some(idt) = id_token {
            fields.push(("id_token".into(), JsonValue::String(idt.to_string())));
        }

        let val = JsonValue::Object(Map::from_iter(fields));
        let json = to_string(&val).unwrap_or_else(|_| "{}".into());
        Response::json(StatusCode::new(200).unwrap(), &json)
    }

    fn oauth_error(&self, error: &str, desc: &str, status: u16) -> Response {
        let mut fields: Vec<(String, JsonValue)> = Vec::new();
        fields.push(("error".into(), JsonValue::String(error.to_string())));
        fields.push((
            "error_description".into(),
            JsonValue::String(desc.to_string()),
        ));
        let val = JsonValue::Object(Map::from_iter(fields));
        let json = to_string(&val).unwrap_or_else(|_| "{}".into());
        Response::json(StatusCode::new(status).unwrap(), &json)
    }

    // -----------------------------------------------------------------------
    // Sweep expired device grants
    // -----------------------------------------------------------------------

    pub fn sweep_expired_grants(&self) -> usize {
        self.device_store.sweep_expired()
    }
}

// ===========================================================================
// Handler implementation
// ===========================================================================

impl Handler for OAuthServer {
    fn handle(&self, request: Request) -> Pin<Box<dyn Future<Output = Response> + Send + '_>> {
        let method = request.method().clone();
        let path = request.uri().path().to_string();
        let body = request.body().map(|b| b.to_vec()).unwrap_or_default();
        let auth_header = request
            .headers()
            .get("Authorization")
            .map(|h| h.as_str().to_string());

        Box::pin(async move {
            let form_body = String::from_utf8_lossy(&body).to_string();

            if path == self.config.discovery_path {
                return self.discovery_response();
            }
            if path == self.config.jwks_path {
                return self.jwks_response();
            }
            if path == self.config.device_code_path && method.as_str() == "POST" {
                return self.handle_device_code(&form_body);
            }
            if path == self.config.token_path && method.as_str() == "POST" {
                return self.handle_token(&form_body);
            }
            if path == self.config.userinfo_path && method.as_str() == "GET" {
                return self.handle_userinfo(auth_header.as_deref());
            }
            if path == self.config.introspect_path && method.as_str() == "POST" {
                return self.handle_introspect(&form_body);
            }

            Response::not_found()
        })
    }
}

// ===========================================================================
// BearerAuthMiddleware
// ===========================================================================

/// Middleware that validates a bearer token and enriches the request with claims.
///
/// Usage:
/// ```no_run
/// use edgerun_oauth::{OAuthServer, BearerAuthMiddleware, Claims};
/// use edgerun_http::{Handler, Request, Response, StatusCode};
/// use std::pin::Pin;
/// use std::future::Future;
/// use std::sync::Arc;
///
/// struct MyHandler;
/// impl Handler for MyHandler {
///     fn handle(&self, req: Request) -> Pin<Box<dyn Future<Output = Response> + Send + '_>> {
///         Box::pin(async move {
///             // Extract claims from request extensions
///             let claims: Option<Claims> = req.extensions().get::<Claims>();
///             match claims {
///                 Some(c) => Response::text(StatusCode::new(200).unwrap(), &format!("Hello, {}", c.sub)),
///                 None => Response::text(StatusCode::new(401).unwrap(), "unauthenticated"),
///             }
///         })
///     }
/// }
///
/// // edgerun_http::Chain::new(MyHandler)
/// //     .with(BearerAuthMiddleware::new(Arc::new(oauth_server)))
/// //     .build();
/// ```
pub struct BearerAuthMiddleware {
    server: Arc<OAuthServer>,
}

impl BearerAuthMiddleware {
    pub fn new(server: Arc<OAuthServer>) -> Self {
        Self { server }
    }
}

impl edgerun_http::Middleware for BearerAuthMiddleware {
    fn call(
        &self,
        req: Request,
        next: edgerun_http::Next,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + '_>> {
        Box::pin(async move {
            // Extract bearer token
            let auth_header = req.headers().get("Authorization").map(|h| h.as_str());
            let token = auth_header.and_then(|h| {
                h.strip_prefix("Bearer ")
                    .or_else(|| h.strip_prefix("bearer "))
            });

            match token {
                Some(t) => {
                    match self.server.verify_bearer_token(t) {
                        Some(claims) => {
                            // Inject claims into request extensions
                            let mut req = req;
                            req.extensions_mut().insert::<Claims>(claims);
                            next.run(req).await
                        }
                        None => Response::new(StatusCode::new(401).unwrap())
                            .with_header("WWW-Authenticate", r#"Bearer error="invalid_token""#),
                    }
                }
                None => Response::new(StatusCode::new(401).unwrap())
                    .with_header("WWW-Authenticate", r#"Bearer error="missing_token""#),
            }
        })
    }
}

// ===========================================================================
// Helpers
// ===========================================================================

fn generate_token(len: usize) -> String {
    let mut bytes = vec![0u8; len];
    fill_random(&mut bytes).expect("random generation failed");
    base64url_nopad_encode(&bytes)
}

/// Parse form-urlencoded body into a HashMap (wraps BTreeMap-based parser).
fn parse_form_urlencoded(body: &str) -> HashMap<String, String> {
    parse_form_btree(body).into_iter().collect()
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_server() -> OAuthServer {
        let config = ServerConfig::new("https://auth.example.com", "https://auth.example.com");
        OAuthServer::new(config).unwrap()
    }

    #[test]
    fn test_server_creation() {
        let server = make_server();
        assert!(server.get_client("nonexistent").is_none());
    }

    #[test]
    fn test_register_client() {
        let server = make_server();
        let client = server.register_client("my-app", vec!["openid".into(), "email".into()]);
        assert_eq!(client.client_name, "my-app");
        assert_eq!(client.scopes, vec!["openid", "email"]);
        assert!(client.client_secret.is_some());

        let fetched = server.get_client(&client.client_id).unwrap();
        assert_eq!(fetched.client_name, "my-app");
    }

    #[test]
    fn test_register_client_with_id() {
        let server = make_server();
        let client =
            server.register_client_with_id("fixed-id", "fixed-secret", vec!["openid".into()]);
        assert_eq!(client.client_id, "fixed-id");
        assert_eq!(client.client_secret, Some("fixed-secret".into()));
    }

    #[test]
    fn test_scope_validation_subset() {
        let server = make_server();
        let client = server
            .register_client("my-app", vec!["openid".into(), "profile".into()])
            .client_id;

        // Request scopes that are a subset of registered
        let granted = {
            let c = server.get_client(&client).unwrap();
            server.validate_scopes(&["openid".into()], &c.scopes)
        };
        assert_eq!(granted, vec!["openid"]);
    }

    #[test]
    fn test_scope_validation_extra_rejected() {
        let server = make_server();
        let client = server
            .register_client("my-app", vec!["openid".into()])
            .client_id;

        let c = server.get_client(&client).unwrap();
        let granted = server.validate_scopes(&["openid".into(), "admin".into()], &c.scopes);
        assert_eq!(granted, vec!["openid"]); // "admin" not granted
    }

    #[test]
    fn test_scope_validation_none_granted() {
        let server = make_server();
        let client = server
            .register_client("my-app", vec!["openid".into()])
            .client_id;

        let c = server.get_client(&client).unwrap();
        let granted = server.validate_scopes(&["admin".into()], &c.scopes);
        assert!(granted.is_empty());
    }

    #[test]
    fn test_device_code_handler_missing_client() {
        let server = make_server();
        let resp = server.handle_device_code("");
        assert_eq!(resp.status().as_u16(), 400);
        let body = resp.body_as_string().unwrap();
        let json: JsonValue = from_str(&body).unwrap();
        assert_eq!(
            json.get("error").and_then(|v| v.as_str()),
            Some("invalid_request")
        );
    }

    #[test]
    fn test_device_code_handler_success() {
        let server = make_server();
        let client = server.register_client("my-app", vec!["openid".into()]);

        let body = format!(
            "client_id={}&scope=openid&code_challenge=abc123&code_challenge_method=S256",
            client.client_id
        );
        let resp = server.handle_device_code(&body);
        assert_eq!(resp.status().as_u16(), 200);

        let json: JsonValue = from_str(&resp.body_as_string().unwrap()).unwrap();
        assert!(json.get("device_code").is_some());
        assert!(json.get("user_code").is_some());
        assert!(json.get("verification_uri").is_some());
        assert!(json.get("expires_in").is_some());
        assert!(json.get("interval").is_some());
    }

    #[test]
    fn test_device_code_unknown_client() {
        let server = make_server();
        let body =
            "client_id=nonexistent&scope=openid&code_challenge=abc&code_challenge_method=S256";
        let resp = server.handle_device_code(body);
        assert_eq!(resp.status().as_u16(), 401);
    }

    #[test]
    fn test_device_code_wrong_method() {
        let server = make_server();
        server.register_client("my-app", vec!["openid".into()]);
        let body = "client_id=fake&scope=openid&code_challenge=abc&code_challenge_method=plain";
        let resp = server.handle_device_code(body);
        assert_eq!(resp.status().as_u16(), 400);
    }

    #[test]
    fn test_discovery_response() {
        let server = make_server();
        let resp = server.discovery_response();
        assert_eq!(resp.status().as_u16(), 200);

        let json: JsonValue = from_str(&resp.body_as_string().unwrap()).unwrap();
        assert_eq!(
            json.get("issuer").and_then(|v| v.as_str()),
            Some("https://auth.example.com")
        );
        assert!(json.get("authorization_endpoint").is_some());
        assert!(json.get("token_endpoint").is_some());
        assert!(json.get("userinfo_endpoint").is_some());
        assert!(json.get("jwks_uri").is_some());
        assert!(json.get("device_authorization_endpoint").is_some());
    }

    #[test]
    fn test_jwks_response() {
        let server = make_server();
        let resp = server.jwks_response();
        assert_eq!(resp.status().as_u16(), 200);

        let json: JsonValue = from_str(&resp.body_as_string().unwrap()).unwrap();
        let keys = json.get("keys").and_then(|v| v.as_array()).unwrap();
        assert_eq!(keys.len(), 1);
        let key = &keys[0];
        assert_eq!(key.get("kty").and_then(|v| v.as_str()), Some("EC"));
        assert_eq!(key.get("crv").and_then(|v| v.as_str()), Some("P-256"));
        assert_eq!(key.get("alg").and_then(|v| v.as_str()), Some("ES256"));
    }

    #[test]
    fn test_introspect_active_token() {
        let server = make_server();
        let client = server.register_client("my-app", vec!["openid".into()]);

        // Manually create a token entry
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        write_lock(&server.tokens).insert(
            "test-token".into(),
            TokenEntry {
                client_id: client.client_id.clone(),
                scopes: vec!["openid".into()],
                active: true,
                token_type: "Bearer".into(),
                exp: now + 3600,
                iat: now,
                sub: Some("user-1".into()),
                raw_token: "test-token".into(),
            },
        );

        let body = "token=test-token";
        let resp = server.handle_introspect(body);
        assert_eq!(resp.status().as_u16(), 200);

        let json: JsonValue = from_str(&resp.body_as_string().unwrap()).unwrap();
        assert_eq!(json.get("active").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(
            json.get("client_id").and_then(|v| v.as_str()),
            Some(client.client_id.as_str())
        );
        assert_eq!(json.get("sub").and_then(|v| v.as_str()), Some("user-1"));
    }

    #[test]
    fn test_introspect_inactive_token() {
        let server = make_server();
        let body = "token=nonexistent";
        let resp = server.handle_introspect(body);
        assert_eq!(resp.status().as_u16(), 200);

        let json: JsonValue = from_str(&resp.body_as_string().unwrap()).unwrap();
        assert_eq!(json.get("active").and_then(|v| v.as_bool()), Some(false));
    }

    #[test]
    fn test_userinfo_no_auth() {
        let server = make_server();
        let resp = server.handle_userinfo(None);
        assert_eq!(resp.status().as_u16(), 401);
    }

    #[test]
    fn test_userinfo_invalid_token() {
        let server = make_server();
        let resp = server.handle_userinfo(Some("Bearer invalid-token"));
        assert_eq!(resp.status().as_u16(), 401);
    }

    #[test]
    fn test_userinfo_success() {
        let server = make_server();
        let client = server.register_client("my-app", vec!["openid".into()]);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        write_lock(&server.tokens).insert(
            "valid-token".into(),
            TokenEntry {
                client_id: client.client_id.clone(),
                scopes: vec!["openid".into(), "email".into()],
                active: true,
                token_type: "Bearer".into(),
                exp: now + 3600,
                iat: now,
                sub: Some("user-123".into()),
                raw_token: "valid-token".into(),
            },
        );

        let resp = server.handle_userinfo(Some("Bearer valid-token"));
        assert_eq!(resp.status().as_u16(), 200);

        let json: JsonValue = from_str(&resp.body_as_string().unwrap()).unwrap();
        assert_eq!(json.get("sub").and_then(|v| v.as_str()), Some("user-123"));
        assert_eq!(
            json.get("scope").and_then(|v| v.as_str()),
            Some("openid email")
        );
        assert_eq!(
            json.get("iss").and_then(|v| v.as_str()),
            Some("https://auth.example.com")
        );
    }

    #[test]
    fn test_bearer_token_verification() {
        let server = make_server();
        let client = server.register_client("my-app", vec!["openid".into()]);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        write_lock(&server.tokens).insert(
            "bearer-token".into(),
            TokenEntry {
                client_id: client.client_id,
                scopes: vec!["openid".into()],
                active: true,
                token_type: "Bearer".into(),
                exp: now + 3600,
                iat: now,
                sub: Some("user-1".into()),
                raw_token: "bearer-token".into(),
            },
        );

        let claims = server.verify_bearer_token("bearer-token").unwrap();
        assert_eq!(claims.sub, "user-1");
        assert_eq!(claims.scopes, vec!["openid"]);
    }

    #[test]
    fn test_bearer_token_expired() {
        let server = make_server();
        let client = server.register_client("my-app", vec!["openid".into()]);

        write_lock(&server.tokens).insert(
            "expired-token".into(),
            TokenEntry {
                client_id: client.client_id,
                scopes: vec!["openid".into()],
                active: true,
                token_type: "Bearer".into(),
                exp: 100, // long expired
                iat: 0,
                sub: Some("user-1".into()),
                raw_token: "expired-token".into(),
            },
        );

        assert!(server.verify_bearer_token("expired-token").is_none());
    }

    #[test]
    fn test_server_config_matches() {
        let config = ServerConfig::new("https://auth.example.com", "https://auth.example.com");
        assert!(config.matches("/.well-known/openid-configuration"));
        assert!(config.matches("/.well-known/jwks.json"));
        assert!(config.matches("/oauth2/device/code"));
        assert!(config.matches("/oauth2/token"));
        assert!(config.matches("/oauth2/userinfo"));
        assert!(config.matches("/oauth2/introspect"));
        assert!(!config.matches("/api/data"));
        assert!(!config.matches("/"));
    }

    #[test]
    fn test_form_urlencoded_parsing() {
        let params =
            parse_form_urlencoded("grant_type=refresh_token&client_id=my-id&refresh_token=xyz");
        assert_eq!(params.get("grant_type"), Some(&"refresh_token".to_string()));
        assert_eq!(params.get("client_id"), Some(&"my-id".to_string()));
        assert_eq!(params.get("refresh_token"), Some(&"xyz".to_string()));
    }

    #[test]
    fn test_form_urlencoded_with_spaces() {
        let params = parse_form_urlencoded("scope=openid+profile+email");
        assert_eq!(
            params.get("scope"),
            Some(&"openid profile email".to_string())
        );
    }
}
