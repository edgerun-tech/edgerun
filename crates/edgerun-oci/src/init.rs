//! Minimal PID 1 init process for containers.
//!
//! In a PID namespace, PID 1 has special semantics:
//! - Signals not explicitly handled are ignored (default disposition)
//! - Orphaned children become zombies that are never reaped
//!
//! This module provides a Rust-based PID 1 init that:
//! 1. Forks the workload as a direct child
//! 2. Forwards SIGTERM/SIGINT/SIGQUIT to the workload child
//! 3. Reaps ALL zombie children via `waitpid(-1)` (not just the workload)
//! 4. Exits with the workload's exit code

use crate::libc;
use crate::prelude::*;
use std::io;

static WORKLOAD_PID: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

/// Fork the workload and enter PID 1 init loop.
///
/// Called from the `pre_exec` closure after namespace/security setup.
///
/// - **Parent (PID 1)**: enters the init loop, never returns
/// - **Child**: returns `Ok(())` so `pre_exec` completes and `exec` proceeds
///
/// The parent reaps ALL zombie children via `waitpid(-1)`.
pub fn fork_and_init() -> io::Result<()> {
    let pid = unsafe { libc::fork() };
    if pid < 0 {
        return Err(io::Error::last_os_error());
    }

    if pid > 0 {
        // Parent: this is PID 1 in the new PID namespace.
        pid1_init_loop(pid);
    }
    // Child: return Ok(()) so pre_exec completes and exec proceeds
    Ok(())
}

fn pid1_init_loop(workload_pid: libc::pid_t) -> ! {
    WORKLOAD_PID.store(workload_pid, std::sync::atomic::Ordering::SeqCst);

    unsafe {
        libc::signal(
            libc::SIGTERM,
            forward_signal as *const () as libc::sighandler_t,
        );
        libc::signal(
            libc::SIGINT,
            forward_signal as *const () as libc::sighandler_t,
        );
        libc::signal(
            libc::SIGQUIT,
            forward_signal as *const () as libc::sighandler_t,
        );
        libc::signal(libc::SIGCHLD, libc::SIG_DFL);
    }

    let mut workload_exited = false;
    let mut workload_status: i32 = 0;

    loop {
        let mut status: i32 = 0;
        let pid = unsafe { libc::waitpid(-1, &mut status, 0) };
        if pid < 0 {
            continue;
        } // EINTR

        if pid == workload_pid {
            workload_exited = true;
            workload_status = status;
        }

        if workload_exited {
            while unsafe { libc::waitpid(-1, &mut status, libc::WNOHANG) } > 0 {}
            if libc::WIFEXITED(workload_status) {
                std::process::exit(libc::WEXITSTATUS(workload_status) as i32);
            } else if libc::WIFSIGNALED(workload_status) {
                std::process::exit(128 + libc::WTERMSIG(workload_status));
            } else {
                std::process::exit(1);
            }
        }
    }
}

extern "C" fn forward_signal(signum: libc::c_int) {
    let pid = WORKLOAD_PID.load(std::sync::atomic::Ordering::Relaxed);
    if pid > 0 {
        unsafe { libc::kill(pid, signum) };
    }
}
