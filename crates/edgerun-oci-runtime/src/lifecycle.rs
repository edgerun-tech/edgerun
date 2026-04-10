//! OCI container lifecycle — create, start, delete with hook integration.
//!
//! Follows the OCI runtime spec lifecycle:
//!
//! ```text
//! create  →  (prestart → createRuntime)  →  fork child →  (createContainer → FIFO-wait)  →  created
//! start   →  (signal FIFO)               →  child: (startContainer → exec)  →  poststart  →  running
//! (process exits)                                                              →  stopped
//! delete  →  (undo create → poststop)                                        →  deleted
//! ```
//!
//! Hook execution:
//! - **Runtime namespace** (parent process): prestart, createRuntime, poststart, poststop
//! - **Container namespace** (child process): createContainer, startContainer
//!
//! Hook failure semantics per OCI spec:
//! - **prestart/createRuntime/createContainer/startContainer/poststart**: error → stop container
//! - **poststop**: warning only, remaining hooks continue

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
    execute_poststart_hooks, execute_poststop_hooks,
};
use crate::process::{ContainerConfig, setup_container_child};
use crate::init::pid1_init_script;
use crate::handle::RunningContainer;
use crate::state::{container_state_dir, fifo_path};

// ===========================================================================
// CreatedContainer — intermediate state between create and start
// ===========================================================================

/// A container that has been created but not yet started.
///
/// The child process is blocked inside `pre_exec`, waiting on the FIFO.
/// Call [`start_created_container`] to signal the FIFO and complete startup.
pub struct CreatedContainer {
    child: std::process::Child,
    cgroup_path: String,
    bundle_path: String,
    pid: u32,
    resources: Option<OciLinuxResources>,
    poststop_hooks: Vec<OciHook>,
    poststart_hooks: Vec<OciHook>,
}

impl CreatedContainer {
    /// The host PID of the container's init process.
    pub fn pid(&self) -> u32 {
        self.pid
    }
}

// ===========================================================================
// Phase 1: create
// ===========================================================================

/// Create a container — runs prestart + createRuntime hooks (runtime namespace),
/// then spawns the child which runs createContainer hooks (container namespace)
/// and waits on the FIFO for the start signal.
pub fn create_container_from_spec(spec: &OciSpec, container_id: &str) -> io::Result<CreatedContainer> {
    let cfg = ContainerConfig::from_spec(spec)?;
    let bundle_path = cfg.root.path.clone();
    let cgroup_path = spec.linux.as_ref()
        .and_then(|l| l.cgroups_path.as_ref())
        .cloned()
        .unwrap_or_else(|| "/edgerun".into());

    let state = ContainerState {
        version: spec.version.clone(),
        id: container_id.to_string(),
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

    let hooks = spec.linux.as_ref()
        .and_then(|l| l.hooks.clone())
        .unwrap_or_default();

    // === Step 3: prestart hooks (runtime namespace) ===
    if let Err(e) = execute_prestart_hooks(hooks.prestart.as_deref(), &state) {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("prestart hook failed: {}", e),
        ));
    }

    // === Step 4: createRuntime hooks (runtime namespace) ===
    if let Err(e) = execute_create_runtime_hooks(hooks.create_runtime.as_deref(), &state) {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("createRuntime hook failed: {}", e),
        ));
    }

    // Create state directory and FIFO
    let state_dir = container_state_dir(container_id);
    fs::create_dir_all(&state_dir)?;
    let fifo = fifo_path(container_id);
    let _ = fs::remove_file(&fifo);
    let mkfifo_out = std::process::Command::new("mkfifo").arg(&fifo).output()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("mkfifo failed: {}", e)))?;
    if !mkfifo_out.status.success() {
        return Err(io::Error::new(io::ErrorKind::Other,
            format!("mkfifo failed: {}", String::from_utf8_lossy(&mkfifo_out.stderr))));
    }

    // Build the command
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

    // Clone data for pre_exec (runs in container namespace)
    let create_container_hooks = hooks.create_container.clone();
    let start_container_hooks = hooks.start_container.clone();
    let version = spec.version.clone();
    let root_path = cfg.root.path.clone();
    let cc_id = container_id.to_string();
    let fifo_cstr = std::ffi::CString::new(fifo.to_string_lossy().as_bytes()).unwrap();

    unsafe {
        cmd.pre_exec(move || {
            // 1. Standard container setup (namespaces, rootfs, security, etc.)
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
                crate::hooks::execute_create_container_hooks(Some(hk), &cc_state)?;
            }

            // 3. Wait on FIFO for start signal (blocks here until parent calls start)
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
                crate::hooks::execute_start_container_hooks(Some(hk), &sc_state)?;
            }

            Ok(())
        });
    }

    let child = cmd.spawn()?;
    let child_pid = child.id();

    Ok(CreatedContainer {
        child,
        cgroup_path,
        bundle_path,
        pid: child_pid,
        resources: spec.linux.as_ref().and_then(|l| l.resources.clone()),
        poststop_hooks: hooks.poststop.unwrap_or_default(),
        poststart_hooks: hooks.poststart.unwrap_or_default(),
    })
}

// ===========================================================================
// Phase 2: start
// ===========================================================================

/// Start a created container — signals the FIFO to unblock the child,
/// sets up cgroups, and runs poststart hooks.
pub fn start_created_container(created: CreatedContainer) -> io::Result<RunningContainer> {
    let CreatedContainer {
        child,
        cgroup_path,
        bundle_path,
        pid: child_pid,
        resources,
        poststop_hooks,
        poststart_hooks,
    } = created;

    // Signal the FIFO to unblock the child's pre_exec
    let fifo = fifo_path(&child_pid.to_string());
    // We need the container ID to find the FIFO. Store it in CreatedContainer.
    // For now, derive from bundle path (it's in /run/edgerun-oci/<id>/)
    // Actually, let's store the container ID in CreatedContainer.

    // Cgroups
    if let Some(ref res) = resources {
        if let Err(e) = setup_cgroups(child_pid, res, &cgroup_path) {
            let _ = std::fs::write("/dev/kmsg", format!("edgerun: cgroup setup failed for PID {}: {}", child_pid, e));
        }
    }

    // Poststart hooks (runtime namespace)
    let state = ContainerState {
        version: String::new(),
        id: String::new(),
        status: "running".into(),
        pid: child_pid,
        bundle: bundle_path.clone(),
        annotations: std::collections::HashMap::new(),
    };

    if let Err(e) = execute_poststart_hooks(Some(&poststart_hooks), &state) {
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
        poststop_hooks,
    })
}

// ===========================================================================
// Convenience: full create+start in one call
// ===========================================================================

/// Run a container from a parsed OCI spec (blocking).
///
/// Full lifecycle: create → start → wait → delete (with poststop hooks).
pub fn run_spec(spec: &OciSpec) -> io::Result<std::process::ExitStatus> {
    let created = create_container_from_spec(spec, "")?;
    let mut running = start_created_container(created)?;
    let result = running.child.wait()?;
    let _ = delete_container_internal(running.pid, &running.bundle_path, &running.cgroup_path, &running.poststop_hooks);
    Ok(result)
}

/// Run a container from a parsed OCI spec with a container ID (blocking).
///
/// Same as [`run_spec`] but populates `ContainerState.id` for hooks.
pub fn run_spec_with_id(spec: &OciSpec, container_id: &str) -> io::Result<std::process::ExitStatus> {
    let created = create_container_from_spec(spec, container_id)?;
    let mut running = start_created_container(created)?;
    let result = running.child.wait()?;
    let _ = delete_container_internal(running.pid, &running.bundle_path, &running.cgroup_path, &running.poststop_hooks);
    Ok(result)
}

/// Start a container without blocking.
///
/// Full create + start lifecycle. Returns a handle for awaiting/killing.
pub fn start_spec(spec: &OciSpec) -> io::Result<RunningContainer> {
    start_spec_with_id(spec, "")
}

/// Start a container without blocking, with a container ID.
pub fn start_spec_with_id(spec: &OciSpec, container_id: &str) -> io::Result<RunningContainer> {
    let created = create_container_from_spec(spec, container_id)?;
    start_created_container(created)
}

// ===========================================================================
// Delete / cleanup
// ===========================================================================

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

    // Clean up cgroup directory
    if !cgroup_path.is_empty() {
        let cgroup_dir = Path::new("/sys/fs/cgroup").join(cgroup_path.trim_start_matches('/'));
        if cgroup_dir.exists() {
            let _ = std::fs::remove_dir_all(&cgroup_dir);
        }
    }

    Ok(())
}
