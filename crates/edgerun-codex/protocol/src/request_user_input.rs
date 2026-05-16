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
    #[schemars(rename = "isOther", default)]
    #[schemars(rename = "isOther", default)]
    #[schemars(rename = "isOther")]
    pub is_other: bool,
    #[schemars(rename = "isSecret", default)]
    #[schemars(rename = "isSecret", default)]
    #[schemars(rename = "isSecret")]
    pub is_secret: bool,
    #[schemars(skip_serializing_if = "Option::is_none")]
    #[schemars(default, skip_serializing_if = "Option::is_none")]
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
    /// Uses `#[schemars(default)]` for backwards compatibility.
    #[schemars(default)]
    #[schemars(default)]
    pub turn_id: String,
    pub questions: Vec<RequestUserInputQuestion>,
}
