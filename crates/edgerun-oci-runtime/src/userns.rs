//! User namespace mapping and capability management.

use std::fs;
use std::io;

use crate::syscalls::{
    cap_name_to_int, do_capset, do_prctl_cap_ambient, do_prctl_cap_bset_drop,
    do_prctl_set_dumpable, do_prctl_set_no_new_privs, prctl_const, setgid, setgroups, setuid,
};

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

// ===========================================================================
// Capability management
// ===========================================================================

/// Convert a list of capability names to a bitmask.
/// Returns an error if any capability name is invalid (not recognized by the kernel).
fn caps_to_bitmask(cap_names: &[String]) -> io::Result<u64> {
    let mut mask: u64 = 0;
    for name in cap_names {
        let cap = cap_name_to_int(name);
        if cap == u32::MAX {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid capability name: {}", name),
            ));
        }
        if cap < 64 {
            mask |= 1u64 << cap;
        }
    }
    Ok(mask)
}

/// Set all process capabilities from the OCI spec.
///
/// This must be called BEFORE dropping privileges (setuid/setgid).
///
/// The OCI spec defines 5 capability sets:
/// - **bounding**: Upper limit on capabilities the process can ever gain
/// - **effective**: Capabilities currently in effect
/// - **inheritable**: Capabilities preserved across execve
/// - **permitted**: Capabilities the process is allowed to use
/// - **ambient**: Capabilities inherited by child processes (requires no_new_privs=false)
///
/// Invalid capability names cause an error (per OCI spec: "Any value which cannot be
/// mapped to a relevant kernel interface MUST cause an error").
pub fn set_capabilities(
    effective: Option<&[String]>,
    permitted: Option<&[String]>,
    inheritable: Option<&[String]>,
    bounding: Option<&[String]>,
    ambient: Option<&[String]>,
) -> io::Result<()> {
    let eff_mask = caps_to_bitmask(effective.unwrap_or(&[]))?;
    let perm_mask = caps_to_bitmask(permitted.unwrap_or(&[]))?;
    let inh_mask = caps_to_bitmask(inheritable.unwrap_or(&[]))?;

    // 1. Set effective, permitted, and inheritable via capset
    do_capset(eff_mask, perm_mask, inh_mask)?;

    // 2. Drop capabilities not in bounding set
    drop_capabilities(bounding)?;

    // 3. Set ambient capabilities (requires CAP_SETPCAP in permitted set)
    // Ambient capabilities are inherited by child processes even after setuid
    if let Some(ambient_caps) = ambient {
        // Clear all ambient capabilities first
        let _ = do_prctl_cap_ambient(prctl_const::PR_CAP_AMBIENT_CLEAR_ALL, 0);
        for name in ambient_caps {
            let cap = cap_name_to_int(name);
            if cap < 64 {
                // Best-effort: may fail if CAP_SETPCAP not in permitted set
                let _ = do_prctl_cap_ambient(prctl_const::PR_CAP_AMBIENT_RAISE, cap as i32);
            }
        }
    }

    Ok(())
}

/// Drop capabilities from the bounding set.
///
/// `bounding_caps` is the list of capabilities to KEEP (e.g., `["CAP_NET_BIND_SERVICE"]`).
/// All other known capabilities are dropped via prctl(PR_CAPBSET_DROP).
///
/// Invalid capability names cause an error (per OCI spec).
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
        "CAP_AUDIT_CONTROL", "CAP_SETFCAP", "CAP_MAC_OVERRIDE", "CAP_MAC_ADMIN",
        "CAP_SYSLOG", "CAP_WAKE_ALARM", "CAP_BLOCK_SUSPEND", "CAP_AUDIT_READ",
        "CAP_PERFMON", "CAP_BPF", "CAP_CHECKPOINT_RESTORE",
    ];

    let keep: Vec<&str> = bounding_caps
        .map(|caps| caps.iter().map(|s| s.as_str()).collect())
        .unwrap_or_default();

    // First validate that all keep-listed capabilities are valid
    for &cap_name in &keep {
        let cap = cap_name_to_int(cap_name);
        if cap == u32::MAX {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid capability name: {}", cap_name),
            ));
        }
    }

    for &cap in ALL_CAPS {
        let cap_stripped = cap.strip_prefix("CAP_").unwrap_or(cap);
        if !keep.contains(&cap_stripped) && !keep.contains(&cap) {
            // Best-effort drop — some caps may already be dropped or unavailable
            let _ = do_prctl_cap_bset_drop(cap);
        }
    }

    Ok(())
}

/// Validate a capability name. Returns true if the name is a valid Linux capability.
pub fn is_valid_capability(name: &str) -> bool {
    cap_name_to_int(name) != u32::MAX
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
