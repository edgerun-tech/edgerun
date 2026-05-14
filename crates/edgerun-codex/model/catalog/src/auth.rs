use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use edgerun_async_trait::async_trait;
use codex_app_server_protocol::AuthMode;
use codex_protocol::account::PlanType;
use edgerun_tokio::sync::RwLock;

#[derive(Clone, Debug)]
pub enum CodexAuth {
    ApiKey(String),
    Chatgpt {
        token: String,
        account_id: Option<String>,
        email: Option<String>,
        plan_type: Option<PlanType>,
        fedramp: bool,
    },
    ChatgptAuthTokens {
        token: String,
        account_id: Option<String>,
        email: Option<String>,
        plan_type: Option<PlanType>,
        fedramp: bool,
    },
}

impl CodexAuth {
    pub fn from_api_key(token: impl Into<String>) -> Self {
        Self::ApiKey(token.into())
    }

    pub fn create_dummy_chatgpt_auth_for_testing() -> Self {
        Self::Chatgpt {
            token: "dummy-chatgpt-token".to_string(),
            account_id: Some("dummy-account-id".to_string()),
            email: Some("user@example.com".to_string()),
            plan_type: Some(PlanType::Pro),
            fedramp: false,
        }
    }

    pub fn auth_mode(&self) -> AuthMode {
        match self {
            Self::ApiKey(_) => AuthMode::ApiKey,
            Self::Chatgpt { .. } => AuthMode::Chatgpt,
            Self::ChatgptAuthTokens { .. } => AuthMode::ChatgptAuthTokens,
        }
    }

    pub fn uses_codex_backend(&self) -> bool {
        !matches!(self, Self::ApiKey(_))
    }

    pub fn get_token(&self) -> Result<String, String> {
        match self {
            Self::ApiKey(token)
            | Self::Chatgpt { token, .. }
            | Self::ChatgptAuthTokens { token, .. } => Ok(token.clone()),
        }
    }

    pub fn get_account_id(&self) -> Option<String> {
        match self {
            Self::ApiKey(_) => None,
            Self::Chatgpt { account_id, .. } | Self::ChatgptAuthTokens { account_id, .. } => {
                account_id.clone()
            }
        }
    }

    pub fn get_account_email(&self) -> Option<String> {
        match self {
            Self::ApiKey(_) => None,
            Self::Chatgpt { email, .. } | Self::ChatgptAuthTokens { email, .. } => email.clone(),
        }
    }

    pub fn account_plan_type(&self) -> Option<PlanType> {
        match self {
            Self::ApiKey(_) => None,
            Self::Chatgpt { plan_type, .. } | Self::ChatgptAuthTokens { plan_type, .. } => {
                plan_type.clone()
            }
        }
    }

    pub fn is_fedramp_account(&self) -> bool {
        match self {
            Self::ApiKey(_) => false,
            Self::Chatgpt { fedramp, .. } | Self::ChatgptAuthTokens { fedramp, .. } => *fedramp,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AuthManager {
    auth: Arc<RwLock<Option<CodexAuth>>>,
    codex_api_key_env_enabled: bool,
    external_auth: Arc<AtomicBool>,
}

impl AuthManager {
    pub fn new(auth: Option<CodexAuth>) -> Self {
        Self {
            auth: Arc::new(RwLock::new(auth)),
            codex_api_key_env_enabled: false,
            external_auth: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn external_bearer_only(
        _config: codex_protocol::config_types::ModelProviderAuthInfo,
    ) -> Self {
        Self {
            auth: Arc::new(RwLock::new(None)),
            codex_api_key_env_enabled: false,
            external_auth: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn from_auth_for_testing(auth: CodexAuth) -> Arc<Self> {
        Arc::new(Self::new(Some(auth)))
    }

    pub async fn auth(&self) -> Option<CodexAuth> {
        self.auth.read().await.clone()
    }

    pub fn auth_cached(&self) -> Option<CodexAuth> {
        self.auth.try_read().ok().and_then(|guard| guard.clone())
    }

    pub fn refresh_failure_for_auth(&self, _auth: &CodexAuth) -> Option<String> {
        None
    }

    pub fn current_auth_uses_codex_backend(&self) -> bool {
        self.auth_cached()
            .as_ref()
            .is_some_and(CodexAuth::uses_codex_backend)
    }

    pub fn codex_api_key_env_enabled(&self) -> bool {
        self.codex_api_key_env_enabled
    }

    pub fn has_external_auth(&self) -> bool {
        self.external_auth.load(Ordering::Relaxed)
    }

    pub fn set_external_auth(&self, external_auth: Arc<dyn ExternalAuth>) {
        self.external_auth.store(true, Ordering::Relaxed);
        if let Ok(mut auth) = self.auth.try_write() {
            *auth = match external_auth.resolve_blocking() {
                Ok(Some(tokens)) => Some(CodexAuth::ApiKey(tokens.access_token)),
                _ => auth.clone(),
            };
        }
    }
}

impl Default for AuthManager {
    fn default() -> Self {
        Self::new(None)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalAuthTokens {
    pub access_token: String,
}

impl ExternalAuthTokens {
    pub fn access_token_only(access_token: impl Into<String>) -> Self {
        Self {
            access_token: access_token.into(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ExternalAuthRefreshContext;

#[async_trait]
pub trait ExternalAuth: Send + Sync + std::fmt::Debug {
    fn auth_mode(&self) -> AuthMode;

    async fn resolve(&self) -> std::io::Result<Option<ExternalAuthTokens>> {
        Ok(None)
    }

    async fn refresh(
        &self,
        _context: ExternalAuthRefreshContext,
    ) -> std::io::Result<ExternalAuthTokens>;

    fn resolve_blocking(&self) -> std::io::Result<Option<ExternalAuthTokens>> {
        if self.auth_mode() == AuthMode::ApiKey {
            Ok(Some(ExternalAuthTokens::access_token_only(
                "test-external-api-key",
            )))
        } else {
            Ok(None)
        }
    }
}
