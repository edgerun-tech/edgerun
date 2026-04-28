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
use edgerun_json::{JsonValue, Map};

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
        edgerun_json::to_string(&self.to_json_value()).unwrap_or_default()
    }

    fn to_json_value(&self) -> JsonValue {
        let annotations = Map::from_iter(
            self.annotations
                .iter()
                .map(|(key, value)| (key.as_str(), value.as_str())),
        );
        let mut object = Map::new();
        object.push_field("ociVersion", self.version.as_str());
        object.push_field("id", self.id.as_str());
        object.push_field("status", self.status.as_str());
        object.push_field("pid", self.pid);
        object.push_field("bundle", self.bundle.as_str());
        object.push_field("annotations", JsonValue::Object(annotations));
        JsonValue::Object(object)
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
#[cfg(all(test, not(target_os = "none")))]
mod tests {
    use super::*;

    #[test]
    fn container_state_to_json_is_valid() {
        let mut annotations = BTreeMap::new();
        annotations.insert("key".into(), "value".into());

        let state = ContainerState {
            version: "1.0.2".into(),
            id: "test-container".into(),
            status: "created".into(),
            pid: 12345,
            bundle: "/var/lib/bundles/test".into(),
            annotations,
        };

        let json = state.to_json();
        let value = edgerun_json::parse_json(&json).unwrap();
        let object = value.as_object().unwrap();
        assert_eq!(object.get_str("ociVersion"), Some("1.0.2"));
        assert_eq!(object.get_str("id"), Some("test-container"));
        assert_eq!(object.get_str("status"), Some("created"));
        assert_eq!(object.get_u32("pid"), Some(12345));
        assert_eq!(object.get_str("bundle"), Some("/var/lib/bundles/test"));
        assert_eq!(
            object.get_object("annotations").unwrap().get_str("key"),
            Some("value")
        );
    }

    #[test]
    fn container_state_empty_annotations() {
        let state = ContainerState {
            version: "1.0.2".into(),
            id: "test".into(),
            status: "running".into(),
            pid: 1,
            bundle: "/rootfs".into(),
            annotations: BTreeMap::new(),
        };
        let json = state.to_json();
        let value = edgerun_json::parse_json(&json).unwrap();
        assert!(value
            .as_object()
            .unwrap()
            .get_object("annotations")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn container_state_json_escapes_strings() {
        let mut annotations = BTreeMap::new();
        annotations.insert("quoted\"key".into(), "line\nvalue".into());

        let state = ContainerState {
            version: "1.0.2".into(),
            id: "test\"container".into(),
            status: "created".into(),
            pid: 12345,
            bundle: "/var/lib/bundles/test".into(),
            annotations,
        };

        let value = edgerun_json::parse_json(&state.to_json()).unwrap();
        let object = value.as_object().unwrap();
        assert_eq!(object.get_str("id"), Some("test\"container"));
        assert_eq!(
            object
                .get_object("annotations")
                .unwrap()
                .get_str("quoted\"key"),
            Some("line\nvalue")
        );
    }

    #[test]
    fn execute_hooks_empty_is_ok() {
        let state = ContainerState {
            version: "1.0.2".into(),
            id: "test".into(),
            status: "created".into(),
            pid: 1,
            bundle: "/rootfs".into(),
            annotations: BTreeMap::new(),
        };
        assert!(execute_hooks(None, &state).is_ok());
        assert!(execute_hooks(Some(&[]), &state).is_ok());
    }

    #[test]
    fn execute_hooks_failing_hook_returns_error() {
        let state = ContainerState {
            version: "1.0.2".into(),
            id: "test".into(),
            status: "created".into(),
            pid: 1,
            bundle: "/rootfs".into(),
            annotations: BTreeMap::new(),
        };

        // Hook with non-existent path should fail
        let hooks = vec![crate::spec::OciHook {
            path: "/usr/bin/nonexistent-hook".into(),
            args: None,
            env: None,
            timeout: None,
        }];
        let result = execute_hooks(Some(&hooks), &state);
        assert!(result.is_err());
    }

    #[test]
    fn execute_hooks_successful_hook() {
        let state = ContainerState {
            version: "1.0.2".into(),
            id: "test".into(),
            status: "created".into(),
            pid: 1,
            bundle: "/rootfs".into(),
            annotations: BTreeMap::new(),
        };

        // Use /bin/true which always exits 0
        let hooks = vec![crate::spec::OciHook {
            path: "/bin/true".into(),
            args: None,
            env: None,
            timeout: None,
        }];
        let result = execute_hooks(Some(&hooks), &state);
        assert!(result.is_ok());
    }

    #[test]
    fn execute_hooks_hook_with_nonzero_exit_fails() {
        let state = ContainerState {
            version: "1.0.2".into(),
            id: "test".into(),
            status: "created".into(),
            pid: 1,
            bundle: "/rootfs".into(),
            annotations: BTreeMap::new(),
        };

        // /bin/false always exits 1
        let hooks = vec![crate::spec::OciHook {
            path: "/bin/false".into(),
            args: None,
            env: None,
            timeout: None,
        }];
        let result = execute_hooks(Some(&hooks), &state);
        assert!(result.is_err());
    }

    #[test]
    fn execute_poststop_hooks_never_fails() {
        let state = ContainerState {
            version: "1.0.2".into(),
            id: "test".into(),
            status: "stopped".into(),
            pid: 1,
            bundle: "/rootfs".into(),
            annotations: BTreeMap::new(),
        };

        // Even with failing hooks, poststop should not return an error
        let hooks = vec![
            crate::spec::OciHook {
                path: "/bin/false".into(),
                args: None,
                env: None,
                timeout: None,
            },
            crate::spec::OciHook {
                path: "/bin/true".into(),
                args: None,
                env: None,
                timeout: None,
            },
        ];
        // Should not panic — poststop logs warnings but continues
        execute_poststop_hooks(Some(&hooks), &state);
    }

    #[test]
    fn execute_hooks_stops_on_first_failure() {
        let state = ContainerState {
            version: "1.0.2".into(),
            id: "test".into(),
            status: "created".into(),
            pid: 1,
            bundle: "/rootfs".into(),
            annotations: BTreeMap::new(),
        };

        // false should fail, true should never run
        let hooks = vec![
            crate::spec::OciHook {
                path: "/bin/false".into(),
                args: None,
                env: None,
                timeout: None,
            },
            crate::spec::OciHook {
                path: "/bin/true".into(),
                args: None,
                env: None,
                timeout: None,
            },
        ];
        let result = execute_hooks(Some(&hooks), &state);
        assert!(result.is_err());
        // The error should be about /bin/false, not /bin/true
        assert!(result.unwrap_err().hook_path.contains("false"));
    }
}
