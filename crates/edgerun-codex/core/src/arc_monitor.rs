use std::env;
use std::time::Duration;

use edgerun_json::ToJson;
use tracing::warn;

use crate::compact::content_items_to_text;
use crate::event_mapping::is_contextual_user_message_content;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use codex_login::default_client::build_reqwest_client;
use codex_protocol::models::MessagePhase;
use codex_protocol::models::ResponseItem;

const ARC_MONITOR_TIMEOUT: Duration = Duration::from_secs(30);
const CODEX_ARC_MONITOR_ENDPOINT_OVERRIDE: &str = "CODEX_ARC_MONITOR_ENDPOINT_OVERRIDE";
const CODEX_ARC_MONITOR_TOKEN: &str = "CODEX_ARC_MONITOR_TOKEN";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ArcMonitorOutcome {
    Ok,
    SteerModel(String),
    AskUser(String),
}

#[derive(Debug, PartialEq)]
struct ArcMonitorRequest {
    metadata: ArcMonitorMetadata,
    messages: Option<Vec<ArcMonitorChatMessage>>,
    input: Option<Vec<ResponseItem>>,
    policies: Option<ArcMonitorPolicies>,
    action: edgerun_json::Map<String, edgerun_json::Value>,
}

impl ToJson for ArcMonitorRequest {
    fn to_json(&self) -> edgerun_json::Value {
        let mut object = edgerun_json::Map::new();
        object.push_field("metadata", self.metadata.to_json());
        if let Some(messages) = &self.messages {
            object.push_field("messages", messages.to_json());
        }
        if let Some(input) = &self.input {
            object.push_field(
                "input",
                edgerun_json::Value::array_from_iter(input.iter().map(|item| {
                    edgerun_json::to_serde_value(item).unwrap_or(edgerun_json::Value::Null)
                })),
            );
        }
        if let Some(policies) = &self.policies {
            object.push_field("policies", policies.to_json());
        }
        object.push_field("action", self.action.to_json());
        edgerun_json::Value::Object(object)
    }
}

#[derive(Debug)]
struct ArcMonitorResult {
    outcome: ArcMonitorResultOutcome,
    short_reason: String,
    rationale: String,
    risk_score: u8,
    risk_level: ArcMonitorRiskLevel,
    evidence_count: usize,
}

#[derive(Debug, PartialEq)]
struct ArcMonitorChatMessage {
    role: String,
    content: edgerun_json::Value,
}

impl ToJson for ArcMonitorChatMessage {
    fn to_json(&self) -> edgerun_json::Value {
        edgerun_json::Value::object(vec![
            ("role", self.role.to_json()),
            ("content", self.content.to_json()),
        ])
    }
}

#[derive(Debug, PartialEq)]
struct ArcMonitorPolicies {
    user: Option<String>,
    developer: Option<String>,
}

impl ToJson for ArcMonitorPolicies {
    fn to_json(&self) -> edgerun_json::Value {
        let mut object = edgerun_json::Map::new();
        object.push_field("user", self.user.to_json());
        object.push_field("developer", self.developer.to_json());
        edgerun_json::Value::Object(object)
    }
}

#[derive(Debug, PartialEq)]
struct ArcMonitorMetadata {
    codex_thread_id: String,
    codex_turn_id: String,
    conversation_id: Option<String>,
    protection_client_callsite: Option<String>,
}

impl ToJson for ArcMonitorMetadata {
    fn to_json(&self) -> edgerun_json::Value {
        let mut object = edgerun_json::Map::new();
        object.push_field("codex_thread_id", self.codex_thread_id.to_json());
        object.push_field("codex_turn_id", self.codex_turn_id.to_json());
        if let Some(conversation_id) = &self.conversation_id {
            object.push_field("conversation_id", conversation_id.to_json());
        }
        if let Some(callsite) = &self.protection_client_callsite {
            object.push_field("protection_client_callsite", callsite.to_json());
        }
        edgerun_json::Value::Object(object)
    }
}

#[derive(Debug)]
enum ArcMonitorResultOutcome {
    Ok,
    SteerModel,
    AskUser,
}

#[derive(Debug)]
enum ArcMonitorRiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

pub(crate) async fn monitor_action(
    sess: &Session,
    turn_context: &TurnContext,
    action: edgerun_json::Value,
    protection_client_callsite: &'static str,
) -> ArcMonitorOutcome {
    let auth = match turn_context.auth_manager.as_ref() {
        Some(auth_manager) => match auth_manager.auth().await {
            Some(auth) if auth.uses_codex_backend() => Some(auth),
            _ => None,
        },
        None => None,
    };
    let env_token = read_non_empty_env_var(CODEX_ARC_MONITOR_TOKEN);
    if env_token.is_none() && auth.is_none() {
        return ArcMonitorOutcome::Ok;
    }

    let url = read_non_empty_env_var(CODEX_ARC_MONITOR_ENDPOINT_OVERRIDE).unwrap_or_else(|| {
        format!(
            "{}/codex/safety/arc",
            turn_context.config.chatgpt_base_url.trim_end_matches('/')
        )
    });
    let action = match action {
        edgerun_json::Value::Object(action) => action,
        _ => {
            warn!("skipping safety monitor because action payload is not an object");
            return ArcMonitorOutcome::Ok;
        }
    };
    let body =
        build_arc_monitor_request(sess, turn_context, action, protection_client_callsite).await;
    let client = build_reqwest_client();
    let mut request = client.post(&url).timeout(ARC_MONITOR_TIMEOUT).json(&body);
    if let Some(token) = env_token {
        request = request.bearer_auth(token);
    } else if let Some(auth) = auth.as_ref() {
        request =
            request.headers(codex_model_provider::auth_provider_from_auth(auth).to_auth_headers());
    }

    let response = match request.send().await {
        Ok(response) => response,
        Err(err) => {
            warn!(error = %err, %url, "safety monitor request failed");
            return ArcMonitorOutcome::Ok;
        }
    };
    let status = response.status();
    if !status.is_success() {
        let response_text = response.text().await.unwrap_or_default();
        warn!(
            %status,
            %url,
            response_text,
            "safety monitor returned non-success status"
        );
        return ArcMonitorOutcome::Ok;
    }

    let response_text = match response.text().await {
        Ok(response_text) => response_text,
        Err(err) => {
            warn!(error = %err, %url, "failed to read safety monitor response");
            return ArcMonitorOutcome::Ok;
        }
    };
    let response = match parse_arc_monitor_response(&response_text) {
        Ok(response) => response,
        Err(err) => {
            warn!(error = %err, %url, "failed to parse safety monitor response");
            return ArcMonitorOutcome::Ok;
        }
    };
    tracing::debug!(
        risk_score = response.risk_score,
        risk_level = ?response.risk_level,
        evidence_count = response.evidence_count,
        "safety monitor completed"
    );

    let short_reason = response.short_reason.trim();
    let rationale = response.rationale.trim();
    match response.outcome {
        ArcMonitorResultOutcome::Ok => ArcMonitorOutcome::Ok,
        ArcMonitorResultOutcome::AskUser => {
            if !short_reason.is_empty() {
                ArcMonitorOutcome::AskUser(short_reason.to_string())
            } else if !rationale.is_empty() {
                ArcMonitorOutcome::AskUser(rationale.to_string())
            } else {
                ArcMonitorOutcome::AskUser(
                    "Additional confirmation is required before this tool call can continue."
                        .to_string(),
                )
            }
        }
        ArcMonitorResultOutcome::SteerModel => {
            if !rationale.is_empty() {
                ArcMonitorOutcome::SteerModel(rationale.to_string())
            } else if !short_reason.is_empty() {
                ArcMonitorOutcome::SteerModel(short_reason.to_string())
            } else {
                ArcMonitorOutcome::SteerModel(
                    "Tool call was cancelled because of safety risks.".to_string(),
                )
            }
        }
    }
}

fn parse_arc_monitor_response(
    payload: &str,
) -> Result<ArcMonitorResult, edgerun_json::JsonValueError> {
    let value: edgerun_json::Value = edgerun_json::from_str(payload)
        .map_err(|err| edgerun_json::JsonValueError::WrongType(err.to_string()))?;
    let outcome = match value.required_str("outcome")? {
        "ok" => ArcMonitorResultOutcome::Ok,
        "steer-model" => ArcMonitorResultOutcome::SteerModel,
        "ask-user" => ArcMonitorResultOutcome::AskUser,
        other => {
            return Err(edgerun_json::JsonValueError::WrongType(format!(
                "unknown ARC outcome `{other}`"
            )));
        }
    };
    let risk_level = match value.required_str("risk_level")? {
        "low" => ArcMonitorRiskLevel::Low,
        "medium" => ArcMonitorRiskLevel::Medium,
        "high" => ArcMonitorRiskLevel::High,
        "critical" => ArcMonitorRiskLevel::Critical,
        other => {
            return Err(edgerun_json::JsonValueError::WrongType(format!(
                "unknown ARC risk level `{other}`"
            )));
        }
    };
    let risk_score = value.required_u64("risk_score")?;
    let risk_score = u8::try_from(risk_score).map_err(|_| {
        edgerun_json::JsonValueError::WrongType("ARC risk_score exceeds u8 range".to_string())
    })?;
    Ok(ArcMonitorResult {
        outcome,
        short_reason: value.required_str("short_reason")?.to_string(),
        rationale: value.required_str("rationale")?.to_string(),
        risk_score,
        risk_level,
        evidence_count: value.required_array("evidence")?.len(),
    })
}

fn read_non_empty_env_var(key: &str) -> Option<String> {
    match env::var(key) {
        Ok(value) => {
            let value = value.trim();
            (!value.is_empty()).then(|| value.to_string())
        }
        Err(env::VarError::NotPresent) => None,
        Err(env::VarError::NotUnicode(_)) => {
            warn!(
                env_var = key,
                "ignoring non-unicode safety monitor env override"
            );
            None
        }
    }
}

async fn build_arc_monitor_request(
    sess: &Session,
    turn_context: &TurnContext,
    action: edgerun_json::Map<String, edgerun_json::Value>,
    protection_client_callsite: &'static str,
) -> ArcMonitorRequest {
    let history = sess.clone_history().await;
    let mut messages = build_arc_monitor_messages(history.raw_items());
    if messages.is_empty() {
        messages.push(build_arc_monitor_message(
            "user",
            edgerun_json::Value::String(
                "No prior conversation history is available for this ARC evaluation.".to_string(),
            ),
        ));
    }

    let conversation_id = sess.conversation_id.to_string();
    ArcMonitorRequest {
        metadata: ArcMonitorMetadata {
            codex_thread_id: conversation_id.clone(),
            codex_turn_id: turn_context.sub_id.clone(),
            conversation_id: Some(conversation_id),
            protection_client_callsite: Some(protection_client_callsite.to_string()),
        },
        messages: Some(messages),
        input: None,
        policies: Some(ArcMonitorPolicies {
            user: None,
            developer: None,
        }),
        action,
    }
}

fn build_arc_monitor_messages(items: &[ResponseItem]) -> Vec<ArcMonitorChatMessage> {
    let last_tool_call_index = items
        .iter()
        .enumerate()
        .rev()
        .find(|(_, item)| {
            matches!(
                item,
                ResponseItem::LocalShellCall { .. }
                    | ResponseItem::FunctionCall { .. }
                    | ResponseItem::CustomToolCall { .. }
                    | ResponseItem::WebSearchCall { .. }
            )
        })
        .map(|(index, _)| index);
    let last_encrypted_reasoning_index = items
        .iter()
        .enumerate()
        .rev()
        .find(|(_, item)| {
            matches!(
                item,
                ResponseItem::Reasoning {
                    encrypted_content: Some(encrypted_content),
                    ..
                } if !encrypted_content.trim().is_empty()
            )
        })
        .map(|(index, _)| index);

    items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            build_arc_monitor_message_item(
                item,
                index,
                last_tool_call_index,
                last_encrypted_reasoning_index,
            )
        })
        .collect()
}

fn build_arc_monitor_message_item(
    item: &ResponseItem,
    index: usize,
    last_tool_call_index: Option<usize>,
    last_encrypted_reasoning_index: Option<usize>,
) -> Option<ArcMonitorChatMessage> {
    match item {
        ResponseItem::Message { role, content, .. } if role == "user" => {
            if is_contextual_user_message_content(content) {
                None
            } else {
                content_items_to_text(content)
                    .map(|text| build_arc_monitor_text_message("user", "input_text", text))
            }
        }
        ResponseItem::Message {
            role,
            content,
            phase: Some(MessagePhase::FinalAnswer),
            ..
        } if role == "assistant" => content_items_to_text(content)
            .map(|text| build_arc_monitor_text_message("assistant", "output_text", text)),
        ResponseItem::Message { .. } => None,
        ResponseItem::Reasoning {
            encrypted_content: Some(encrypted_content),
            ..
        } if Some(index) == last_encrypted_reasoning_index
            && !encrypted_content.trim().is_empty() =>
        {
            Some(build_arc_monitor_message(
                "assistant",
                edgerun_json::json!([{
                    "type": "encrypted_reasoning",
                    "encrypted_content": encrypted_content,
                }]),
            ))
        }
        ResponseItem::Reasoning { .. } => None,
        ResponseItem::LocalShellCall { action, .. } if Some(index) == last_tool_call_index => {
            Some(build_arc_monitor_message(
                "assistant",
                edgerun_json::json!([{
                    "type": "tool_call",
                    "tool_name": "shell",
                    "action": action,
                }]),
            ))
        }
        ResponseItem::FunctionCall {
            name, arguments, ..
        } if Some(index) == last_tool_call_index => Some(build_arc_monitor_message(
            "assistant",
            edgerun_json::json!([{
                "type": "tool_call",
                "tool_name": name,
                "arguments": arguments,
            }]),
        )),
        ResponseItem::CustomToolCall { name, input, .. } if Some(index) == last_tool_call_index => {
            Some(build_arc_monitor_message(
                "assistant",
                edgerun_json::json!([{
                    "type": "tool_call",
                    "tool_name": name,
                    "input": input,
                }]),
            ))
        }
        ResponseItem::WebSearchCall { action, .. } if Some(index) == last_tool_call_index => {
            Some(build_arc_monitor_message(
                "assistant",
                edgerun_json::json!([{
                    "type": "tool_call",
                    "tool_name": "web_search",
                    "action": action,
                }]),
            ))
        }
        ResponseItem::LocalShellCall { .. }
        | ResponseItem::FunctionCall { .. }
        | ResponseItem::CustomToolCall { .. }
        | ResponseItem::ToolSearchCall { .. }
        | ResponseItem::WebSearchCall { .. }
        | ResponseItem::FunctionCallOutput { .. }
        | ResponseItem::CustomToolCallOutput { .. }
        | ResponseItem::ToolSearchOutput { .. }
        | ResponseItem::ImageGenerationCall { .. }
        | ResponseItem::Compaction { .. }
        | ResponseItem::ContextCompaction { .. }
        | ResponseItem::Other => None,
    }
}

fn build_arc_monitor_text_message(
    role: &str,
    part_type: &str,
    text: String,
) -> ArcMonitorChatMessage {
    build_arc_monitor_message(
        role,
        edgerun_json::json!([{
            "type": part_type,
            "text": text,
        }]),
    )
}

fn build_arc_monitor_message(role: &str, content: edgerun_json::Value) -> ArcMonitorChatMessage {
    ArcMonitorChatMessage {
        role: role.to_string(),
        content,
    }
}

#[cfg(test)]
#[path = "arc_monitor_tests.rs"]
mod tests;
