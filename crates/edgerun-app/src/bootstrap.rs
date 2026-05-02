use edgerun_proto::edgerun::v0::stream::{CommandEnvelope, CommandType};

#[derive(Debug, Clone)]
pub struct SimulationResult {
    pub accepted: bool,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct BootstrapState {
    pub phase: BootstrapPhase,
    pub has_identity: bool,
    pub has_controller: bool,
    pub commands_dispatched: u32,
    pub commands_accepted: u32,
    pub commands_rejected: u32,
}

impl BootstrapState {
    pub fn start(&mut self) {
        self.phase = BootstrapPhase::Running;
    }

    pub fn mark_identity_created(&mut self) {
        self.has_identity = true;
    }

    pub fn mark_controller_added(&mut self) {
        self.has_controller = true;
    }

    pub fn is_bootstrap_complete(&self) -> bool {
        self.has_identity && self.has_controller
    }

    pub fn complete(&mut self) {
        self.phase = BootstrapPhase::Complete;
    }
}

impl Default for BootstrapState {
    fn default() -> Self {
        Self {
            phase: BootstrapPhase::NotStarted,
            has_identity: false,
            has_controller: false,
            commands_dispatched: 0,
            commands_accepted: 0,
            commands_rejected: 0,
        }
    }
}

pub struct BootstrapOrchestrator {
    state: BootstrapState,
    settings_app_path: Option<String>,
}

impl BootstrapOrchestrator {
    pub fn new(settings_app_path: Option<String>) -> Self {
        Self {
            state: BootstrapState::default(),
            settings_app_path,
        }
    }

    pub fn should_run_bootstrap(&self) -> bool {
        self.settings_app_path.is_some()
            && !self.state.has_identity
            && self.state.phase != BootstrapPhase::Complete
            && self.state.phase != BootstrapPhase::Skipped
    }

    pub fn mark_identity_created(&mut self) {
        self.state.has_identity = true;
    }

    pub fn mark_controller_added(&mut self) {
        self.state.has_controller = true;
    }

    pub fn is_bootstrap_complete(&self) -> bool {
        self.state.has_identity && self.state.has_controller
    }

    pub fn complete(&mut self) {
        self.state.phase = BootstrapPhase::Complete;
    }

    pub fn start(&mut self) {
        self.state.phase = BootstrapPhase::Running;
    }

    pub fn state(&self) -> &BootstrapState {
        &self.state
    }

    pub fn settings_app_path(&self) -> Option<&str> {
        self.settings_app_path.as_deref()
    }
}

pub struct DispatchResult {
    pub accepted: bool,
    pub response_bytes: Vec<u8>,
    pub reason: String,
}

pub fn dispatch_commands_local(
    commands: Vec<CommandEnvelope>,
    state: &mut BootstrapState,
) -> Vec<SimulationResult> {
    let mut results = Vec::new();

    for cmd in commands {
        let cmd_type = CommandType::from_i32(cmd.command_type);
        let accepted = matches!(
            cmd_type,
            Some(CommandType::CreateIdentity)
                | Some(CommandType::ImportIdentity)
                | Some(CommandType::AddController)
                | Some(CommandType::AddBootstrapNode)
                | Some(CommandType::AddReachabilityHint)
                | Some(CommandType::QueryNodeState)
        );

        state.commands_dispatched += 1;
        if accepted {
            state.commands_accepted += 1;
        } else {
            state.commands_rejected += 1;
        }

        results.push(SimulationResult {
            accepted,
            reason: if accepted {
                String::new()
            } else {
                String::from("unsupported_command_type")
            },
        });
    }

    results
}
