use schemars::JsonSchema;

#[derive(Debug, Clone, Default, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct MemoryCitation {
    pub entries: Vec<MemoryCitationEntry>,
    pub rollout_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct MemoryCitationEntry {
    pub path: String,
    pub line_start: u32,
    pub line_end: u32,
    pub note: String,
}
