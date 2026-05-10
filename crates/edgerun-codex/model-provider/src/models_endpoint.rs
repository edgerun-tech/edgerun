use std::sync::Arc;
use std::time::Duration;

use codex_api::HttpTransport;
use codex_api::ModelsClient;
use codex_api::RequestTelemetry;
use codex_api::TransportError;
use codex_api::auth_header_telemetry;
use codex_api::map_api_error;
use codex_model_provider_info::ModelProviderInfo;
use codex_models_manager::auth::AuthManager;
use codex_models_manager::auth::CodexAuth;
use codex_models_manager::manager::ModelsEndpointClient;
use codex_protocol::error::CodexErr;
use codex_protocol::error::Result as CoreResult;
use codex_protocol::openai_models::ModelInfo;
use codex_response_debug_context::extract_response_debug_context;
use codex_response_debug_context::telemetry_transport_error_message;
use edgerun_async_trait::async_trait;
use edgerun_http::HeaderMap;
use edgerun_tokio::time::timeout;

use crate::auth::resolve_provider_auth;

const MODELS_REFRESH_TIMEOUT: Duration = Duration::from_secs(5);
const MODELS_ENDPOINT: &str = "/models";

/// Provider-owned OpenAI-compatible `/models` endpoint.
#[derive(Clone)]
pub(crate) struct OpenAiModelsEndpoint {
    provider_info: ModelProviderInfo,
    auth_manager: Option<Arc<AuthManager>>,
    transport: Arc<dyn HttpTransport>,
}

impl std::fmt::Debug for OpenAiModelsEndpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAiModelsEndpoint")
            .field("provider_info", &self.provider_info)
            .field("auth_manager", &self.auth_manager.as_ref().map(|_| "<auth-manager>"))
            .field("transport", &"<http-transport>")
            .finish()
    }
}

impl OpenAiModelsEndpoint {
    pub(crate) fn new(
        provider_info: ModelProviderInfo,
        auth_manager: Option<Arc<AuthManager>>,
        transport: Arc<dyn HttpTransport>,
    ) -> Self {
        Self {
            provider_info,
            auth_manager,
            transport,
        }
    }

    async fn auth(&self) -> Option<CodexAuth> {
        match self.auth_manager.as_ref() {
            Some(auth_manager) => auth_manager.auth().await,
            None => None,
        }
    }
}

#[async_trait]
impl ModelsEndpointClient for OpenAiModelsEndpoint {
    fn has_command_auth(&self) -> bool {
        self.provider_info.has_command_auth()
    }

    async fn uses_codex_backend(&self) -> bool {
        self.auth()
            .await
            .as_ref()
            .is_some_and(CodexAuth::uses_codex_backend)
    }

    async fn list_models(
        &self,
        client_version: &str,
    ) -> CoreResult<(Vec<ModelInfo>, Option<String>)> {
        let auth = self.auth().await;
        let auth_mode = auth.as_ref().map(CodexAuth::auth_mode);
        let api_provider = self.provider_info.to_api_provider(auth_mode)?;
        let api_auth = resolve_provider_auth(auth.as_ref(), &self.provider_info)?;
        let transport = Arc::clone(&self.transport);
        let auth_telemetry = auth_header_telemetry(api_auth.as_ref());
        let request_telemetry: Arc<dyn RequestTelemetry> = Arc::new(ModelsRequestTelemetry {
            auth_header_attached: auth_telemetry.attached,
            auth_header_name: auth_telemetry.name,
        });
        let client = ModelsClient::new(transport, api_provider, api_auth)
            .with_telemetry(Some(request_telemetry));

        timeout(
            MODELS_REFRESH_TIMEOUT,
            client.list_models(client_version, HeaderMap::new()),
        )
        .await
        .map_err(|_| CodexErr::Timeout)?
        .map_err(map_api_error)
    }
}

#[derive(Clone)]
struct ModelsRequestTelemetry {
    auth_header_attached: bool,
    auth_header_name: Option<&'static str>,
}

impl RequestTelemetry for ModelsRequestTelemetry {
    fn on_request(
        &self,
        attempt: u64,
        status: Option<edgerun_http::StatusCode>,
        error: Option<&TransportError>,
        duration: Duration,
    ) {
        let success = status.is_some_and(|code| code.is_success()) && error.is_none();
        let error_message = error.map(telemetry_transport_error_message);
        let response_debug = error
            .map(extract_response_debug_context)
            .unwrap_or_default();
        let status = status.map(|status| status.as_u16());
        tracing::event!(
            target: "codex_otel.log_only",
            tracing::Level::INFO,
            event.name = "codex.api_request",
            duration_ms = %duration.as_millis(),
            http.response.status_code = status,
            success = success,
            error.message = error_message.as_deref(),
            attempt = attempt,
            endpoint = MODELS_ENDPOINT,
            auth.header_attached = self.auth_header_attached,
            auth.header_name = self.auth_header_name,
            auth.request_id = response_debug.request_id.as_deref(),
            auth.cf_ray = response_debug.cf_ray.as_deref(),
            auth.error = response_debug.auth_error.as_deref(),
            auth.error_code = response_debug.auth_error_code.as_deref(),
        );
        tracing::event!(
            target: "codex_otel.trace_safe",
            tracing::Level::INFO,
            event.name = "codex.api_request",
            duration_ms = %duration.as_millis(),
            http.response.status_code = status,
            success = success,
            error.message = error_message.as_deref(),
            attempt = attempt,
            endpoint = MODELS_ENDPOINT,
            auth.header_attached = self.auth_header_attached,
            auth.header_name = self.auth_header_name,
            auth.request_id = response_debug.request_id.as_deref(),
            auth.cf_ray = response_debug.cf_ray.as_deref(),
            auth.error = response_debug.auth_error.as_deref(),
            auth.error_code = response_debug.auth_error_code.as_deref(),
        );
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU64;

    use super::*;
    use codex_client::Request;
    use codex_client::Response;
    use codex_client::StreamResponse;
    use codex_protocol::config_types::ModelProviderAuthInfo;

    #[derive(Debug)]
    struct UnusedTransport;

    #[async_trait]
    impl HttpTransport for UnusedTransport {
        async fn execute(&self, _req: Request) -> Result<Response, TransportError> {
            unreachable!("these endpoint metadata tests do not execute HTTP requests")
        }

        async fn stream(&self, _req: Request) -> Result<StreamResponse, TransportError> {
            unreachable!("these endpoint metadata tests do not execute HTTP requests")
        }
    }

    fn unused_transport() -> Arc<dyn HttpTransport> {
        Arc::new(UnusedTransport)
    }

    fn provider_info_with_command_auth() -> ModelProviderInfo {
        ModelProviderInfo {
            auth: Some(ModelProviderAuthInfo {
                command: "print-token".to_string(),
                args: Vec::new(),
                timeout_ms: NonZeroU64::new(5_000).expect("timeout should be non-zero"),
                refresh_interval_ms: 300_000,
                cwd: std::env::current_dir()
                    .expect("current dir should be available")
                    .try_into()
                    .expect("current dir should be absolute"),
            }),
            requires_openai_auth: false,
            ..ModelProviderInfo::create_openai_provider(/*base_url*/ None)
        }
    }

    #[test]
    fn command_auth_provider_reports_command_auth_without_cached_auth() {
        let endpoint = OpenAiModelsEndpoint::new(
            provider_info_with_command_auth(),
            /*auth_manager*/ None,
            unused_transport(),
        );

        assert!(endpoint.has_command_auth());
    }

    #[test]
    fn provider_without_command_auth_reports_no_command_auth() {
        let endpoint = OpenAiModelsEndpoint::new(
            ModelProviderInfo::create_openai_provider(/*base_url*/ None),
            /*auth_manager*/ None,
            unused_transport(),
        );

        assert!(!endpoint.has_command_auth());
    }
}
