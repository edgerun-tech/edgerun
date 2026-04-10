//! OCI container lifecycle — composable phases with full hook support.
//!
//! Follows the OCI runtime spec lifecycle:
//!
//! ```text
//! create  →  (prestart → createRuntime)  →  fork child →  (createContainer → FIFO-wait → startContainer → exec)  →  created
//! start   →  signal FIFO                →  poststart   →  running
//! (process exits)                                                              →  stopped
//! delete  →  (poststop + cgroup cleanup)                                      →  deleted
//! ```
//!
//! Hook execution:
//! - **Runtime namespace** (parent): prestart, createRuntime, poststart, poststop
//! - **Container namespace** (child): createContainer, startContainer
//!
//! Hook failure semantics: error → stop container (except poststop: warn + continue)

use std::fs;
use std::io;
use std::io::Write;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

use crate::json::{OciLinuxResources, OciSpec, OciHook};
use crate::cgroups::setup_cgroups;
use crate::hooks::{
    ContainerState,
    execute_prestart_hooks, execute_create_runtime_hooks,
    execute_create_container_hooks, execute_start_container_hooks,
    execute_poststart_hooks, execute_poststop_hooks,
};
use crate::process::{ContainerConfig, setup_container_child};
use crate::init::pid1_init_script;
use crate::handle::RunningContainer;
use crate::state::{container_state_dir, fifo_path, save_state, ContainerState as StateContainerState};

// ===========================================================================
// Step 1: prestart hooks
// ===========================================================================

/// Run prestart hooks (runtime namespace). Returns hook state for subsequent steps.
pub fn run_prestart_hooks(spec: &OciSpec, container_id: &str) -> io::Result<()> {
    let hooks = spec.linux.as_ref()
        .and_then(|l| l.hooks.as_ref())
        .map(|h| h.clone())
        .unwrap_or_default();

    let state = make_state(spec, container_id, "creating", 0);

    if let Some(ref prestart) = hooks.prestart {
        if !prestart.is_empty() {
            if let Err(e) = execute_prestart_hooks(Some(prestart), &state) {
                return Err(io::Error::new(io::ErrorKind::Other, format!("prestart hook failed: {}", e)));
            }
        }
    }
    Ok(())
}

// ===========================================================================
// Step 2: createRuntime hooks
// ===========================================================================

/// Run createRuntime hooks (runtime namespace).
pub fn run_create_runtime_hooks(spec: &OciSpec, container_id: &str) -> io::Result<()> {
    let hooks = spec.linux.as_ref()
        .and_then(|l| l.hooks.as_ref())
        .map(|h| h.clone())
        .unwrap_or_default();

    let state = make_state(spec, container_id, "creating", 0);

    if let Some(ref create_runtime) = hooks.create_runtime {
        if !create_runtime.is_empty() {
            if let Err(e) = execute_create_runtime_hooks(Some(create_runtime), &state) {
                return Err(io::Error::new(io::ErrorKind::Other, format!("createRuntime hook failed: {}", e)));
            }
        }
    }
    Ok(())
}

// ===========================================================================
// Step 3: fork child — runs setup + createContainer + FIFO-wait + startContainer
// ===========================================================================

/// Fork the container child. The child runs:
/// 1. setup_container_child (namespaces, rootfs, security, etc.)
/// 2. createContainer hooks (container namespace)
/// 3. Waits on FIFO for start signal
/// 4. startContainer hooks (container namespace)
/// 5. execs the workload
///
/// Returns the child PID and a reference to the spec-derived config.
pub fn fork_container_child(spec: &OciSpec, container_id: &str) -> io::Result<ForkedChild> {
    let cfg = ContainerConfig::from_spec(spec)?;
    let bundle_path = cfg.root.path.clone();
    let cgroup_path = spec.linux.as_ref()
        .and_then(|l| l.cgroups_path.as_ref())
        .cloned()
        .unwrap_or_else(|| "/edgerun".into());
    let resources = spec.linux.as_ref().and_then(|l| l.resources.clone());

    // Create state directory and FIFO
    let state_dir = container_state_dir(container_id);
    fs::create_dir_all(&state_dir)?;
    let fifo = fifo_path(container_id);
    let _ = fs::remove_file(&fifo);
    let fifo_cstr = std::ffi::CString::new(fifo.to_string_lossy().as_bytes()).unwrap();
    let mkfifo_ret = unsafe { libc::mkfifo(fifo_cstr.as_ptr(), 0o600) };
    if mkfifo_ret != 0 {
        return Err(io::Error::last_os_error());
    }

    // Extract hooks for the child
    let hooks = spec.linux.as_ref()
        .and_then(|l| l.hooks.as_ref())
        .map(|h| h.clone())
        .unwrap_or_default();

    // Build the workload command
    let process = spec.process.clone().unwrap_or_default();
    let args = process.args.clone().unwrap_or_else(|| vec!["/bin/sh".into()]);
    let env = process.env.clone().unwrap_or_else(|| crate::process::DEFAULT_ENV.iter().map(|s| s.to_string()).collect());
    let cwd = process.cwd.clone().unwrap_or_else(|| "/".into());

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

    // Clone data for pre_exec
    let create_container_hooks = hooks.create_container.clone();
    let start_container_hooks = hooks.start_container.clone();
    let version = spec.version.clone();
    let root_path = cfg.root.path.clone();
    let cc_id = container_id.to_string();
    let fifo_cstr = std::ffi::CString::new(fifo.to_string_lossy().as_bytes()).unwrap();

    unsafe {
        cmd.pre_exec(move || {
            // 1. Standard container setup
            setup_container_child(&cfg)?;

            // 2. createContainer hooks (container namespace)
            let cc_state = ContainerState {
                version: version.clone(),
                id: cc_id.clone(),
                status: "creating".into(),
                pid: 0,
                bundle: root_path.clone(),
                annotations: std::collections::HashMap::new(),
            };
            if let Some(ref hk) = create_container_hooks {
                if !hk.is_empty() {
                    execute_create_container_hooks(Some(hk), &cc_state)?;
                }
            }

            // 3. Wait on FIFO for start signal
            let fifo_fd = libc::open(fifo_cstr.as_ptr(), libc::O_RDONLY);
            if fifo_fd < 0 {
                return Err(io::Error::last_os_error());
            }
            let mut buf = [0u8; 4];
            let n = libc::read(fifo_fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len());
            libc::close(fifo_fd);
            if n <= 0 {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "FIFO closed before start signal"));
            }

            // 4. startContainer hooks (container namespace)
            let sc_state = ContainerState {
                version: version.clone(),
                id: cc_id.clone(),
                status: "created".into(),
                pid: 0,
                bundle: root_path.clone(),
                annotations: std::collections::HashMap::new(),
            };
            if let Some(ref hk) = start_container_hooks {
                if !hk.is_empty() {
                    execute_start_container_hooks(Some(hk), &sc_state)?;
                }
            }

            Ok(())
        });
    }

    let child = cmd.spawn()?;
    let child_pid = child.id();

    Ok(ForkedChild {
        child,
        pid: child_pid,
        bundle_path,
        cgroup_path,
        resources,
        poststop_hooks: hooks.poststop.unwrap_or_default(),
        poststart_hooks: hooks.poststart.unwrap_or_default(),
    })
}

/// Result of forking a container child.
pub struct ForkedChild {
    child: std::process::Child,
    pid: u32,
    bundle_path: String,
    cgroup_path: String,
    resources: Option<OciLinuxResources>,
    poststop_hooks: Vec<OciHook>,
    poststart_hooks: Vec<OciHook>,
}

impl ForkedChild {
    pub fn pid(&self) -> u32 { self.pid }
    pub fn bundle_path(&self) -> &str { &self.bundle_path }
    pub fn cgroup_path(&self) -> &str { &self.cgroup_path }
}

// ===========================================================================
// Step 4: save state as "created"
// ===========================================================================

/// Save the container state as "created".
pub fn save_created_state(spec: &OciSpec, container_id: &str, pid: u32) -> io::Result<()> {
    let state = StateContainerState {
        oci_version: spec.version.clone(),
        id: container_id.to_string(),
        status: "created".to_string(),
        pid: Some(pid),
        bundle: spec.root.as_ref().map(|r| r.path.clone()).unwrap_or_default(),
        annotations: spec.annotations.clone(),
    };
    save_state(&state, container_id)
}

// ===========================================================================
// Step 5: signal start
// ===========================================================================

/// Signal the container child to start by writing to the FIFO.
pub fn signal_start(container_id: &str) -> io::Result<()> {
    let fifo = fifo_path(container_id);
    let mut fifo_file = fs::File::create(&fifo)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("failed to open start FIFO: {}", e)))?;
    let _ = fifo_file.write_all(b"go\n");
    let _ = fifo_file.flush();
    // Keep the FIFO open briefly to ensure the reader gets the data
    std::thread::sleep(std::time::Duration::from_millis(100));
    Ok(())
}

// ===========================================================================
// Step 6: cgroups
// ===========================================================================

/// Set up cgroups for the container.
pub fn setup_container_cgroups(pid: u32, resources: &OciLinuxResources, cgroup_path: &str) {
    if let Err(e) = setup_cgroups(pid, resources, cgroup_path) {
        let _ = std::fs::write("/dev/kmsg", format!("edgerun: cgroup setup failed for PID {}: {}", pid, e));
    }
}

// ===========================================================================
// Step 7: poststart hooks
// ===========================================================================

/// Run poststart hooks (runtime namespace).
pub fn run_poststart_hooks(spec: &OciSpec, container_id: &str, pid: u32) -> io::Result<()> {
    let hooks = spec.linux.as_ref()
        .and_then(|l| l.hooks.as_ref())
        .map(|h| h.clone())
        .unwrap_or_default();

    let state = make_state(spec, container_id, "running", pid);

    if let Some(ref poststart) = hooks.poststart {
        if !poststart.is_empty() {
            if let Err(e) = execute_poststart_hooks(Some(poststart), &state) {
                let _ = unsafe { crate::syscalls::kill(pid as std::os::raw::c_int, crate::syscalls::SIGKILL) };
                return Err(io::Error::new(io::ErrorKind::Other, format!("poststart hook failed: {}", e)));
            }
        }
    }
    Ok(())
}

// ===========================================================================
// Step 8: update state to "running"
// ===========================================================================

/// Update the container state to "running".
pub fn update_state_running(container_id: &str, pid: u32) -> io::Result<()> {
    let state = StateContainerState {
        oci_version: String::new(),
        id: container_id.to_string(),
        status: "running".to_string(),
        pid: Some(pid),
        bundle: String::new(),
        annotations: None,
    };
    // Only update the status field — the state file already exists
    // We need to load and update, not overwrite
    if let Ok(mut existing) = crate::state::load_state(container_id) {
        existing.status = "running".to_string();
        existing.pid = Some(pid);
        crate::state::save_state(&existing, container_id)
    } else {
        save_state(&state, container_id)
    }
}

// ===========================================================================
// Step 9: RunningContainer wrapper
// ===========================================================================

/// Wrap a ForkedChild into a RunningContainer for the library API.
pub fn into_running_container(child: ForkedChild) -> RunningContainer {
    RunningContainer {
        child: child.child,
        cgroup_path: child.cgroup_path,
        bundle_path: child.bundle_path,
        pid: child.pid,
        poststop_hooks: child.poststop_hooks,
    }
}

// ===========================================================================
// Delete
// ===========================================================================

/// Run poststop hooks and clean up cgroups.
pub fn run_poststop_and_cleanup(_container_id: &str, pid: u32, bundle_path: &str, cgroup_path: &str, spec: &OciSpec) {
    // Poststop hooks
    let hooks = spec.linux.as_ref()
        .and_then(|l| l.hooks.as_ref())
        .map(|h| h.clone())
        .unwrap_or_default();

    let state = ContainerState {
        version: String::new(),
        id: String::new(),
        status: "stopped".into(),
        pid,
        bundle: bundle_path.to_string(),
        annotations: std::collections::HashMap::new(),
    };

    if let Some(ref poststop) = hooks.poststop {
        if !poststop.is_empty() {
            execute_poststop_hooks(Some(poststop), &state);
        }
    }

    // Clean up cgroup directory
    if !cgroup_path.is_empty() {
        let cgroup_dir = Path::new("/sys/fs/cgroup").join(cgroup_path.trim_start_matches('/'));
        if cgroup_dir.exists() {
            // First try to kill all processes in the cgroup (cgroup v2)
            let _ = fs::write(cgroup_dir.join("cgroup.kill"), "1");
            // Give processes a moment to exit
            std::thread::sleep(std::time::Duration::from_millis(50));
            // Now remove the directory
            let _ = std::fs::remove_dir_all(&cgroup_dir);
        }
    }
}

// ===========================================================================
// Convenience: full create+start in one call (library API)
// ===========================================================================

/// Run a container from a parsed OCI spec (blocking).
pub fn run_spec(spec: &OciSpec) -> io::Result<std::process::ExitStatus> {
    run_spec_with_id(spec, "")
}

/// Run a container from a parsed OCI spec with a container ID (blocking).
pub fn run_spec_with_id(spec: &OciSpec, container_id: &str) -> io::Result<std::process::ExitStatus> {
    run_prestart_hooks(spec, container_id)?;
    run_create_runtime_hooks(spec, container_id)?;
    let child = fork_container_child(spec, container_id)?;
    let pid = child.pid();
    let bundle = child.bundle_path().to_string();
    let cgroup = child.cgroup_path().to_string();
    let running = start_created_container_internal(child, spec, container_id)?;
    let mut running = running;
    let result = running.child.wait()?;
    run_poststop_and_cleanup(container_id, pid, &bundle, &cgroup, spec);
    Ok(result)
}

/// Start a container without blocking.
pub fn start_spec(spec: &OciSpec) -> io::Result<RunningContainer> {
    start_spec_with_id(spec, "")
}

/// Start a container without blocking, with a container ID.
pub fn start_spec_with_id(spec: &OciSpec, container_id: &str) -> io::Result<RunningContainer> {
    run_prestart_hooks(spec, container_id)?;
    run_create_runtime_hooks(spec, container_id)?;
    let child = fork_container_child(spec, container_id)?;
    start_created_container_internal(child, spec, container_id)
}

/// Start a forked child — signals FIFO, sets up cgroups, runs poststart hooks.
fn start_created_container_internal(child: ForkedChild, spec: &OciSpec, container_id: &str) -> io::Result<RunningContainer> {
    let pid = child.pid();
    let resources = child.resources.clone();
    let cgroup_path = child.cgroup_path().to_string();
    let poststop_hooks = child.poststop_hooks.clone();
    let _poststart_hooks = child.poststart_hooks.clone();
    let bundle_path = child.bundle_path().to_string();

    // Signal FIFO
    signal_start(container_id)?;

    // Cgroups
    if let Some(ref res) = resources {
        setup_container_cgroups(pid, res, &cgroup_path);
    }

    // Poststart hooks
    run_poststart_hooks(spec, container_id, pid)?;

    // Update state
    update_state_running(container_id, pid)?;

    Ok(RunningContainer {
        child: child.child,
        cgroup_path,
        bundle_path,
        pid,
        poststop_hooks,
    })
}

/// Delete a container and run poststop hooks.
pub fn delete_container(container: RunningContainer) {
    let _ = delete_container_internal(
        container.pid,
        &container.bundle_path,
        &container.cgroup_path,
        &container.poststop_hooks,
    );
}

fn delete_container_internal(
    pid: u32,
    bundle_path: &str,
    cgroup_path: &str,
    poststop_hooks: &[OciHook],
) -> io::Result<()> {
    let state = ContainerState {
        version: String::new(),
        id: String::new(),
        status: "stopped".into(),
        pid,
        bundle: bundle_path.to_string(),
        annotations: std::collections::HashMap::new(),
    };

    execute_poststop_hooks(Some(poststop_hooks), &state);

    if !cgroup_path.is_empty() {
        let cgroup_dir = Path::new("/sys/fs/cgroup").join(cgroup_path.trim_start_matches('/'));
        if cgroup_dir.exists() {
            let _ = std::fs::remove_dir_all(&cgroup_dir);
        }
    }

    Ok(())
}

// ===========================================================================
// Helpers
// ===========================================================================

fn make_state(spec: &OciSpec, container_id: &str, status: &str, pid: u32) -> ContainerState {
    ContainerState {
        version: spec.version.clone(),
        id: container_id.to_string(),
        status: status.into(),
        pid,
        bundle: spec.root.as_ref().map(|r| r.path.clone()).unwrap_or_default(),
        annotations: spec.linux.as_ref()
            .and_then(|l| l.sysctl.as_ref())
            .map(|sysctl| {
                sysctl.iter()
                    .map(|(k, v)| (format!("sysctl:{}", k), v.clone()))
                    .collect()
            })
            .unwrap_or_default(),
    }
}
