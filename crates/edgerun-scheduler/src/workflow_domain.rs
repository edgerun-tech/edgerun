// SPDX-License-Identifier: LicenseRef-Edgerun-Proprietary

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutorKind {
    LocalAction,
    GithubActions,
    AgentTask,
    IngestTransform,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowStep {
    pub step_id: String,
    pub requires_caps: BTreeMap<String, String>,
    pub executor: ExecutorKind,
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowPlan {
    pub workflow_id: String,
    pub intent_id: String,
    pub steps: Vec<WorkflowStep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeCapabilitySnapshot {
    pub node_id: String,
    pub attrs: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulingChoice {
    pub step_id: String,
    pub node_id: String,
    pub rationale: String,
}

pub fn planner_version() -> &'static str {
    "workflow-domain-v1"
}

pub fn validate_plan(plan: &WorkflowPlan) -> Result<(), String> {
    if plan.workflow_id.trim().is_empty() {
        return Err("workflow_id is required".to_string());
    }
    if plan.intent_id.trim().is_empty() {
        return Err("intent_id is required".to_string());
    }
    let mut ids = BTreeSet::new();
    for step in &plan.steps {
        if step.step_id.trim().is_empty() {
            return Err("step_id is required".to_string());
        }
        if !ids.insert(step.step_id.clone()) {
            return Err(format!("duplicate step_id: {}", step.step_id));
        }
    }
    for step in &plan.steps {
        for dep in &step.depends_on {
            if !ids.contains(dep) {
                return Err(format!(
                    "step {} depends on unknown step {}",
                    step.step_id, dep
                ));
            }
        }
    }
    Ok(())
}

pub fn node_matches_step(step: &WorkflowStep, node: &NodeCapabilitySnapshot) -> bool {
    step.requires_caps
        .iter()
        .all(|(key, value)| node.attrs.get(key) == Some(value))
}

pub fn pick_node_for_step(
    step: &WorkflowStep,
    nodes: &[NodeCapabilitySnapshot],
) -> Option<SchedulingChoice> {
    let selected = nodes.iter().find(|node| node_matches_step(step, node))?;
    Some(SchedulingChoice {
        step_id: step.step_id.clone(),
        node_id: selected.node_id.clone(),
        rationale: "first capability match".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_basic_plan() {
        let plan = WorkflowPlan {
            workflow_id: "wf-1".to_string(),
            intent_id: "intent-1".to_string(),
            steps: vec![WorkflowStep {
                step_id: "s1".to_string(),
                requires_caps: BTreeMap::new(),
                executor: ExecutorKind::LocalAction,
                depends_on: vec![],
            }],
        };
        assert!(validate_plan(&plan).is_ok());
    }

    #[test]
    fn matches_required_caps() {
        let mut required = BTreeMap::new();
        required.insert("gpu".to_string(), "nvidia".to_string());
        let step = WorkflowStep {
            step_id: "s1".to_string(),
            requires_caps: required,
            executor: ExecutorKind::AgentTask,
            depends_on: vec![],
        };
        let mut attrs = BTreeMap::new();
        attrs.insert("gpu".to_string(), "nvidia".to_string());
        let node = NodeCapabilitySnapshot {
            node_id: "node-1".to_string(),
            attrs,
        };
        assert!(node_matches_step(&step, &node));
    }
}
