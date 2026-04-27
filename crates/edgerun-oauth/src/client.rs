//! OAuth 2.0 client — device flow, authorization code flow, token refresh.

use crate::discovery::OidcDiscoveryDocument;
use crate::errors::{DeviceError, OAuthError, OAuthResult};
use crate::jwt::IdToken;
use crate::pkce::PkcePair;
use crate::token_store::TokenStore;
use crate::types::{
    ClientConfig, Credentials, DeviceAuthorizationRequest, DeviceAuthorizationResponse,
    TokenRequest, TokenResponse,
};
use edgerun_bare_rt::sleep;
use edgerun_crypto::sha256;
use edgerun_encoding::base64::base64url_nopad_encode;
use edgerun_http::HttpClient;
use std::time::{Duration, Instant};

/// Callback trait for the device flow UI.
pub trait DeviceFlowCallback: Send + Sync {
    /// Called with the verification URL and user code to display.
    fn display_verification(&self, uri: &str, user_code: &str);
}

/// OAuth 2.0 client for interacting with an authorization server.
pub struct OAuthClient {
    config: ClientConfig,
    http: HttpClient,
    token_store: Option<TokenStore>,
}

impl OAuthClient {
    /// Create a new OAuth client.
    pub fn new(config: ClientConfig) -> Self {
        Self {
            config,
            http: HttpClient::new(),
            token_store: None,
        }
    }

    /// Attach a token store for persistence.
    pub fn with_token_store(mut self, store: TokenStore) -> Self {
        self.token_store = Some(store);
        self
    }

    // -----------------------------------------------------------------------
    // Device Flow (RFC 8628)
    // -----------------------------------------------------------------------

    /// Execute the full OAuth 2.0 device flow.
    ///
    /// 1. Generate PKCE pair
    /// 2. Request device code from the server
    /// 3. Display verification URL to the user
    /// 4. Poll the token endpoint until authorized or expired
    pub async fn device_flow(&self, callback: &dyn DeviceFlowCallback) -> OAuthResult<Credentials> {
        // Step 1: Generate PKCE
        let pkce = PkcePair::generate().map_err(|e| OAuthError::PkceError(e.to_string()))?;

        // Step 2: Request device code
        let device_resp = self.request_device_code(&pkce).await?;

        // Step 3: Display verification URL
        callback.display_verification(
            &device_resp.verification_uri_complete,
            &device_resp.user_code,
        );

        // Step 4: Poll for token
        let creds = self
            .poll_for_token(
                &device_resp.device_code,
                &pkce.code_verifier,
                self.config.device_flow_timeout_secs,
                device_resp.interval,
            )
            .await?;

        // Save to token store if available
        if let Some(ref store) = self.token_store {
            let _ = store.save(&creds);
        }

        Ok(creds)
    }

    /// Request a device code from the authorization server.
    async fn request_device_code(
        &self,
        _pkce: &PkcePair,
    ) -> OAuthResult<DeviceAuthorizationResponse> {
        let req = DeviceAuthorizationRequest {
            client_id: self.config.client_id.clone(),
            scope: crate::types::Scope::format_list(&self.config.scopes),
            code_challenge: _pkce.code_challenge.clone(),
            code_challenge_method: "S256".into(),
        };
        let body = req.to_form_body();

        let resp = self
            .http
            .post(&self.config.device_code_url(), body.as_bytes())
            .await
            .map_err(|e| OAuthError::HttpError(e.to_string()))?;

        let body = String::from_utf8_lossy(resp.body()).to_string();

        if resp.status().as_u16() != 200 {
            let token_resp = TokenResponse::from_json(&body).unwrap_or(TokenResponse {
                access_token: None,
                token_type: None,
                expires_in: None,
                refresh_token: None,
                id_token: None,
                scope: None,
                error: Some("unknown".into()),
                error_description: Some(body.chars().take(200).collect()),
            });
            let error = token_resp.error.unwrap_or_else(|| "unknown".into());
            let desc = token_resp
                .error_description
                .unwrap_or_else(|| "Unknown error".into());
            return Err(OAuthError::ServerError {
                error,
                error_description: desc,
            });
        }

        DeviceAuthorizationResponse::from_json(&body)
            .map_err(|e| OAuthError::DeviceError(DeviceError::InvalidResponse(e)))
    }

    /// Poll the token endpoint until the user authorizes, denies, or times out.
    async fn poll_for_token(
        &self,
        device_code: &str,
        code_verifier: &str,
        timeout_secs: u64,
        initial_interval: u64,
    ) -> OAuthResult<Credentials> {
        let start = Instant::now();
        let mut interval = Duration::from_secs(initial_interval);
        let timeout = Duration::from_secs(timeout_secs);

        loop {
            if start.elapsed() > timeout {
                return Err(OAuthError::DeviceError(DeviceError::Timeout));
            }

            sleep(interval).await;

            let req = TokenRequest {
                grant_type: "urn:ietf:params:oauth:grant-type:device_code".into(),
                client_id: self.config.client_id.clone(),
                client_secret: self.config.client_secret.clone(),
                device_code: Some(device_code.to_string()),
                code: None,
                redirect_uri: None,
                code_verifier: Some(code_verifier.to_string()),
                refresh_token: None,
                scope: None,
            };
            let body = req.to_form_body();

            let resp = self
                .http
                .post(&self.config.token_url(), body.as_bytes())
                .await
                .map_err(|e| OAuthError::HttpError(e.to_string()))?;

            let resp_body = String::from_utf8_lossy(resp.body()).to_string();
            let token_resp = TokenResponse::from_json(&resp_body).map_err(OAuthError::JsonError)?;

            if let Some(error) = token_resp.error {
                match error.as_str() {
                    "authorization_pending" => continue,
                    "slow_down" => {
                        interval = interval.saturating_add(Duration::from_secs(2));
                        continue;
                    }
                    "expired_token" => return Err(OAuthError::DeviceError(DeviceError::Expired)),
                    "access_denied" => return Err(OAuthError::DeviceError(DeviceError::Denied)),
                    _ => {
                        let desc = token_resp
                            .error_description
                            .unwrap_or_else(|| "Unknown error".into());
                        return Err(OAuthError::ServerError {
                            error,
                            error_description: desc,
                        });
                    }
                }
            }

            return Ok(token_resp.into_credentials());
        }
    }

    // -----------------------------------------------------------------------
    // Authorization Code Flow with PKCE (RFC 6749 + RFC 7636)
    // -----------------------------------------------------------------------

    /// Generate the authorization URL for the user to visit.
    ///
    /// Returns the URL and the PKCE pair (needed for token exchange).
    pub fn authorization_code_url(
        &self,
        redirect_uri: &str,
        state: &str,
    ) -> OAuthResult<(String, PkcePair)> {
        let pkce = PkcePair::generate().map_err(|e| OAuthError::PkceError(e.to_string()))?;

        let mut params = Vec::new();
        let scope_str = crate::types::Scope::format_list(&self.config.scopes);
        params.push(("response_type", "code"));
        params.push(("client_id", self.config.client_id.as_str()));
        params.push(("redirect_uri", redirect_uri));
        params.push(("scope", &scope_str));
        params.push(("state", state));
        params.push(("code_challenge", pkce.code_challenge.as_str()));
        params.push(("code_challenge_method", "S256"));

        let query: String = params
            .iter()
            .map(|(k, v)| edgerun_encoding::percent::url_encode_pair(k, v))
            .collect::<Vec<_>>()
            .join("&");

        let url = format!("{}?{}", self.config.authorize_url(), query);
        Ok((url, pkce))
    }

    /// Exchange an authorization code for tokens (second step of auth code flow).
    pub async fn exchange_code(
        &self,
        code: &str,
        redirect_uri: &str,
        pkce: &PkcePair,
    ) -> OAuthResult<Credentials> {
        let req = TokenRequest {
            grant_type: "authorization_code".into(),
            client_id: self.config.client_id.clone(),
            client_secret: self.config.client_secret.clone(),
            device_code: None,
            code: Some(code.to_string()),
            redirect_uri: Some(redirect_uri.to_string()),
            code_verifier: Some(pkce.code_verifier.clone()),
            refresh_token: None,
            scope: None,
        };
        let body = req.to_form_body();

        let resp = self
            .http
            .post(&self.config.token_url(), body.as_bytes())
            .await
            .map_err(|e| OAuthError::HttpError(e.to_string()))?;

        let resp_body = String::from_utf8_lossy(resp.body()).to_string();
        if resp.status().as_u16() != 200 {
            return self.parse_server_error(&resp_body);
        }

        let token_resp = TokenResponse::from_json(&resp_body).map_err(OAuthError::JsonError)?;
        if let Some(error) = token_resp.error {
            let desc = token_resp
                .error_description
                .unwrap_or_else(|| "Unknown error".into());
            return Err(OAuthError::ServerError {
                error,
                error_description: desc,
            });
        }

        let creds = token_resp.into_credentials();

        if let Some(ref store) = self.token_store {
            let _ = store.save(&creds);
        }

        Ok(creds)
    }

    // -----------------------------------------------------------------------
    // Token Refresh
    // -----------------------------------------------------------------------

    /// Refresh an access token using a refresh token.
    pub async fn refresh_token(&self, refresh_token: &str) -> OAuthResult<Credentials> {
        let req = TokenRequest {
            grant_type: "refresh_token".into(),
            client_id: self.config.client_id.clone(),
            client_secret: self.config.client_secret.clone(),
            device_code: None,
            code: None,
            redirect_uri: None,
            code_verifier: None,
            refresh_token: Some(refresh_token.to_string()),
            scope: Some(crate::types::Scope::format_list(&self.config.scopes)),
        };
        let body = req.to_form_body();

        let resp = self
            .http
            .post(&self.config.token_url(), body.as_bytes())
            .await
            .map_err(|e| OAuthError::HttpError(e.to_string()))?;

        let resp_body = String::from_utf8_lossy(resp.body()).to_string();
        if resp.status().as_u16() != 200 {
            return self.parse_server_error(&resp_body);
        }

        let token_resp = TokenResponse::from_json(&resp_body).map_err(OAuthError::JsonError)?;
        if let Some(error) = token_resp.error {
            let desc = token_resp
                .error_description
                .unwrap_or_else(|| "Unknown error".into());
            return Err(OAuthError::ServerError {
                error,
                error_description: desc,
            });
        }

        let creds = token_resp.into_credentials();

        if let Some(ref store) = self.token_store {
            let _ = store.save(&creds);
        }

        Ok(creds)
    }

    /// Get valid credentials from the token store, refreshing if needed.
    /// Returns `None` if no valid credentials exist and no refresh token is available.
    pub async fn get_valid_credentials(&self, grace_secs: u64) -> OAuthResult<Option<Credentials>> {
        let Some(ref store) = self.token_store else {
            return Ok(None);
        };

        // Try cached credentials first
        if let Some(creds) = store.get_valid(grace_secs) {
            return Ok(Some(creds));
        }

        // Load from file (may be expired)
        if let Some(creds) = store
            .load()
            .map_err(|e| OAuthError::IoError(e.to_string()))?
        {
            if let Some(ref refresh_token) = creds.refresh_token {
                return self.refresh_token(refresh_token).await.map(Some);
            }
        }

        Ok(None)
    }

    // -----------------------------------------------------------------------
    // OIDC Discovery
    // -----------------------------------------------------------------------

    /// Fetch the OIDC discovery document.
    pub async fn discover(&self) -> OAuthResult<OidcDiscoveryDocument> {
        OidcDiscoveryDocument::discover(&self.config.base_url).await
    }

    // -----------------------------------------------------------------------
    // ID Token Helpers
    // -----------------------------------------------------------------------

    /// Parse and verify an ID token (JWT) with a given verifier.
    /// Also validates standard OIDC claims.
    pub fn verify_id_token(
        &self,
        raw_id_token: &str,
        verifier: &crate::jwt::JwtVerifier,
        expected_nonce: Option<&str>,
    ) -> OAuthResult<IdToken> {
        let token = IdToken::parse_unverified(raw_id_token)?;

        // Verify signature
        token.verify(verifier)?;

        // Validate OIDC claims
        if !token.payload.verify_aud(&self.config.client_id) {
            return Err(OAuthError::JwtError(
                "aud claim does not match client_id".into(),
            ));
        }

        if token.payload.is_expired(30) {
            return Err(OAuthError::JwtError("ID token is expired".into()));
        }

        if !token.payload.verify_nonce(expected_nonce) {
            return Err(OAuthError::JwtError("nonce mismatch".into()));
        }

        Ok(token)
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    fn parse_server_error(&self, body: &str) -> OAuthResult<Credentials> {
        let token_resp = TokenResponse::from_json(body).unwrap_or(TokenResponse {
            access_token: None,
            token_type: None,
            expires_in: None,
            refresh_token: None,
            id_token: None,
            scope: None,
            error: Some("unknown".into()),
            error_description: Some(body.chars().take(200).collect()),
        });
        let error = token_resp.error.unwrap_or_else(|| "unknown".into());
        let desc = token_resp
            .error_description
            .unwrap_or_else(|| "Unknown error".into());
        Err(OAuthError::ServerError {
            error,
            error_description: desc,
        })
    }
}
