//! OCI container lifecycle — create, start, delete with hook integration.
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

use std::ffi::CString;
use std::fs;
use std::io;
use std::path::Path;

use crate::json::{OciLinuxResources, OciSpec, OciHook};
use crate::cgroups::setup_cgroups;
use crate::hooks::{
    ContainerState,
    execute_prestart_hooks, execute_create_runtime_hooks,
    execute_create_container_hooks, execute_start_container_hooks,
    execute_poststart_hooks, execute_poststop_hooks,
};
use crate::process::{ContainerConfig, setup_container_child};
pub use crate::handle::RunningContainer;
use crate::state::{container_state_dir, fifo_path, save_state, ContainerState as StateContainerState};

/// Extract hooks from an OCI spec, returning a default-empty set if absent.
fn get_hooks(spec: &OciSpec) -> crate::json::OciHooks {
    spec.linux.as_ref()
        .and_then(|l| l.hooks.as_ref())
        .cloned()
        .unwrap_or_default()
}

// ===========================================================================
// Step 1: prestart hooks
// ===========================================================================

/// Run prestart hooks (runtime namespace). Returns hook state for subsequent steps.
pub fn run_prestart_hooks(spec: &OciSpec, container_id: &str) -> io::Result<()> {
    let hooks = get_hooks(spec);

    let state = make_state(spec, container_id, "creating", 0);

    if let Some(ref prestart) = hooks.prestart {
        if !prestart.is_empty() {
            if let Err(e) = execute_prestart_hooks(Some(prestart), &state) {
                return Err(io::Error::other(format!("prestart hook failed: {}", e)));
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
    let hooks = get_hooks(spec);

    let state = make_state(spec, container_id, "creating", 0);

    if let Some(ref create_runtime) = hooks.create_runtime {
        if !create_runtime.is_empty() {
            if let Err(e) = execute_create_runtime_hooks(Some(create_runtime), &state) {
                return Err(io::Error::other(format!("createRuntime hook failed: {}", e)));
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
/// 2. createContainer hooks
/// 3. Waits on FIFO for start signal
/// 4. startContainer hooks
/// 5. execs the workload
///
/// Returns the child PID and a reference to the spec-derived config.
pub fn fork_container_child(spec: &OciSpec, container_id: &str) -> io::Result<ForkedChild> {
    let mut cfg = ContainerConfig::from_spec(spec)?;
    let bundle_path = cfg.root.path.clone();

    // Resolve rootfs path to absolute path before forking
    // The child process inherits CWD but it's safer to have absolute paths
    if !cfg.root.path.starts_with('/') {
        if let Ok(abs) = std::fs::canonicalize(&cfg.root.path) {
            cfg.root.path = abs.to_string_lossy().to_string();
        }
    }

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
    let fifo_cstr = CString::new(fifo.to_string_lossy().as_bytes()).unwrap();
    let mkfifo_ret = unsafe { libc::mkfifo(fifo_cstr.as_ptr(), 0o600) };
    if mkfifo_ret != 0 {
        return Err(io::Error::last_os_error());
    }

    // Extract hooks for the child
    let hooks = get_hooks(spec);

    // Build the workload command
    let process = spec.process.clone().unwrap_or_default();
    let args = process.args.clone().unwrap_or_else(|| vec!["/bin/sh".into()]);
    let env = process.env.clone().unwrap_or_else(|| crate::process::DEFAULT_ENV.iter().map(|s| s.to_string()).collect());
    let cwd = process.cwd.clone().unwrap_or_else(|| "/".into());

    let use_pid1_init = cfg.has_pid_ns();

    // Clone data for the child
    let create_container_hooks = hooks.create_container.clone();
    let start_container_hooks = hooks.start_container.clone();
    let version = spec.version.clone();
    let root_path = cfg.root.path.clone();
    let cc_id = container_id.to_string();
    let fifo_cstr_child = CString::new(fifo.to_string_lossy().as_bytes()).unwrap();
    let workload_args = args.clone();

    // Fork manually to avoid Command::pre_exec issues with unshare
    let child_pid = unsafe { libc::fork() };
    if child_pid < 0 {
        return Err(io::Error::last_os_error());
    }

    if child_pid == 0 {
        // Child process

        // Open FIFO for reading BEFORE pivot_root (path becomes invalid after pivot_root)
        let fifo_fd = unsafe { libc::open(fifo_cstr_child.as_ptr(), libc::O_RDONLY) };
        if fifo_fd < 0 {
            unsafe { libc::_exit(1) };
        }

        // 1. Standard container setup (mount ns, pivot_root, etc.)
        if let Err(_e) = setup_container_child(&cfg) {
            unsafe { libc::_exit(1) };
        }

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
                if let Err(_e) = execute_create_container_hooks(Some(hk), &cc_state) {
                    // hook failed
                    unsafe { libc::_exit(1) };
                }
            }
        }

        // 3. Wait on FIFO fd for start signal (fd was opened before pivot_root)
        let mut buf = [0u8; 4];
        let n = unsafe { libc::read(fifo_fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        unsafe { libc::close(fifo_fd) };
        if n <= 0 {
            // fifo closed
            unsafe { libc::_exit(1) };
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
                if let Err(_e) = execute_start_container_hooks(Some(hk), &sc_state) {
                    // hook failed
                    unsafe { libc::_exit(1) };
                }
            }
        }

        // 5. If PID namespace: fork so parent becomes PID 1 init, child exec's workload
        if use_pid1_init {
            if let Err(_e) = crate::init::fork_and_init() {
                // init failed
                unsafe { libc::_exit(1) };
            }
        }

        // 6. Exec the workload

        // Set environment
        for e in &env {
            if let Some((k, v)) = e.split_once('=') {
                let k_c = CString::new(k.as_bytes()).unwrap();
                let v_c = CString::new(v.as_bytes()).unwrap();
                unsafe { libc::setenv(k_c.as_ptr(), v_c.as_ptr(), 1) };
            }
        }

        // Clear the environment first
        unsafe { libc::clearenv() };
        // Re-set environment
        for e in &env {
            if let Some((k, v)) = e.split_once('=') {
                let k_c = CString::new(k.as_bytes()).unwrap();
                let v_c = CString::new(v.as_bytes()).unwrap();
                unsafe { libc::setenv(k_c.as_ptr(), v_c.as_ptr(), 1) };
            }
        }

        // chdir
        let cwd_c = CString::new(cwd.as_bytes()).unwrap();
        unsafe { libc::chdir(cwd_c.as_ptr()) };

        // Resolve executable path
        let exe_path = if workload_args[0].starts_with('/') {
            workload_args[0].clone()
        } else {
            let exe_name = &workload_args[0];
            let path_env = env.iter()
                .find(|e| e.starts_with("PATH="))
                .map(|e| &e[5..])
                .unwrap_or("/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin");
            let mut found = None;
            for dir in path_env.split(':') {
                let candidate = format!("{}/{}", dir, exe_name);
                if std::path::Path::new(&candidate).exists() {
                    found = Some(candidate);
                    break;
                }
            }
            found.unwrap_or_else(|| exe_name.clone())
        };

        // execvp — variables are used by execvp which is noreturn
        #[allow(unused_variables, unused_assignments)]
        {
            let exe_cstr = CString::new(exe_path.as_bytes()).unwrap();
            let c_args: Vec<CString> = workload_args.iter()
                .map(|a| CString::new(a.as_bytes()).unwrap())
                .collect();
            let c_ptrs: Vec<*const libc::c_char> = c_args.iter()
                .map(|s| s.as_ptr())
                .chain(std::iter::once(std::ptr::null()))
                .collect();
            unsafe { libc::execvp(exe_cstr.as_ptr(), c_ptrs.as_ptr() as *const *const libc::c_char) };
        }
        // exec failed
        unsafe { libc::_exit(127) };
    }

    // Parent returns with child PID
    // Note: We can't use std::process::Child::from_raw (unstable),
    // so we construct a minimal Child struct manually.
    // Child { stdin, stdout, stderr, process } - we only need the PID for tracking.
    // Since we're using raw fork(), we manage the child via syscalls directly.
    Ok(ForkedChild {
        pid: child_pid as u32,
        bundle_path,
        cgroup_path,
        resources,
        poststop_hooks: hooks.poststop.unwrap_or_default(),
        poststart_hooks: hooks.poststart.unwrap_or_default(),
    })
}

/// Result of forking a container child.
pub struct ForkedChild {
    pub pid: u32,
    pub bundle_path: String,
    pub cgroup_path: String,
    pub resources: Option<OciLinuxResources>,
    pub poststop_hooks: Vec<OciHook>,
    pub poststart_hooks: Vec<OciHook>,
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
    let fifo = crate::state::fifo_path(container_id);
    crate::fifo::signal_start(&fifo)
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
    let hooks = get_hooks(spec);

    let state = ContainerState {
        version: spec.version.clone(),
        id: container_id.to_string(),
        status: "running".into(),
        pid,
        bundle: spec.root.as_ref().map(|r| r.path.clone()).unwrap_or_default(),
        annotations: std::collections::HashMap::new(),
    };

    if let Some(ref poststart) = hooks.poststart {
        if !poststart.is_empty() {
            if let Err(e) = execute_poststart_hooks(Some(poststart), &state) {
                let _ = unsafe { crate::syscalls::kill(pid as std::os::raw::c_int, crate::syscalls::SIGKILL) };
                return Err(io::Error::other(format!("poststart hook failed: {}", e)));
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
    if let Ok(mut existing) = crate::state::load_state(container_id) {
        existing.status = "running".to_string();
        existing.pid = Some(pid);
        crate::state::save_state(&existing, container_id)
    } else {
        let state = StateContainerState {
            oci_version: String::new(),
            id: container_id.to_string(),
            status: "running".to_string(),
            pid: Some(pid),
            bundle: String::new(),
            annotations: None,
        };
        save_state(&state, container_id)
    }
}

// ===========================================================================
// Step 9: RunningContainer wrapper
// ===========================================================================

/// Wrap a ForkedChild into a RunningContainer for the library API.
pub fn into_running_container(child: ForkedChild) -> RunningContainer {
    RunningContainer {
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
pub fn run_poststop_and_cleanup(container_id: &str, pid: u32, bundle_path: &str, cgroup_path: &str, spec: &OciSpec) {
    // Poststop hooks
    let hooks = spec.linux.as_ref()
        .and_then(|l| l.hooks.as_ref())
        .map(|h| h.clone())
        .unwrap_or_default();

    let state = ContainerState {
        version: spec.version.clone(),
        id: container_id.to_string(),
        status: "stopped".into(),
        pid,
        bundle: bundle_path.to_string(),
        annotations: spec.annotations.clone().unwrap_or_default(),
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
            let _ = std::fs::remove_dir_all(&cgroup_dir);
        }
    }
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
        annotations: spec.annotations.clone().unwrap_or_default(),
    }
}

// ===========================================================================
// High-level convenience functions (used by container.rs)
// ===========================================================================

/// Run a container from a spec (blocking). Extracts ID from spec annotations or uses "default".
pub fn run_spec(spec: &OciSpec) -> io::Result<std::process::ExitStatus> {
    let container_id = spec.annotations.as_ref()
        .and_then(|a| a.get("org.edgerun.container.id"))
        .cloned()
        .unwrap_or_else(|| "default".to_string());
    run_spec_with_id(spec, &container_id)
}

/// Run a container from a spec with explicit ID (blocking).
pub fn run_spec_with_id(spec: &OciSpec, container_id: &str) -> io::Result<std::process::ExitStatus> {
    // Full blocking lifecycle: create → start → wait → delete
    let child = start_spec_with_id(spec, container_id)?;
    child.wait()
}

/// Start a container from a spec (non-blocking).
pub fn start_spec(spec: &OciSpec) -> io::Result<RunningContainer> {
    let container_id = spec.annotations.as_ref()
        .and_then(|a| a.get("org.edgerun.container.id"))
        .cloned()
        .unwrap_or_else(|| "default".to_string());
    start_spec_with_id(spec, &container_id)
}

/// Start a container from an OCI spec with explicit ID (non-blocking).
pub fn start_spec_with_id(spec: &OciSpec, container_id: &str) -> io::Result<RunningContainer> {
    // Step 1: prestart hooks
    run_prestart_hooks(spec, container_id)?;

    // Step 2: createRuntime hooks
    run_create_runtime_hooks(spec, container_id)?;

    // Step 3: fork child
    let child = fork_container_child(spec, container_id)?;
    let pid = child.pid();
    let resources = child.resources.clone();

    // Step 4: save created state
    save_created_state(spec, container_id, pid)?;

    // Step 5: setup cgroups
    if let Some(ref res) = resources {
        if let Some(ref linux) = spec.linux {
            let cgroup_path = linux.cgroups_path.as_deref().unwrap_or("/edgerun");
            setup_container_cgroups(pid, res, cgroup_path);
        }
    }

    // Step 6: signal start
    signal_start(container_id)?;

    // Step 7: poststart hooks
    run_poststart_hooks(spec, container_id, pid)?;

    // Step 8: update state to running
    update_state_running(container_id, pid)?;

    // Step 9: return handle
    Ok(into_running_container(child))
}

/// Run an OCI bundle (directory containing config.json + rootfs/).
pub fn run_bundle(bundle_path: &Path) -> io::Result<std::process::ExitStatus> {
    let config_data = fs::read(bundle_path.join("config.json"))?;
    let spec: OciSpec = crate::json::parse_oci_spec(&config_data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("invalid OCI config: {}", e)))?;
    run_spec(&spec)
}

/// Start a container from an OCI bundle without blocking.
pub fn start_bundle(bundle_path: &Path) -> io::Result<RunningContainer> {
    let config_data = fs::read(bundle_path.join("config.json"))?;
    let spec: OciSpec = crate::json::parse_oci_spec(&config_data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("invalid OCI config: {}", e)))?;
    start_spec(&spec)
}
