/// Marker trait for protocol types that can signal experimental usage.
///
/// The lifted client does not gate experimental fields, so every type reports
/// stable by default.
pub trait ExperimentalApi {
    fn experimental_reason(&self) -> Option<&'static str>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExperimentalField {
    pub type_name: &'static str,
    pub field_name: &'static str,
    pub reason: &'static str,
}

edgerun_inventory::collect!(ExperimentalField);

static CONFIG_APPROVALS_REVIEWER: ExperimentalField = ExperimentalField {
    type_name: "Config",
    field_name: "approvalsReviewer",
    reason: "config/read.approvalsReviewer",
};

static PROFILE_APPROVALS_REVIEWER: ExperimentalField = ExperimentalField {
    type_name: "ProfileV2",
    field_name: "approvalsReviewer",
    reason: "config/read.approvalsReviewer",
};

static THREAD_START_MOCK_FIELD: ExperimentalField = ExperimentalField {
    type_name: "ThreadStartParams",
    field_name: "mockExperimentalField",
    reason: "mock/experimentalField",
};

static TURN_START_ENVIRONMENTS: ExperimentalField = ExperimentalField {
    type_name: "TurnStartParams",
    field_name: "environments",
    reason: "turn/start.environments",
};

static COMMAND_EXECUTION_ADDITIONAL_PERMISSIONS: ExperimentalField = ExperimentalField {
    type_name: "CommandExecutionRequestApprovalParams",
    field_name: "additionalPermissions",
    reason: "item/commandExecution/requestApproval.additionalPermissions",
};

pub fn experimental_fields() -> Vec<&'static ExperimentalField> {
    vec![
        &CONFIG_APPROVALS_REVIEWER,
        &PROFILE_APPROVALS_REVIEWER,
        &THREAD_START_MOCK_FIELD,
        &TURN_START_ENVIRONMENTS,
        &COMMAND_EXECUTION_ADDITIONAL_PERMISSIONS,
    ]
}

pub fn experimental_required_message(reason: &str) -> String {
    format!("{reason} requires experimentalApi capability")
}
