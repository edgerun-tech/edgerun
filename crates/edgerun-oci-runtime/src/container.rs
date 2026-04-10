//! Container lifecycle management — blocking and non-blocking execution.
//!
//! Fixes applied:
//! - **Seccomp fail-closed in `start_spec`** (was silently logging to /tmp)
//! - **UID/GID mapping from spec** (was hardcoded 65534)
//! - **Capability dropping from spec** (was not implemented)

use std::fs;
use std::io;
use std::os::raw::c_int;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

use crate::json::{OciIdMapping, OciLinuxDevice, OciRoot, OciSpec};
use crate::default_namespaces;
use crate::namespace_flags;
use crate::cgroups::setup_cgroups;
use crate::rootfs::setup_rootfs;
use crate::seccomp::apply_seccomp;
use crate::syscalls::{
    do_set_hostname, do_unshare, kill,
    SIGKILL, SIGTERM,
};
use crate::userns::{
    apply_security_hardening, drop_capabilities, do_setgid, do_setuid,
    set_supplementary_gids,
};

/// Serialize spec devices to a JSON string (Clone-friendly for FnMut closure).
fn serialize_devices(devices: Option<&[OciLinuxDevice]>) -> String {
    match devices {
        Some(devs) => edgerun_json::to_string(devs).unwrap_or_default(),
        None => String::new(),
    }
}

/// Deserialize spec devices from a JSON string.
fn deserialize_devices(json: &str) -> Vec<OciLinuxDevice> {
    if json.is_empty() {
        return Vec::new();
    }
    edgerun_json::from_slice::<Vec<OciLinuxDevice>>(json.as_bytes())
        .unwrap_or_default()
}

// ===========================================================================
// UID/GID map content serialization (Clone-friendly for FnMut closure)
// ===========================================================================

/// Format UID mappings into the /proc/self/uid_map file content.
fn format_uid_map_content(mappings: Option<&[OciIdMapping]>) -> String {
    if let Some(maps) = mappings {
        if maps.is_empty() {
            return "0 65534 1\n".to_string();
        }
        maps.iter()
            .map(|m| format!("{} {} {}\n", m.container_id, m.host_id, m.size))
            .collect()
    } else {
        "0 65534 1\n".to_string()
    }
}

/// Format GID mappings into the /proc/self/gid_map file content.
fn format_gid_map_content(mappings: Option<&[OciIdMapping]>) -> String {
    if let Some(maps) = mappings {
        if maps.is_empty() {
            return "0 65534 1\n".to_string();
        }
        maps.iter()
            .map(|m| format!("{} {} {}\n", m.container_id, m.host_id, m.size))
            .collect()
    } else {
        "0 65534 1\n".to_string()
    }
}

/// Write UID map content to /proc/self/uid_map and deny setgroups.
fn write_uid_map_content(content: &str) -> io::Result<()> {
    fs::write("/proc/self/uid_map", content)?;
    let _ = fs::write("/proc/self/setgroups", "deny");
    Ok(())
}

/// Write GID map content to /proc/self/gid_map.
fn write_gid_map_content(content: &str) -> io::Result<()> {
    fs::write("/proc/self/gid_map", content)?;
    Ok(())
}

// ===========================================================================
// Blocking container execution
// ===========================================================================

/// Run an OCI bundle (directory containing config.json + rootfs/).
pub fn run_bundle(bundle_path: &Path) -> io::Result<std::process::ExitStatus> {
    let config_path = bundle_path.join("config.json");
    let config_data = fs::read(&config_path)?;
    let spec: OciSpec = crate::json::parse_oci_spec(&config_data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("invalid OCI config: {}", e)))?;
    run_spec(&spec)
}

/// Run a container from a parsed OCI spec.
pub fn run_spec(spec: &OciSpec) -> io::Result<std::process::ExitStatus> {
    let root = spec.root.clone()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no root in OCI spec"))?;

    let namespaces = spec.linux.as_ref()
        .and_then(|l| l.namespaces.as_ref())
        .cloned()
        .unwrap_or_else(default_namespaces);
    let ns_flags = namespace_flags(&namespaces);

    let hostname = spec.hostname.clone().unwrap_or_else(|| "edgerun".into());
    let process = spec.process.clone().unwrap_or_default();
    let linux = spec.linux.clone().unwrap_or_default();

    let args = process.args.clone().unwrap_or_else(|| vec!["/bin/sh".into()]);
    let env = process.env.clone().unwrap_or_else(|| vec![
        "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".into(),
        "TERM=xterm".into(),
    ]);
    let cwd = process.cwd.clone().unwrap_or_else(|| "/".into());

    let uid = process.user.as_ref().and_then(|u| u.uid).unwrap_or(0);
    let gid = process.user.as_ref().and_then(|u| u.gid).unwrap_or(0);
    let additional_gids = process.user.as_ref()
        .and_then(|u| u.additional_gids.as_ref())
        .cloned()
        .unwrap_or_default();
    let no_new_privs = process.no_new_privileges.unwrap_or(true);

    // Extract bounding capabilities from spec
    let bounding_caps: Option<Vec<String>> = process.capabilities.as_ref()
        .and_then(|c| c.bounding.clone());

    let root_path = root.path.clone();
    let root_readonly = root.readonly;
    let mounts = spec.mounts.clone();
    let masked = linux.masked_paths.clone();
    let readonly = linux.readonly_paths.clone();
    let resources = linux.resources.clone();
    let cgroup_path = linux.cgroups_path.clone().unwrap_or_else(|| "/edgerun".into());
    let hostname_c = hostname.clone();

    // Serialize UID/GID mappings to strings (Clone, works with FnMut closure)
    let uid_map_content = format_uid_map_content(linux.uid_mappings.as_deref());
    let gid_map_content = format_gid_map_content(linux.gid_mappings.as_deref());

    // Serialize spec devices to JSON string (Clone-friendly for FnMut closure)
    let spec_devices_json = serialize_devices(linux.devices.as_deref());

    let mut cmd = Command::new(&args[0]);
    cmd.args(&args[1..]);
    cmd.current_dir(&cwd);
    cmd.env_clear();
    for e in &env {
        if let Some((k, v)) = e.split_once('=') {
            cmd.env(k, v);
        }
    }

    // Pre-exec: unshare namespaces, setup rootfs, drop privileges
    // SAFETY: This runs in a forked child before execve.
    unsafe {
        cmd.pre_exec(move || {
            // 1. Unshare namespaces
            do_unshare(ns_flags)?;

            // 2. Write uid_map/gid_map from spec
            write_uid_map_content(&uid_map_content)?;
            write_gid_map_content(&gid_map_content)?;

            // 3. Set hostname
            let _ = do_set_hostname(&hostname_c);

            // 4. Security hardening: no_new_privs + non-dumpable
            apply_security_hardening(no_new_privs)?;

            // 5. Drop capabilities not in spec's bounding set
            drop_capabilities(bounding_caps.as_deref())?;

            // 6. Apply seccomp-BPF filter (deny all but essential syscalls)
            // Fail-closed: if seccomp cannot be applied, the container is not secure.
            apply_seccomp().map_err(|e| {
                io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!("seccomp filter failed to apply: {}. Container startup aborted for security.", e),
                )
            })?;

            // 7. Setup rootfs (pivot_root, mount filesystems, create devices)
            let oci_root = OciRoot { path: root_path.clone(), readonly: root_readonly };
            let spec_devices = deserialize_devices(&spec_devices_json);
            setup_rootfs(
                &oci_root,
                mounts.as_deref(),
                masked.as_deref(),
                readonly.as_deref(),
                if spec_devices.is_empty() { None } else { Some(&spec_devices) },
            )?;

            // 8. Set supplementary groups
            if !additional_gids.is_empty() {
                set_supplementary_gids(&additional_gids);
            }

            // 9. Drop GID then UID
            do_setgid(gid)?;
            do_setuid(uid)?;

            Ok(())
        });
    }

    // Spawn the child
    let mut child = cmd.spawn()?;

    // Set cgroups from parent (we have the child's PID)
    if let Some(ref res) = resources {
        let _ = setup_cgroups(child.id(), res, &cgroup_path);
    }

    child.wait()
}

// ===========================================================================
// Non-blocking container lifecycle (for preemption support)
// ===========================================================================

/// A handle to a running container that can be awaited or killed.
pub struct RunningContainer {
    child: std::process::Child,
    cgroup_path: String,
}

impl RunningContainer {
    /// Block until the container exits and return its exit status.
    pub fn wait(mut self) -> io::Result<std::process::ExitStatus> {
        self.child.wait()
    }

    /// Kill the container with SIGTERM, then SIGKILL if it doesn't exit.
    /// Returns the exit status after killing.
    pub fn kill(mut self) -> io::Result<std::process::ExitStatus> {
        let pid = self.child.id();

        // Try SIGTERM first
        let _ = unsafe { kill(pid as c_int, SIGTERM) };

        // Wait up to 5 seconds for graceful exit
        for _ in 0..50 {
            match self.child.try_wait()? {
                Some(status) => return Ok(status),
                None => std::thread::sleep(std::time::Duration::from_millis(100)),
            }
        }

        // SIGKILL if still alive
        let _ = unsafe { kill(pid as c_int, SIGKILL) };
        self.child.wait()
    }

    /// Kill the container cgroup (kills all processes in the cgroup).
    /// More reliable than killing a single PID for container preemption.
    pub fn kill_cgroup(&self) {
        let cgroup_root = Path::new("/sys/fs/cgroup").join(self.cgroup_path.trim_start_matches('/'));
        // Write 1 to cgroup.kill (cgroup v2, kernel 5.15+)
        let _ = fs::write(cgroup_root.join("cgroup.kill"), "1");
    }

    /// The host PID of the container's init process.
    pub fn pid(&self) -> u32 {
        self.child.id()
    }
}

/// Start a container from an OCI bundle without blocking.
/// Returns a `RunningContainer` handle that can be awaited or killed.
pub fn start_bundle(bundle_path: &Path) -> io::Result<RunningContainer> {
    let config_path = bundle_path.join("config.json");
    let config_data = fs::read(&config_path)?;
    let spec: OciSpec = crate::json::parse_oci_spec(&config_data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("invalid OCI config: {}", e)))?;
    start_spec(&spec)
}

/// Start a container from a parsed OCI spec without blocking.
///
/// **Seccomp is fail-closed** — if the filter cannot be applied, the container
/// will not start. This prevents accidentally running an unsandboxed container.
pub fn start_spec(spec: &OciSpec) -> io::Result<RunningContainer> {
    let root = spec.root.clone()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no root in OCI spec"))?;

    let namespaces = spec.linux.as_ref()
        .and_then(|l| l.namespaces.as_ref())
        .cloned()
        .unwrap_or_else(default_namespaces);
    let ns_flags = namespace_flags(&namespaces);

    let hostname = spec.hostname.clone().unwrap_or_else(|| "edgerun".into());
    let process = spec.process.clone().unwrap_or_default();
    let linux = spec.linux.clone().unwrap_or_default();

    let args = process.args.clone().unwrap_or_else(|| vec!["/bin/sh".into()]);
    let env = process.env.clone().unwrap_or_else(|| vec![
        "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".into(),
        "TERM=xterm".into(),
    ]);
    let cwd = process.cwd.clone().unwrap_or_else(|| "/".into());

    let uid = process.user.as_ref().and_then(|u| u.uid).unwrap_or(0);
    let gid = process.user.as_ref().and_then(|u| u.gid).unwrap_or(0);
    let additional_gids = process.user.as_ref()
        .and_then(|u| u.additional_gids.as_ref())
        .cloned()
        .unwrap_or_default();
    let no_new_privs = process.no_new_privileges.unwrap_or(true);

    // Extract bounding capabilities from spec
    let bounding_caps: Option<Vec<String>> = process.capabilities.as_ref()
        .and_then(|c| c.bounding.clone());

    let root_path = root.path.clone();
    let root_readonly = root.readonly;
    let mounts = spec.mounts.clone();
    let masked = linux.masked_paths.clone();
    let readonly = linux.readonly_paths.clone();
    let resources = linux.resources.clone();
    let cgroup_path = linux.cgroups_path.clone().unwrap_or_else(|| "/edgerun".into());
    let hostname_c = hostname.clone();

    // Serialize UID/GID mappings to strings (Clone, works with FnMut closure)
    let uid_map_content = format_uid_map_content(linux.uid_mappings.as_deref());
    let gid_map_content = format_gid_map_content(linux.gid_mappings.as_deref());

    // Serialize spec devices to JSON string (Clone-friendly for FnMut closure)
    let spec_devices_json = serialize_devices(linux.devices.as_deref());

    let mut cmd = Command::new(&args[0]);
    cmd.args(&args[1..]);
    cmd.current_dir(&cwd);
    cmd.env_clear();
    for e in &env {
        if let Some((k, v)) = e.split_once('=') {
            cmd.env(k, v);
        }
    }

    // Pre-exec: unshare namespaces, setup rootfs, drop privileges
    unsafe {
        cmd.pre_exec(move || {
            // 1. Unshare namespaces
            do_unshare(ns_flags)?;

            // 2. Write uid_map/gid_map from spec
            write_uid_map_content(&uid_map_content)?;
            write_gid_map_content(&gid_map_content)?;

            // 3. Set hostname
            let _ = do_set_hostname(&hostname_c);

            // 4. Security hardening: no_new_privs + non-dumpable
            apply_security_hardening(no_new_privs)?;

            // 5. Drop capabilities not in spec's bounding set
            drop_capabilities(bounding_caps.as_deref())?;

            // 6. Apply seccomp-BPF filter — FAIL-CLOSED
            // If seccomp cannot be applied, the container MUST NOT start.
            apply_seccomp().map_err(|e| {
                io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!("seccomp filter failed to apply: {}. Container startup aborted for security.", e),
                )
            })?;

            // 7. Setup rootfs (pivot_root, mount filesystems, create devices)
            let oci_root = OciRoot { path: root_path.clone(), readonly: root_readonly };
            let spec_devices = deserialize_devices(&spec_devices_json);
            setup_rootfs(
                &oci_root,
                mounts.as_deref(),
                masked.as_deref(),
                readonly.as_deref(),
                if spec_devices.is_empty() { None } else { Some(&spec_devices) },
            )?;

            // 8. Set supplementary groups
            if !additional_gids.is_empty() {
                set_supplementary_gids(&additional_gids);
            }

            // 9. Drop GID then UID
            do_setgid(gid)?;
            do_setuid(uid)?;

            Ok(())
        });
    }

    // Spawn the child
    let child = cmd.spawn()?;

    // Set cgroups from parent
    if let Some(ref res) = resources {
        let _ = setup_cgroups(child.id(), res, &cgroup_path);
    }

    Ok(RunningContainer {
        child,
        cgroup_path,
    })
}
