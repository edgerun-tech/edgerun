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
#[derive(Debug, Clone)]
pub struct ContainerState {
    pub version: String,
    pub id: String,
    pub status: String,
    pub pid: u32,
    pub bundle: String,
    pub annotations: HashMap<String, String>,
}

impl ContainerState {
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

/// Execute hooks in runtime namespace context.
pub fn execute_hooks(
    hooks: Option<&[OciHook]>,
    state: &ContainerState,
) -> Result<(), HookError> {
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
            return Err(HookError { hook_path: hook.path.clone(),
                error: io::Error::new(io::ErrorKind::NotFound, format!("hook not found: {}", hook.path)) });
        }

        let args = hook.args.as_deref().unwrap_or(&[]);
        let env = hook.env.as_deref().unwrap_or(&[]);

        let mut cmd = Command::new(&hook.path);
        cmd.args(args);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::null());
        if capture_stderr { cmd.stderr(Stdio::piped()); } else { cmd.stderr(Stdio::null()); }
        for e in env { if let Some((k, v)) = e.split_once('=') { cmd.env(k, v); } }

        let mut child = cmd.spawn().map_err(|e| HookError { hook_path: hook.path.clone(), error: e })?;
        if let Some(mut stdin) = child.stdin.take() { let _ = stdin.write_all(state_json.as_bytes()); }

        let timeout_secs = hook.timeout.unwrap_or(0);
        let start = Instant::now();
        let status = if timeout_secs > 0 {
            let timeout = Duration::from_secs(timeout_secs);
            loop {
                if let Some(s) = child.try_wait().map_err(|e| HookError { hook_path: hook.path.clone(), error: e })? { break s; }
                if start.elapsed() > timeout { let _ = child.kill(); return Err(HookError {
                    hook_path: hook.path.clone(),
                    error: io::Error::new(io::ErrorKind::TimedOut, format!("hook {:?} timed out after {}s", hook.path, timeout_secs)) }); }
                std::thread::sleep(Duration::from_millis(50));
            }
        } else { child.wait().map_err(|e| HookError { hook_path: hook.path.clone(), error: e })? };

        if !status.success() {
            return Err(HookError { hook_path: hook.path.clone(),
                error: io::Error::new(io::ErrorKind::Other, format!("hook exited with code {:?} (took {:?})", status.code(), start.elapsed())) });
        }
    }
    Ok(())
}

// ===========================================================================
// Lifecycle-specific hook execution helpers
// ===========================================================================

pub fn execute_prestart_hooks(hooks: Option<&[OciHook]>, state: &ContainerState) -> Result<(), HookError> {
    execute_hooks(hooks, state)
}

pub fn execute_create_runtime_hooks(hooks: Option<&[OciHook]>, state: &ContainerState) -> Result<(), HookError> {
    execute_hooks(hooks, state)
}

pub fn execute_create_container_hooks(hooks: Option<&[OciHook]>, state: &ContainerState) -> io::Result<()> {
    execute_hooks_in_context(hooks, state)
}

pub fn execute_start_container_hooks(hooks: Option<&[OciHook]>, state: &ContainerState) -> io::Result<()> {
    execute_hooks_in_context(hooks, state)
}

pub fn execute_poststart_hooks(hooks: Option<&[OciHook]>, state: &ContainerState) -> Result<(), HookError> {
    execute_hooks(hooks, state)
}

pub fn execute_poststop_hooks(hooks: Option<&[OciHook]>, state: &ContainerState) {
    let Some(hooks) = hooks else { return };
    let state_json = state.to_json();
    for hook in hooks {
        let timeout_secs = hook.timeout.unwrap_or(0);
        let path = Path::new(&hook.path);
        if !path.exists() { let _ = fs::write("/dev/kmsg", format!("edgerun: poststop hook {:?} not found (warning only)", hook.path)); continue; }
        let args = hook.args.as_deref().unwrap_or(&[]);
        let env = hook.env.as_deref().unwrap_or(&[]);
        let mut cmd = Command::new(path);
        cmd.args(args); cmd.stdin(Stdio::piped()); cmd.stdout(Stdio::null()); cmd.stderr(Stdio::piped());
        for e in env { if let Some((k, v)) = e.split_once('=') { cmd.env(k, v); } }
        match cmd.spawn() {
            Ok(mut child) => {
                if let Some(mut stdin) = child.stdin.take() { let _ = stdin.write_all(state_json.as_bytes()); }
                let timeout = Duration::from_secs(timeout_secs);
                let start = Instant::now();
                let ok = if timeout_secs > 0 {
                    loop { if let Some(s) = child.try_wait().ok().flatten() { break s.success(); }
                        if start.elapsed() > timeout { let _ = child.kill(); break false; }
                        std::thread::sleep(Duration::from_millis(50)); }
                } else { child.wait().map_or(false, |s| s.success()) };
                if !ok { let _ = fs::write("/dev/kmsg", format!("edgerun: poststop hook {:?} failed (warning only)", hook.path)); }
            }
            Err(e) => { let _ = fs::write("/dev/kmsg", format!("edgerun: poststop hook {:?} failed: {} (warning only)", hook.path, e)); }
        }
    }
}
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
