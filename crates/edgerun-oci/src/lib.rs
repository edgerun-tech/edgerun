//! edgerun-oci — OCI container runtime and registry client.
//!
//! The default build is a no_std, alloc-backed OCI data model and parser layer.
//! Host runtime, registry network I/O, archive extraction, and CLI support are
//! available behind the `std` feature.

#![no_std]

extern crate alloc;

#[cfg_attr(all(feature = "std", not(target_os = "none")), macro_use)]
#[cfg(all(feature = "std", not(target_os = "none")))]
extern crate std;

pub mod prelude {
    pub use alloc::boxed::Box;
    pub use alloc::format;
    pub use alloc::string::{String, ToString};
    pub use alloc::vec;
    pub use alloc::vec::Vec;

    #[cfg(all(feature = "std", not(target_os = "none")))]
    pub use std::{eprintln, print, println};
}

#[cfg(feature = "json")]
pub mod image_apply;
#[cfg(feature = "json")]
pub mod image_plan;
pub mod json;
#[cfg(feature = "json")]
pub mod layer_pipeline;
#[cfg(feature = "json")]
pub mod oci_path;
pub mod runtime_config;
#[cfg(feature = "json")]
pub mod tar_layer;
#[cfg(all(test, feature = "json"))]
pub(crate) mod test_support;
pub mod validate;

mod registry {
    pub mod auth;
    #[cfg(all(feature = "std", not(target_os = "none")))]
    pub(crate) mod client;
    pub mod config;
    #[cfg(all(feature = "std", not(target_os = "none")))]
    pub(crate) mod dbus_client;
    pub mod errors;
    #[cfg(all(feature = "std", not(target_os = "none")))]
    pub(crate) mod layer;
    pub mod manifest;
    #[cfg(feature = "json")]
    pub(crate) mod oci_spec;
    #[cfg(all(feature = "std", not(target_os = "none")))]
    pub(crate) mod urlencoding;
}

#[cfg(feature = "json")]
pub mod bare_rootfs;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub mod bundle;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub mod cgroups;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub mod config_builder;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub mod criu;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub mod ebpf_devices;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub mod ebpf_netcls;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub mod error;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub mod fifo;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub mod handle;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub mod hooks;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub mod init;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub mod lifecycle;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub mod process;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub mod rootfs;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub mod rootless;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub mod seccomp;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub mod state;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub mod syscalls;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub mod userns;

#[cfg(all(feature = "std", not(target_os = "none")))]
pub mod cli;

#[cfg(feature = "json")]
pub use bare_rootfs::{BareRootfs, BareRootfsEntry, BareRootfsEntryKind};
#[cfg(all(feature = "std", not(target_os = "none")))]
pub use bundle::{create_bundle, write_bundle};
#[cfg(all(feature = "std", not(target_os = "none")))]
pub use config_builder::ContainerProcessConfig;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub use error::{
    CapabilityError, CgroupError, ConfigError, FifoError, LifecycleError, NamespaceError, OciError,
    RootfsError, SeccompError,
};
#[cfg(all(feature = "std", not(target_os = "none")))]
pub use handle::RunningContainer;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub use hooks::execute_poststop_hooks;
#[cfg(feature = "json")]
pub use image_apply::{
    apply_bare_image_layer_blobs, apply_bare_image_layer_blobs_sha256, BareImageApplyError,
    BareImageApplyReport,
};
#[cfg(feature = "json")]
pub use image_plan::{
    parse_single_manifest_bytes, platform_matches, select_manifest_for_current_target,
    select_manifest_for_target, selected_manifest_digest_for_current_target,
    selected_manifest_digest_from_index_bytes, single_manifest, validate_digest_reference,
    BareImagePlan, ImagePlanError,
};
#[cfg(all(feature = "std", not(target_os = "none")))]
pub use init::fork_and_init;
pub use json::*;
#[cfg(feature = "json")]
pub use layer_pipeline::{
    apply_layer_chunks, bytes_to_hex, format_digest, sha256_layer_digest,
    validate_layer_descriptor, LayerApplyReport, LayerDigest, LayerPipelineError, LayerSink,
    Sha256LayerDigest,
};
#[cfg(all(feature = "std", not(target_os = "none")))]
pub use lifecycle::{
    delete_container, fork_container_child, run_bundle, run_create_runtime_hooks,
    run_poststart_hooks, run_poststop_and_cleanup, run_prestart_hooks, run_spec, run_spec_with_id,
    save_created_state, setup_container_cgroups, signal_start, start_bundle, start_spec,
    start_spec_with_id, update_state_running,
};
#[cfg(feature = "json")]
pub use oci_path::{layer_path_safe, normalize_layer_path};
#[cfg(all(feature = "std", not(target_os = "none")))]
pub use process::{setup_container_child, ContainerConfig};
#[cfg(all(feature = "std", not(target_os = "none")))]
pub use rootfs::setup_rootfs;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub use rootless::{generate_gid_map, generate_uid_map};
pub use runtime_config::{BareNamespace, BareNamespaceKind, BareRuntimeConfig};
#[cfg(all(feature = "std", not(target_os = "none")))]
pub use state::{
    container_state_dir, delete_state, fifo_path, load_state, save_state, state_exists,
    state_file_path, ContainerState,
};
#[cfg(all(feature = "std", not(target_os = "none")))]
pub use syscalls::*;
#[cfg(feature = "json")]
pub use tar_layer::{
    apply_uncompressed_tar_layer, apply_validated_tar_layer, apply_validated_tar_layer_sha256,
    apply_validated_uncompressed_tar_layer, decompress_gzip_layer, decompress_zstd_layer,
    layer_compression, validate_and_decode_tar_layer, validate_and_decode_tar_layer_sha256,
    DecodedTarLayer, OciLayerCompression, OciWhiteout, TarEntry, TarEntryKind, TarLayerApplyError,
    TarLayerApplyReport, TarLayerError, TarLayerSink,
};
#[cfg(all(feature = "std", not(target_os = "none")))]
pub use userns::drop_capabilities;
pub use validate::{host_arch, host_os, validate_spec, OciValidationError};

pub use registry::auth::{decode_basic_auth, parse_bearer_auth, RegistryAuth};
#[cfg(all(feature = "std", not(target_os = "none")))]
pub use registry::client::{ImageRef, RegistryClient};
#[cfg(feature = "json")]
pub use registry::config::{
    parse_image_config, parse_json_bytes, parse_manifest, parse_single_manifest, HistoryEntry,
    ImageConfig, ImageConfigInner, RootFs,
};
#[cfg(not(feature = "json"))]
pub use registry::config::{HistoryEntry, ImageConfig, ImageConfigInner, RootFs};
#[cfg(all(feature = "std", not(target_os = "none")))]
pub use registry::dbus_client::SecretClient;
pub use registry::errors::RegistryError;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub use registry::layer::{
    build_rootfs as registry_build_rootfs, extract_layer, verify_blob_digest,
};
pub use registry::manifest::{
    ImageManifest, LayerDescriptor, ManifestDescriptor, PlatformDescriptor, SingleManifest,
};
#[cfg(feature = "json")]
pub use registry::oci_spec::{generate_oci_spec, generate_oci_spec_model};

pub const DEFAULT_NAMESPACES: &[(&str, Option<&str>)] = &[
    ("mount", None),
    ("pid", None),
    ("network", None),
    ("ipc", None),
    ("uts", None),
    ("cgroup", None),
];

pub fn default_namespaces() -> alloc::vec::Vec<OciNamespace> {
    use alloc::string::{String, ToString};

    DEFAULT_NAMESPACES
        .iter()
        .map(|(t, p)| OciNamespace {
            ns_type: t.to_string(),
            path: p.map(String::from),
        })
        .collect()
}

pub fn namespace_flags(namespaces: &[OciNamespace]) -> i32 {
    let mut flags = 0;
    for ns in namespaces {
        #[cfg(all(feature = "std", not(target_os = "none")))]
        if let Some(flag) = process::ns_type_to_flag(&ns.ns_type) {
            flags |= flag;
        }

        #[cfg(any(not(feature = "std"), target_os = "none"))]
        {
            let _ = ns;
        }
    }
    flags
}
