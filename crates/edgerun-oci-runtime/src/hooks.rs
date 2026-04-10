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

use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::json::OciHook;

/// Container state passed to hooks via stdin.
/// Per OCI runtime spec: the state of the container MUST be passed to hooks
/// over stdin so that they may do work appropriate to the current state.
#[derive(Debug, Clone)]
pub struct ContainerState {
    pub version: String,
    pub id: String,
    pub status: String, // "creating", "created", "running", "stopped"
    pub pid: u32,
    pub bundle: String,
    pub annotations: HashMap<String, String>,
}

impl ContainerState {
    /// Serialize to JSON for hook stdin.
    /// Format matches OCI runtime spec state schema.
    pub fn to_json(&self) -> String {
        let mut json = String::from("{\n");
        json.push_str(&format!("  \"ociVersion\": \"{}\",\n", self.version));
        json.push_str(&format!("  \"id\": \"{}\",\n", self.id));
        json.push_str(&format!("  \"status\": \"{}\",\n", self.status));
        json.push_str(&format!("  \"pid\": {},\n", self.pid));
        json.push_str(&format!("  \"bundle\": \"{}\",\n", self.bundle));
        if !self.annotations.is_empty() {
            json.push_str("  \"annotations\": {\n");
            let entries: Vec<_> = self.annotations.iter()
                .map(|(k, v)| format!("    \"{}\": \"{}\"", k, v))
                .collect();
            json.push_str(&entries.join(",\n"));
            json.push_str("\n  }\n");
        } else {
            json.push_str("  \"annotations\": {}\n");
        }
        json.push_str("}\n");
        json
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

/// Execute a list of hooks with container state on stdin.
///
/// Hooks are executed in order. If any hook fails (non-zero exit or timeout),
/// execution stops and an error is returned.
///
/// This is the generic hook executor used for runtime-namespace hooks.
pub fn execute_hooks(
    hooks: Option<&[OciHook]>,
    state: &ContainerState,
) -> Result<(), HookError> {
    let Some(hooks) = hooks else { return Ok(()) };

    let state_json = state.to_json();

    for hook in hooks {
        let timeout_secs = hook.timeout.unwrap_or(0);

        let start = Instant::now();
        let result = run_hook(hook, &state_json, timeout_secs);

        match result {
            Ok(exit_status) => {
                if !exit_status.success() {
                    return Err(HookError {
                        hook_path: hook.path.clone(),
                        error: io::Error::new(
                            io::ErrorKind::Other,
                            format!(
                                "hook exited with code {:?} (took {:?})",
                                exit_status.code(),
                                start.elapsed(),
                            ),
                        ),
                    });
                }
            }
            Err(e) => {
                return Err(HookError {
                    hook_path: hook.path.clone(),
                    error: e,
                });
            }
        }
    }

    Ok(())
}

/// Execute a single hook with container state on stdin.
fn run_hook(hook: &OciHook, state_json: &str, timeout_secs: u64) -> io::Result<std::process::ExitStatus> {
    let path = Path::new(&hook.path);
    if !path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("hook not found: {}", hook.path),
        ));
    }

    let args = hook.args.as_deref().unwrap_or(&[]);
    let env = hook.env.as_deref().unwrap_or(&[]);

    let mut cmd = Command::new(path);
    cmd.args(args);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::piped());

    // Inherit current environment, then override with hook env
    for e in env {
        if let Some((k, v)) = e.split_once('=') {
            cmd.env(k, v);
        }
    }

    let mut child = cmd.spawn()?;

    // Write state to hook's stdin
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(state_json.as_bytes());
        // Drop stdin to signal EOF to the hook process
    }

    // Wait for hook to complete with optional timeout
    let exit_status = if timeout_secs > 0 {
        let timeout = Duration::from_secs(timeout_secs);
        let start = Instant::now();
        loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if start.elapsed() > timeout {
                // Kill the hook on timeout
                let _ = child.kill();
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    format!("hook {:?} timed out after {}s", hook.path, timeout_secs),
                ));
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    } else {
        child.wait()?
    };

    Ok(exit_status)
}

/// Execute hooks that run in the container namespace (inside pre_exec).
///
/// This is used for createContainer and startContainer hooks that must run
/// inside the container's namespace context. The hook executable path is
/// resolved from the runtime namespace, but execution happens inside the
/// container namespace.
///
/// Since this runs in pre_exec, it uses a synchronous blocking approach.
pub fn execute_hooks_in_context(
    hooks: Option<&[OciHook]>,
    state: &ContainerState,
) -> io::Result<()> {
    let Some(hooks) = hooks else { return Ok(()) };

    let state_json = state.to_json();

    for hook in hooks {
        let path = Path::new(&hook.path);
        if !path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("hook not found: {}", hook.path),
            ));
        }

        let args = hook.args.as_deref().unwrap_or(&[]);
        let env = hook.env.as_deref().unwrap_or(&[]);

        let mut cmd = Command::new(path);
        cmd.args(args);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());

        // Inherit current environment, then override with hook env
        for e in env {
            if let Some((k, v)) = e.split_once('=') {
                cmd.env(k, v);
            }
        }

        let mut child = cmd.spawn()?;

        // Write state to hook's stdin
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(state_json.as_bytes());
        }

        let timeout_secs = hook.timeout.unwrap_or(0);
        let status = if timeout_secs > 0 {
            let timeout = Duration::from_secs(timeout_secs);
            let start = Instant::now();
            loop {
                if let Some(s) = child.try_wait()? {
                    break s;
                }
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        format!("hook {:?} timed out after {}s", hook.path, timeout_secs),
                    ));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        } else {
            child.wait()?
        };

        if !status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("hook {:?} failed with exit code {:?}", hook.path, status.code()),
            ));
        }
    }

    Ok(())
}

// ===========================================================================
// Lifecycle-specific hook execution helpers
// ===========================================================================

/// Execute prestart hooks (deprecated, but still supported).
/// Called during create, after runtime env created, before pivot_root.
/// Runs in runtime namespace.
pub fn execute_prestart_hooks(
    hooks: Option<&[OciHook]>,
    state: &ContainerState,
) -> Result<(), HookError> {
    execute_hooks(hooks, state)
}

/// Execute createRuntime hooks.
/// Called during create, after runtime env created, before pivot_root.
/// Runs in runtime namespace.
pub fn execute_create_runtime_hooks(
    hooks: Option<&[OciHook]>,
    state: &ContainerState,
) -> Result<(), HookError> {
    execute_hooks(hooks, state)
}

/// Execute createContainer hooks.
/// Called during create, after runtime env created, before pivot_root.
/// Runs in container namespace (must be called from inside pre_exec).
pub fn execute_create_container_hooks(
    hooks: Option<&[OciHook]>,
    state: &ContainerState,
) -> io::Result<()> {
    execute_hooks_in_context(hooks, state)
}

/// Execute startContainer hooks.
/// Called during start, before user process exec.
/// Runs in container namespace (must be called from inside pre_exec).
pub fn execute_start_container_hooks(
    hooks: Option<&[OciHook]>,
    state: &ContainerState,
) -> io::Result<()> {
    execute_hooks_in_context(hooks, state)
}

/// Execute poststart hooks.
/// Called after user process started, before start returns.
/// Runs in runtime namespace.
pub fn execute_poststart_hooks(
    hooks: Option<&[OciHook]>,
    state: &ContainerState,
) -> Result<(), HookError> {
    execute_hooks(hooks, state)
}

/// Execute poststop hooks.
/// Called after container deleted, before delete returns.
/// Runs in runtime namespace.
/// Per OCI spec: if poststop hook fails, log warning but continue.
pub fn execute_poststop_hooks(
    hooks: Option<&[OciHook]>,
    state: &ContainerState,
) {
    let Some(hooks) = hooks else { return };

    let state_json = state.to_json();

    for hook in hooks {
        let timeout_secs = hook.timeout.unwrap_or(0);
        let result = run_hook(hook, &state_json, timeout_secs);

        match result {
            Ok(exit_status) => {
                if !exit_status.success() {
                    // Per OCI spec: log warning, but continue
                    let _ = fs::write(
                        "/dev/kmsg",
                        format!(
                            "edgerun: poststop hook {:?} exited with code {:?} (warning only)",
                            hook.path,
                            exit_status.code(),
                        ),
                    );
                }
            }
            Err(e) => {
                // Per OCI spec: log warning, but continue
                let _ = fs::write(
                    "/dev/kmsg",
                    format!("edgerun: poststop hook {:?} failed: {} (warning only)", hook.path, e),
                );
            }
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn container_state_to_json_is_valid() {
        let mut annotations = HashMap::new();
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
        assert!(json.contains("\"ociVersion\": \"1.0.2\""));
        assert!(json.contains("\"id\": \"test-container\""));
        assert!(json.contains("\"status\": \"created\""));
        assert!(json.contains("\"pid\": 12345"));
        assert!(json.contains("\"bundle\": \"/var/lib/bundles/test\""));
        assert!(json.contains("\"key\": \"value\""));
    }

    #[test]
    fn container_state_empty_annotations() {
        let state = ContainerState {
            version: "1.0.2".into(),
            id: "test".into(),
            status: "running".into(),
            pid: 1,
            bundle: "/rootfs".into(),
            annotations: HashMap::new(),
        };
        let json = state.to_json();
        assert!(json.contains("\"annotations\": {}"));
    }

    #[test]
    fn oci_hooks_deserializes_all_types() {
        let json = r#"{"prestart":[{"path":"/usr/bin/prestart"}],"createRuntime":[{"path":"/usr/bin/create-runtime","args":["arg1"],"env":["FOO=bar"],"timeout":10}],"createContainer":[{"path":"/usr/bin/create-container"}],"startContainer":[{"path":"/usr/bin/start-container"}],"poststart":[{"path":"/usr/bin/poststart","timeout":5}],"poststop":[{"path":"/usr/bin/poststop"}]}"#;

        let hooks: crate::json::OciHooks = edgerun_json::from_slice::<crate::json::OciHooks>(json.as_bytes()).unwrap();

        assert!(hooks.prestart.is_some());
        assert_eq!(hooks.prestart.as_ref().unwrap().len(), 1);
        assert_eq!(hooks.prestart.as_ref().unwrap()[0].path, "/usr/bin/prestart");

        assert!(hooks.create_runtime.is_some());
        let cr = hooks.create_runtime.as_ref().unwrap();
        assert_eq!(cr.len(), 1);
        assert_eq!(cr[0].path, "/usr/bin/create-runtime");
        assert_eq!(cr[0].args, Some(vec!["arg1".into()]));
        assert_eq!(cr[0].env, Some(vec!["FOO=bar".into()]));
        assert_eq!(cr[0].timeout, Some(10));

        assert!(hooks.create_container.is_some());
        assert!(hooks.start_container.is_some());
        assert!(hooks.poststart.is_some());
        assert!(hooks.poststop.is_some());
    }

    #[test]
    fn oci_hooks_deserializes_empty() {
        let json = r#"{}"#;
        let hooks: crate::json::OciHooks = edgerun_json::from_slice::<crate::json::OciHooks>(json.as_bytes()).unwrap();
        assert!(hooks.prestart.is_none());
        assert!(hooks.create_runtime.is_none());
        assert!(hooks.create_container.is_none());
        assert!(hooks.start_container.is_none());
        assert!(hooks.poststart.is_none());
        assert!(hooks.poststop.is_none());
    }

    #[test]
    fn execute_hooks_empty_is_ok() {
        let state = ContainerState {
            version: "1.0.2".into(),
            id: "test".into(),
            status: "created".into(),
            pid: 1,
            bundle: "/rootfs".into(),
            annotations: HashMap::new(),
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
            annotations: HashMap::new(),
        };

        // Hook with non-existent path should fail
        let hooks = vec![crate::json::OciHook {
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
            annotations: HashMap::new(),
        };

        // Use /bin/true which always exits 0
        let hooks = vec![crate::json::OciHook {
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
            annotations: HashMap::new(),
        };

        // /bin/false always exits 1
        let hooks = vec![crate::json::OciHook {
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
            annotations: HashMap::new(),
        };

        // Even with failing hooks, poststop should not return an error
        let hooks = vec![
            crate::json::OciHook {
                path: "/bin/false".into(),
                args: None,
                env: None,
                timeout: None,
            },
            crate::json::OciHook {
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
            annotations: HashMap::new(),
        };

        // false should fail, true should never run
        let hooks = vec![
            crate::json::OciHook {
                path: "/bin/false".into(),
                args: None,
                env: None,
                timeout: None,
            },
            crate::json::OciHook {
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
