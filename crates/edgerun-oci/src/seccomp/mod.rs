//! Seccomp-BPF filtering for OCI containers.
//!
//! Generates BPF programs that filter syscalls for container processes.

use crate::prelude::*;
// ===========================================================================
// Architecture mapping
// ===========================================================================

#[cfg(target_arch = "x86_64")]
pub const AUDIT_ARCH_X86_64: u32 = 0xc000003e;
#[cfg(target_arch = "aarch64")]
pub const AUDIT_ARCH_AARCH64: u32 = 0xc00000b7;

#[cfg(target_arch = "x86_64")]
const CURRENT_ARCH: u32 = AUDIT_ARCH_X86_64;
#[cfg(target_arch = "aarch64")]
const CURRENT_ARCH: u32 = AUDIT_ARCH_AARCH64;

mod actions;
mod bpf;
mod rules;
mod syscall;

use crate::json::{OciLinuxSeccomp, OciSeccompAction};
use crate::syscalls::{
    do_seccomp, SECCOMP_FILTER_FLAG_NEW_LISTENER, SECCOMP_FILTER_FLAG_TSYNC,
    SECCOMP_SET_MODE_FILTER,
};
use bpf::{bpf_insn, bpf_long_skip};
pub use rules::{build_seccomp_prog, seccomp_bpf_prog};
use std::io;
use std::os::raw::c_void;
pub use syscall::syscall_nr;

#[cfg(all(test, not(target_os = "none")))]
mod tests;

/// Requires prctl(PR_SET_NO_NEW_PRIVS, 1) first.
pub fn apply_seccomp() -> io::Result<()> {
    let (_insn_bytes, prog) = seccomp_bpf_prog();
    let ret = unsafe {
        do_seccomp(
            SECCOMP_SET_MODE_FILTER,
            SECCOMP_FILTER_FLAG_TSYNC,
            prog.as_ptr() as *const c_void,
        )
    };
    if ret == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Check if the seccomp spec uses the NOTIFY action.
///
/// When NOTIFY is used, the runtime must create a seccomp user notification
/// listener fd (via SECCOMP_FILTER_FLAG_NEW_LISTENER) to handle blocked syscalls.
pub fn uses_notify_action(spec: &OciLinuxSeccomp) -> bool {
    let entries = spec.syscalls.as_deref().unwrap_or(&[]);
    entries
        .iter()
        .any(|e| e.action.as_ref() == Some(&OciSeccompAction::Notify))
        || spec.default_action.as_ref() == Some(&OciSeccompAction::Notify)
}

/// Apply seccomp filtering from OCI spec rules.
///
/// If `spec` is None or has no syscalls, falls back to the built-in allow-list.
/// Requires prctl(PR_SET_NO_NEW_PRIVS, 1) first.
///
/// When the spec uses the NOTIFY action, this function uses
/// `SECCOMP_FILTER_FLAG_NEW_LISTENER` and returns the listener fd.
/// The caller is responsible for handling seccomp notifications
/// (or closing the fd if no handler is available).
///
/// Note: Does NOT use TSYNC flag — the container child is single-threaded at this
/// point (just forked). TSYNC requires CAP_SYS_ADMIN even with no_new_privs.
pub fn apply_seccomp_from_spec(
    spec: Option<&OciLinuxSeccomp>,
    bundle_path: &str,
) -> io::Result<Option<i32>> {
    let has_rules = spec
        .as_ref()
        .and_then(|s| s.syscalls.as_ref())
        .map(|s| !s.is_empty())
        .unwrap_or(false);

    // Both functions return (insn_bytes, prog) where prog is sock_fprog
    // pointing into insn_bytes. We keep insn_bytes alive until the syscall.
    let (_insn_bytes, prog) = if has_rules {
        build_seccomp_prog(spec.unwrap())
    } else {
        seccomp_bpf_prog()
    };

    // Detect if NOTIFY action is used — requires NEW_LISTENER flag
    let use_listener = spec.map(uses_notify_action).unwrap_or(false);
    let flags = if use_listener {
        SECCOMP_FILTER_FLAG_NEW_LISTENER
    } else {
        0
    };

    let ret = unsafe {
        do_seccomp(
            SECCOMP_SET_MODE_FILTER,
            flags,
            prog.as_ptr() as *const c_void,
        )
    };

    // Write listenerMetadata to bundle directory if specified
    if let Some(metadata) = spec.and_then(|s| s.listener_metadata.as_ref()) {
        let metadata_path = format!("{}/.edgerun-seccomp-metadata", bundle_path);
        let _ = std::fs::write(&metadata_path, metadata.as_bytes());
    }

    if ret < 0 {
        Err(io::Error::last_os_error())
    } else if use_listener {
        // seccomp syscall returns the listener fd when NEW_LISTENER flag is set
        let _ = std::fs::write(
            "/dev/kmsg",
            "edgerun: seccomp NOTIFY listener created (fd not handled by runtime)",
        );
        Ok(Some(ret))
    } else {
        Ok(None)
    }
}
