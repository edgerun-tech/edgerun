use edgerun_serde::Deserialize;
use edgerun_serde::Serialize;
use edgerun_json::FromJson;
use edgerun_json::JsonValueError;
use edgerun_json::Value;
use schemars::JsonSchema;
use ts_rs::TS;

// Types for the TODO tool arguments matching codex-vscode/todo-mcp/src/main.rs
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    InProgress,
    Completed,
}

impl FromJson for StepStatus {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        match String::from_json(value)?.as_str() {
            "pending" => Ok(Self::Pending),
            "in_progress" => Ok(Self::InProgress),
            "completed" => Ok(Self::Completed),
            other => Err(JsonValueError::WrongType(format!(
                "unknown step status `{other}`"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct PlanItemArg {
    pub step: String,
    pub status: StepStatus,
}

impl FromJson for PlanItemArg {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("PlanItemArg")?;
        Ok(Self {
            step: object.take_required("step")?,
            status: object.take_required("status")?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct UpdatePlanArgs {
    /// Arguments for the `update_plan` todo/checklist tool (not plan mode).
    #[serde(default)]
    pub explanation: Option<String>,
    pub plan: Vec<PlanItemArg>,
}

impl FromJson for UpdatePlanArgs {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("UpdatePlanArgs")?;
        Ok(Self {
            explanation: object.take_optional("explanation")?,
            plan: object.take_required("plan")?,
        })
    }
}
