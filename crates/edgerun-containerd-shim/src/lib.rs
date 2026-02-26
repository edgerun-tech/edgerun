// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::fs::OpenOptions;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use edgerun_runtime_proto::wire::TaskServiceOpV1;
use edgerun_runtime_proto::{RuntimeTaskEvent, RuntimeTaskEventKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskLifecycleState {
    Created,
    Running,
    Exited,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskCommand {
    Create,
    Start,
    Exit,
    Kill,
}

#[derive(Debug, thiserror::Error)]
pub enum LifecycleError {
    #[error("invalid transition from {from:?} using command {command:?}")]
    InvalidTransition {
        from: TaskLifecycleState,
        command: TaskCommand,
    },
    #[error("task not found: {0}")]
    TaskNotFound(String),
    #[error("unsupported task api operation")]
    UnsupportedTaskApiOperation,
    #[error("runtime start failed: {0}")]
    RuntimeStartFailed(String),
    #[error("runtime signal failed: {0}")]
    RuntimeSignalFailed(String),
}

#[derive(Debug, Clone)]
pub struct TaskLifecycle {
    namespace: String,
    task_id: String,
    state: TaskLifecycleState,
}

impl TaskLifecycle {
    pub fn new(namespace: String, task_id: String) -> Self {
        Self {
            namespace,
            task_id,
            state: TaskLifecycleState::Created,
        }
    }

    pub fn state(&self) -> TaskLifecycleState {
        self.state
    }

    pub fn transition(
        &mut self,
        command: TaskCommand,
    ) -> Result<RuntimeTaskEventKind, LifecycleError> {
        match (self.state, command) {
            (TaskLifecycleState::Created, TaskCommand::Create) => Ok(RuntimeTaskEventKind::Created),
            (TaskLifecycleState::Created, TaskCommand::Start) => {
                self.state = TaskLifecycleState::Running;
                Ok(RuntimeTaskEventKind::Started)
            }
            (TaskLifecycleState::Running, TaskCommand::Exit) => {
                self.state = TaskLifecycleState::Exited;
                Ok(RuntimeTaskEventKind::Exited)
            }
            (TaskLifecycleState::Running, TaskCommand::Kill) => {
                self.state = TaskLifecycleState::Exited;
                Ok(RuntimeTaskEventKind::Killed)
            }
            _ => Err(LifecycleError::InvalidTransition {
                from: self.state,
                command,
            }),
        }
    }

    pub fn build_event(
        &self,
        kind: RuntimeTaskEventKind,
        event_id: String,
        pid: Option<u32>,
        exit_code: Option<u32>,
        detail: Option<String>,
    ) -> RuntimeTaskEvent {
        RuntimeTaskEvent {
            schema_version: 1,
            namespace: self.namespace.clone(),
            task_id: self.task_id.clone(),
            event_id,
            kind,
            ts_unix_ms: now_unix_ms(),
            pid,
            exit_code,
            detail,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TaskApiResponse {
    pub state: Option<TaskLifecycleState>,
    pub pid: Option<u32>,
    pub exit_code: Option<u32>,
    pub pending: bool,
    pub message: String,
    pub runtime_name: Option<String>,
}

#[derive(Debug)]
struct ManagedTask {
    lifecycle: TaskLifecycle,
    pid: Option<u32>,
    exit_code: Option<u32>,
    runtime_name: Option<String>,
    runtime_selector_source: Option<String>,
    bundle_path: Option<String>,
    stdin_path: Option<String>,
    stdout_path: Option<String>,
    stderr_path: Option<String>,
    child: Option<Child>,
}

#[derive(Debug, Default)]
pub struct ShimTaskService {
    tasks: HashMap<String, ManagedTask>,
}

impl ShimTaskService {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
        }
    }

    pub fn apply(
        &mut self,
        namespace: &str,
        task_id: &str,
        op: TaskServiceOpV1,
        signal: Option<u32>,
        runtime_name: Option<&str>,
        runtime_selector_source: Option<&str>,
        bundle_path: Option<&str>,
        stdin_path: Option<&str>,
        stdout_path: Option<&str>,
        stderr_path: Option<&str>,
    ) -> Result<TaskApiResponse, LifecycleError> {
        let key = task_key(namespace, task_id);
        match op {
            TaskServiceOpV1::Create => {
                let task = self.tasks.entry(key).or_insert_with(|| ManagedTask {
                    lifecycle: TaskLifecycle::new(namespace.to_string(), task_id.to_string()),
                    pid: None,
                    exit_code: None,
                    runtime_name: None,
                    runtime_selector_source: None,
                    bundle_path: None,
                    stdin_path: None,
                    stdout_path: None,
                    stderr_path: None,
                    child: None,
                });
                if let Some(name) = runtime_name.map(str::trim).filter(|v| !v.is_empty()) {
                    task.runtime_name = Some(name.to_string());
                }
                if let Some(source) = runtime_selector_source
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                {
                    task.runtime_selector_source = Some(source.to_string());
                }
                if let Some(bundle) = bundle_path.map(str::trim).filter(|v| !v.is_empty()) {
                    task.bundle_path = Some(bundle.to_string());
                }
                if let Some(path) = stdin_path.map(str::trim).filter(|v| !v.is_empty()) {
                    task.stdin_path = Some(path.to_string());
                }
                if let Some(path) = stdout_path.map(str::trim).filter(|v| !v.is_empty()) {
                    task.stdout_path = Some(path.to_string());
                }
                if let Some(path) = stderr_path.map(str::trim).filter(|v| !v.is_empty()) {
                    task.stderr_path = Some(path.to_string());
                }
                let _ = task.lifecycle.transition(TaskCommand::Create).ok();
                let mut message = "created".to_string();
                if let Some(name) = &task.runtime_name {
                    message.push_str(" runtime=");
                    message.push_str(name);
                }
                Ok(TaskApiResponse {
                    state: Some(task.lifecycle.state()),
                    pid: task.pid,
                    exit_code: task.exit_code,
                    pending: false,
                    message,
                    runtime_name: task.runtime_name.clone(),
                })
            }
            TaskServiceOpV1::Start => {
                let task = self
                    .tasks
                    .get_mut(&key)
                    .ok_or_else(|| LifecycleError::TaskNotFound(key.clone()))?;
                let _ = task.lifecycle.transition(TaskCommand::Start)?;
                if task.pid.is_none() {
                    let runtime_name = task
                        .runtime_name
                        .clone()
                        .unwrap_or_else(|| "crun".to_string());
                    let derived_bundle =
                        format!("/run/containerd/io.containerd.runtime.v2.task/{namespace}/{task_id}");
                    let bundle = task
                        .bundle_path
                        .clone()
                        .filter(|p| Path::new(p).exists())
                        .or_else(|| {
                            if Path::new(&derived_bundle).exists() {
                                Some(derived_bundle)
                            } else {
                                None
                            }
                        });
                    if let Some(bundle) = bundle {
                        let mut command = Command::new("crun");
                        command
                            .arg("run")
                            .arg("--bundle")
                            .arg(&bundle)
                            .arg(task_id);
                        command.stdin(Stdio::null());
                        if let Some(path) = task.stdout_path.clone() {
                            if let Ok(file) = OpenOptions::new().write(true).open(path) {
                                command.stdout(Stdio::from(file));
                            }
                        }
                        if let Some(path) = task.stderr_path.clone() {
                            if let Ok(file) = OpenOptions::new().write(true).open(path) {
                                command.stderr(Stdio::from(file));
                            }
                        }
                        let child = command
                            .spawn()
                            .map_err(|err| LifecycleError::RuntimeStartFailed(err.to_string()))?;
                        task.pid = Some(child.id());
                        task.child = Some(child);
                    } else {
                        task.pid = Some((now_unix_ms() % (u32::MAX as u64)) as u32);
                    }
                    if task.runtime_name.is_none() {
                        task.runtime_name = Some(runtime_name);
                    }
                }
                Ok(TaskApiResponse {
                    state: Some(task.lifecycle.state()),
                    pid: task.pid,
                    exit_code: task.exit_code,
                    pending: false,
                    message: "started".to_string(),
                    runtime_name: task.runtime_name.clone(),
                })
            }
            TaskServiceOpV1::State => {
                let task = self
                    .tasks
                    .get(&key)
                    .ok_or_else(|| LifecycleError::TaskNotFound(key.clone()))?;
                Ok(TaskApiResponse {
                    state: Some(task.lifecycle.state()),
                    pid: task.pid,
                    exit_code: task.exit_code,
                    pending: false,
                    message: "state".to_string(),
                    runtime_name: task.runtime_name.clone(),
                })
            }
            TaskServiceOpV1::Kill => {
                let task = self
                    .tasks
                    .get_mut(&key)
                    .ok_or_else(|| LifecycleError::TaskNotFound(key.clone()))?;
                let _ = task.lifecycle.transition(TaskCommand::Kill)?;
                if let Some(sig) = signal {
                    if let Some(pid) = task.pid {
                        let status = Command::new("kill")
                            .arg(format!("-{sig}"))
                            .arg(pid.to_string())
                            .status()
                            .map_err(|err| LifecycleError::RuntimeSignalFailed(err.to_string()))?;
                        if !status.success() {
                            return Err(LifecycleError::RuntimeSignalFailed(format!(
                                "kill returned non-zero for pid={pid} signal={sig}"
                            )));
                        }
                    }
                } else if let Some(child) = task.child.as_mut() {
                    child
                        .kill()
                        .map_err(|err| LifecycleError::RuntimeSignalFailed(err.to_string()))?;
                }
                task.exit_code = signal.or(Some(137));
                Ok(TaskApiResponse {
                    state: Some(task.lifecycle.state()),
                    pid: task.pid,
                    exit_code: task.exit_code,
                    pending: false,
                    message: "killed".to_string(),
                    runtime_name: task.runtime_name.clone(),
                })
            }
            TaskServiceOpV1::Delete => {
                let mut removed = self.tasks.remove(&key);
                if removed.is_none() {
                    return Err(LifecycleError::TaskNotFound(key));
                }
                if let Some(task) = removed.as_mut() {
                    if let Some(child) = task.child.as_mut() {
                        let _ = child.kill();
                        let _ = child.wait();
                    }
                }
                Ok(TaskApiResponse {
                    state: None,
                    pid: None,
                    exit_code: None,
                    pending: false,
                    message: "deleted".to_string(),
                    runtime_name: removed.and_then(|t| t.runtime_name),
                })
            }
            TaskServiceOpV1::Wait => {
                let task = self
                    .tasks
                    .get_mut(&key)
                    .ok_or_else(|| LifecycleError::TaskNotFound(key.clone()))?;
                let mut pending = task.lifecycle.state() != TaskLifecycleState::Exited;
                if let Some(child) = task.child.as_mut() {
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            if task.lifecycle.state() == TaskLifecycleState::Running {
                                let _ = task.lifecycle.transition(TaskCommand::Exit);
                            }
                            task.exit_code = Some(status.code().unwrap_or_default() as u32);
                            pending = false;
                            task.child = None;
                        }
                        Ok(None) => {
                            pending = true;
                        }
                        Err(err) => {
                            return Err(LifecycleError::RuntimeStartFailed(err.to_string()));
                        }
                    }
                } else if task.lifecycle.state() == TaskLifecycleState::Running {
                    let _ = task.lifecycle.transition(TaskCommand::Exit);
                    if task.exit_code.is_none() {
                        task.exit_code = Some(0);
                    }
                    pending = false;
                }
                Ok(TaskApiResponse {
                    state: Some(task.lifecycle.state()),
                    pid: task.pid,
                    exit_code: task.exit_code,
                    pending,
                    message: if pending { "waiting" } else { "completed" }.to_string(),
                    runtime_name: task.runtime_name.clone(),
                })
            }
            TaskServiceOpV1::Unspecified => Err(LifecycleError::UnsupportedTaskApiOperation),
        }
    }
}

fn task_key(namespace: &str, task_id: &str) -> String {
    format!("{namespace}/{task_id}")
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_transitions() {
        let mut lifecycle = TaskLifecycle::new("default".to_string(), "task-1".to_string());
        assert_eq!(
            lifecycle.transition(TaskCommand::Start).expect("start"),
            RuntimeTaskEventKind::Started
        );
        assert_eq!(lifecycle.state(), TaskLifecycleState::Running);
        assert_eq!(
            lifecycle.transition(TaskCommand::Exit).expect("exit"),
            RuntimeTaskEventKind::Exited
        );
        assert_eq!(lifecycle.state(), TaskLifecycleState::Exited);
    }

    #[test]
    fn task_service_flow() {
        let mut svc = ShimTaskService::new();
        let c = svc
            .apply(
                "ns",
                "task-a",
                TaskServiceOpV1::Create,
                None,
                Some("edgerun"),
                Some("annotation:io.edgerun.runtime=edgerun"),
                Some("/tmp/bundle"),
                Some("/tmp/stdin"),
                Some("/tmp/stdout"),
                Some("/tmp/stderr"),
            )
            .expect("create");
        assert_eq!(c.state, Some(TaskLifecycleState::Created));
        assert_eq!(c.runtime_name.as_deref(), Some("edgerun"));
        let s = svc
            .apply(
                "ns",
                "task-a",
                TaskServiceOpV1::Start,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .expect("start");
        assert_eq!(s.state, Some(TaskLifecycleState::Running));
        let w = svc
            .apply(
                "ns",
                "task-a",
                TaskServiceOpV1::Wait,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .expect("wait");
        assert!(!w.pending);
        assert_eq!(w.state, Some(TaskLifecycleState::Exited));
        let d = svc
            .apply(
                "ns",
                "task-a",
                TaskServiceOpV1::Delete,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .expect("delete");
        assert_eq!(d.message, "deleted");
    }
}

mod task_api;

pub use task_api::{
    build_task_api_request, decode_task_api_response, ContainerdTaskClient, TaskApiClientError,
};

mod task_service;

pub use task_service::{
    ShimTaskTtrpcService, TaskApiBackend, TaskCreateRequest, TaskDeleteRequest, TaskKillRequest,
    TaskRpcResponse, TaskStartRequest, TaskStateRequest, TaskWaitRequest,
};

mod ttrpc_task;

pub use ttrpc_task::{register_task_ttrpc_service, ContainerdTaskTtrpcService};
