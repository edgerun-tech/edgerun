//! OCI runtime hooks execution.
//!
//! Hooks are external executables called at specific lifecycle points:
//! - **prestart** (deprecated): After env created, before pivot_root
//! - **createRuntime**: After env created, before pivot_root (runtime namespace)
//! - **createContainer**: After env created, before pivot_root (container namespace)
//! - **startContainer**: Before user process exec (container namespace)
//! - **poststart**: After user process started, before start returns (runtime namespace)
//! - **poststop**: After container deleted, before delete returns (runtime namespace)
//!
//! Each hook receives container state JSON on stdin.
//! Per the OCI spec:
//! - Hooks MUST be called in listed order
//! - If prestart/createRuntime/createContainer/startContainer/poststart fails:
//!   runtime MUST generate an error, stop container, continue lifecycle at step 12
//! - If poststop fails: runtime MUST log a warning, but remaining hooks continue

use crate::prelude::*;
use alloc::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::spec::OciHook;

/// Container state passed to hooks via stdin.
#[derive(Debug, Clone)]
pub struct ContainerState {
    pub version: String,
    pub id: String,
    pub status: String,
    pub pid: u32,
    pub bundle: String,
    pub annotations: BTreeMap<String, String>,
}

impl ContainerState {
    pub fn to_json(&self) -> String {
        edgerun_json::to_json_string(self).unwrap_or_default()
    }
}

edgerun_json::impl_json_struct! {
    ContainerState {
        required {
            version: "ociVersion" => String,
            id: "id" => String,
            status: "status" => String,
            pid: "pid" => u32,
            bundle: "bundle" => String,
            annotations: "annotations" => BTreeMap<String, String>,
        }
        optional {}
    }
}

/// Error type for hook execution that tracks which hook failed.
#[derive(Debug)]
pub struct HookError {
    pub hook_path: String,
    pub error: io::Error,
}

impl std::fmt::Display for HookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "hook {:?} failed: {}", self.hook_path, self.error)
    }
}

/// Execute hooks in runtime namespace context.
pub fn execute_hooks(hooks: Option<&[OciHook]>, state: &ContainerState) -> Result<(), HookError> {
    run_hook_chain(hooks, state, true)
}

/// Execute hooks in container namespace context (inside pre_exec).
pub fn execute_hooks_in_context(
    hooks: Option<&[OciHook]>,
    state: &ContainerState,
) -> io::Result<()> {
    run_hook_chain(hooks, state, false).map_err(|e| e.error)
}

/// Shared hook chain executor.
fn run_hook_chain(
    hooks: Option<&[OciHook]>,
    state: &ContainerState,
    capture_stderr: bool,
) -> Result<(), HookError> {
    let Some(hooks) = hooks else { return Ok(()) };
    let state_json = state.to_json();

    for hook in hooks {
        if !Path::new(&hook.path).exists() {
            return Err(HookError {
                hook_path: hook.path.clone(),
                error: io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("hook not found: {}", hook.path),
                ),
            });
        }

        let args = hook.args.as_deref().unwrap_or(&[]);
        let env = hook.env.as_deref().unwrap_or(&[]);

        let mut cmd = Command::new(&hook.path);
        cmd.args(args);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::null());
        if capture_stderr {
            cmd.stderr(Stdio::piped());
        } else {
            cmd.stderr(Stdio::null());
        }
        for e in env {
            if let Some((k, v)) = e.split_once('=') {
                cmd.env(k, v);
            }
        }

        let mut child = cmd.spawn().map_err(|e| HookError {
            hook_path: hook.path.clone(),
            error: e,
        })?;
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(state_json.as_bytes());
        }

        let timeout_secs = hook.timeout.unwrap_or(0);
        let start = Instant::now();
        let status = if timeout_secs > 0 {
            let timeout = Duration::from_secs(timeout_secs);
            loop {
                if let Some(s) = child.try_wait().map_err(|e| HookError {
                    hook_path: hook.path.clone(),
                    error: e,
                })? {
                    break s;
                }
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    return Err(HookError {
                        hook_path: hook.path.clone(),
                        error: io::Error::new(
                            io::ErrorKind::TimedOut,
                            format!("hook {:?} timed out after {}s", hook.path, timeout_secs),
                        ),
                    });
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        } else {
            child.wait().map_err(|e| HookError {
                hook_path: hook.path.clone(),
                error: e,
            })?
        };

        if !status.success() {
            return Err(HookError {
                hook_path: hook.path.clone(),
                error: io::Error::other(format!(
                    "hook exited with code {:?} (took {:?})",
                    status.code(),
                    start.elapsed()
                )),
            });
        }
    }
    Ok(())
}

// ===========================================================================
// Lifecycle-specific hook execution helpers
// ===========================================================================

pub fn execute_prestart_hooks(
    hooks: Option<&[OciHook]>,
    state: &ContainerState,
) -> Result<(), HookError> {
    execute_hooks(hooks, state)
}

pub fn execute_create_runtime_hooks(
    hooks: Option<&[OciHook]>,
    state: &ContainerState,
) -> Result<(), HookError> {
    execute_hooks(hooks, state)
}

pub fn execute_create_container_hooks(
    hooks: Option<&[OciHook]>,
    state: &ContainerState,
) -> io::Result<()> {
    execute_hooks_in_context(hooks, state)
}

pub fn execute_start_container_hooks(
    hooks: Option<&[OciHook]>,
    state: &ContainerState,
) -> io::Result<()> {
    execute_hooks_in_context(hooks, state)
}

pub fn execute_poststart_hooks(
    hooks: Option<&[OciHook]>,
    state: &ContainerState,
) -> Result<(), HookError> {
    execute_hooks(hooks, state)
}

pub fn execute_poststop_hooks(hooks: Option<&[OciHook]>, state: &ContainerState) {
    let Some(hooks) = hooks else { return };
    let state_json = state.to_json();
    for hook in hooks {
        let timeout_secs = hook.timeout.unwrap_or(0);
        let path = Path::new(&hook.path);
        if !path.exists() {
            let _ = fs::write(
                "/dev/kmsg",
                format!(
                    "edgerun: poststop hook {:?} not found (warning only)",
                    hook.path
                ),
            );
            continue;
        }
        let args = hook.args.as_deref().unwrap_or(&[]);
        let env = hook.env.as_deref().unwrap_or(&[]);
        let mut cmd = Command::new(path);
        cmd.args(args);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::piped());
        for e in env {
            if let Some((k, v)) = e.split_once('=') {
                cmd.env(k, v);
            }
        }
        match cmd.spawn() {
            Ok(mut child) => {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(state_json.as_bytes());
                }
                let timeout = Duration::from_secs(timeout_secs);
                let start = Instant::now();
                let ok = if timeout_secs > 0 {
                    loop {
                        if let Some(s) = child.try_wait().ok().flatten() {
                            break s.success();
                        }
                        if start.elapsed() > timeout {
                            let _ = child.kill();
                            break false;
                        }
                        std::thread::sleep(Duration::from_millis(50));
                    }
                } else {
                    child.wait().is_ok_and(|s| s.success())
                };
                if !ok {
                    let _ = fs::write(
                        "/dev/kmsg",
                        format!(
                            "edgerun: poststop hook {:?} failed (warning only)",
                            hook.path
                        ),
                    );
                }
            }
            Err(e) => {
                let _ = fs::write(
                    "/dev/kmsg",
                    format!(
                        "edgerun: poststop hook {:?} failed: {} (warning only)",
                        hook.path, e
                    ),
                );
            }
        }
    }
}
// ===========================================================================
