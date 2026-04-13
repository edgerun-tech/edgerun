//! Minimal OCI container runtime — kernel-only, no external tools.
//!
//! Uses Linux kernel primitives: namespaces, cgroups v2, pivot_root, mount.
//! No Docker, no runc, no systemd, no libc crate. Just raw syscalls and std.
//!
//! ## Module structure
//!
//! - `syscalls` — Raw syscall FFI, constants, wrappers
//! - `seccomp`  — Seccomp-BPF filtering (arch-aware)
//! - `userns`   — UID/GID mapping, capability dropping
//! - `cgroups`  — Cgroups v2 resource management
//! - `rootfs`   — Rootfs setup: pivot_root, mounts, devices, whiteouts
//! - `bundle`   — OCI bundle creation and serialization
//! - `container` — Blocking and non-blocking container lifecycle

pub mod json;
pub use json::*;

pub mod seccomp;
pub mod userns;
pub mod cgroups;
pub mod rootfs;
pub mod fifo;
pub mod error;
pub mod config_builder;
pub mod ebpf_devices;
pub mod ebpf_netcls;
mod bundle;
pub mod process;
mod handle;
mod lifecycle;
mod hooks;
mod init;
mod state;
mod rootless;
pub use state::{state_exists, state_file_path, fifo_path, set_state_dir, is_root, STATE_DIR};
pub use rootless::{
    generate_uid_map, generate_gid_map, resolve_cgroup_delegation_path,
    resolve_container_cgroup_path, is_cgroup_v2_available,
    get_current_user_subuids, get_current_user_subgids,
};
pub mod syscalls;
pub mod cli;

// Re-export public API
pub use bundle::{create_bundle, write_bundle};
pub use cgroups::shares_to_weight;
pub use config_builder::{ContainerProcessConfig, ContainerConfigBuilder};
pub use lifecycle::{
    run_bundle, run_spec, run_spec_with_id, start_bundle, start_spec, start_spec_with_id,
    ForkedChild, save_created_state, signal_start, setup_container_cgroups,
    run_poststart_hooks, update_state_running, into_running_container,
    run_poststop_and_cleanup, RunningContainer, delete_container,
};
pub use rootfs::{apply_whiteouts, build_rootfs};
pub use hooks::{
    ContainerState,
    execute_prestart_hooks, execute_create_runtime_hooks,
    execute_create_container_hooks, execute_start_container_hooks,
    execute_poststart_hooks, execute_poststop_hooks,
};
pub use error::{
    OciError, LifecycleError, RootfsError, SeccompError,
    CgroupError, ConfigError, FifoError, CapabilityError, NamespaceError,
};

// ===========================================================================
// Namespace helpers (need to be here since they use syscalls::ns)
// ===========================================================================

pub fn default_namespaces() -> Vec<OciNamespace> {
    vec![
        OciNamespace { ns_type: "user".into(), path: None },
        OciNamespace { ns_type: "mount".into(), path: None },
        OciNamespace { ns_type: "pid".into(), path: None },
        OciNamespace { ns_type: "network".into(), path: None },
        OciNamespace { ns_type: "ipc".into(), path: None },
        OciNamespace { ns_type: "uts".into(), path: None },
        OciNamespace { ns_type: "cgroup".into(), path: None },
    ]
}

/// Resolve namespace clone flags from OCI namespace type strings.
/// Returns `syscalls::ns::CONTAINER` (all six namespaces) when the spec has no
/// explicit namespace configuration — the OCI default is to create new
/// namespaces for the container.
pub fn namespace_flags(namespaces: &[OciNamespace]) -> std::os::raw::c_int {
    use syscalls::ns;
    use std::os::raw::c_int;

    if namespaces.is_empty() {
        return ns::CONTAINER;
    }
    let mut flags: c_int = 0;
    for ns in namespaces {
        flags |= process::ns_type_to_flag(&ns.ns_type).unwrap_or(0);
    }
    flags
}
