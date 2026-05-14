use codex_protocol::config_types::ApprovalsReviewer as CoreApprovalsReviewer;
use codex_protocol::config_types::SandboxMode as CoreSandboxMode;
use codex_protocol::protocol::AskForApproval as CoreAskForApproval;
use codex_protocol::protocol::CodexErrorInfo as CoreCodexErrorInfo;
use codex_protocol::protocol::GranularApprovalConfig as CoreGranularApprovalConfig;
use codex_protocol::protocol::NonSteerableTurnKind as CoreNonSteerableTurnKind;
use edgerun_json::FromJson;
use edgerun_json::JsonValue;
use edgerun_json::JsonValueError;
use edgerun_json::Map;
use edgerun_json::ToJson;
use edgerun_serde::Deserialize;
use edgerun_serde::Serialize;
use schemars::JsonSchema;
use schemars::r#gen::SchemaGenerator;
use schemars::schema::InstanceType;
use schemars::schema::Metadata;
use schemars::schema::Schema;
use schemars::schema::SchemaObject;
use ts_rs::TS;

// Macro to declare a camelCased API v2 enum mirroring a core enum which
// tends to use either snake_case or kebab-case.
macro_rules! v2_enum_from_core {
    (
        $(#[$enum_meta:meta])*
        pub enum $Name:ident from $Src:path {
            $( $(#[$variant_meta:meta])* $Variant:ident ),+ $(,)?
        }
    ) => {
        #[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
        $(#[$enum_meta])*
        #[serde(rename_all = "camelCase")]
        #[ts(export_to = "v2/")]
        pub enum $Name {
            $( $(#[$variant_meta])* $Variant ),+
        }

        impl $Name {
            pub fn to_core(self) -> $Src {
                match self { $( $Name::$Variant => <$Src>::$Variant ),+ }
            }
        }

        impl From<$Src> for $Name {
            fn from(value: $Src) -> Self {
                match value { $( <$Src>::$Variant => $Name::$Variant ),+ }
            }
        }
    };
}

pub(super) use v2_enum_from_core;

pub(super) const fn default_enabled() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum NonSteerableTurnKind {
    Review,
    Compact,
}

impl ToJson for NonSteerableTurnKind {
    fn to_json(&self) -> JsonValue {
        JsonValue::from(match self {
            Self::Review => "review",
            Self::Compact => "compact",
        })
    }
}

/// This translation layer make sure that we expose codex error code in camel case.
///
/// When an upstream HTTP status is available (for example, from the Responses API or a provider),
/// it is forwarded in `httpStatusCode` on the relevant `codexErrorInfo` variant.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum CodexErrorInfo {
    ContextWindowExceeded,
    UsageLimitExceeded,
    ServerOverloaded,
    CyberPolicy,
    HttpConnectionFailed {
        #[serde(rename = "httpStatusCode")]
        #[ts(rename = "httpStatusCode")]
        http_status_code: Option<u16>,
    },
    /// Failed to connect to the response SSE stream.
    ResponseStreamConnectionFailed {
        #[serde(rename = "httpStatusCode")]
        #[ts(rename = "httpStatusCode")]
        http_status_code: Option<u16>,
    },
    InternalServerError,
    Unauthorized,
    BadRequest,
    ThreadRollbackFailed,
    SandboxError,
    /// The response SSE stream disconnected in the middle of a turn before completion.
    ResponseStreamDisconnected {
        #[serde(rename = "httpStatusCode")]
        #[ts(rename = "httpStatusCode")]
        http_status_code: Option<u16>,
    },
    /// Reached the retry limit for responses.
    ResponseTooManyFailedAttempts {
        #[serde(rename = "httpStatusCode")]
        #[ts(rename = "httpStatusCode")]
        http_status_code: Option<u16>,
    },
    /// Returned when `turn/start` or `turn/steer` is submitted while the current active turn
    /// cannot accept same-turn steering, for example `/review` or manual `/compact`.
    ActiveTurnNotSteerable {
        #[serde(rename = "turnKind")]
        #[ts(rename = "turnKind")]
        turn_kind: NonSteerableTurnKind,
    },
    Other,
}

impl ToJson for CodexErrorInfo {
    fn to_json(&self) -> JsonValue {
        match self {
            Self::ContextWindowExceeded => JsonValue::from("contextWindowExceeded"),
            Self::UsageLimitExceeded => JsonValue::from("usageLimitExceeded"),
            Self::ServerOverloaded => JsonValue::from("serverOverloaded"),
            Self::CyberPolicy => JsonValue::from("cyberPolicy"),
            Self::InternalServerError => JsonValue::from("internalServerError"),
            Self::Unauthorized => JsonValue::from("unauthorized"),
            Self::BadRequest => JsonValue::from("badRequest"),
            Self::ThreadRollbackFailed => JsonValue::from("threadRollbackFailed"),
            Self::SandboxError => JsonValue::from("sandboxError"),
            Self::Other => JsonValue::from("other"),
            Self::HttpConnectionFailed { http_status_code } => {
                http_status_error("httpConnectionFailed", *http_status_code)
            }
            Self::ResponseStreamConnectionFailed { http_status_code } => {
                http_status_error("responseStreamConnectionFailed", *http_status_code)
            }
            Self::ResponseStreamDisconnected { http_status_code } => {
                http_status_error("responseStreamDisconnected", *http_status_code)
            }
            Self::ResponseTooManyFailedAttempts { http_status_code } => {
                http_status_error("responseTooManyFailedAttempts", *http_status_code)
            }
            Self::ActiveTurnNotSteerable { turn_kind } => {
                let mut payload = Map::with_capacity(1);
                payload.push_field("turnKind", turn_kind.to_json());
                let mut object = Map::with_capacity(1);
                object.push_field("activeTurnNotSteerable", JsonValue::Object(payload));
                JsonValue::Object(object)
            }
        }
    }
}

fn http_status_error(variant: &str, http_status_code: Option<u16>) -> JsonValue {
    let mut payload = Map::with_capacity(1);
    payload.push_field("httpStatusCode", http_status_code.to_json());
    let mut object = Map::with_capacity(1);
    object.push_field(variant, JsonValue::Object(payload));
    JsonValue::Object(object)
}

impl From<CoreCodexErrorInfo> for CodexErrorInfo {
    fn from(value: CoreCodexErrorInfo) -> Self {
        match value {
            CoreCodexErrorInfo::ContextWindowExceeded => CodexErrorInfo::ContextWindowExceeded,
            CoreCodexErrorInfo::UsageLimitExceeded => CodexErrorInfo::UsageLimitExceeded,
            CoreCodexErrorInfo::ServerOverloaded => CodexErrorInfo::ServerOverloaded,
            CoreCodexErrorInfo::CyberPolicy => CodexErrorInfo::CyberPolicy,
            CoreCodexErrorInfo::HttpConnectionFailed { http_status_code } => {
                CodexErrorInfo::HttpConnectionFailed { http_status_code }
            }
            CoreCodexErrorInfo::ResponseStreamConnectionFailed { http_status_code } => {
                CodexErrorInfo::ResponseStreamConnectionFailed { http_status_code }
            }
            CoreCodexErrorInfo::InternalServerError => CodexErrorInfo::InternalServerError,
            CoreCodexErrorInfo::Unauthorized => CodexErrorInfo::Unauthorized,
            CoreCodexErrorInfo::BadRequest => CodexErrorInfo::BadRequest,
            CoreCodexErrorInfo::ThreadRollbackFailed => CodexErrorInfo::ThreadRollbackFailed,
            CoreCodexErrorInfo::SandboxError => CodexErrorInfo::SandboxError,
            CoreCodexErrorInfo::ResponseStreamDisconnected { http_status_code } => {
                CodexErrorInfo::ResponseStreamDisconnected { http_status_code }
            }
            CoreCodexErrorInfo::ResponseTooManyFailedAttempts { http_status_code } => {
                CodexErrorInfo::ResponseTooManyFailedAttempts { http_status_code }
            }
            CoreCodexErrorInfo::ActiveTurnNotSteerable { turn_kind } => {
                CodexErrorInfo::ActiveTurnNotSteerable {
                    turn_kind: turn_kind.into(),
                }
            }
            CoreCodexErrorInfo::Other => CodexErrorInfo::Other,
        }
    }
}

impl From<CoreNonSteerableTurnKind> for NonSteerableTurnKind {
    fn from(value: CoreNonSteerableTurnKind) -> Self {
        match value {
            CoreNonSteerableTurnKind::Review => Self::Review,
            CoreNonSteerableTurnKind::Compact => Self::Compact,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case", export_to = "v2/")]
pub enum AskForApproval {
    #[serde(rename = "untrusted")]
    #[ts(rename = "untrusted")]
    UnlessTrusted,
    OnFailure,
    OnRequest,
    Granular {
        rules: bool,
        #[serde(default)]
        skill_approval: bool,
        mcp_elicitations: bool,
    },
    Never,
}

impl ToJson for AskForApproval {
    fn to_json(&self) -> JsonValue {
        match self {
            Self::UnlessTrusted => JsonValue::from("untrusted"),
            Self::OnFailure => JsonValue::from("on-failure"),
            Self::OnRequest => JsonValue::from("on-request"),
            Self::Never => JsonValue::from("never"),
            Self::Granular {
                rules,
                skill_approval,
                mcp_elicitations,
            } => {
                let mut granular = Map::with_capacity(3);
                granular.push_field("rules", *rules);
                granular.push_field("skill_approval", *skill_approval);
                granular.push_field("mcp_elicitations", *mcp_elicitations);

                let mut object = Map::with_capacity(1);
                object.push_field("granular", JsonValue::Object(granular));
                JsonValue::Object(object)
            }
        }
    }
}

impl FromJson for AskForApproval {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        if let Some(value) = value.as_str() {
            return match value {
                "untrusted" => Ok(Self::UnlessTrusted),
                "on-failure" => Ok(Self::OnFailure),
                "on-request" => Ok(Self::OnRequest),
                "never" => Ok(Self::Never),
                other => Err(JsonValueError::WrongType(format!(
                    "unknown approval policy `{other}`"
                ))),
            };
        }

        let mut object = value.into_object("AskForApproval")?;
        let mut granular: Map = object.take_required("granular")?;
        Ok(Self::Granular {
            rules: granular.take_required("rules")?,
            skill_approval: granular.take_optional("skill_approval")?.unwrap_or(false),
            mcp_elicitations: granular.take_required("mcp_elicitations")?,
        })
    }
}

impl AskForApproval {
    pub fn to_core(self) -> CoreAskForApproval {
        match self {
            AskForApproval::UnlessTrusted => CoreAskForApproval::UnlessTrusted,
            AskForApproval::OnFailure => CoreAskForApproval::OnFailure,
            AskForApproval::OnRequest => CoreAskForApproval::OnRequest,
            AskForApproval::Granular {
                rules,
                skill_approval,
                mcp_elicitations,
            } => CoreAskForApproval::Granular(CoreGranularApprovalConfig {
                rules,
                skill_approval,
                mcp_elicitations,
            }),
            AskForApproval::Never => CoreAskForApproval::Never,
        }
    }
}

impl From<CoreAskForApproval> for AskForApproval {
    fn from(value: CoreAskForApproval) -> Self {
        match value {
            CoreAskForApproval::UnlessTrusted => AskForApproval::UnlessTrusted,
            CoreAskForApproval::OnFailure => AskForApproval::OnFailure,
            CoreAskForApproval::OnRequest => AskForApproval::OnRequest,
            CoreAskForApproval::Granular(granular_config) => AskForApproval::Granular {
                rules: granular_config.rules,
                skill_approval: granular_config.skill_approval,
                mcp_elicitations: granular_config.mcp_elicitations,
            },
            CoreAskForApproval::Never => AskForApproval::Never,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, TS)]
#[ts(
    type = r#""user" | "auto_review" | "guardian_subagent""#,
    export_to = "v2/"
)]
/// Configures who approval requests are routed to for review. Examples
/// include sandbox escapes, blocked network access, MCP approval prompts, and
/// ARC escalations. Defaults to `user`. `auto_review` uses a carefully
/// prompted subagent to gather relevant context and apply a risk-based
/// decision framework before approving or denying the request.
pub enum ApprovalsReviewer {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "guardian_subagent", alias = "auto_review")]
    AutoReview,
}

impl JsonSchema for ApprovalsReviewer {
    fn schema_name() -> String {
        "ApprovalsReviewer".to_string()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        string_enum_schema_with_description(
            &["user", "auto_review", "guardian_subagent"],
            "Configures who approval requests are routed to for review. Examples include sandbox escapes, blocked network access, MCP approval prompts, and ARC escalations. Defaults to `user`. `auto_review` uses a carefully prompted subagent to gather relevant context and apply a risk-based decision framework before approving or denying the request. The legacy value `guardian_subagent` is accepted for compatibility.",
        )
    }
}

impl ToJson for ApprovalsReviewer {
    fn to_json(&self) -> JsonValue {
        JsonValue::String(
            match self {
                Self::User => "user",
                Self::AutoReview => "guardian_subagent",
            }
            .to_string(),
        )
    }
}

impl FromJson for ApprovalsReviewer {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        match String::from_json(value)?.as_str() {
            "user" => Ok(Self::User),
            "auto_review" | "guardian_subagent" => Ok(Self::AutoReview),
            other => Err(JsonValueError::WrongType(format!(
                "unknown approvals reviewer `{other}`"
            ))),
        }
    }
}

fn string_enum_schema_with_description(values: &[&str], description: &str) -> Schema {
    let mut schema = SchemaObject {
        instance_type: Some(InstanceType::String.into()),
        metadata: Some(Box::new(Metadata {
            description: Some(description.to_string()),
            ..Default::default()
        })),
        ..Default::default()
    };
    schema.enum_values = Some(
        values
            .iter()
            .map(|value| schemars::schema::JsonValue::String((*value).to_string()))
            .collect(),
    );
    Schema::Object(schema)
}

impl ApprovalsReviewer {
    pub fn to_core(self) -> CoreApprovalsReviewer {
        match self {
            ApprovalsReviewer::User => CoreApprovalsReviewer::User,
            ApprovalsReviewer::AutoReview => CoreApprovalsReviewer::AutoReview,
        }
    }
}

impl From<CoreApprovalsReviewer> for ApprovalsReviewer {
    fn from(value: CoreApprovalsReviewer) -> Self {
        match value {
            CoreApprovalsReviewer::User => ApprovalsReviewer::User,
            CoreApprovalsReviewer::AutoReview => ApprovalsReviewer::AutoReview,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case", export_to = "v2/")]
pub enum SandboxMode {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

impl SandboxMode {
    pub fn to_core(self) -> CoreSandboxMode {
        match self {
            SandboxMode::ReadOnly => CoreSandboxMode::ReadOnly,
            SandboxMode::WorkspaceWrite => CoreSandboxMode::WorkspaceWrite,
            SandboxMode::DangerFullAccess => CoreSandboxMode::DangerFullAccess,
        }
    }
}

impl From<CoreSandboxMode> for SandboxMode {
    fn from(value: CoreSandboxMode) -> Self {
        match value {
            CoreSandboxMode::ReadOnly => SandboxMode::ReadOnly,
            CoreSandboxMode::WorkspaceWrite => SandboxMode::WorkspaceWrite,
            CoreSandboxMode::DangerFullAccess => SandboxMode::DangerFullAccess,
        }
    }
}

impl ToJson for SandboxMode {
    fn to_json(&self) -> JsonValue {
        JsonValue::from(match self {
            SandboxMode::ReadOnly => "read-only",
            SandboxMode::WorkspaceWrite => "workspace-write",
            SandboxMode::DangerFullAccess => "danger-full-access",
        })
    }
}

impl FromJson for SandboxMode {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        match String::from_json(value)?.as_str() {
            "read-only" => Ok(SandboxMode::ReadOnly),
            "workspace-write" => Ok(SandboxMode::WorkspaceWrite),
            "danger-full-access" => Ok(SandboxMode::DangerFullAccess),
            other => Err(JsonValueError::WrongType(format!(
                "unknown sandbox mode `{other}`"
            ))),
        }
    }
}
