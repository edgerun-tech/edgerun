//! Stubbed `codex-core` facade for the lifted workspace.
//!
//! The original `codex-core` crate is the product integration layer. It wires
//! config loading, auth persistence, telemetry, rollout storage, plugins,
//! sandboxing, MCP, hooks, state DBs, and terminal behavior into the agent
//! loop. Those are deliberately outside this lift.
//!
//! This facade keeps the crate name available while exposing the useful lower
//! layers that have been separated and made to compile without the product
//! platform.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;

pub use codex_api as api;
pub use codex_api::Compression;
pub use codex_api::OpenAiVerbosity;
pub use codex_api::Provider;
pub use codex_api::Reasoning;
pub use codex_api::ResponseEvent;
pub use codex_api::ResponseStream;
pub use codex_api::ResponsesOptions;
pub use codex_api::TextControls;
pub use codex_apply_patch as apply_patch;
pub use codex_client::HttpTransport;
#[cfg(feature = "native-transport")]
pub use codex_client::ReqwestTransport;
#[cfg(feature = "model-provider")]
pub use codex_model_provider as model_provider;
#[cfg(feature = "model-provider")]
pub use codex_model_provider_info as model_provider_info;
#[cfg(feature = "model-provider")]
pub use codex_models_manager as models_manager;
pub use codex_protocol as protocol;
pub use codex_shell_command as shell_command;
pub use codex_tools as tools;

pub use codex_protocol::config_types::ModelProviderAuthInfo;
pub use codex_protocol::config_types::Personality;
pub use codex_protocol::config_types::ReasoningSummary;
pub use codex_protocol::config_types::Verbosity;
pub use codex_protocol::models::BaseInstructions;
pub use codex_protocol::models::ResponseItem;
pub use codex_protocol::openai_models::ReasoningEffort;
pub use codex_protocol::protocol::SessionSource;
pub use codex_protocol::protocol::TokenUsage;
pub use codex_protocol::SessionId;
pub use codex_protocol::ThreadId;
pub use codex_tools::ToolSpec;
pub use edgerun_json::JsonValue;
use edgerun_http::HeaderValue;

/// Review thread system prompt placeholder.
pub const REVIEW_PROMPT: &str = "";

/// Header names retained for callers that used the old core constants.
pub const X_CODEX_INSTALLATION_ID_HEADER: &str = "x-codex-installation-id";
pub const X_CODEX_TURN_METADATA_HEADER: &str = "x-codex-turn-metadata";
pub const X_RESPONSESAPI_INCLUDE_TIMING_METRICS_HEADER: &str =
    "x-responsesapi-include-timing-metrics";

/// API request payload for a single model turn.
#[derive(Debug, Clone)]
pub struct Prompt {
    pub input: Vec<ResponseItem>,
    pub tools: Vec<ToolSpec>,
    pub parallel_tool_calls: bool,
    pub base_instructions: BaseInstructions,
    pub personality: Option<Personality>,
    pub output_schema: Option<JsonValue>,
    pub output_schema_strict: bool,
}

impl Default for Prompt {
    fn default() -> Self {
        Self {
            input: Vec::new(),
            tools: Vec::new(),
            parallel_tool_calls: false,
            base_instructions: BaseInstructions::default(),
            personality: None,
            output_schema: None,
            output_schema_strict: true,
        }
    }
}

/// Error type for the lifted, transport-injected model client.
#[derive(Debug)]
pub enum ModelClientError {
    Api(codex_api::ApiError),
    ToolSerialization(edgerun_json::Error),
}

impl std::fmt::Display for ModelClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Api(error) => write!(f, "{error}"),
            Self::ToolSerialization(error) => {
                write!(f, "failed to serialize Responses API tools: {error}")
            }
        }
    }
}

impl std::error::Error for ModelClientError {}

impl From<codex_api::ApiError> for ModelClientError {
    fn from(error: codex_api::ApiError) -> Self {
        Self::Api(error)
    }
}

/// Per-request controls for a model turn.
#[derive(Debug, Clone)]
pub struct TurnRequest {
    pub prompt: Prompt,
    pub reasoning_effort: Option<ReasoningEffort>,
    pub reasoning_summary: Option<ReasoningSummary>,
    pub verbosity: Option<Verbosity>,
    pub tool_choice: String,
    pub include: Vec<String>,
    pub store: bool,
    pub service_tier: Option<String>,
    pub prompt_cache_key: Option<String>,
    pub client_metadata: Option<HashMap<String, String>>,
    pub session_id: Option<String>,
    pub thread_id: Option<String>,
    pub session_source: Option<SessionSource>,
    pub compression: Compression,
    pub turn_state: Option<Arc<OnceLock<String>>>,
}

impl Default for TurnRequest {
    fn default() -> Self {
        Self {
            prompt: Prompt::default(),
            reasoning_effort: None,
            reasoning_summary: None,
            verbosity: None,
            tool_choice: "auto".to_string(),
            include: Vec::new(),
            store: false,
            service_tier: None,
            prompt_cache_key: None,
            client_metadata: None,
            session_id: None,
            thread_id: None,
            session_source: None,
            compression: Compression::None,
            turn_state: None,
        }
    }
}

/// Summary produced by draining a Responses stream.
#[derive(Debug, Clone, Default)]
pub struct TurnOutput {
    pub response_id: Option<String>,
    pub output_text: String,
    pub reasoning_summary_text: String,
    pub reasoning_content_text: String,
    pub output_items: Vec<ResponseItem>,
    pub token_usage: Option<TokenUsage>,
    pub end_turn: Option<bool>,
    pub server_model: Option<String>,
    pub server_reasoning_included: bool,
}

/// Session-scoped model client for the lifted workspace.
///
/// This is intentionally transport-injected. Native callers can pass
/// `codex_client::ReqwestTransport` through an `Arc`, while browser/WASM
/// callers can pass a fetch-backed `HttpTransport`. The client does not touch
/// Codex product state, telemetry databases, plugin scanning, terminal state,
/// or rollout logs.
pub struct ModelClient {
    model: String,
    session_id: String,
    thread_id: String,
    installation_id: String,
    session_source: SessionSource,
    responses: codex_api::ResponsesClient<Arc<dyn HttpTransport>>,
}

impl std::fmt::Debug for ModelClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModelClient")
            .field("model", &self.model)
            .field("responses", &"<responses-client>")
            .finish()
    }
}

impl ModelClient {
    pub fn new(
        model: impl Into<String>,
        provider: Provider,
        auth: codex_api::SharedAuthProvider,
        transport: Arc<dyn HttpTransport>,
    ) -> Self {
        let thread_id = ThreadId::new();
        let session_id = SessionId::from(thread_id);
        Self {
            model: model.into(),
            session_id: session_id.to_string(),
            thread_id: thread_id.to_string(),
            installation_id: ThreadId::new().to_string(),
            session_source: SessionSource::Cli,
            responses: codex_api::ResponsesClient::new(transport, provider, auth),
        }
    }

    #[cfg(feature = "native-transport")]
    pub fn new_native(
        model: impl Into<String>,
        provider: Provider,
        auth: codex_api::SharedAuthProvider,
    ) -> Self {
        Self::new(
            model,
            provider,
            auth,
            Arc::new(ReqwestTransport::new_default()),
        )
    }

    #[cfg(feature = "model-provider")]
    pub async fn from_model_provider(
        model: impl Into<String>,
        provider: &dyn codex_model_provider::ModelProvider,
        transport: Arc<dyn HttpTransport>,
    ) -> codex_protocol::error::Result<Self> {
        let api_provider = provider.api_provider().await?;
        let auth = provider.api_auth().await?;
        Ok(Self::new(model, api_provider, auth, transport))
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub async fn stream_turn(
        &self,
        request: TurnRequest,
    ) -> Result<ResponseStream, ModelClientError> {
        let (api_request, options) = self.build_request(request)?;
        Ok(self.responses.stream_request(api_request, options).await?)
    }

    pub async fn collect_turn(&self, request: TurnRequest) -> Result<TurnOutput, ModelClientError> {
        let mut stream = self.stream_turn(request).await?;
        let mut output = TurnOutput::default();

        while let Some(event) = stream.rx_event.recv().await {
            match event? {
                ResponseEvent::Created => {}
                ResponseEvent::OutputItemDone(item) => output.output_items.push(item),
                ResponseEvent::OutputItemAdded(_) => {}
                ResponseEvent::OutputTextDelta(delta) => output.output_text.push_str(&delta),
                ResponseEvent::ReasoningSummaryDelta { delta, .. } => {
                    output.reasoning_summary_text.push_str(&delta);
                }
                ResponseEvent::ReasoningContentDelta { delta, .. } => {
                    output.reasoning_content_text.push_str(&delta);
                }
                ResponseEvent::Completed {
                    response_id,
                    token_usage,
                    end_turn,
                } => {
                    output.response_id = Some(response_id);
                    output.token_usage = token_usage;
                    output.end_turn = end_turn;
                }
                ResponseEvent::ServerModel(model) => output.server_model = Some(model),
                ResponseEvent::ServerReasoningIncluded(included) => {
                    output.server_reasoning_included = included;
                }
                ResponseEvent::ToolCallInputDelta { .. }
                | ResponseEvent::ReasoningSummaryPartAdded { .. }
                | ResponseEvent::RateLimits(_)
                | ResponseEvent::ModelVerifications(_)
                | ResponseEvent::ModelsEtag(_) => {}
            }
        }

        Ok(output)
    }

    fn build_request(
        &self,
        request: TurnRequest,
    ) -> Result<(codex_api::ResponsesApiRequest, ResponsesOptions), ModelClientError> {
        let TurnRequest {
            prompt,
            reasoning_effort,
            reasoning_summary,
            verbosity,
            tool_choice,
            include,
            store,
            service_tier,
            prompt_cache_key,
            client_metadata,
            session_id,
            thread_id,
            session_source,
            compression,
            turn_state,
        } = request;

        let Prompt {
            input,
            tools,
            parallel_tool_calls,
            base_instructions,
            personality: _,
            output_schema,
            output_schema_strict,
        } = prompt;

        let tools = codex_tools::create_tools_json_for_responses_api(&tools)
            .map_err(ModelClientError::ToolSerialization)?;
        let text = codex_api::create_text_param_for_request(
            verbosity,
            &output_schema,
            output_schema_strict,
        );

        let reasoning =
            (reasoning_effort.is_some() || reasoning_summary.is_some()).then_some(Reasoning {
                effort: reasoning_effort,
                summary: reasoning_summary,
            });

        let api_request = codex_api::ResponsesApiRequest {
            model: self.model.clone(),
            instructions: base_instructions.text,
            input,
            tools,
            tool_choice,
            parallel_tool_calls,
            reasoning,
            store,
            stream: true,
            include,
            service_tier,
            prompt_cache_key,
            text,
            client_metadata: Some(self.client_metadata(client_metadata)),
        };

        let session_id = session_id.or_else(|| Some(self.session_id.clone()));
        let thread_id = thread_id.or_else(|| Some(self.thread_id.clone()));
        let session_source = session_source.or_else(|| Some(self.session_source.clone()));
        let prompt_cache_key = api_request
            .prompt_cache_key
            .clone()
            .or_else(|| Some(self.thread_id.clone()));
        let mut api_request = api_request;
        api_request.prompt_cache_key = prompt_cache_key;

        let mut extra_headers = edgerun_http::HeaderMap::new();
        if let Ok(value) = HeaderValue::from_str(&self.installation_id) {
            extra_headers.insert(X_CODEX_INSTALLATION_ID_HEADER, value);
        }
        let window_id = format!("{}:0", self.thread_id);
        if let Ok(value) = HeaderValue::from_str(&window_id) {
            extra_headers.insert("x-codex-window-id", value);
        }

        let options = ResponsesOptions {
            session_id,
            thread_id,
            session_source,
            extra_headers,
            compression,
            turn_state,
        };

        Ok((api_request, options))
    }

    fn client_metadata(
        &self,
        existing: Option<HashMap<String, String>>,
    ) -> HashMap<String, String> {
        let mut metadata = existing.unwrap_or_default();
        metadata
            .entry(X_CODEX_INSTALLATION_ID_HEADER.to_string())
            .or_insert_with(|| self.installation_id.clone());
        metadata
            .entry("x-codex-window-id".to_string())
            .or_insert_with(|| format!("{}:0", self.thread_id));
        metadata
    }
}

/// Placeholder for the old per-turn model client session.
#[derive(Debug, Clone, Copy, Default)]
pub struct ModelClientSession;

/// Placeholder for the old thread manager.
#[derive(Debug, Clone, Copy, Default)]
pub struct ThreadManager;

#[deprecated(note = "use ThreadManager")]
pub type ConversationManager = ThreadManager;

/// Placeholder for a new thread request.
#[derive(Debug, Clone, Copy, Default)]
pub struct NewThread;

#[deprecated(note = "use NewThread")]
pub type NewConversation = NewThread;

/// Placeholder for a running Codex thread.
#[derive(Debug, Clone, Copy, Default)]
pub struct CodexThread;

#[deprecated(note = "use CodexThread")]
pub type CodexConversation = CodexThread;

/// Stub error retained for callers that reference steering APIs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteerInputError;

impl std::fmt::Display for SteerInputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("steer input is unavailable in the lifted core stub")
    }
}

impl std::error::Error for SteerInputError {}

/// Stub event parser. The real mapping code can be lifted later if needed.
pub fn parse_turn_item(_item: &ResponseItem) -> Option<codex_protocol::items::TurnItem> {
    None
}

/// Minimal text extraction helper retained from the old public surface.
pub fn content_items_to_text(items: &[codex_protocol::models::ContentItem]) -> String {
    items
        .iter()
        .filter_map(|item| match item {
            codex_protocol::models::ContentItem::InputText { text }
            | codex_protocol::models::ContentItem::OutputText { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Stub model manager builder. Call `codex_model_provider::create_model_provider`
/// directly in the lifted client.
pub fn build_models_manager() {}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_api::AuthProvider;
    use codex_client::Request;
    use codex_client::Response;
    use codex_client::StreamResponse;
    use codex_client::TransportError;
    use edgerun_bytes::Bytes;
    use edgerun_http::HeaderMap;
    use edgerun_http::StatusCode;
    use edgerun_json::Value;
    use edgerun_json::json;
    use std::sync::Mutex;
    use std::time::Duration;

    #[derive(Clone, Default)]
    struct NoAuth;

    impl AuthProvider for NoAuth {
        fn add_auth_headers(&self, _headers: &mut HeaderMap) {}
    }

    #[derive(Clone)]
    struct RecordingTransport {
        requests: Arc<Mutex<Vec<Request>>>,
        body: String,
    }

    impl RecordingTransport {
        fn new(body: String) -> Self {
            Self {
                requests: Arc::new(Mutex::new(Vec::new())),
                body,
            }
        }

        fn take_requests(&self) -> Vec<Request> {
            let mut guard = self.requests.lock().expect("request log lock");
            std::mem::take(&mut *guard)
        }
    }

    impl HttpTransport for RecordingTransport {
        fn execute<'async_trait>(
            &'async_trait self,
            _req: Request,
        ) -> core::pin::Pin<
            Box<
                dyn core::future::Future<Output = Result<Response, TransportError>>
                    + Send
                    + 'async_trait,
            >,
        >
        where
            Self: 'async_trait,
        {
            Box::pin(async move {
                Err(TransportError::Build("execute should not run".to_string()))
            })
        }

        fn stream<'async_trait>(
            &'async_trait self,
            req: Request,
        ) -> core::pin::Pin<
            Box<
                dyn core::future::Future<Output = Result<StreamResponse, TransportError>>
                    + Send
                    + 'async_trait,
            >,
        >
        where
            Self: 'async_trait,
        {
            Box::pin(async move {
                self.requests.lock().expect("request log lock").push(req);
                let bytes = edgerun_futures::stream::iter(vec![Ok::<Bytes, TransportError>(
                    Bytes::from(self.body.clone()),
                )]);
                Ok(StreamResponse {
                    status: StatusCode::OK,
                    headers: HeaderMap::new(),
                    bytes: Box::pin(bytes),
                })
            })
        }
    }

    fn provider() -> Provider {
        Provider {
            name: "openai".to_string(),
            base_url: "https://example.com/v1".to_string(),
            query_params: None,
            headers: HeaderMap::new(),
            retry: codex_api::RetryConfig {
                max_attempts: 1,
                base_delay: Duration::from_millis(1),
                retry_429: false,
                retry_5xx: false,
                retry_transport: false,
            },
            stream_idle_timeout: Duration::from_millis(200),
        }
    }

    fn sse(events: &[Value]) -> String {
        events
            .iter()
            .map(|event| {
                let kind = event
                    .get("type")
                    .and_then(Value::as_str)
                    .expect("fixture event type");
                format!("event: {kind}\ndata: {event}\n\n")
            })
            .collect()
    }

    #[test]
    fn collect_turn_builds_responses_request_and_collects_stream() {
        let runtime = edgerun_tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        runtime.block_on(async {
            collect_turn_builds_responses_request_and_collects_stream_async().await;
        });
    }

    async fn collect_turn_builds_responses_request_and_collects_stream_async() {
        let body = sse(&[
            json!({
                "type": "response.output_text.delta",
                "delta": "Hello "
            }),
            json!({
                "type": "response.output_text.delta",
                "delta": "there"
            }),
            json!({
                "type": "response.reasoning_summary_text.delta",
                "summary_index": 0,
                "delta": "Checked transport."
            }),
            json!({
                "type": "response.completed",
                "response": {
                    "id": "resp-1",
                    "end_turn": true,
                    "usage": {
                        "input_tokens": 3,
                        "input_tokens_details": { "cached_tokens": 1 },
                        "output_tokens": 5,
                        "output_tokens_details": { "reasoning_tokens": 2 },
                        "total_tokens": 8
                    }
                }
            }),
        ]);
        let transport = RecordingTransport::new(body);
        let client = ModelClient::new(
            "gpt-test",
            provider(),
            Arc::new(NoAuth),
            Arc::new(transport.clone()),
        );

        let mut metadata = HashMap::new();
        metadata.insert("surface".to_string(), "portable-core-test".to_string());

        let output = client
            .collect_turn(TurnRequest {
                prompt: Prompt {
                    base_instructions: BaseInstructions {
                        text: "Use concise answers.".to_string(),
                    },
                    parallel_tool_calls: true,
                    ..Prompt::default()
                },
                reasoning_effort: Some(ReasoningEffort::Low),
                reasoning_summary: Some(ReasoningSummary::Auto),
                verbosity: Some(Verbosity::Low),
                include: vec!["reasoning.encrypted_content".to_string()],
                client_metadata: Some(metadata),
                session_id: Some("session-1".to_string()),
                thread_id: Some("thread-1".to_string()),
                ..TurnRequest::default()
            })
            .await
            .expect("collect turn");

        assert_eq!(output.response_id.as_deref(), Some("resp-1"));
        assert_eq!(output.output_text, "Hello there");
        assert_eq!(output.reasoning_summary_text, "Checked transport.");
        assert_eq!(output.end_turn, Some(true));
        let usage = output.token_usage.expect("token usage");
        assert_eq!(usage.input_tokens, 3);
        assert_eq!(usage.cached_input_tokens, 1);
        assert_eq!(usage.output_tokens, 5);
        assert_eq!(usage.reasoning_output_tokens, 2);
        assert_eq!(usage.total_tokens, 8);

        let requests = transport.take_requests();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        assert_eq!(request.method, edgerun_http::Method::POST);
        assert_eq!(request.url, "https://example.com/v1/responses");
        assert_eq!(
            request
                .headers
                .get("x-client-request-id")
                .and_then(|value| value.to_str().ok()),
            Some("thread-1")
        );

        let request_body = match request.body.as_ref() {
            Some(codex_client::RequestBody::Json(body)) => body,
            other => panic!("expected json request body, got {other:?}"),
        };
        let request_json: Value = edgerun_json::from_str(&request_body.to_string()).expect("request json");

        assert_eq!(request_json["model"], "gpt-test");
        assert_eq!(request_json["instructions"], "Use concise answers.");
        assert_eq!(request_json["stream"], true);
        assert_eq!(request_json["parallel_tool_calls"], true);
        assert_eq!(request_json["reasoning"]["effort"], "low");
        assert_eq!(request_json["reasoning"]["summary"], "auto");
        assert_eq!(request_json["text"]["verbosity"], "low");
        assert_eq!(request_json["include"][0], "reasoning.encrypted_content");
        assert_eq!(
            request_json["client_metadata"]["surface"],
            "portable-core-test"
        );
    }
}
