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

use crate::prelude::*;
use std::ffi::CString;
use std::fs;
use std::io;
use std::path::Path;

use crate::cgroups::setup_cgroups;
pub use crate::handle::RunningContainer;
use crate::hooks::{
    execute_create_container_hooks, execute_create_runtime_hooks, execute_poststart_hooks,
    execute_poststop_hooks, execute_prestart_hooks, execute_start_container_hooks, ContainerState,
};
use crate::process::{setup_container_child, setup_container_child_rootless, ContainerConfig};
use crate::spec::{OciHook, OciLinuxResources, OciSpec};
use crate::state::{
    container_state_dir, fifo_path, is_root, save_runtime_spec, save_state,
    ContainerState as StateContainerState,
};

/// Extract hooks from an OCI spec, returning a default-empty set if absent.
fn get_hooks(spec: &OciSpec) -> crate::spec::OciHooks {
    spec.linux
        .as_ref()
        .and_then(|l| l.hooks.as_ref())
        .cloned()
        .unwrap_or_default()
}

/// Write a diagnostic message to stderr and the kernel log when available.
fn kmsg(msg: &str) {
    eprintln!("ert child: {msg}");
    let _ = fs::write("/dev/kmsg", format!("edgerun-oci: {msg}"));
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
                return Err(io::Error::other(format!(
                    "createRuntime hook failed: {}",
                    e
                )));
            }
        }
    }
    Ok(())
}

/// Context for running the child process after namespace setup.
///
/// Holds the hooks, version, ID, bundle path, environment, and workload
/// command — grouped together to avoid passing 10+ arguments.
struct ChildExecContext {
    create_container_hooks: Option<Vec<OciHook>>,
    start_container_hooks: Option<Vec<OciHook>>,
    version: String,
    container_id: String,
    bundle_path: String,
    use_pid1_init: bool,
    env: Vec<String>,
    cwd: String,
    workload_args: Vec<String>,
    terminal_socket_fd: Option<i32>,
}

// ===========================================================================
// Step 3: fork child — runs setup + createContainer + FIFO-wait + startContainer
// ===========================================================================

/// Common child code AFTER namespace setup: hooks, FIFO wait, PID init, exec.
fn run_child_post_setup(fifo_fd: i32, ctx: &ChildExecContext) {
    // 1. createContainer hooks (container namespace)
    let cc_state = ContainerState {
        version: ctx.version.clone(),
        id: ctx.container_id.clone(),
        status: "creating".into(),
        pid: 0,
        bundle: ctx.bundle_path.clone(),
        annotations: alloc::collections::BTreeMap::new(),
    };
    if let Some(ref hk) = ctx.create_container_hooks {
        if !hk.is_empty() {
            if let Err(e) = execute_create_container_hooks(Some(hk), &cc_state) {
                kmsg(&format!("child: createContainer hook failed: {}", e));
                unsafe { libc::_exit(1) };
            }
        }
    }

    // 2. Wait on FIFO fd for start signal
    let mut buf = [0u8; 4];
    let n = unsafe { libc::read(fifo_fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
    unsafe { libc::close(fifo_fd) };
    if n <= 0 {
        kmsg("child: FIFO closed before start signal received");
        unsafe { libc::_exit(1) };
    }

    // 3. startContainer hooks (container namespace)
    let sc_state = ContainerState {
        version: ctx.version.clone(),
        id: ctx.container_id.clone(),
        status: "created".into(),
        pid: 0,
        bundle: ctx.bundle_path.clone(),
        annotations: alloc::collections::BTreeMap::new(),
    };
    if let Some(ref hk) = ctx.start_container_hooks {
        if !hk.is_empty() {
            if let Err(e) = execute_start_container_hooks(Some(hk), &sc_state) {
                kmsg(&format!("child: startContainer hook failed: {}", e));
                unsafe { libc::_exit(1) };
            }
        }
    }

    // 4. If PID namespace: fork so parent becomes PID 1 init, child exec's workload
    if ctx.use_pid1_init && std::env::var_os("_ERT_PIDNS_READY").is_none() {
        if let Err(e) = crate::init::fork_and_init() {
            kmsg(&format!("child: fork_and_init failed: {}", e));
            unsafe { libc::_exit(1) };
        }
    }

    // 5. Exec the workload
    unsafe { libc::clearenv() };
    for e in &ctx.env {
        if let Some((k, v)) = e.split_once('=') {
            let k_c = CString::new(k.as_bytes()).unwrap();
            let v_c = CString::new(v.as_bytes()).unwrap();
            unsafe { libc::setenv(k_c.as_ptr(), v_c.as_ptr(), 1) };
        }
    }

    let cwd_c = CString::new(ctx.cwd.as_bytes()).unwrap();
    unsafe { libc::chdir(cwd_c.as_ptr()) };

    let exe_path = if ctx.workload_args[0].starts_with('/') {
        ctx.workload_args[0].clone()
    } else {
        let exe_name = &ctx.workload_args[0];
        let path_env = ctx
            .env
            .iter()
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

    {
        let exe_cstr = CString::new(exe_path.as_bytes()).unwrap();
        let c_args: Vec<CString> = ctx
            .workload_args
            .iter()
            .map(|a| CString::new(a.as_bytes()).unwrap())
            .collect();
        let c_ptrs: Vec<*const libc::c_char> = c_args
            .iter()
            .map(|s| s.as_ptr())
            .chain(std::iter::once(std::ptr::null()))
            .collect();
        unsafe { libc::execvp(exe_cstr.as_ptr(), c_ptrs.as_ptr()) };
    }
    kmsg(&format!(
        "child: execvp({}) failed: {}",
        ctx.workload_args[0],
        io::Error::last_os_error()
    ));
    unsafe { libc::_exit(127) };
}

#[derive(Clone, Copy)]
enum ChildSetupMode {
    Rooted,
    Rootless,
}

fn fork_with_setup_mode(
    cfg: &ContainerConfig,
    fifo_cstr_child: &CString,
    ctx: ChildExecContext,
    mode: ChildSetupMode,
) -> io::Result<i32> {
    let child_pid = unsafe { libc::fork() };
    if child_pid < 0 {
        return Err(io::Error::last_os_error());
    }

    if child_pid == 0 {
        // Open FIFO
        let fifo_fd = unsafe { libc::open(fifo_cstr_child.as_ptr(), libc::O_RDONLY) };
        if fifo_fd < 0 {
            kmsg(&format!(
                "child: failed to open start FIFO: {}",
                io::Error::last_os_error()
            ));
            unsafe { libc::_exit(1) };
        }

        let setup_result = match mode {
            ChildSetupMode::Rooted => setup_container_child(cfg, ctx.terminal_socket_fd),
            ChildSetupMode::Rootless => setup_container_child_rootless(cfg, ctx.terminal_socket_fd),
        };
        if let Err(e) = setup_result {
            let setup_name = match mode {
                ChildSetupMode::Rooted => "setup_container_child",
                ChildSetupMode::Rootless => "setup_container_child_rootless",
            };
            kmsg(&format!("child: {setup_name} failed: {e}"));
            unsafe { libc::_exit(1) };
        }

        // Common post-setup: hooks, FIFO wait, PID init, exec
        run_child_post_setup(fifo_fd, &ctx);
        unreachable!();
    }

    Ok(child_pid)
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
/// In rootless mode, uses a two-stage fork: user namespace is created first,
/// parent writes uid/gid maps, then child continues with remaining namespaces.
///
/// Returns the child PID and a reference to the spec-derived config.
pub fn fork_container_child(spec: &OciSpec, container_id: &str) -> io::Result<ForkedChild> {
    fork_container_child_with_terminal_socket(spec, container_id, None)
}

pub fn fork_container_child_with_terminal_socket(
    spec: &OciSpec,
    container_id: &str,
    terminal_socket_fd: Option<i32>,
) -> io::Result<ForkedChild> {
    // Validate platform compatibility before creating the container.
    // Per OCI spec: the runtime MUST reject bundles whose platform does not
    // match the host platform (unless no platform is specified).
    if let Some(ref platform) = spec.platform {
        if !platform.matches_host() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "container platform mismatch: bundle targets {:?}/{:?}, host is {}/{}",
                    platform.os.as_deref().unwrap_or("unknown"),
                    platform.arch.as_deref().unwrap_or("unknown"),
                    crate::process::host_os(),
                    crate::process::host_arch(),
                ),
            ));
        }
    }

    let mut cfg = ContainerConfig::from_spec(spec)?;
    let bundle_path = cfg.root.path.clone();

    // Resolve rootfs path to absolute path before forking
    // The child process inherits CWD but it's safer to have absolute paths
    if !cfg.root.path.starts_with('/') {
        if let Ok(abs) = std::fs::canonicalize(&cfg.root.path) {
            cfg.root.path = abs.to_string_lossy().to_string();
        }
    }

    let cgroup_path = spec
        .linux
        .as_ref()
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
    let args = process
        .args
        .clone()
        .unwrap_or_else(crate::validate::default_process_args);
    let env = process
        .env
        .clone()
        .unwrap_or_else(crate::validate::default_process_env);
    let cwd = process.cwd.clone().unwrap_or_else(|| "/".into());

    // Determine if we're running rootless (user namespace already exists from re-exec)
    let rootless = crate::state::is_rootless();

    // Build child exec context
    let ctx = ChildExecContext {
        create_container_hooks: hooks.create_container.clone(),
        start_container_hooks: hooks.start_container.clone(),
        version: spec.version.clone(),
        container_id: container_id.to_string(),
        bundle_path: cfg.root.path.clone(),
        use_pid1_init: cfg.has_pid_ns(),
        env: env.clone(),
        cwd: cwd.clone(),
        workload_args: args.clone(),
        terminal_socket_fd,
    };
    let fifo_cstr_child = CString::new(fifo.to_string_lossy().as_bytes()).unwrap();

    let child_pid = if rootless {
        // ROOTLESS MODE: simple fork — user namespace was created by re-exec in main()
        fork_with_setup_mode(&cfg, &fifo_cstr_child, ctx, ChildSetupMode::Rootless)?
    } else {
        // ROOT MODE: Single fork, all namespaces at once
        fork_with_setup_mode(&cfg, &fifo_cstr_child, ctx, ChildSetupMode::Rooted)?
    };

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
    pub fn pid(&self) -> u32 {
        self.pid
    }
    pub fn bundle_path(&self) -> &str {
        &self.bundle_path
    }
    pub fn cgroup_path(&self) -> &str {
        &self.cgroup_path
    }
}

// ===========================================================================
// Step 4: save state as "created"
// ===========================================================================

/// Save the container state as "created".
pub fn save_created_state(
    spec: &OciSpec,
    container_id: &str,
    pid: u32,
    bundle_path: &str,
) -> io::Result<()> {
    let state = StateContainerState {
        oci_version: spec.version.clone(),
        id: container_id.to_string(),
        status: "created".to_string(),
        pid: Some(pid),
        bundle: bundle_path.to_string(),
        annotations: spec.annotations.clone(),
    };
    save_state(&state, container_id)?;
    save_runtime_spec(spec, container_id)
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
        let _ = std::fs::write(
            "/dev/kmsg",
            format!("edgerun: cgroup setup failed for PID {}: {}", pid, e),
        );
    }
}

/// Set up cgroups declared by the spec before the container is started.
pub fn setup_spec_cgroups(pid: u32, spec: &OciSpec) {
    let Some(linux) = spec.linux.as_ref() else {
        return;
    };
    let Some(resources) = linux.resources.as_ref() else {
        return;
    };

    let raw_cgroup_path = linux.cgroups_path.as_deref().unwrap_or("");
    let rootless = !is_root();
    let cgroup_path = crate::rootless::resolve_container_cgroup_path(rootless, raw_cgroup_path)
        .unwrap_or_else(|e| {
            let _ = fs::write(
                "/dev/kmsg",
                format!("edgerun: cgroup resolution failed: {}", e),
            );
            raw_cgroup_path.to_string()
        });
    setup_container_cgroups(pid, resources, &cgroup_path);
}

/// Start a created container: cgroups, FIFO signal, poststart hooks, and state.
pub fn start_created_container(spec: &OciSpec, container_id: &str, pid: u32) -> io::Result<()> {
    setup_spec_cgroups(pid, spec);
    signal_start(container_id)?;
    run_poststart_hooks(spec, container_id, pid)?;
    update_state_running(container_id, pid)
}

/// Save created state for a freshly forked child, then start it.
pub fn save_and_start_forked_child(
    spec: &OciSpec,
    container_id: &str,
    pid: u32,
    bundle_path: &str,
) -> io::Result<()> {
    save_created_state(spec, container_id, pid, bundle_path)?;
    start_created_container(spec, container_id, pid)
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
        bundle: spec
            .root
            .as_ref()
            .map(|r| r.path.clone())
            .unwrap_or_default(),
        annotations: alloc::collections::BTreeMap::new(),
    };

    if let Some(ref poststart) = hooks.poststart {
        if !poststart.is_empty() {
            if let Err(e) = execute_poststart_hooks(Some(poststart), &state) {
                let _ = unsafe {
                    crate::syscalls::kill(pid as std::os::raw::c_int, crate::syscalls::SIGKILL)
                };
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
pub fn run_poststop_and_cleanup(
    container_id: &str,
    pid: u32,
    bundle_path: &str,
    cgroup_path: &str,
    spec: &OciSpec,
) {
    // Poststop hooks
    let hooks = spec
        .linux
        .as_ref()
        .and_then(|l| l.hooks.as_ref())
        .cloned()
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
        annotations: alloc::collections::BTreeMap::new(),
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
        bundle: spec
            .root
            .as_ref()
            .map(|r| r.path.clone())
            .unwrap_or_default(),
        annotations: spec.annotations.clone().unwrap_or_default(),
    }
}

// ===========================================================================
// High-level convenience functions (used by container.rs)
// ===========================================================================

/// Run a container from a spec (blocking). Extracts ID from spec annotations or uses "default".
pub fn run_spec(spec: &OciSpec) -> io::Result<std::process::ExitStatus> {
    let container_id = spec
        .annotations
        .as_ref()
        .and_then(|a| a.get("org.edgerun.container.id"))
        .cloned()
        .unwrap_or_else(|| "default".to_string());
    run_spec_with_id(spec, &container_id)
}

/// Run a container from a spec with explicit ID (blocking).
pub fn run_spec_with_id(
    spec: &OciSpec,
    container_id: &str,
) -> io::Result<std::process::ExitStatus> {
    // Full blocking lifecycle: create → start → wait → delete
    let child = start_spec_with_id(spec, container_id)?;
    child.wait()
}

/// Start a container from a spec (non-blocking).
pub fn start_spec(spec: &OciSpec) -> io::Result<RunningContainer> {
    let container_id = spec
        .annotations
        .as_ref()
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

    save_and_start_forked_child(spec, container_id, pid, child.bundle_path())?;

    // Step 9: return handle
    Ok(into_running_container(child))
}

/// Run an OCI bundle (directory containing config.json + rootfs/).
pub fn run_bundle(bundle_path: &Path) -> io::Result<std::process::ExitStatus> {
    let config_data = fs::read(bundle_path.join("config.json"))?;
    let spec: OciSpec = crate::spec::parse_oci_spec(&config_data).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid OCI config: {}", e),
        )
    })?;
    run_spec(&spec)
}

/// Start a container from an OCI bundle without blocking.
pub fn start_bundle(bundle_path: &Path) -> io::Result<RunningContainer> {
    let config_data = fs::read(bundle_path.join("config.json"))?;
    let spec: OciSpec = crate::spec::parse_oci_spec(&config_data).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid OCI config: {}", e),
        )
    })?;
    start_spec(&spec)
}
