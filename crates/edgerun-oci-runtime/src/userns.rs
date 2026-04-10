//! User namespace mapping and capability management.
//!
//! - Writes uid_map/gid_map from the OCI spec (not hardcoded values).
//! - Drops capabilities from the bounding set that aren't in the spec.

use std::fs;
use std::io;

use crate::syscalls::{do_prctl_cap_bset_drop, do_prctl_set_dumpable, do_prctl_set_no_new_privs, setgid, setgroups, setuid};

// ===========================================================================
// User namespace mapping
// ===========================================================================

// Note: The uid/gid mapping logic has been moved to container.rs where it
// serializes mappings to strings (Clone-friendly for FnMut closures).

/// Write a single UID mapping entry to /proc/self/uid_map.
#[allow(dead_code)]
pub fn write_uid_map(inside_uid: u32, outside_uid: u32, count: u32) -> io::Result<()> {
    let content = format!("{} {} {}\n", inside_uid, outside_uid, count);
    fs::write("/proc/self/uid_map", &content)?;
    let _ = fs::write("/proc/self/setgroups", "deny");
    Ok(())
}

/// Write a single GID mapping entry to /proc/self/gid_map.
#[allow(dead_code)]
pub fn write_gid_map(inside_gid: u32, outside_gid: u32, count: u32) -> io::Result<()> {
    let content = format!("{} {} {}\n", inside_gid, outside_gid, count);
    fs::write("/proc/self/gid_map", &content)?;
    Ok(())
}

// ===========================================================================
// Security hardening
// ===========================================================================

/// Set no_new_privs and dumpable flags.
pub fn apply_security_hardening(no_new_privs: bool) -> io::Result<()> {
    if no_new_privs {
        do_prctl_set_no_new_privs()?;
        do_prctl_set_dumpable(false)?;
    }
    Ok(())
}

/// Drop capabilities from the bounding set.
///
/// `bounding_caps` is the list of capabilities to KEEP (e.g., `["CAP_NET_BIND_SERVICE"]`).
/// All other known capabilities are dropped via prctl(PR_CAPBSET_DROP).
///
/// This must be called BEFORE dropping privileges (setuid/setgid).
pub fn drop_capabilities(bounding_caps: Option<&[String]>) -> io::Result<()> {
    // All known Linux capabilities that we can attempt to drop.
    const ALL_CAPS: &[&str] = &[
        "CAP_CHOWN", "CAP_DAC_OVERRIDE", "CAP_DAC_READ_SEARCH", "CAP_FOWNER",
        "CAP_FSETID", "CAP_KILL", "CAP_SETGID", "CAP_SETUID",
        "CAP_SETPCAP", "CAP_LINUX_IMMUTABLE", "CAP_NET_BIND_SERVICE",
        "CAP_NET_BROADCAST", "CAP_NET_ADMIN", "CAP_NET_RAW",
        "CAP_IPC_LOCK", "CAP_IPC_OWNER", "CAP_SYS_MODULE", "CAP_SYS_RAWIO",
        "CAP_SYS_CHROOT", "CAP_SYS_PTRACE", "CAP_SYS_PACCT", "CAP_SYS_ADMIN",
        "CAP_SYS_BOOT", "CAP_SYS_NICE", "CAP_SYS_RESOURCE", "CAP_SYS_TIME",
        "CAP_SYS_TTY_CONFIG", "CAP_MKNOD", "CAP_LEASE", "CAP_AUDIT_WRITE",
        "CAP_AUDIT_CONTROL", "CAP_SETFCAP",
    ];

    let keep: Vec<&str> = bounding_caps
        .map(|caps| caps.iter().map(|s| s.as_str()).collect())
        .unwrap_or_default();

    for &cap in ALL_CAPS {
        let cap_stripped = cap.strip_prefix("CAP_").unwrap_or(cap);
        if !keep.contains(&cap_stripped) && !keep.contains(&cap) {
            // Best-effort drop — some caps may already be dropped or unavailable
            let _ = do_prctl_cap_bset_drop(cap);
        }
    }

    Ok(())
}

/// Set supplementary groups.
pub fn set_supplementary_gids(gids: &[u32]) {
    if gids.is_empty() {
        return;
    }
    // SAFETY: setgroups is a standard syscall; gids slice is valid.
    unsafe {
        setgroups(gids.len(), gids.as_ptr());
    }
}

/// Wrapper for setgid syscall.
pub fn do_setgid(gid: u32) -> io::Result<()> {
    let ret = unsafe { setgid(gid) };
    if ret == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}

/// Wrapper for setuid syscall.
pub fn do_setuid(uid: u32) -> io::Result<()> {
    let ret = unsafe { setuid(uid) };
    if ret == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}
