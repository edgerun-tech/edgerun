// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExecutorKind {
    LocalAction,
    GithubActions,
    AgentTask,
    IngestTransform,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ExecutionRequest {
    pub run_id: String,
    pub step_id: String,
    pub payload: Vec<u8>,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ExecutionResult {
    pub ok: bool,
    pub message: String,
}

pub trait ExecutorPlugin: Send + Sync {
    fn kind(&self) -> ExecutorKind;
    fn name(&self) -> &'static str;
    #[allow(dead_code)]
    fn execute(&self, req: &ExecutionRequest) -> ExecutionResult;
}

pub struct ExecutorRegistry {
    plugins: BTreeMap<ExecutorKind, Box<dyn ExecutorPlugin>>,
}

impl ExecutorRegistry {
    pub fn new() -> Self {
        Self {
            plugins: BTreeMap::new(),
        }
    }

    pub fn register(&mut self, plugin: Box<dyn ExecutorPlugin>) {
        self.plugins.insert(plugin.kind(), plugin);
    }

    #[allow(dead_code)]
    pub fn contains(&self, kind: &ExecutorKind) -> bool {
        self.plugins.contains_key(kind)
    }

    #[allow(dead_code)]
    pub fn execute(&self, kind: &ExecutorKind, req: &ExecutionRequest) -> Option<ExecutionResult> {
        self.plugins.get(kind).map(|plugin| plugin.execute(req))
    }

    pub fn list_names(&self) -> Vec<&'static str> {
        self.plugins.values().map(|plugin| plugin.name()).collect()
    }
}

macro_rules! stub_executor {
    ($name:ident, $kind:expr, $label:expr) => {
        pub struct $name;
        impl ExecutorPlugin for $name {
            fn kind(&self) -> ExecutorKind {
                $kind
            }
            fn name(&self) -> &'static str {
                $label
            }
            fn execute(&self, req: &ExecutionRequest) -> ExecutionResult {
                let _ = req;
                ExecutionResult {
                    ok: true,
                    message: format!("stub executor {} accepted step", $label),
                }
            }
        }
    };
}

stub_executor!(
    LocalActionExecutor,
    ExecutorKind::LocalAction,
    "local_action"
);
stub_executor!(
    GithubActionsExecutor,
    ExecutorKind::GithubActions,
    "github_actions"
);
stub_executor!(AgentTaskExecutor, ExecutorKind::AgentTask, "agent_task");
stub_executor!(
    IngestTransformExecutor,
    ExecutorKind::IngestTransform,
    "ingest_transform"
);

pub fn default_registry() -> ExecutorRegistry {
    let mut registry = ExecutorRegistry::new();
    registry.register(Box::new(LocalActionExecutor));
    registry.register(Box::new(GithubActionsExecutor));
    registry.register(Box::new(AgentTaskExecutor));
    registry.register(Box::new(IngestTransformExecutor));
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_contains_all_default_executors() {
        let registry = default_registry();
        assert!(registry.contains(&ExecutorKind::LocalAction));
        assert!(registry.contains(&ExecutorKind::GithubActions));
        assert!(registry.contains(&ExecutorKind::AgentTask));
        assert!(registry.contains(&ExecutorKind::IngestTransform));
    }
}
