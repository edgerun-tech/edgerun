// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::fs;
use std::fs::OpenOptions;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use edgerun_runtime_proto::wire::TaskServiceOpV1;
use edgerun_runtime_proto::{RuntimeTaskEvent, RuntimeTaskEventKind};

const CRUN_STATE_ROOT: &str = "/run/edgerun-shim/crun";

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
    #[error("runtime config failed: {0}")]
    RuntimeConfigFailed(String),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaskBackend {
    Crun,
    NativeEdgerun,
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
    rootfs_source: Option<String>,
    rootfs_readonly: Option<bool>,
    rootfs_type: Option<String>,
    rootfs_options_csv: Option<String>,
    mounted_rootfs_path: Option<String>,
    backend: TaskBackend,
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

    #[allow(clippy::too_many_arguments)]
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
        rootfs_source: Option<&str>,
        rootfs_readonly: Option<bool>,
        rootfs_type: Option<&str>,
        rootfs_options_csv: Option<&str>,
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
                    rootfs_source: None,
                    rootfs_readonly: None,
                    rootfs_type: None,
                    rootfs_options_csv: None,
                    mounted_rootfs_path: None,
                    backend: TaskBackend::Crun,
                    child: None,
                });
                if let Some(name) = runtime_name.map(str::trim).filter(|v| !v.is_empty()) {
                    task.runtime_name = Some(name.to_string());
                    task.backend = select_backend(name);
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
                if let Some(path) = rootfs_source.map(str::trim).filter(|v| !v.is_empty()) {
                    task.rootfs_source = Some(path.to_string());
                }
                if let Some(readonly) = rootfs_readonly {
                    task.rootfs_readonly = Some(readonly);
                }
                if let Some(value) = rootfs_type.map(str::trim).filter(|v| !v.is_empty()) {
                    task.rootfs_type = Some(value.to_string());
                }
                if let Some(value) = rootfs_options_csv.map(str::trim).filter(|v| !v.is_empty()) {
                    task.rootfs_options_csv = Some(value.to_string());
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
                    let derived_bundle = format!(
                        "/run/containerd/io.containerd.runtime.v2.task/{namespace}/{task_id}"
                    );
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
                        let mut command = if task.backend == TaskBackend::NativeEdgerun {
                            native_edgerun_command(&bundle, task_id)?
                        } else {
                            if let Some(rootfs_path) = prepare_bundle_rootfs(task, &bundle)? {
                                patch_bundle_rootfs_config(
                                    &bundle,
                                    &rootfs_path,
                                    task.rootfs_readonly.unwrap_or(false),
                                )?;
                            }
                            let mut cmd = crun_command();
                            cmd.arg("run").arg("--bundle").arg(&bundle).arg(task_id);
                            cmd
                        };
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
                if task.backend == TaskBackend::Crun {
                    if let Some(state) = read_crun_state(task_id)? {
                        return Ok(TaskApiResponse {
                            state: Some(map_crun_status_to_state(&state.status)),
                            pid: state.pid,
                            exit_code: state.exit_code,
                            pending: matches!(state.status.as_str(), "running" | "created"),
                            message: format!("state:{}", state.status),
                            runtime_name: task.runtime_name.clone(),
                        });
                    }
                }
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

                if task.backend == TaskBackend::Crun {
                    if let Some(sig) = signal {
                        let output = crun_command()
                            .arg("kill")
                            .arg(task_id)
                            .arg(sig.to_string())
                            .output()
                            .map_err(|err| LifecycleError::RuntimeSignalFailed(err.to_string()))?;
                        if !output.status.success() {
                            let stderr =
                                String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
                            let ignorable = stderr.contains("does not exist")
                                || stderr.contains("not found")
                                || stderr.contains("not running");
                            if !ignorable {
                                return Err(LifecycleError::RuntimeSignalFailed(format!(
                                    "crun kill returned non-zero task_id={task_id} signal={sig}"
                                )));
                            }
                        }
                    } else if let Some(child) = task.child.as_mut() {
                        child
                            .kill()
                            .map_err(|err| LifecycleError::RuntimeSignalFailed(err.to_string()))?;
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
                if let Some(task) = removed.as_ref() {
                    if task.backend == TaskBackend::Crun {
                        let _ = crun_command().arg("delete").arg("-f").arg(task_id).status();
                    }
                }
                if let Some(task) = removed.as_mut() {
                    if let Some(child) = task.child.as_mut() {
                        let _ = child.kill();
                        let _ = child.wait();
                    }
                    if let Some(path) = task.mounted_rootfs_path.as_deref() {
                        let _ = Command::new("umount").arg("-l").arg(path).status();
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

                if task.backend == TaskBackend::Crun {
                    if let Some(state) = read_crun_state(task_id)? {
                        if matches!(state.status.as_str(), "running" | "created") {
                            let output = crun_command().arg("wait").arg(task_id).output().map_err(
                                |err| LifecycleError::RuntimeStartFailed(err.to_string()),
                            )?;
                            if !output.status.success() {
                                let stderr =
                                    String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
                                let should_fallback = stderr.contains("unknown command")
                                    || stderr.contains("does not exist")
                                    || stderr.contains("not found");
                                if !should_fallback {
                                    return Err(LifecycleError::RuntimeStartFailed(format!(
                                        "crun wait returned non-zero task_id={task_id} stderr={}",
                                        String::from_utf8_lossy(&output.stderr)
                                    )));
                                }
                            } else {
                                let raw =
                                    String::from_utf8_lossy(&output.stdout).trim().to_string();
                                let parsed_exit =
                                    raw.parse::<u32>().ok().or(state.exit_code).or(Some(0));
                                if task.lifecycle.state() == TaskLifecycleState::Running {
                                    let _ = task.lifecycle.transition(TaskCommand::Exit);
                                }
                                task.exit_code = parsed_exit;
                                return Ok(TaskApiResponse {
                                    state: Some(TaskLifecycleState::Exited),
                                    pid: state.pid.or(task.pid),
                                    exit_code: task.exit_code,
                                    pending: false,
                                    message: "completed".to_string(),
                                    runtime_name: task.runtime_name.clone(),
                                });
                            }
                        }
                        if matches!(state.status.as_str(), "stopped" | "exited") {
                            if task.lifecycle.state() == TaskLifecycleState::Running {
                                let _ = task.lifecycle.transition(TaskCommand::Exit);
                            }
                            task.exit_code = state.exit_code.or(task.exit_code).or(Some(0));
                            return Ok(TaskApiResponse {
                                state: Some(TaskLifecycleState::Exited),
                                pid: state.pid.or(task.pid),
                                exit_code: task.exit_code,
                                pending: false,
                                message: "completed".to_string(),
                                runtime_name: task.runtime_name.clone(),
                            });
                        }
                    }
                }

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

fn select_backend(runtime_name: &str) -> TaskBackend {
    match runtime_name.trim().to_ascii_lowercase().as_str() {
        "edgerun" | "edgerun-wasi" | "wasi" | "wasm" => TaskBackend::NativeEdgerun,
        _ => TaskBackend::Crun,
    }
}

fn native_edgerun_command(bundle_path: &str, task_id: &str) -> Result<Command, LifecycleError> {
    let config = read_bundle_config(bundle_path)?;
    let bundle = resolve_native_bundle_path(bundle_path, &config)?;
    let output = resolve_native_output_path(task_id, &config);
    let binary = std::env::var("EDGERUN_WASI_EXECUTOR_BIN")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "/usr/bin/edgerun-runtime".to_string());
    let mut command = Command::new(binary);
    command
        .arg("run")
        .arg("--bundle")
        .arg(bundle)
        .arg("--output")
        .arg(output);
    Ok(command)
}

fn read_bundle_config(bundle_path: &str) -> Result<serde_json::Value, LifecycleError> {
    let config_path = Path::new(bundle_path).join("config.json");
    let raw = fs::read(&config_path).map_err(|err| {
        LifecycleError::RuntimeConfigFailed(format!("read {}: {err}", config_path.display()))
    })?;
    serde_json::from_slice(&raw).map_err(|err| {
        LifecycleError::RuntimeConfigFailed(format!("parse {}: {err}", config_path.display()))
    })
}

fn resolve_native_bundle_path(
    bundle_path: &str,
    config: &serde_json::Value,
) -> Result<String, LifecycleError> {
    let annotation_keys = [
        "io.edgerun.bundle.path",
        "io.edgerun.wasm.bundle",
        "io.edgerun.runtime.bundle",
    ];
    for key in annotation_keys {
        let Some(value) = config
            .get("annotations")
            .and_then(|v| v.get(key))
            .and_then(|v| v.as_str())
        else {
            continue;
        };
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        let candidate = absolutize_bundle_path(bundle_path, trimmed);
        if Path::new(&candidate).exists() {
            return Ok(candidate);
        }
    }

    let process_args = config
        .get("process")
        .and_then(|v| v.get("args"))
        .and_then(|v| v.as_array());
    if let Some(args) = process_args {
        let mut prev_is_bundle = false;
        for item in args {
            let Some(arg) = item.as_str() else {
                continue;
            };
            if prev_is_bundle {
                let candidate = absolutize_bundle_path(bundle_path, arg);
                if Path::new(&candidate).exists() {
                    return Ok(candidate);
                }
                prev_is_bundle = false;
                continue;
            }
            prev_is_bundle = arg == "--bundle";
        }
    }

    let candidates = [
        "edgerun.bundle",
        "bundle.edgerun",
        "rootfs/edgerun.bundle",
        "rootfs/bundle.edgerun",
    ];
    for rel in candidates {
        let candidate = absolutize_bundle_path(bundle_path, rel);
        if Path::new(&candidate).exists() {
            return Ok(candidate);
        }
    }

    Err(LifecycleError::RuntimeConfigFailed(
        "unable to resolve native EdgeRun bundle path from OCI annotations/args".to_string(),
    ))
}

fn resolve_native_output_path(task_id: &str, config: &serde_json::Value) -> String {
    if let Some(value) = config
        .get("annotations")
        .and_then(|v| v.get("io.edgerun.output.path"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        return value.to_string();
    }
    format!("/var/lib/edgerun/shim/{task_id}.native.out")
}

fn absolutize_bundle_path(bundle_path: &str, candidate: &str) -> String {
    if Path::new(candidate).is_absolute() {
        candidate.to_string()
    } else {
        Path::new(bundle_path)
            .join(candidate)
            .to_string_lossy()
            .to_string()
    }
}

fn task_key(namespace: &str, task_id: &str) -> String {
    format!("{namespace}/{task_id}")
}

#[derive(Debug)]
struct CrunStateSnapshot {
    status: String,
    pid: Option<u32>,
    exit_code: Option<u32>,
}

fn read_crun_state(task_id: &str) -> Result<Option<CrunStateSnapshot>, LifecycleError> {
    let output = crun_command()
        .arg("state")
        .arg(task_id)
        .output()
        .map_err(|err| LifecycleError::RuntimeConfigFailed(format!("crun state: {err}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
        if stderr.contains("does not exist") || stderr.contains("not found") {
            return Ok(None);
        }
        return Err(LifecycleError::RuntimeConfigFailed(format!(
            "crun state non-zero task_id={task_id} stderr={}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).map_err(|err| {
        LifecycleError::RuntimeConfigFailed(format!("parse crun state json: {err}"))
    })?;
    let status = parsed
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_ascii_lowercase();
    let pid = parsed
        .get("pid")
        .and_then(|v| v.as_u64())
        .and_then(|v| u32::try_from(v).ok());
    let exit_code = parsed
        .get("exitCode")
        .or_else(|| parsed.get("exit_code"))
        .and_then(|v| v.as_u64())
        .and_then(|v| u32::try_from(v).ok());
    Ok(Some(CrunStateSnapshot {
        status,
        pid,
        exit_code,
    }))
}

fn map_crun_status_to_state(status: &str) -> TaskLifecycleState {
    match status {
        "running" => TaskLifecycleState::Running,
        "created" => TaskLifecycleState::Created,
        "stopped" | "exited" => TaskLifecycleState::Exited,
        _ => TaskLifecycleState::Created,
    }
}

fn crun_command() -> Command {
    let root = crun_state_root();
    let _ = fs::create_dir_all(&root);
    let mut command = Command::new("crun");
    command.arg("--root").arg(root);
    command
}

fn crun_state_root() -> String {
    std::env::var("EDGERUN_CRUN_STATE_ROOT").unwrap_or_else(|_| {
        if cfg!(test) {
            "/tmp/edgerun-shim/crun-test".to_string()
        } else {
            CRUN_STATE_ROOT.to_string()
        }
    })
}

fn prepare_bundle_rootfs(
    task: &mut ManagedTask,
    bundle_path: &str,
) -> Result<Option<String>, LifecycleError> {
    let source = task.rootfs_source.clone().filter(|v| !v.trim().is_empty());
    let Some(source) = source else {
        return Ok(None);
    };

    let rootfs_type = task
        .rootfs_type
        .clone()
        .unwrap_or_else(|| "bind".to_string())
        .to_ascii_lowercase();

    if rootfs_type == "overlay" {
        let target = Path::new(bundle_path).join("rootfs");
        fs::create_dir_all(&target).map_err(|err| {
            LifecycleError::RuntimeConfigFailed(format!("create {}: {err}", target.display()))
        })?;
        let opts = task.rootfs_options_csv.clone().unwrap_or_default();
        let output = Command::new("mount")
            .arg("-t")
            .arg("overlay")
            .arg(if source.is_empty() {
                "overlay"
            } else {
                source.as_str()
            })
            .arg("-o")
            .arg(&opts)
            .arg(&target)
            .output()
            .map_err(|err| LifecycleError::RuntimeConfigFailed(format!("mount overlay: {err}")))?;
        if !output.status.success() {
            if let Some(lowerdir) = overlay_option_value(&opts, "lowerdir") {
                let candidate = lowerdir.split(':').next().unwrap_or_default().trim();
                if !candidate.is_empty() && Path::new(candidate).exists() {
                    return Ok(Some(candidate.to_string()));
                }
            }
            if let Some(upperdir) = overlay_option_value(&opts, "upperdir") {
                let candidate = upperdir.trim();
                if !candidate.is_empty() && Path::new(candidate).exists() {
                    return Ok(Some(candidate.to_string()));
                }
            }
            return Ok(Some(target.to_string_lossy().to_string()));
        }
        let target_str = target.to_string_lossy().to_string();
        task.mounted_rootfs_path = Some(target_str.clone());
        return Ok(Some(target_str));
    }

    Ok(Some(source))
}

fn overlay_option_value(options_csv: &str, key: &str) -> Option<String> {
    options_csv.split(',').find_map(|item| {
        let mut parts = item.splitn(2, '=');
        let k = parts.next()?.trim();
        let v = parts.next()?.trim();
        if k == key && !v.is_empty() {
            Some(v.to_string())
        } else {
            None
        }
    })
}

fn patch_bundle_rootfs_config(
    bundle_path: &str,
    rootfs_source: &str,
    readonly: bool,
) -> Result<(), LifecycleError> {
    let config_path = Path::new(bundle_path).join("config.json");
    let raw = fs::read(&config_path).map_err(|err| {
        LifecycleError::RuntimeConfigFailed(format!("read {}: {err}", config_path.display()))
    })?;
    let mut parsed: serde_json::Value = serde_json::from_slice(&raw).map_err(|err| {
        LifecycleError::RuntimeConfigFailed(format!("parse {}: {err}", config_path.display()))
    })?;
    let root = parsed
        .as_object_mut()
        .ok_or_else(|| {
            LifecycleError::RuntimeConfigFailed("config root must be object".to_string())
        })?
        .entry("root")
        .or_insert_with(|| serde_json::json!({}));
    let root_obj = root.as_object_mut().ok_or_else(|| {
        LifecycleError::RuntimeConfigFailed("config.root must be object".to_string())
    })?;
    root_obj.insert(
        "path".to_string(),
        serde_json::Value::String(rootfs_source.to_string()),
    );
    root_obj.insert("readonly".to_string(), serde_json::Value::Bool(readonly));
    let bytes = serde_json::to_vec_pretty(&parsed).map_err(|err| {
        LifecycleError::RuntimeConfigFailed(format!("encode {}: {err}", config_path.display()))
    })?;
    fs::write(&config_path, bytes).map_err(|err| {
        LifecycleError::RuntimeConfigFailed(format!("write {}: {err}", config_path.display()))
    })?;
    Ok(())
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
                Some("/tmp/rootfs"),
                Some(true),
                Some("bind"),
                Some("ro,rbind"),
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
