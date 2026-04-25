//! OAuth-aware HTTP client using edgerun-http's client middleware system.
//!
//! Wraps [`edgerun_http::HttpClient`] with two middleware layers:
//! 1. **BearerTokenMiddleware** — injects `Authorization: Bearer <token>` from the token store
//! 2. **AutoRefreshMiddleware** — on 401, refreshes the token and retries (once)
//!
//! # Example
//! ```no_run
//! use edgerun_oauth::{OAuthClientBuilder, ClientConfig, TokenStore};
//!
//! # edgerun_rt::block_on(async {
//! let config = ClientConfig::device_flow("https://provider.example.com", "my-client-id");
//! let store = TokenStore::new().unwrap();
//!
//! let client = OAuthClientBuilder::new(config)
//!     .with_token_store(store)
//!     .build()
//!     .unwrap();
//!
//! // Requests automatically include the bearer token.
//! // On 401, the client refreshes and retries once.
//! let resp = client.get("https://api.example.com/data").await?;
//! # });
//! ```

use crate::client::DeviceFlowCallback;
use crate::errors::OAuthError;
use crate::pkce::PkcePair;
use crate::token_store::TokenStore;
use crate::types::{ClientConfig, Credentials};
use edgerun_crypto::sha256;
use edgerun_encoding::base64::base64url_nopad_encode;
use edgerun_http::client_middleware::{
    Chain, Client, ClientExtensions, ClientMiddleware, ClientNext, ClientRequest,
};
use edgerun_http::{HttpClient, Request, Response, Result};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

// ===========================================================================
// BearerTokenMiddleware
// ===========================================================================

/// Injects `Authorization: Bearer <token>` from the token store into every request.
///
/// If no valid token is available, the request proceeds without an auth header.
/// Downstream middleware or the server will return 401, which triggers
/// `AutoRefreshMiddleware`.
pub struct BearerTokenMiddleware {
    store: Arc<TokenStore>,
}

impl BearerTokenMiddleware {
    pub fn new(store: Arc<TokenStore>) -> Self {
        Self { store }
    }
}

impl ClientMiddleware for BearerTokenMiddleware {
    fn call(
        &self,
        mut req: ClientRequest,
        next: ClientNext,
    ) -> Pin<Box<dyn Future<Output = Result<Response>> + Send + '_>> {
        // Try to get a valid token (with 30s grace)
        if let Some(creds) = self.store.get_valid(30) {
            if let Some(token) = creds.bearer_token() {
                req.headers_mut()
                    .insert("Authorization", &format!("Bearer {token}"));
            }
        }

        Box::pin(async move { next.run(req).await })
    }
}

// ===========================================================================
// AutoRefreshMiddleware
// ===========================================================================

/// On 401 response, refreshes the access token and retries the request once.
///
/// Uses the refresh token from the token store to get a new access token.
/// If refresh fails or no refresh token exists, returns the original 401.
pub struct AutoRefreshMiddleware {
    config: ClientConfig,
    store: Arc<TokenStore>,
    http: HttpClient,
}

impl AutoRefreshMiddleware {
    pub fn new(config: ClientConfig, store: Arc<TokenStore>) -> Self {
        Self {
            config,
            store,
            http: HttpClient::new(),
        }
    }

    async fn refresh_and_retry(&self, original_req: ClientRequest) -> Result<Response> {
        // Load credentials to get the refresh token
        let Some(creds) = self.store.load().map_err(|e| {
            edgerun_http::Error::ProtocolError(format!("failed to load tokens: {e}"))
        })?
        else {
            // No tokens — return original request (will get 401)
            return self.execute_without_auth(original_req).await;
        };

        let Some(refresh_token) = creds.refresh_token else {
            // No refresh token — can't refresh
            return self.execute_without_auth(original_req).await;
        };

        // Perform refresh
        match self.do_refresh(&refresh_token).await {
            Ok(new_creds) => {
                // Save new credentials
                let _ = self.store.save(&new_creds);

                // Clone original request and retry with new token
                let mut retry_req = original_req.clone();
                if let Some(token) = new_creds.bearer_token() {
                    retry_req
                        .headers_mut()
                        .insert("Authorization", &format!("Bearer {token}"));
                }
                self.execute_without_auth(retry_req).await
            }
            Err(_) => {
                // Refresh failed — return original response
                self.execute_without_auth(original_req).await
            }
        }
    }

    async fn do_refresh(
        &self,
        refresh_token: &str,
    ) -> std::result::Result<Credentials, OAuthError> {
        let body = format!(
            "grant_type=refresh_token&client_id={}&refresh_token={}",
            percent_encode(&self.config.client_id),
            percent_encode(refresh_token),
        );

        let resp = self
            .http
            .post(&self.config.token_url(), body.as_bytes())
            .await
            .map_err(|e| OAuthError::HttpError(e.to_string()))?;

        let resp_body = String::from_utf8_lossy(resp.body()).to_string();

        if resp.status().as_u16() != 200 {
            return Err(OAuthError::HttpError(format!(
                "refresh failed: HTTP {}",
                resp.status().as_u16()
            )));
        }

        let token_resp =
            crate::types::TokenResponse::from_json(&resp_body).map_err(OAuthError::JsonError)?;

        if let Some(error) = token_resp.error {
            let desc = token_resp
                .error_description
                .unwrap_or_else(|| "Unknown error".into());
            return Err(OAuthError::ServerError {
                error,
                error_description: desc,
            });
        }

        Ok(token_resp.into_credentials())
    }

    async fn execute_without_auth(&self, req: ClientRequest) -> Result<Response> {
        // Create a bare HttpClient without middleware for the retry
        let client = HttpClient::new();
        client.execute(req.request()).await
    }
}

impl ClientMiddleware for AutoRefreshMiddleware {
    fn call(
        &self,
        req: ClientRequest,
        next: ClientNext,
    ) -> Pin<Box<dyn Future<Output = Result<Response>> + Send + '_>> {
        let refresh_clone = Arc::new(Self {
            config: self.config.clone(),
            store: Arc::clone(&self.store),
            http: HttpClient::new(),
        });

        Box::pin(async move {
            let resp = next.run(req.clone()).await?;

            // Check for 401
            if resp.status().as_u16() == 401 {
                refresh_clone.refresh_and_retry(req).await
            } else {
                Ok(resp)
            }
        })
    }
}

// ===========================================================================
// OAuthClientBuilder
// ===========================================================================

/// Builder for an OAuth-aware [`Client`] with middleware.
///
/// Creates a client that automatically injects bearer tokens and
/// refreshes on 401.
pub struct OAuthClientBuilder {
    config: ClientConfig,
    token_store: Option<TokenStore>,
}

impl OAuthClientBuilder {
    /// Create a new builder with the given OAuth client config.
    pub fn new(config: ClientConfig) -> Self {
        Self {
            config,
            token_store: None,
        }
    }

    /// Attach a token store for persistence.
    pub fn with_token_store(mut self, store: TokenStore) -> Self {
        self.token_store = Some(store);
        self
    }

    /// Build the OAuth-aware [`Client`].
    pub fn build(self) -> std::io::Result<Client> {
        let store = match self.token_store {
            Some(s) => Arc::new(s),
            None => Arc::new(TokenStore::new()?),
        };

        let client = Chain::new(HttpClient::new())
            .with(BearerTokenMiddleware::new(Arc::clone(&store)))
            .with(AutoRefreshMiddleware::new(self.config, store))
            .build();

        Ok(client)
    }
}

// ===========================================================================
// OAuthClient — high-level API combining client + auth flows
// ===========================================================================

/// High-level OAuth client combining authentication flows with HTTP requests.
///
/// Provides:
/// - Device flow authentication
/// - Automatic token injection via middleware
/// - Auto-refresh on 401
/// - All HTTP methods from `edgerun_http::Client`
pub struct OAuthClient {
    config: ClientConfig,
    client: Client,
    store: Arc<TokenStore>,
}

impl OAuthClient {
    /// Create a builder for a new OAuth client.
    pub fn builder(config: ClientConfig) -> OAuthClientBuilder {
        OAuthClientBuilder::new(config)
    }

    /// Create a new OAuth client. Requires a token store.
    /// For more control, use [`OAuthClient::builder`].
    pub fn new(config: ClientConfig, store: TokenStore) -> Self {
        let store = Arc::new(store);
        let client = Chain::new(HttpClient::new())
            .with(BearerTokenMiddleware::new(Arc::clone(&store)))
            .with(AutoRefreshMiddleware::new(
                config.clone(),
                Arc::clone(&store),
            ))
            .build();

        Self {
            config,
            client,
            store,
        }
    }

    /// Execute the device flow and save credentials to the token store.
    pub async fn device_flow(
        &self,
        callback: &dyn DeviceFlowCallback,
    ) -> std::result::Result<Credentials, OAuthError> {
        let pkce = PkcePair::generate().map_err(|e| OAuthError::PkceError(e.to_string()))?;

        // Request device code
        let req = crate::types::DeviceAuthorizationRequest {
            client_id: self.config.client_id.clone(),
            scope: crate::types::Scope::format_list(&self.config.scopes),
            code_challenge: pkce.code_challenge.clone(),
            code_challenge_method: "S256".into(),
        };
        let body = req.to_form_body();

        let http = HttpClient::new();
        let resp = http
            .post(&self.config.device_code_url(), body.as_bytes())
            .await
            .map_err(|e| OAuthError::HttpError(e.to_string()))?;

        let body = String::from_utf8_lossy(resp.body()).to_string();
        if resp.status().as_u16() != 200 {
            return Err(OAuthError::HttpError(format!(
                "device code request failed: HTTP {}",
                resp.status().as_u16()
            )));
        }

        let device_resp = crate::types::DeviceAuthorizationResponse::from_json(&body)
            .map_err(|e| OAuthError::DeviceError(crate::errors::DeviceError::InvalidResponse(e)))?;

        callback.display_verification(
            &device_resp.verification_uri_complete,
            &device_resp.user_code,
        );

        // Poll for token
        let creds = self
            .poll_for_token(
                &device_resp.device_code,
                &pkce.code_verifier,
                self.config.device_flow_timeout_secs,
                device_resp.interval,
            )
            .await?;

        self.store
            .save(&creds)
            .map_err(|e| OAuthError::IoError(e.to_string()))?;
        Ok(creds)
    }

    /// Refresh the access token using the stored refresh token.
    pub async fn refresh(&self) -> std::result::Result<Option<Credentials>, OAuthError> {
        let Some(creds) = self
            .store
            .load()
            .map_err(|e| OAuthError::IoError(e.to_string()))?
        else {
            return Ok(None);
        };
        let Some(ref refresh_token) = creds.refresh_token else {
            return Ok(None);
        };

        let new_creds = self.do_refresh(refresh_token).await?;
        self.store
            .save(&new_creds)
            .map_err(|e| OAuthError::IoError(e.to_string()))?;
        Ok(Some(new_creds))
    }

    async fn do_refresh(
        &self,
        refresh_token: &str,
    ) -> std::result::Result<Credentials, OAuthError> {
        let body = format!(
            "grant_type=refresh_token&client_id={}&refresh_token={}",
            percent_encode(&self.config.client_id),
            percent_encode(refresh_token),
        );

        let http = HttpClient::new();
        let resp = http
            .post(&self.config.token_url(), body.as_bytes())
            .await
            .map_err(|e| OAuthError::HttpError(e.to_string()))?;

        let resp_body = String::from_utf8_lossy(resp.body()).to_string();
        if resp.status().as_u16() != 200 {
            return Err(OAuthError::HttpError(format!(
                "refresh failed: HTTP {}",
                resp.status().as_u16()
            )));
        }

        let token_resp =
            crate::types::TokenResponse::from_json(&resp_body).map_err(OAuthError::JsonError)?;

        if let Some(error) = token_resp.error {
            let desc = token_resp
                .error_description
                .unwrap_or_else(|| "Unknown error".into());
            return Err(OAuthError::ServerError {
                error,
                error_description: desc,
            });
        }

        Ok(token_resp.into_credentials())
    }

    async fn poll_for_token(
        &self,
        device_code: &str,
        code_verifier: &str,
        timeout_secs: u64,
        initial_interval: u64,
    ) -> std::result::Result<Credentials, OAuthError> {
        use edgerun_rt::sleep;
        use std::time::{Duration, Instant};

        let start = Instant::now();
        let mut interval = Duration::from_secs(initial_interval);
        let timeout = Duration::from_secs(timeout_secs);

        loop {
            if start.elapsed() > timeout {
                return Err(OAuthError::DeviceError(crate::errors::DeviceError::Timeout));
            }

            sleep(interval).await;

            let req = crate::types::TokenRequest {
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

            let http = HttpClient::new();
            let resp = http
                .post(&self.config.token_url(), body.as_bytes())
                .await
                .map_err(|e| OAuthError::HttpError(e.to_string()))?;

            let resp_body = String::from_utf8_lossy(resp.body()).to_string();
            let token_resp = crate::types::TokenResponse::from_json(&resp_body)
                .map_err(OAuthError::JsonError)?;

            if let Some(error) = token_resp.error {
                match error.as_str() {
                    "authorization_pending" => continue,
                    "slow_down" => {
                        interval = interval.saturating_add(Duration::from_secs(2));
                        continue;
                    }
                    "expired_token" => {
                        return Err(OAuthError::DeviceError(crate::errors::DeviceError::Expired))
                    }
                    "access_denied" => {
                        return Err(OAuthError::DeviceError(crate::errors::DeviceError::Denied))
                    }
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
    // HTTP methods (delegated to the middleware-wrapped Client)
    // -----------------------------------------------------------------------

    /// GET request with automatic bearer token injection.
    pub async fn get(&self, uri: &str) -> Result<Response> {
        self.client.get(uri).await
    }

    /// POST request with automatic bearer token injection.
    pub async fn post(&self, uri: &str, body: &[u8]) -> Result<Response> {
        self.client.post(uri, body).await
    }

    /// POST request with JSON body and automatic bearer token injection.
    pub async fn post_json(&self, uri: &str, json: &str) -> Result<Response> {
        self.client.post_json(uri, json).await
    }

    /// PUT request with automatic bearer token injection.
    pub async fn put(&self, uri: &str, body: &[u8]) -> Result<Response> {
        self.client.put(uri, body).await
    }

    /// DELETE request with automatic bearer token injection.
    pub async fn delete(&self, uri: &str) -> Result<Response> {
        self.client.delete(uri).await
    }

    /// PATCH request with automatic bearer token injection.
    pub async fn patch(&self, uri: &str, body: &[u8]) -> Result<Response> {
        self.client.patch(uri, body).await
    }

    /// Execute a [`ClientRequest`] through the full middleware stack.
    pub async fn execute(&self, req: ClientRequest) -> Result<Response> {
        self.client.execute(req).await
    }

    /// Get the underlying middleware [`Client`] for advanced usage.
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Get the token store.
    pub fn token_store(&self) -> &TokenStore {
        &self.store
    }
}

// ===========================================================================
// Helpers
// ===========================================================================

pub(crate) fn percent_encode(s: &str) -> String {
    edgerun_encoding::percent::percent_encode(s)
}
