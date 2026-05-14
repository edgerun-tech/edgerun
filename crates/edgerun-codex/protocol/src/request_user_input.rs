use std::collections::HashMap;

use edgerun_json::FromJson;
use edgerun_json::ToJson;
use schemars::JsonSchema;

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, ToJson, FromJson)]
pub struct RequestUserInputQuestionOption {
    pub label: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, ToJson, FromJson)]
pub struct RequestUserInputQuestion {
    pub id: String,
    pub header: String,
    pub question: String,
    #[serde(rename = "isOther", default)]
    #[json(rename = "isOther", default)]
    #[schemars(rename = "isOther")]
    pub is_other: bool,
    #[serde(rename = "isSecret", default)]
    #[json(rename = "isSecret", default)]
    #[schemars(rename = "isSecret")]
    pub is_secret: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[json(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<RequestUserInputQuestionOption>>,
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, ToJson, FromJson)]
pub struct RequestUserInputArgs {
    pub questions: Vec<RequestUserInputQuestion>,
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, ToJson, FromJson)]
pub struct RequestUserInputAnswer {
    pub answers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, ToJson, FromJson)]
pub struct RequestUserInputResponse {
    pub answers: HashMap<String, RequestUserInputAnswer>,
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, ToJson, FromJson)]
pub struct RequestUserInputEvent {
    /// Responses API call id for the associated tool call, if available.
    pub call_id: String,
    /// Turn ID that this request belongs to.
    /// Uses `#[serde(default)]` for backwards compatibility.
    #[serde(default)]
    #[json(default)]
    pub turn_id: String,
    pub questions: Vec<RequestUserInputQuestion>,
}
