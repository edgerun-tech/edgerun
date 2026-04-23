//! edgerun-oci — OCI container runtime and registry client.
//!
//! ## Runtime
//! Full OCI Runtime Spec v1.0.2 implementation with support for:
//! - Container lifecycle: create, start, exec, kill, delete
//! - Cgroups v2 resource management
//! - Seccomp-BPF filtering
//! - Rootless operation (Podman-style clone + re-exec)
//! - All 6 OCI hook types
//! - Network and device eBPF rules
//! - Terminal/PTY with SCM_RIGHTS
//!
//! ## Registry
//! Pull and push images from any OCI-compliant registry:
//! - `RegistryClient::pull()` — resolve manifest, download layers, extract rootfs
//! - `RegistryClient::push()` — create layers, upload blobs, push manifest
//! - `SecretClient` — biometric-gated credential storage via edgerun secret service
//! - Docker Hub, GHCR, GCR, GitLab, Quay, and any private registry

use std::collections::HashMap;

// Runtime modules
pub mod bundle;
pub mod cgroups;
pub mod config_builder;
pub mod criu;
pub mod ebpf_devices;
pub mod ebpf_netcls;
pub mod error;
pub mod fifo;
pub mod handle;
pub mod hooks;
pub mod init;
pub mod json;
pub mod lifecycle;
pub mod process;
pub mod rootfs;
pub mod rootless;
pub mod seccomp;
pub mod state;
pub mod syscalls;
pub mod userns;

/// CLI command implementations.
pub mod cli;

// Registry modules (private, re-exported below)
mod registry {
    pub(crate) mod auth;
    pub(crate) mod client;
    pub(crate) mod config;
    pub(crate) mod dbus_client;
    pub(crate) mod errors;
    pub(crate) mod layer;
    pub(crate) mod manifest;
    pub(crate) mod oci_spec;
    pub(crate) mod urlencoding;
}

// Re-export runtime types
pub use bundle::{create_bundle, write_bundle};
pub use config_builder::ContainerProcessConfig;
pub use error::{CgroupError, CapabilityError, ConfigError, FifoError, LifecycleError, NamespaceError, OciError, RootfsError, SeccompError};
pub use handle::RunningContainer;
pub use hooks::execute_poststop_hooks;
pub use init::fork_and_init;
pub use json::*;
pub use lifecycle::{
    delete_container, fork_container_child, run_bundle, run_create_runtime_hooks,
    run_poststart_hooks, run_prestart_hooks, run_spec, run_spec_with_id,
    run_poststop_and_cleanup, save_created_state, setup_container_cgroups,
    signal_start, start_bundle, start_spec, start_spec_with_id,
    update_state_running,
};
pub use process::{setup_container_child, ContainerConfig};
pub use rootfs::setup_rootfs;
pub use rootless::{generate_gid_map, generate_uid_map};
pub use state::{ContainerState, container_state_dir, fifo_path, load_state, save_state, state_exists, state_file_path, delete_state};
pub use syscalls::*;
pub use userns::drop_capabilities;

// Re-export registry types
pub use registry::auth::{decode_basic_auth, parse_bearer_auth, RegistryAuth};
pub use registry::client::{ImageRef, RegistryClient};
pub use registry::config::{HistoryEntry, ImageConfig, ImageConfigInner, RootFs};
pub use registry::dbus_client::SecretClient;
pub use registry::errors::RegistryError;
pub use registry::layer::{build_rootfs as registry_build_rootfs, extract_layer, verify_blob_digest};
pub use registry::manifest::{
    ImageManifest, LayerDescriptor, ManifestDescriptor, PlatformDescriptor,
    SingleManifest,
};
pub use registry::oci_spec::generate_oci_spec;

// ---------------------------------------------------------------------------
// Namespace helpers — used by bundle.rs, config_builder.rs, process.rs, spec.rs
// ---------------------------------------------------------------------------

pub const DEFAULT_NAMESPACES: &[(&str, Option<&str>)] = &[
    ("mount", None),
    ("pid", None),
    ("network", None),
    ("ipc", None),
    ("uts", None),
    ("cgroup", None),
];

pub fn default_namespaces() -> Vec<OciNamespace> {
    DEFAULT_NAMESPACES.iter().map(|(t, p)| OciNamespace {
        ns_type: t.to_string(),
        path: p.map(String::from),
    }).collect()
}

pub fn namespace_flags(namespaces: &[OciNamespace]) -> i32 {
    let mut flags = 0;
    for ns in namespaces {
        if let Some(flag) = process::ns_type_to_flag(&ns.ns_type) {
            flags |= flag;
        }
    }
    flags
}
