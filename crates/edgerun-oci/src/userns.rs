//! User namespace mapping and capability management.

use crate::prelude::*;
use std::io;

use crate::syscalls::{
    cap_name_to_int, do_capset, do_prctl_cap_ambient, do_prctl_cap_bset_drop,
    do_prctl_set_dumpable, do_prctl_set_no_new_privs, prctl_const, setgid, setgroups, setuid,
};

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
    // If no capabilities are specified at all, skip capset to keep parent's caps.
    // Per OCI spec: if not specified, inherit from caller.
    if effective.is_none() && permitted.is_none() && inheritable.is_none() && bounding.is_none() {
        return Ok(());
    }

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

    for &cap in crate::validate::KNOWN_CAPABILITIES {
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
    if ret == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Wrapper for setuid syscall.
pub fn do_setuid(uid: u32) -> io::Result<()> {
    let ret = unsafe { setuid(uid) };
    if ret == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(all(test, not(target_os = "none")))]
mod tests {
    use super::*;

    #[test]
    fn is_valid_capability_known() {
        assert!(is_valid_capability("CAP_CHOWN"));
        assert!(is_valid_capability("CAP_NET_BIND_SERVICE"));
        assert!(is_valid_capability("CAP_SYS_ADMIN"));
        assert!(is_valid_capability("CAP_NET_RAW"));
        assert!(is_valid_capability("CAP_KILL"));
    }

    #[test]
    fn is_valid_capability_unknown() {
        assert!(!is_valid_capability("CAP_NONEXISTENT"));
        assert!(!is_valid_capability(""));
        assert!(!is_valid_capability("NOT_A_CAP"));
    }

    #[test]
    fn is_valid_capability_case_sensitive() {
        assert!(!is_valid_capability("cap_chown"));
        assert!(!is_valid_capability("Cap_Chown"));
    }

    #[test]
    fn caps_to_bitmask_single() {
        let caps = vec!["CAP_CHOWN".to_string()];
        let mask = caps_to_bitmask(&caps).unwrap();
        assert!(mask != 0);
    }

    #[test]
    fn caps_to_bitmask_multiple() {
        let caps = vec!["CAP_CHOWN".to_string(), "CAP_KILL".to_string()];
        let mask = caps_to_bitmask(&caps).unwrap();
        assert!(mask != 0);
        // Each should set a different bit
        let chown = caps_to_bitmask(&["CAP_CHOWN".to_string()]).unwrap();
        let kill = caps_to_bitmask(&["CAP_KILL".to_string()]).unwrap();
        assert_eq!(mask, chown | kill);
    }

    #[test]
    fn caps_to_bitmask_empty() {
        let caps: Vec<String> = vec![];
        let mask = caps_to_bitmask(&caps).unwrap();
        assert_eq!(mask, 0);
    }

    #[test]
    fn caps_to_bitmask_invalid_fails() {
        let caps = vec!["CAP_CHOWN".to_string(), "CAP_FAKE".to_string()];
        let result = caps_to_bitmask(&caps);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("CAP_FAKE"));
    }

    #[test]
    fn drop_capabilities_validation_invalid_cap() {
        // Invalid capability name in keep list should fail
        let result = drop_capabilities(Some(&["CAP_FAKE".to_string()]));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("CAP_FAKE"));
    }

    #[test]
    fn drop_capabilities_validation_empty_is_ok() {
        // Empty keep list is valid (drop everything)
        let result = drop_capabilities(Some(&[]));
        assert!(result.is_ok());
    }

    #[test]
    fn drop_capabilities_none_is_ok() {
        // None keep list is valid
        let result = drop_capabilities(None);
        assert!(result.is_ok());
    }

    #[test]
    fn set_supplementary_gids_empty_is_noop() {
        // Empty slice should be a no-op
        set_supplementary_gids(&[]);
        // No panic = test passes
    }
}
