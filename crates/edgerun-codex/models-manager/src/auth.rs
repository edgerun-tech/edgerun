use std::sync::Arc;

use codex_app_server_protocol::AuthMode;
use codex_protocol::account::PlanType;
use tokio::sync::RwLock;

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

#[derive(Clone, Debug, Default)]
pub struct AuthManager {
    auth: Arc<RwLock<Option<CodexAuth>>>,
    codex_api_key_env_enabled: bool,
}

impl AuthManager {
    pub fn new(auth: Option<CodexAuth>) -> Self {
        Self {
            auth: Arc::new(RwLock::new(auth)),
            codex_api_key_env_enabled: false,
        }
    }

    pub fn external_bearer_only(
        _config: codex_protocol::config_types::ModelProviderAuthInfo,
    ) -> Self {
        Self::default()
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
}
