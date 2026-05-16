use crate::endpoint::realtime_websocket::protocol_v1::parse_realtime_event_v1;
use crate::endpoint::realtime_websocket::protocol_v2::parse_realtime_event_v2;
pub use codex_protocol::protocol::RealtimeAudioFrame;
pub use codex_protocol::protocol::RealtimeEvent;
pub use codex_protocol::protocol::RealtimeOutputModality;
pub use codex_protocol::protocol::RealtimeTranscriptEntry;
pub use codex_protocol::protocol::RealtimeVoice;
use edgerun_json::Map;
use edgerun_json::ToJson;
use edgerun_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RealtimeEventParser {
    V1,
    RealtimeV2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RealtimeSessionMode {
    Conversational,
    Transcription,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealtimeSessionConfig {
    pub instructions: String,
    pub model: Option<String>,
    pub session_id: Option<String>,
    pub event_parser: RealtimeEventParser,
    pub session_mode: RealtimeSessionMode,
    pub output_modality: RealtimeOutputModality,
    pub voice: RealtimeVoice,
}

#[derive(Debug, Clone)]
pub(super) enum RealtimeOutboundMessage {
    InputAudioBufferAppend {
        audio: String,
    },
    ConversationHandoffAppend {
        handoff_id: String,
        output_text: String,
    },
    ResponseCreate,
    SessionUpdate {
        session: SessionUpdateSession,
    },
    ConversationItemCreate {
        item: ConversationItemPayload,
    },
}

impl ToJson for RealtimeOutboundMessage {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        match self {
            Self::InputAudioBufferAppend { audio } => {
                object.push_field("type", "input_audio_buffer.append");
                object.push_field("audio", audio.as_str());
            }
            Self::ConversationHandoffAppend {
                handoff_id,
                output_text,
            } => {
                object.push_field("type", "conversation.handoff.append");
                object.push_field("handoff_id", handoff_id.as_str());
                object.push_field("output_text", output_text.as_str());
            }
            Self::ResponseCreate => {
                object.push_field("type", "response.create");
            }
            Self::SessionUpdate { session } => {
                object.push_field("type", "session.update");
                object.push_field("session", session.to_json());
            }
            Self::ConversationItemCreate { item } => {
                object.push_field("type", "conversation.item.create");
                object.push_field("item", item.to_json());
            }
        }
        object.into()
    }
}

#[derive(Debug, Clone)]
pub(super) struct SessionUpdateSession {
    pub(super) id: Option<String>,
    pub(super) r#type: SessionType,
    pub(super) model: Option<String>,
    pub(super) instructions: Option<String>,
    pub(super) output_modalities: Option<Vec<String>>,
    pub(super) audio: SessionAudio,
    pub(super) tools: Option<Vec<SessionFunctionTool>>,
    pub(super) tool_choice: Option<String>,
}

impl ToJson for SessionUpdateSession {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.push_opt_field("id", self.id.as_ref().map(ToJson::to_json));
        object.push_field("type", session_type_json(self.r#type));
        object.push_opt_field("model", self.model.as_ref().map(ToJson::to_json));
        object.push_opt_field(
            "instructions",
            self.instructions.as_ref().map(ToJson::to_json),
        );
        object.push_opt_field(
            "output_modalities",
            self.output_modalities.as_ref().map(ToJson::to_json),
        );
        object.push_field("audio", self.audio.to_json());
        object.push_opt_field("tools", self.tools.as_ref().map(ToJson::to_json));
        object.push_opt_field(
            "tool_choice",
            self.tool_choice.as_ref().map(ToJson::to_json),
        );
        object.into()
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum SessionType {
    Quicksilver,
    Realtime,
    Transcription,
}

#[derive(Debug, Clone)]
pub(super) struct SessionAudio {
    pub(super) input: SessionAudioInput,
    pub(super) output: Option<SessionAudioOutput>,
}

impl ToJson for SessionAudio {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.push_field("input", self.input.to_json());
        object.push_opt_field("output", self.output.as_ref().map(ToJson::to_json));
        object.into()
    }
}

#[derive(Debug, Clone)]
pub(super) struct SessionAudioInput {
    pub(super) format: SessionAudioFormat,
    pub(super) noise_reduction: Option<SessionNoiseReduction>,
    pub(super) transcription: Option<SessionInputAudioTranscription>,
    pub(super) turn_detection: Option<SessionTurnDetection>,
}

impl ToJson for SessionAudioInput {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.push_field("format", self.format.to_json());
        object.push_opt_field(
            "noise_reduction",
            self.noise_reduction.as_ref().map(ToJson::to_json),
        );
        object.push_opt_field(
            "transcription",
            self.transcription.as_ref().map(ToJson::to_json),
        );
        object.push_opt_field(
            "turn_detection",
            self.turn_detection.as_ref().map(ToJson::to_json),
        );
        object.into()
    }
}

#[derive(Debug, Clone)]
pub(super) struct SessionInputAudioTranscription {
    pub(super) model: String,
}

impl ToJson for SessionInputAudioTranscription {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.push_field("model", self.model.as_str());
        object.into()
    }
}

#[derive(Debug, Clone)]
pub(super) struct SessionAudioFormat {
    pub(super) r#type: AudioFormatType,
    pub(super) rate: u32,
}

impl ToJson for SessionAudioFormat {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.push_field("type", audio_format_type_json(self.r#type));
        object.push_field("rate", self.rate);
        object.into()
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum AudioFormatType {
    AudioPcm,
}

#[derive(Debug, Clone)]
pub(super) struct SessionAudioOutput {
    pub(super) format: Option<SessionAudioOutputFormat>,
    pub(super) voice: RealtimeVoice,
}

impl ToJson for SessionAudioOutput {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.push_opt_field("format", self.format.as_ref().map(ToJson::to_json));
        object.push_field("voice", self.voice.wire_name());
        object.into()
    }
}

#[derive(Debug, Clone)]
pub(super) struct SessionNoiseReduction {
    pub(super) r#type: NoiseReductionType,
}

impl ToJson for SessionNoiseReduction {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.push_field("type", noise_reduction_type_json(self.r#type));
        object.into()
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum NoiseReductionType {
    NearField,
}

#[derive(Debug, Clone)]
pub(super) struct SessionTurnDetection {
    pub(super) r#type: TurnDetectionType,
    pub(super) interrupt_response: bool,
    pub(super) create_response: bool,
    pub(super) silence_duration_ms: u32,
}

impl ToJson for SessionTurnDetection {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.push_field("type", turn_detection_type_json(self.r#type));
        object.push_field("interrupt_response", self.interrupt_response);
        object.push_field("create_response", self.create_response);
        object.push_field("silence_duration_ms", self.silence_duration_ms);
        object.into()
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum TurnDetectionType {
    ServerVad,
}

#[derive(Debug, Clone)]
pub(super) struct SessionAudioOutputFormat {
    pub(super) r#type: AudioFormatType,
    pub(super) rate: u32,
}

impl ToJson for SessionAudioOutputFormat {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.push_field("type", audio_format_type_json(self.r#type));
        object.push_field("rate", self.rate);
        object.into()
    }
}

#[derive(Debug, Clone)]
pub(super) struct ConversationMessageItem {
    pub(super) r#type: ConversationItemType,
    pub(super) role: ConversationRole,
    pub(super) content: Vec<ConversationItemContent>,
}

impl ToJson for ConversationMessageItem {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.push_field("type", conversation_item_type_json(self.r#type));
        object.push_field("role", conversation_role_json(self.role));
        object.push_field("content", self.content.to_json());
        object.into()
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum ConversationItemType {
    Message,
    FunctionCallOutput,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum ConversationRole {
    User,
}

#[derive(Debug, Clone)]
pub(super) enum ConversationItemPayload {
    Message(ConversationMessageItem),
    FunctionCallOutput(ConversationFunctionCallOutputItem),
}

impl ToJson for ConversationItemPayload {
    fn to_json(&self) -> Value {
        match self {
            Self::Message(item) => item.to_json(),
            Self::FunctionCallOutput(item) => item.to_json(),
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct ConversationFunctionCallOutputItem {
    pub(super) r#type: ConversationItemType,
    pub(super) call_id: String,
    pub(super) output: String,
}

impl ToJson for ConversationFunctionCallOutputItem {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.push_field("type", conversation_item_type_json(self.r#type));
        object.push_field("call_id", self.call_id.as_str());
        object.push_field("output", self.output.as_str());
        object.into()
    }
}

#[derive(Debug, Clone)]
pub(super) struct ConversationItemContent {
    pub(super) r#type: ConversationContentType,
    pub(super) text: String,
}

impl ToJson for ConversationItemContent {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.push_field("type", conversation_content_type_json(self.r#type));
        object.push_field("text", self.text.as_str());
        object.into()
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum ConversationContentType {
    Text,
    InputText,
}

#[derive(Debug, Clone)]
pub(super) struct SessionFunctionTool {
    pub(super) r#type: SessionToolType,
    pub(super) name: String,
    pub(super) description: String,
    pub(super) parameters: Value,
}

impl ToJson for SessionFunctionTool {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.push_field("type", session_tool_type_json(self.r#type));
        object.push_field("name", self.name.as_str());
        object.push_field("description", self.description.as_str());
        object.push_field("parameters", self.parameters.clone());
        object.into()
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum SessionToolType {
    Function,
}

fn session_type_json(value: SessionType) -> Value {
    Value::String(
        match value {
            SessionType::Quicksilver => "quicksilver",
            SessionType::Realtime => "realtime",
            SessionType::Transcription => "transcription",
        }
        .to_string(),
    )
}

fn audio_format_type_json(value: AudioFormatType) -> Value {
    match value {
        AudioFormatType::AudioPcm => Value::String("audio/pcm".to_string()),
    }
}

fn noise_reduction_type_json(value: NoiseReductionType) -> Value {
    match value {
        NoiseReductionType::NearField => Value::String("near_field".to_string()),
    }
}

fn turn_detection_type_json(value: TurnDetectionType) -> Value {
    match value {
        TurnDetectionType::ServerVad => Value::String("server_vad".to_string()),
    }
}

fn conversation_item_type_json(value: ConversationItemType) -> Value {
    Value::String(
        match value {
            ConversationItemType::Message => "message",
            ConversationItemType::FunctionCallOutput => "function_call_output",
        }
        .to_string(),
    )
}

fn conversation_role_json(value: ConversationRole) -> Value {
    match value {
        ConversationRole::User => Value::String("user".to_string()),
    }
}

fn conversation_content_type_json(value: ConversationContentType) -> Value {
    Value::String(
        match value {
            ConversationContentType::Text => "text",
            ConversationContentType::InputText => "input_text",
        }
        .to_string(),
    )
}

fn session_tool_type_json(value: SessionToolType) -> Value {
    match value {
        SessionToolType::Function => Value::String("function".to_string()),
    }
}

pub(super) fn parse_realtime_event(
    payload: &str,
    event_parser: RealtimeEventParser,
) -> Option<RealtimeEvent> {
    match event_parser {
        RealtimeEventParser::V1 => parse_realtime_event_v1(payload),
        RealtimeEventParser::RealtimeV2 => parse_realtime_event_v2(payload),
    }
}
