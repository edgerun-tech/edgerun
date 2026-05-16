use codex_protocol::config_types::ApprovalsReviewer as CoreApprovalsReviewer;
use codex_protocol::protocol::CodexErrorInfo as CoreCodexErrorInfo;
use codex_protocol::protocol::NonSteerableTurnKind as CoreNonSteerableTurnKind;
use edgerun_json::FromJson;
use edgerun_json::JsonValue;
use edgerun_json::JsonValueError;
use edgerun_json::Map;
use edgerun_json::ToJson;
use schemars::JsonSchema;
use schemars::r#gen::SchemaGenerator;
use schemars::schema::InstanceType;
use schemars::schema::Metadata;
use schemars::schema::Schema;
use schemars::schema::SchemaObject;

// Macro to declare a camelCased API v2 enum mirroring a core enum which
// tends to use either snake_case or kebab-case.
macro_rules! v2_enum_from_core {
    (
        $(#[$enum_meta:meta])*
        pub enum $Name:ident from $Src:path {
            $( $(#[$variant_meta:meta])* $Variant:ident ),+ $(,)?
        }
    ) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
        $(#[$enum_meta])*
        #[schemars(rename_all = "camelCase")]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, JsonSchema, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
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
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub enum CodexErrorInfo {
    ContextWindowExceeded,
    UsageLimitExceeded,
    ServerOverloaded,
    CyberPolicy,
    HttpConnectionFailed {
        #[schemars(rename = "httpStatusCode")]
        http_status_code: Option<u16>,
    },
    /// Failed to connect to the response SSE stream.
    ResponseStreamConnectionFailed {
        #[schemars(rename = "httpStatusCode")]
        http_status_code: Option<u16>,
    },
    InternalServerError,
    Unauthorized,
    BadRequest,
    ThreadRollbackFailed,
    SandboxError,
    /// The response SSE stream disconnected in the middle of a turn before completion.
    ResponseStreamDisconnected {
        #[schemars(rename = "httpStatusCode")]
        http_status_code: Option<u16>,
    },
    /// Reached the retry limit for responses.
    ResponseTooManyFailedAttempts {
        #[schemars(rename = "httpStatusCode")]
        http_status_code: Option<u16>,
    },
    /// Returned when `turn/start` or `turn/steer` is submitted while the current active turn
    /// cannot accept same-turn steering, for example `/review` or manual `/compact`.
    ActiveTurnNotSteerable {
        #[schemars(rename = "turnKind")]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Configures who approval requests are routed to for review. Examples
/// include sandbox escapes, blocked network access, MCP approval prompts, and
/// ARC escalations. Defaults to `user`. `auto_review` uses a carefully
/// prompted subagent to gather relevant context and apply a risk-based
/// decision framework before approving or denying the request.
pub enum ApprovalsReviewer {
    User,
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
