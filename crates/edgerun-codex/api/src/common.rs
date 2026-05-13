use crate::error::ApiError;
use codex_protocol::config_types::ReasoningSummary as ReasoningSummaryConfig;
use codex_protocol::config_types::Verbosity as VerbosityConfig;
use codex_protocol::models::ResponseItem;
use codex_protocol::openai_models::ReasoningEffort as ReasoningEffortConfig;
use codex_protocol::protocol::ModelVerification;
use codex_protocol::protocol::RateLimitSnapshot;
use codex_protocol::protocol::TokenUsage;
use codex_protocol::protocol::W3cTraceContext;
use edgerun_futures::Stream;
use edgerun_json::FromJson;
use edgerun_json::JsonValue;
use edgerun_json::Map;
use edgerun_json::ToJson;
use edgerun_json::Value;
use edgerun_tokio::sync::mpsc;
use std::collections::HashMap;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

pub const WS_REQUEST_HEADER_TRACEPARENT_CLIENT_METADATA_KEY: &str = "ws_request_header_traceparent";
pub const WS_REQUEST_HEADER_TRACESTATE_CLIENT_METADATA_KEY: &str = "ws_request_header_tracestate";

/// Canonical input payload for the compaction endpoint.
#[derive(Debug, Clone)]
pub struct CompactionInput<'a> {
    pub model: &'a str,
    pub input: &'a [ResponseItem],
    pub instructions: &'a str,
    pub tools: Vec<Value>,
    pub parallel_tool_calls: bool,
    pub reasoning: Option<Reasoning>,
    pub service_tier: Option<&'a str>,
    pub prompt_cache_key: Option<&'a str>,
    pub text: Option<TextControls>,
}

impl ToJson for CompactionInput<'_> {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.push_field("model", self.model);
        object.push_field("input", response_items_json(self.input));
        if !self.instructions.is_empty() {
            object.push_field("instructions", self.instructions);
        }
        object.push_field("tools", self.tools.to_json());
        object.push_field("parallel_tool_calls", self.parallel_tool_calls);
        object.push_opt_field("reasoning", self.reasoning.as_ref().map(ToJson::to_json));
        object.push_opt_field("service_tier", self.service_tier);
        object.push_opt_field("prompt_cache_key", self.prompt_cache_key);
        object.push_opt_field("text", self.text.as_ref().map(ToJson::to_json));
        object.into()
    }
}

/// Canonical input payload for the memory summarize endpoint.
#[derive(Debug, Clone, ToJson)]
pub struct MemorySummarizeInput {
    pub model: String,
    #[json(rename = "traces")]
    pub raw_memories: Vec<RawMemory>,
    pub reasoning: Option<Reasoning>,
}

#[derive(Debug, Clone, ToJson)]
pub struct RawMemory {
    pub id: String,
    pub metadata: RawMemoryMetadata,
    pub items: Vec<JsonValue>,
}

#[derive(Debug, Clone, ToJson)]
pub struct RawMemoryMetadata {
    pub source_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemorySummarizeOutput {
    pub raw_memory: String,
    pub memory_summary: String,
}

impl FromJson for MemorySummarizeOutput {
    fn from_json(value: Value) -> Result<Self, edgerun_json::JsonValueError> {
        let mut object = value.into_object("MemorySummarizeOutput")?;
        Ok(Self {
            raw_memory: object.take_required_any(&["trace_summary", "raw_memory"])?,
            memory_summary: object.take_required("memory_summary")?,
        })
    }
}

#[derive(Debug)]
pub enum ResponseEvent {
    Created,
    OutputItemDone(ResponseItem),
    OutputItemAdded(ResponseItem),
    /// Emitted when the server includes `OpenAI-Model` on the stream response.
    /// This can differ from the requested model when backend safety routing applies.
    ServerModel(String),
    /// Emitted when the server recommends additional account verification.
    ModelVerifications(Vec<ModelVerification>),
    /// Emitted when `X-Reasoning-Included: true` is present on the response,
    /// meaning the server already accounted for past reasoning tokens and the
    /// client should not re-estimate them.
    ServerReasoningIncluded(bool),
    Completed {
        response_id: String,
        token_usage: Option<TokenUsage>,
        /// Did the model affirmatively end its turn? Some providers do not set this,
        /// so we rely on fallback logic when this is `None`.
        end_turn: Option<bool>,
    },
    OutputTextDelta(String),
    ToolCallInputDelta {
        item_id: String,
        call_id: Option<String>,
        delta: String,
    },
    ReasoningSummaryDelta {
        delta: String,
        summary_index: i64,
    },
    ReasoningContentDelta {
        delta: String,
        content_index: i64,
    },
    ReasoningSummaryPartAdded {
        summary_index: i64,
    },
    RateLimits(RateLimitSnapshot),
    ModelsEtag(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Reasoning {
    pub effort: Option<ReasoningEffortConfig>,
    pub summary: Option<ReasoningSummaryConfig>,
}

impl ToJson for Reasoning {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.push_opt_field("effort", self.effort.map(reasoning_effort_json));
        object.push_opt_field("summary", self.summary.map(reasoning_summary_json));
        object.into()
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub enum TextFormatType {
    #[default]
    JsonSchema,
}

impl ToJson for TextFormatType {
    fn to_json(&self) -> Value {
        match self {
            Self::JsonSchema => Value::String("json_schema".to_string()),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct TextFormat {
    /// Format type used by the OpenAI text controls.
    pub r#type: TextFormatType,
    /// When true, the server is expected to strictly validate responses.
    pub strict: bool,
    /// JSON schema for the desired output.
    pub schema: Value,
    /// Friendly name for the format, used in telemetry/debugging.
    pub name: String,
}

impl ToJson for TextFormat {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.push_field("type", self.r#type.to_json());
        object.push_field("strict", self.strict);
        object.push_field("schema", self.schema.clone());
        object.push_field("name", self.name.as_str());
        object.into()
    }
}

/// Controls the `text` field for the Responses API, combining verbosity and
/// optional JSON schema output formatting.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct TextControls {
    pub verbosity: Option<OpenAiVerbosity>,
    pub format: Option<TextFormat>,
}

impl ToJson for TextControls {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.push_opt_field("verbosity", self.verbosity.as_ref().map(ToJson::to_json));
        object.push_opt_field("format", self.format.as_ref().map(ToJson::to_json));
        object.into()
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub enum OpenAiVerbosity {
    Low,
    #[default]
    Medium,
    High,
}

impl ToJson for OpenAiVerbosity {
    fn to_json(&self) -> Value {
        Value::String(
            match self {
                Self::Low => "low",
                Self::Medium => "medium",
                Self::High => "high",
            }
            .to_string(),
        )
    }
}

impl From<VerbosityConfig> for OpenAiVerbosity {
    fn from(v: VerbosityConfig) -> Self {
        match v {
            VerbosityConfig::Low => OpenAiVerbosity::Low,
            VerbosityConfig::Medium => OpenAiVerbosity::Medium,
            VerbosityConfig::High => OpenAiVerbosity::High,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResponsesApiRequest {
    pub model: String,
    pub instructions: String,
    pub input: Vec<ResponseItem>,
    pub tools: Vec<edgerun_json::Value>,
    pub tool_choice: String,
    pub parallel_tool_calls: bool,
    pub reasoning: Option<Reasoning>,
    pub store: bool,
    pub stream: bool,
    pub include: Vec<String>,
    pub service_tier: Option<String>,
    pub prompt_cache_key: Option<String>,
    pub text: Option<TextControls>,
    pub client_metadata: Option<HashMap<String, String>>,
}

impl ToJson for ResponsesApiRequest {
    fn to_json(&self) -> Value {
        response_create_fields_json(
            &self.model,
            &self.instructions,
            None,
            &self.input,
            &self.tools,
            &self.tool_choice,
            self.parallel_tool_calls,
            self.reasoning.as_ref(),
            self.store,
            self.stream,
            &self.include,
            self.service_tier.as_deref(),
            self.prompt_cache_key.as_deref(),
            self.text.as_ref(),
            None,
            self.client_metadata.as_ref(),
        )
    }
}

impl From<&ResponsesApiRequest> for ResponseCreateWsRequest {
    fn from(request: &ResponsesApiRequest) -> Self {
        Self {
            model: request.model.clone(),
            instructions: request.instructions.clone(),
            previous_response_id: None,
            input: request.input.clone(),
            tools: request.tools.clone(),
            tool_choice: request.tool_choice.clone(),
            parallel_tool_calls: request.parallel_tool_calls,
            reasoning: request.reasoning.clone(),
            store: request.store,
            stream: request.stream,
            include: request.include.clone(),
            service_tier: request.service_tier.clone(),
            prompt_cache_key: request.prompt_cache_key.clone(),
            text: request.text.clone(),
            generate: None,
            client_metadata: request.client_metadata.clone(),
        }
    }
}

#[derive(Debug)]
pub struct ResponseCreateWsRequest {
    pub model: String,
    pub instructions: String,
    pub previous_response_id: Option<String>,
    pub input: Vec<ResponseItem>,
    pub tools: Vec<Value>,
    pub tool_choice: String,
    pub parallel_tool_calls: bool,
    pub reasoning: Option<Reasoning>,
    pub store: bool,
    pub stream: bool,
    pub include: Vec<String>,
    pub service_tier: Option<String>,
    pub prompt_cache_key: Option<String>,
    pub text: Option<TextControls>,
    pub generate: Option<bool>,
    pub client_metadata: Option<HashMap<String, String>>,
}

impl ToJson for ResponseCreateWsRequest {
    fn to_json(&self) -> Value {
        response_create_fields_json(
            &self.model,
            &self.instructions,
            self.previous_response_id.as_deref(),
            &self.input,
            &self.tools,
            &self.tool_choice,
            self.parallel_tool_calls,
            self.reasoning.as_ref(),
            self.store,
            self.stream,
            &self.include,
            self.service_tier.as_deref(),
            self.prompt_cache_key.as_deref(),
            self.text.as_ref(),
            self.generate,
            self.client_metadata.as_ref(),
        )
    }
}

#[derive(Debug)]
pub struct ResponseProcessedWsRequest {
    pub response_id: String,
}

pub fn response_create_client_metadata(
    client_metadata: Option<HashMap<String, String>>,
    trace: Option<&W3cTraceContext>,
) -> Option<HashMap<String, String>> {
    let mut client_metadata = client_metadata.unwrap_or_default();

    if let Some(traceparent) = trace.and_then(|trace| trace.traceparent.as_deref()) {
        client_metadata.insert(
            WS_REQUEST_HEADER_TRACEPARENT_CLIENT_METADATA_KEY.to_string(),
            traceparent.to_string(),
        );
    }
    if let Some(tracestate) = trace.and_then(|trace| trace.tracestate.as_deref()) {
        client_metadata.insert(
            WS_REQUEST_HEADER_TRACESTATE_CLIENT_METADATA_KEY.to_string(),
            tracestate.to_string(),
        );
    }

    (!client_metadata.is_empty()).then_some(client_metadata)
}

impl ToJson for ResponseProcessedWsRequest {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.push_field("response_id", self.response_id.as_str());
        object.into()
    }
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum ResponsesWsRequest {
    ResponseCreate(ResponseCreateWsRequest),
    ResponseProcessed(ResponseProcessedWsRequest),
}

impl ToJson for ResponsesWsRequest {
    fn to_json(&self) -> Value {
        match self {
            Self::ResponseCreate(request) => {
                let mut object = request
                    .to_json()
                    .into_object("ResponsesWsRequest")
                    .expect("response.create is an object");
                object.push_field("type", "response.create");
                object.into()
            }
            Self::ResponseProcessed(request) => {
                let mut object = request
                    .to_json()
                    .into_object("ResponsesWsRequest")
                    .expect("response.processed is an object");
                object.push_field("type", "response.processed");
                object.into()
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn response_create_fields_json(
    model: &str,
    instructions: &str,
    previous_response_id: Option<&str>,
    input: &[ResponseItem],
    tools: &[Value],
    tool_choice: &str,
    parallel_tool_calls: bool,
    reasoning: Option<&Reasoning>,
    store: bool,
    stream: bool,
    include: &[String],
    service_tier: Option<&str>,
    prompt_cache_key: Option<&str>,
    text: Option<&TextControls>,
    generate: Option<bool>,
    client_metadata: Option<&HashMap<String, String>>,
) -> Value {
    let mut object = Map::new();
    object.push_field("model", model);
    if !instructions.is_empty() {
        object.push_field("instructions", instructions);
    }
    object.push_opt_field("previous_response_id", previous_response_id);
    object.push_field("input", response_items_json(input));
    object.push_field("tools", tools.to_vec().to_json());
    object.push_field("tool_choice", tool_choice);
    object.push_field("parallel_tool_calls", parallel_tool_calls);
    object.push_opt_field("reasoning", reasoning.map(ToJson::to_json));
    object.push_field("store", store);
    object.push_field("stream", stream);
    object.push_field("include", include.to_vec().to_json());
    object.push_opt_field("service_tier", service_tier);
    object.push_opt_field("prompt_cache_key", prompt_cache_key);
    object.push_opt_field("text", text.map(ToJson::to_json));
    object.push_opt_field("generate", generate);
    object.push_opt_field("client_metadata", client_metadata.map(string_map_json));
    object.into()
}

fn response_items_json(items: &[ResponseItem]) -> Value {
    Value::Array(
        items
            .iter()
            .map(|item| edgerun_json::to_serde_value(item).unwrap_or(Value::Null))
            .collect(),
    )
}

fn string_map_json(map: &HashMap<String, String>) -> Value {
    let mut object = Map::new();
    for (key, value) in map {
        object.push_field(key.as_str(), value.as_str());
    }
    object.into()
}

fn reasoning_effort_json(value: ReasoningEffortConfig) -> Value {
    Value::String(value.to_string())
}

fn reasoning_summary_json(value: ReasoningSummaryConfig) -> Value {
    Value::String(value.to_string())
}

pub fn create_text_param_for_request(
    verbosity: Option<VerbosityConfig>,
    output_schema: &Option<Value>,
    output_schema_strict: bool,
) -> Option<TextControls> {
    if verbosity.is_none() && output_schema.is_none() {
        return None;
    }

    Some(TextControls {
        verbosity: verbosity.map(std::convert::Into::into),
        format: output_schema.as_ref().map(|schema| TextFormat {
            r#type: TextFormatType::JsonSchema,
            strict: output_schema_strict,
            schema: schema.clone(),
            name: "codex_output_schema".to_string(),
        }),
    })
}

pub struct ResponseStream {
    pub rx_event: mpsc::Receiver<Result<ResponseEvent, ApiError>>,
    /// Server-assigned `x-request-id` response header, when present.
    pub upstream_request_id: Option<String>,
}

impl Stream for ResponseStream {
    type Item = Result<ResponseEvent, ApiError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx_event.poll_recv(cx)
    }
}
