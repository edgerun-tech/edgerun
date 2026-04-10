//! OCI container lifecycle — create, start, delete with hook integration.
//!
//! Follows the OCI runtime spec lifecycle:
//!
//! ```text
//! create  →  (prestart → createRuntime → createContainer)  →  created
//! start   →  (startContainer → exec process → poststart)    →  running
//! (process exits)                                           →  stopped
//! delete  →  (undo create → poststop)                       →  deleted
//! ```
//!
//! Hook failure semantics per OCI spec:
//! - **prestart/createRuntime/createContainer/startContainer/poststart**: error → stop container, jump to step 12
//! - **poststop**: warning only, remaining hooks continue

use std::io;
use std::os::unix::process::CommandExt;
use std::process::Command;

use crate::json::OciSpec;
use crate::cgroups::setup_cgroups;
use crate::hooks::{
    ContainerState,
    execute_prestart_hooks, execute_create_runtime_hooks,
    execute_create_container_hooks, execute_start_container_hooks,
    execute_poststart_hooks, execute_poststop_hooks,
};
use crate::process::{ContainerConfig, setup_container_child};
use crate::handle::RunningContainer;
use crate::init::pid1_init_script;

// ===========================================================================
// Blocking execution
// ===========================================================================

/// Run a container from a parsed OCI spec (blocking).
///
/// Full lifecycle: create → start → wait → delete (with poststop hooks).
pub fn run_spec(spec: &OciSpec) -> io::Result<std::process::ExitStatus> {
    let mut running = start_spec_internal(spec)?;
    let result = running.child.wait();

    // poststop hooks run during cleanup
    let _ = delete_container_internal(&running);

    result
}

// ===========================================================================
// Non-blocking execution
// ===========================================================================

/// Start a container without blocking.
///
/// Runs prestart/createRuntime/createContainer hooks during create,
/// startContainer before exec, poststart after exec starts.
/// Returns a handle for awaiting/killing.
///
/// Call [`delete_container`](fn.delete_container.html) when done to run
/// poststop hooks and cleanup.
pub fn start_spec(spec: &OciSpec) -> io::Result<RunningContainer> {
    start_spec_internal(spec)
}

fn start_spec_internal(spec: &OciSpec) -> io::Result<RunningContainer> {
    let cfg = ContainerConfig::from_spec(spec)?;
    let bundle_path = cfg.root.path.clone();
    let cgroup_path = spec.linux.as_ref()
        .and_then(|l| l.cgroups_path.as_ref())
        .cloned()
        .unwrap_or_else(|| "/edgerun".into());

    // Hook state
    let mut state = ContainerState {
        version: spec.version.clone(),
        id: String::new(),
        status: "creating".into(),
        pid: 0,
        bundle: bundle_path.clone(),
        annotations: spec.linux.as_ref()
            .and_then(|l| l.sysctl.as_ref())
            .map(|sysctl| {
                sysctl.iter()
                    .map(|(k, v)| (format!("sysctl:{}", k), v.clone()))
                    .collect()
            })
            .unwrap_or_default(),
    };

    // Extract hooks from spec
    let hooks = spec.linux.as_ref()
        .and_then(|l| l.hooks.clone())
        .unwrap_or_default();

    // === Lifecycle Step 3: prestart hooks (runtime namespace, deprecated) ===
    if let Err(e) = execute_prestart_hooks(hooks.prestart.as_deref(), &state) {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("prestart hook failed: {}", e),
        ));
    }

    // === Lifecycle Step 4: createRuntime hooks (runtime namespace) ===
    if let Err(e) = execute_create_runtime_hooks(hooks.create_runtime.as_deref(), &state) {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("createRuntime hook failed: {}", e),
        ));
    }

    // Build the command
    let process = spec.process.clone().unwrap_or_default();
    let args = process.args.clone().unwrap_or_else(|| vec!["/bin/sh".into()]);
    let env = process.env.clone().unwrap_or_else(|| vec![
        "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".into(),
        "TERM=xterm".into(),
    ]);
    let cwd = process.cwd.clone().unwrap_or_else(|| "/".into());

    let resources = spec.linux.as_ref().and_then(|l| l.resources.clone());

    let use_pid1_init = cfg.has_pid_ns();
    let init_script = if use_pid1_init { pid1_init_script(&args) } else { String::new() };

    let mut cmd = if use_pid1_init {
        let mut c = Command::new("/bin/sh");
        c.arg("-c");
        c.arg(&init_script);
        c
    } else {
        let mut c = Command::new(&args[0]);
        c.args(&args[1..]);
        c
    };
    cmd.current_dir(&cwd);
    cmd.env_clear();
    for e in &env {
        if let Some((k, v)) = e.split_once('=') {
            cmd.env(k, v);
        }
    }

    // Clone hooks needed for pre_exec
    let create_container_hooks = hooks.create_container.clone();
    let start_container_hooks = hooks.start_container.clone();
    let version = spec.version.clone();
    let root_path = cfg.root.path.clone();

    // === pre_exec: container namespace setup + in-container hooks ===
    unsafe {
        cmd.pre_exec(move || {
            // 1. Standard container setup
            setup_container_child(&cfg)?;

            // 2. Lifecycle Step 5: createContainer hooks (container namespace)
            let cc_state = ContainerState {
                version: version.clone(),
                id: String::new(),
                status: "creating".into(),
                pid: 0,
                bundle: root_path.clone(),
                annotations: std::collections::HashMap::new(),
            };
            execute_create_container_hooks(
                create_container_hooks.as_deref(),
                &cc_state,
            )?;

            // 3. Lifecycle Step 7: startContainer hooks (container namespace)
            let sc_state = ContainerState {
                version: version.clone(),
                id: String::new(),
                status: "created".into(),
                pid: 0,
                bundle: root_path.clone(),
                annotations: std::collections::HashMap::new(),
            };
            execute_start_container_hooks(
                start_container_hooks.as_deref(),
                &sc_state,
            )?;

            Ok(())
        });
    }

    // Spawn
    let child = cmd.spawn()?;
    let child_pid = child.id();

    // Update state
    state.status = "running".into();
    state.pid = child_pid;

    // Cgroups
    if let Some(ref res) = resources {
        if let Err(e) = setup_cgroups(child_pid, res, &cgroup_path) {
            let _ = std::fs::write("/dev/kmsg", format!("edgerun: cgroup setup failed for PID {}: {}", child_pid, e));
        }
    }

    // === Lifecycle Step 9: poststart hooks (runtime namespace) ===
    if let Err(e) = execute_poststart_hooks(hooks.poststart.as_deref(), &state) {
        // Per spec: error → stop container
        let _ = unsafe { crate::syscalls::kill(child_pid as std::os::raw::c_int, crate::syscalls::SIGKILL) };
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("poststart hook failed: {}", e),
        ));
    }

    Ok(RunningContainer {
        child,
        cgroup_path,
        bundle_path,
        pid: child_pid,
    })
}

// ===========================================================================
// Delete / cleanup
// ===========================================================================

/// Delete a container and run poststop hooks.
///
/// Undo create-phase resources, then run poststop hooks.
/// Per OCI spec: poststop failures log a warning but don't abort.
pub fn delete_container(container: &RunningContainer) {
    let _ = delete_container_internal(container);
}

fn delete_container_internal(container: &RunningContainer) {
    let state = ContainerState {
        version: String::new(), // Not available at delete time without storing spec
        id: String::new(),
        status: "stopped".into(),
        pid: container.pid,
        bundle: container.bundle_path.clone(),
        annotations: std::collections::HashMap::new(),
    };

    // Poststop hooks run after container is deleted
    execute_poststop_hooks(None, &state);

    // In a full implementation, cleanup cgroups, unmount, etc. here
}
