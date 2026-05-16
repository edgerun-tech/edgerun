use crate::libc;
use crate::prelude::*;
use std::ffi::CString;
use std::fs;
use std::io;

use crate::hooks::{ContainerState, execute_create_container_hooks, execute_start_container_hooks};
use crate::process::{ContainerConfig, setup_container_child, setup_container_child_rootless};
use crate::process_exec::exec_with_env_and_cwd;
use crate::spec::OciHook;

/// Context for running the child process after namespace setup.
///
/// Holds the hooks, version, ID, bundle path, environment, workload command,
/// and terminal socket as one unit so the fork path has a narrow signature.
pub(crate) struct ChildExecContext {
    pub create_container_hooks: Option<Vec<OciHook>>,
    pub start_container_hooks: Option<Vec<OciHook>>,
    pub version: String,
    pub container_id: String,
    pub bundle_path: String,
    pub use_pid1_init: bool,
    pub env: Vec<String>,
    pub cwd: String,
    pub workload_args: Vec<String>,
    pub terminal_socket_fd: Option<i32>,
}

#[derive(Clone, Copy)]
pub(crate) enum ChildSetupMode {
    Rooted,
    Rootless,
}

/// Write a diagnostic message to stderr and the kernel log when available.
fn kmsg(msg: &str) {
    eprintln!("ert child: {msg}");
    let _ = fs::write("/dev/kmsg", format!("edgerun-oci: {msg}"));
}

/// Fork the container child and run namespace setup before waiting for start.
pub(crate) fn fork_with_setup_mode(
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

        run_child_post_setup(fifo_fd, &ctx);
        unreachable!();
    }

    Ok(child_pid)
}

/// Common child code after namespace setup: hooks, FIFO wait, PID init, exec.
fn run_child_post_setup(fifo_fd: i32, ctx: &ChildExecContext) {
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

    let mut buf = [0u8; 4];
    let n = unsafe { libc::read(fifo_fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
    unsafe { libc::close(fifo_fd) };
    if n <= 0 {
        kmsg("child: FIFO closed before start signal received");
        unsafe { libc::_exit(1) };
    }

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

    close_extra_fds();

    if ctx.use_pid1_init && std::env::var_os("_ERT_PIDNS_READY").is_none() {
        if let Err(e) = crate::init::fork_and_init() {
            kmsg(&format!("child: fork_and_init failed: {}", e));
            unsafe { libc::_exit(1) };
        }
    }

    let error = exec_with_env_and_cwd(&ctx.workload_args, &ctx.env, &ctx.cwd)
        .err()
        .unwrap_or_else(|| {
            crate::process_exec::ProcessExecError::Exec(io::Error::other(
                "execvp unexpectedly returned success",
            ))
        });
    kmsg(&format!(
        "child: execvp({}) failed: {}",
        ctx.workload_args[0], error
    ));
    unsafe { libc::_exit(error.exit_code()) };
}

fn close_extra_fds() {
    let max_fd = unsafe { libc::sysconf(libc::_SC_OPEN_MAX) };
    let max_fd = if max_fd > 0 { max_fd as i32 } else { 1024 };
    for fd in 3..max_fd.min(4096) {
        unsafe { libc::close(fd) };
    }
}
