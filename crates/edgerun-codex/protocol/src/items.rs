use crate::compat::absolute_path::AbsolutePathBuf;
use crate::mcp::CallToolResult;
use crate::memory_citation::MemoryCitation;
use crate::models::ContentItem;
use crate::models::MessagePhase;
use crate::models::ResponseItem;
use crate::models::WebSearchAction;
use crate::protocol::AgentMessageEvent;
use crate::protocol::AgentReasoningEvent;
use crate::protocol::AgentReasoningRawContentEvent;
use crate::protocol::ContextCompactedEvent;
use crate::protocol::EventMsg;
use crate::protocol::FileChange;
use crate::protocol::ImageGenerationEndEvent;
use crate::protocol::McpInvocation;
use crate::protocol::McpToolCallBeginEvent;
use crate::protocol::McpToolCallEndEvent;
use crate::protocol::PatchApplyBeginEvent;
use crate::protocol::PatchApplyEndEvent;
use crate::protocol::PatchApplyStatus;
use crate::protocol::UserMessageEvent;
use crate::protocol::ViewImageToolCallEvent;
use crate::protocol::WebSearchEndEvent;
use crate::user_input::ByteRange;
use crate::user_input::TextElement;
use crate::user_input::UserInput;
use schemars::JsonSchema;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(tag = "type")]
pub enum TurnItem {
    UserMessage(UserMessageItem),
    HookPrompt(HookPromptItem),
    AgentMessage(AgentMessageItem),
    Plan(PlanItem),
    Reasoning(ReasoningItem),
    WebSearch(WebSearchItem),
    ImageView(ImageViewItem),
    ImageGeneration(ImageGenerationItem),
    FileChange(FileChangeItem),
    McpToolCall(McpToolCallItem),
    ContextCompaction(ContextCompactionItem),
}

#[derive(Debug, Clone, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
pub struct UserMessageItem {
    pub id: String,
    pub content: Vec<UserInput>,
}

#[derive(Debug, Clone, JsonSchema, PartialEq, Eq, edgerun_json::ToJson, edgerun_json::FromJson)]
pub struct HookPromptItem {
    pub id: String,
    pub fragments: Vec<HookPromptFragment>,
}

#[derive(Debug, Clone, JsonSchema, PartialEq, Eq, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct HookPromptFragment {
    pub text: String,
    pub hook_run_id: String,
}

#[derive(Debug, Clone, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(tag = "type")]
pub enum AgentMessageContent {
    Text { text: String },
}

#[derive(Debug, Clone, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
/// Assistant-authored message payload used in turn-item streams.
///
/// `phase` is optional because not all providers/models emit it. Consumers
/// should use it when present, but retain legacy completion semantics when it
/// is `None`.
pub struct AgentMessageItem {
    pub id: String,
    pub content: Vec<AgentMessageContent>,
    /// Optional phase metadata carried through from `ResponseItem::Message`.
    ///
    /// This is currently used by TUI rendering to distinguish mid-turn
    /// commentary from a final answer and avoid status-indicator jitter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<MessagePhase>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_citation: Option<MemoryCitation>,
}

#[derive(Debug, Clone, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
pub struct PlanItem {
    pub id: String,
    pub text: String,
}

#[derive(Debug, Clone, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
pub struct ReasoningItem {
    pub id: String,
    pub summary_text: Vec<String>,
    #[serde(default)]
    pub raw_content: Vec<String>,
}

#[derive(Debug, Clone, JsonSchema, PartialEq, edgerun_json::ToJson, edgerun_json::FromJson)]
pub struct WebSearchItem {
    pub id: String,
    pub query: String,
    pub action: WebSearchAction,
}

#[derive(Debug, Clone, JsonSchema, PartialEq, edgerun_json::ToJson, edgerun_json::FromJson)]
pub struct ImageViewItem {
    pub id: String,
    pub path: AbsolutePathBuf,
}

#[derive(Debug, Clone, JsonSchema, PartialEq, edgerun_json::ToJson, edgerun_json::FromJson)]
pub struct ImageGenerationItem {
    pub id: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revised_prompt: Option<String>,
    pub result: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saved_path: Option<AbsolutePathBuf>,
}

#[derive(Debug, Clone, JsonSchema, PartialEq, edgerun_json::ToJson, edgerun_json::FromJson)]
pub struct FileChangeItem {
    pub id: String,
    pub changes: HashMap<PathBuf, FileChange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<PatchApplyStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_approved: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
}

#[derive(Debug, Clone, JsonSchema, PartialEq, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct McpToolCallItem {
    pub id: String,
    pub server: String,
    pub tool: String,
    pub arguments: edgerun_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_app_resource_uri: Option<String>,
    pub status: McpToolCallStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<CallToolResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<McpToolCallError>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<Duration>,
}

#[derive(Debug, Clone, Copy, JsonSchema, PartialEq, Eq, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub enum McpToolCallStatus {
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, JsonSchema, PartialEq, Eq, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct McpToolCallError {
    pub message: String,
}

#[derive(Debug, Clone, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
pub struct ContextCompactionItem {
    pub id: String,
}

impl ContextCompactionItem {
    pub fn new() -> Self {
        Self {
            id: crate::local_uuid::Uuid::new_v4().to_string(),
        }
    }

    pub fn as_legacy_event(&self) -> EventMsg {
        EventMsg::ContextCompacted(ContextCompactedEvent {})
    }
}

impl Default for ContextCompactionItem {
    fn default() -> Self {
        Self::new()
    }
}

impl UserMessageItem {
    pub fn new(content: &[UserInput]) -> Self {
        Self {
            id: crate::local_uuid::Uuid::new_v4().to_string(),
            content: content.to_vec(),
        }
    }

    pub fn as_legacy_event(&self) -> EventMsg {
        // Legacy user-message events flatten only text inputs into `message` and
        // rebase text element ranges onto that concatenated text.
        EventMsg::UserMessage(UserMessageEvent {
            message: self.message(),
            images: Some(self.image_urls()),
            local_images: self.local_image_paths(),
            text_elements: self.text_elements(),
        })
    }

    pub fn message(&self) -> String {
        self.content
            .iter()
            .map(|c| match c {
                UserInput::Text { text, .. } => text.clone(),
                _ => String::new(),
            })
            .collect::<Vec<String>>()
            .join("")
    }

    pub fn text_elements(&self) -> Vec<TextElement> {
        let mut out = Vec::new();
        let mut offset = 0usize;
        for input in &self.content {
            if let UserInput::Text {
                text,
                text_elements,
            } = input
            {
                // Text element ranges are relative to each text chunk; offset them so they align
                // with the concatenated message returned by `message()`.
                for elem in text_elements {
                    let byte_range = ByteRange {
                        start: offset + elem.byte_range.start,
                        end: offset + elem.byte_range.end,
                    };
                    out.push(TextElement::new(
                        byte_range,
                        elem.placeholder(text).map(str::to_string),
                    ));
                }
                offset += text.len();
            }
        }
        out
    }

    pub fn image_urls(&self) -> Vec<String> {
        self.content
            .iter()
            .filter_map(|c| match c {
                UserInput::Image { image_url } => Some(image_url.clone()),
                _ => None,
            })
            .collect()
    }

    pub fn local_image_paths(&self) -> Vec<std::path::PathBuf> {
        self.content
            .iter()
            .filter_map(|c| match c {
                UserInput::LocalImage { path } => Some(path.clone()),
                _ => None,
            })
            .collect()
    }
}

impl HookPromptItem {
    pub fn from_fragments(id: Option<&String>, fragments: Vec<HookPromptFragment>) -> Self {
        Self {
            id: id
                .cloned()
                .unwrap_or_else(|| crate::local_uuid::Uuid::new_v4().to_string()),
            fragments,
        }
    }
}

impl HookPromptFragment {
    pub fn from_single_hook(text: impl Into<String>, hook_run_id: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            hook_run_id: hook_run_id.into(),
        }
    }
}

pub fn build_hook_prompt_message(fragments: &[HookPromptFragment]) -> Option<ResponseItem> {
    let content = fragments
        .iter()
        .filter(|fragment| !fragment.hook_run_id.trim().is_empty())
        .filter_map(|fragment| {
            serialize_hook_prompt_fragment(&fragment.text, &fragment.hook_run_id)
                .map(|text| ContentItem::InputText { text })
        })
        .collect::<Vec<_>>();

    if content.is_empty() {
        return None;
    }

    Some(ResponseItem::Message {
        id: Some(crate::local_uuid::Uuid::new_v4().to_string()),
        role: "user".to_string(),
        content,
        phase: None,
    })
}

pub fn parse_hook_prompt_message(
    id: Option<&String>,
    content: &[ContentItem],
) -> Option<HookPromptItem> {
    let fragments = content
        .iter()
        .map(|content_item| {
            let ContentItem::InputText { text } = content_item else {
                return None;
            };
            parse_hook_prompt_fragment(text)
        })
        .collect::<Option<Vec<_>>>()?;

    if fragments.is_empty() {
        return None;
    }

    Some(HookPromptItem::from_fragments(id, fragments))
}

pub fn parse_hook_prompt_fragment(text: &str) -> Option<HookPromptFragment> {
    let trimmed = text.trim();
    let (text, hook_run_id) = parse_hook_prompt_xml(trimmed)?;
    if hook_run_id.trim().is_empty() {
        return None;
    }

    Some(HookPromptFragment { text, hook_run_id })
}

fn serialize_hook_prompt_fragment(text: &str, hook_run_id: &str) -> Option<String> {
    if hook_run_id.trim().is_empty() {
        return None;
    }
    Some(format!(
        r#"<hook_prompt hook_run_id="{}">{}</hook_prompt>"#,
        escape_xml_attr(hook_run_id),
        escape_xml_text(text)
    ))
}

fn parse_hook_prompt_xml(input: &str) -> Option<(String, String)> {
    const OPEN: &str = "<hook_prompt";
    const CLOSE: &str = "</hook_prompt>";

    let rest = input.strip_prefix(OPEN)?;
    let tag_end = rest.find('>')?;
    let attrs = &rest[..tag_end];
    let body = &rest[tag_end + 1..];
    let body = body.strip_suffix(CLOSE)?;
    let hook_run_id = parse_xml_attr(attrs, "hook_run_id")?;
    Some((unescape_xml(body)?, unescape_xml(&hook_run_id)?))
}

fn parse_xml_attr(attrs: &str, name: &str) -> Option<String> {
    let key = format!("{name}=");
    let start = attrs.find(&key)? + key.len();
    let quote = attrs[start..].chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let value_start = start + quote.len_utf8();
    let value_end = attrs[value_start..].find(quote)? + value_start;
    Some(attrs[value_start..value_end].to_string())
}

fn escape_xml_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_xml_attr(value: &str) -> String {
    escape_xml_text(value)
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn unescape_xml(value: &str) -> Option<String> {
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(index) = rest.find('&') {
        out.push_str(&rest[..index]);
        rest = &rest[index + 1..];
        let semi = rest.find(';')?;
        let entity = &rest[..semi];
        match entity {
            "amp" => out.push('&'),
            "lt" => out.push('<'),
            "gt" => out.push('>'),
            "quot" => out.push('"'),
            "apos" => out.push('\''),
            _ => return None,
        }
        rest = &rest[semi + 1..];
    }
    out.push_str(rest);
    Some(out)
}

impl AgentMessageItem {
    pub fn new(content: &[AgentMessageContent]) -> Self {
        Self {
            id: crate::local_uuid::Uuid::new_v4().to_string(),
            content: content.to_vec(),
            phase: None,
            memory_citation: None,
        }
    }

    pub fn as_legacy_events(&self) -> Vec<EventMsg> {
        self.content
            .iter()
            .map(|c| match c {
                AgentMessageContent::Text { text } => EventMsg::AgentMessage(AgentMessageEvent {
                    message: text.clone(),
                    phase: self.phase.clone(),
                    memory_citation: self.memory_citation.clone(),
                }),
            })
            .collect()
    }
}

impl ReasoningItem {
    pub fn as_legacy_events(&self, show_raw_agent_reasoning: bool) -> Vec<EventMsg> {
        let mut events = Vec::new();
        for summary in &self.summary_text {
            events.push(EventMsg::AgentReasoning(AgentReasoningEvent {
                text: summary.clone(),
            }));
        }

        if show_raw_agent_reasoning {
            for entry in &self.raw_content {
                events.push(EventMsg::AgentReasoningRawContent(
                    AgentReasoningRawContentEvent {
                        text: entry.clone(),
                    },
                ));
            }
        }

        events
    }
}

impl WebSearchItem {
    pub fn as_legacy_event(&self) -> EventMsg {
        EventMsg::WebSearchEnd(WebSearchEndEvent {
            call_id: self.id.clone(),
            query: self.query.clone(),
            action: self.action.clone(),
        })
    }
}

impl ImageGenerationItem {
    pub fn as_legacy_event(&self) -> EventMsg {
        EventMsg::ImageGenerationEnd(ImageGenerationEndEvent {
            call_id: self.id.clone(),
            status: self.status.clone(),
            revised_prompt: self.revised_prompt.clone(),
            result: self.result.clone(),
            saved_path: self.saved_path.clone(),
        })
    }
}

impl FileChangeItem {
    pub fn as_legacy_begin_event(&self, turn_id: String) -> EventMsg {
        EventMsg::PatchApplyBegin(PatchApplyBeginEvent {
            call_id: self.id.clone(),
            turn_id,
            auto_approved: self.auto_approved.unwrap_or(false),
            changes: self.changes.clone(),
        })
    }

    pub fn as_legacy_end_event(&self, turn_id: String) -> Option<EventMsg> {
        let status = self.status.clone()?;
        Some(EventMsg::PatchApplyEnd(PatchApplyEndEvent {
            call_id: self.id.clone(),
            turn_id,
            stdout: self.stdout.clone().unwrap_or_default(),
            stderr: self.stderr.clone().unwrap_or_default(),
            success: status == PatchApplyStatus::Completed,
            changes: self.changes.clone(),
            status,
        }))
    }
}

impl McpToolCallItem {
    pub fn as_legacy_begin_event(&self) -> EventMsg {
        EventMsg::McpToolCallBegin(McpToolCallBeginEvent {
            call_id: self.id.clone(),
            invocation: McpInvocation {
                server: self.server.clone(),
                tool: self.tool.clone(),
                arguments: (!self.arguments.is_null()).then(|| self.arguments.clone()),
            },
            mcp_app_resource_uri: self.mcp_app_resource_uri.clone(),
        })
    }

    pub fn as_legacy_end_event(&self) -> Option<EventMsg> {
        let result = match (&self.result, &self.error) {
            (Some(result), _) => Ok(result.clone()),
            (None, Some(error)) => Err(error.message.clone()),
            (None, None) => return None,
        };

        Some(EventMsg::McpToolCallEnd(McpToolCallEndEvent {
            call_id: self.id.clone(),
            invocation: McpInvocation {
                server: self.server.clone(),
                tool: self.tool.clone(),
                arguments: (!self.arguments.is_null()).then(|| self.arguments.clone()),
            },
            mcp_app_resource_uri: self.mcp_app_resource_uri.clone(),
            duration: self.duration?,
            result,
        }))
    }
}

impl TurnItem {
    pub fn id(&self) -> String {
        match self {
            TurnItem::UserMessage(item) => item.id.clone(),
            TurnItem::HookPrompt(item) => item.id.clone(),
            TurnItem::AgentMessage(item) => item.id.clone(),
            TurnItem::Plan(item) => item.id.clone(),
            TurnItem::Reasoning(item) => item.id.clone(),
            TurnItem::WebSearch(item) => item.id.clone(),
            TurnItem::ImageView(item) => item.id.clone(),
            TurnItem::ImageGeneration(item) => item.id.clone(),
            TurnItem::FileChange(item) => item.id.clone(),
            TurnItem::McpToolCall(item) => item.id.clone(),
            TurnItem::ContextCompaction(item) => item.id.clone(),
        }
    }

    pub fn as_legacy_events(&self, show_raw_agent_reasoning: bool) -> Vec<EventMsg> {
        match self {
            TurnItem::UserMessage(item) => vec![item.as_legacy_event()],
            TurnItem::HookPrompt(_) => Vec::new(),
            TurnItem::AgentMessage(item) => item.as_legacy_events(),
            TurnItem::Plan(_) => Vec::new(),
            TurnItem::WebSearch(item) => vec![item.as_legacy_event()],
            TurnItem::ImageView(item) => {
                vec![EventMsg::ViewImageToolCall(ViewImageToolCallEvent {
                    call_id: item.id.clone(),
                    path: item.path.clone(),
                })]
            }
            TurnItem::ImageGeneration(item) => vec![item.as_legacy_event()],
            TurnItem::FileChange(item) => item
                .as_legacy_end_event(String::new())
                .into_iter()
                .collect(),
            TurnItem::McpToolCall(item) => item.as_legacy_end_event().into_iter().collect(),
            TurnItem::Reasoning(item) => item.as_legacy_events(show_raw_agent_reasoning),
            TurnItem::ContextCompaction(item) => vec![item.as_legacy_event()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn hook_prompt_roundtrips_multiple_fragments() {
        let original = vec![
            HookPromptFragment::from_single_hook("Retry with care & joy.", "hook-run-1"),
            HookPromptFragment::from_single_hook("Then summarize cleanly.", "hook-run-2"),
        ];
        let message = build_hook_prompt_message(&original).expect("hook prompt");

        let ResponseItem::Message { content, .. } = message else {
            panic!("expected hook prompt message");
        };

        let parsed = parse_hook_prompt_message(/*id*/ None, &content).expect("parsed hook prompt");
        assert_eq!(parsed.fragments, original);
    }

    #[test]
    fn hook_prompt_parses_legacy_single_hook_run_id() {
        let parsed = parse_hook_prompt_fragment(
            r#"<hook_prompt hook_run_id="hook-run-1">Retry with tests.</hook_prompt>"#,
        )
        .expect("legacy hook prompt");

        assert_eq!(
            parsed,
            HookPromptFragment {
                text: "Retry with tests.".to_string(),
                hook_run_id: "hook-run-1".to_string(),
            }
        );
    }
}
