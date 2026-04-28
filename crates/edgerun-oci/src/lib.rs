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

pub mod image_apply;
pub mod image_plan;
pub mod layer_pipeline;
pub mod oci_path;
pub mod rootfs_access;
pub mod runtime_config;
pub mod spec;
mod spec_json;
mod tar_compression;
pub mod tar_layer;
mod tar_whiteout;
#[cfg(all(test, not(target_os = "none")))]
pub(crate) mod test_support;
mod util;
pub mod validate;

mod registry {
    pub mod auth;
    #[cfg(all(feature = "std", not(target_os = "none")))]
    pub(crate) mod bundle_push;
    #[cfg(any(
        feature = "registry-client",
        all(feature = "std", not(target_os = "none"))
    ))]
    pub(crate) mod client;
    pub mod config;
    #[cfg(all(feature = "std", not(target_os = "none")))]
    pub(crate) mod dbus_client;
    #[cfg(feature = "edgefs")]
    pub(crate) mod edgefs_pull;
    pub mod errors;
    pub(crate) mod image_ref;
    #[cfg(all(feature = "std", not(target_os = "none")))]
    pub(crate) mod layer;
    pub mod manifest;
    pub(crate) mod oci_spec;
    #[cfg(all(feature = "std", not(target_os = "none")))]
    pub(crate) mod pull;
    #[cfg(all(feature = "std", not(target_os = "none")))]
    pub(crate) mod push_manifest;
    #[cfg(all(feature = "std", not(target_os = "none")))]
    pub(crate) mod tar_push;
    #[cfg(any(
        feature = "registry-client",
        all(feature = "std", not(target_os = "none"))
    ))]
    pub(crate) mod urlencoding;
}

pub mod bare_rootfs;
pub mod bare_syscall;
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
#[cfg(feature = "edgefs")]
pub mod edgefs;
pub mod elf;
mod elf_memory;
mod elf_stack;
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
pub(crate) mod lifecycle_child;
mod linux_catalog;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub mod process;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub(crate) mod process_config;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub(crate) mod process_exec;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub mod rootfs;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub mod rootfs_copy;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub(crate) mod rootfs_devices;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub(crate) mod rootfs_idmap;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub mod rootfs_layers;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub mod rootless;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub mod seccomp;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub mod state;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub mod syscalls;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub(crate) mod terminal;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub mod userns;

#[cfg(all(feature = "std", not(target_os = "none")))]
#[path = "../cli/mod.rs"]
pub mod cli;

pub use bare_rootfs::{BareRootfs, BareRootfsEntry, BareRootfsEntryKind};
pub use bare_syscall::{
    dispatch_linux_syscall, dispatch_x86_64_linux_syscall_frame, OciBufferSyscallSink,
    OciSliceSyscallMemory, OciSyscallAction, OciSyscallError, OciSyscallMemory, OciSyscallSink,
    OciX86_64SyscallFrame, OCI_LINUX_SYS_EXIT, OCI_LINUX_SYS_EXIT_GROUP, OCI_LINUX_SYS_WRITE,
};
#[cfg(all(feature = "std", not(target_os = "none")))]
pub use bundle::{create_bundle, write_bundle};
#[cfg(all(feature = "std", not(target_os = "none")))]
pub use config_builder::ContainerProcessConfig;
#[cfg(feature = "edgefs")]
pub use edgefs::EdgeFsLayerSink;
pub use elf::{
    build_elf64_auxv, build_elf_load_plan, build_elf_memory_map,
    build_elf_memory_map_with_load_bias, build_elf_memory_map_with_page_size,
    build_elf_memory_map_with_page_size_and_load_bias, build_elf_runtime_layout,
    build_elf_runtime_mapping_list, build_elf_runtime_memory_map,
    build_elf_runtime_memory_map_with_load_bias, build_elf_runtime_memory_map_with_page_size,
    build_elf_runtime_memory_map_with_page_size_and_load_bias, build_elf_runtime_plan,
    build_launch_elf_load_plan, build_launch_elf_runtime_plan, inspect_elf, inspect_launch_elf,
    load_prepared_elf64_program, load_prepared_elf64_program_aligned,
    prepare_and_load_oci_elf_program, prepare_and_load_oci_elf_program_with_load_bias,
    prepare_and_load_oci_elf_program_with_page_size_and_load_bias, prepare_oci_elf_program,
    prepare_oci_elf_program_with_load_bias, prepare_oci_elf_program_with_page_size,
    prepare_oci_elf_program_with_page_size_and_load_bias, read_elf_mapping_chunk,
    read_elf_runtime_mapping_chunk, read_elf_runtime_mapping_list_chunk,
    read_elf_runtime_segment_chunk, read_elf_segment_chunk, write_elf64_initial_stack,
    write_elf64_initial_stack_aligned, write_prepared_elf64_initial_stack,
    write_prepared_elf64_initial_stack_aligned, write_prepared_elf64_launch_state,
    write_prepared_elf64_launch_state_aligned, OciElfAuxvEntry, OciElfError, OciElfImage,
    OciElfInfo, OciElfInitialStack, OciElfLoadBias, OciElfLoadPlan, OciElfLoadSegment,
    OciElfMachine, OciElfMapper, OciElfMapping, OciElfMemoryMap, OciElfPermissions,
    OciElfProgramHeader, OciElfRuntimeLayout, OciElfRuntimeMapping, OciElfRuntimeMemoryMap,
    OciElfRuntimePlan, OciElfSegmentRead, OciElfType, OciElfUnsafeIdentityMapper,
    OciPreparedLaunchState, OciPreparedProgram, OCI_ELF_AT_BASE, OCI_ELF_AT_ENTRY,
    OCI_ELF_AT_FLAGS, OCI_ELF_AT_NULL, OCI_ELF_AT_PAGESZ, OCI_ELF_AT_PHDR, OCI_ELF_AT_PHENT,
    OCI_ELF_AT_PHNUM,
};
#[cfg(target_arch = "x86_64")]
pub use elf::{enter_elf64, enter_elf64_launch_state};
#[cfg(all(feature = "std", not(target_os = "none")))]
pub use error::{
    CapabilityError, CgroupError, ConfigError, FifoError, LifecycleError, NamespaceError, OciError,
    RootfsError, SeccompError,
};
#[cfg(all(feature = "std", not(target_os = "none")))]
pub use handle::RunningContainer;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub use hooks::execute_poststop_hooks;
pub use image_apply::{
    apply_bare_image_layer_blobs, apply_bare_image_layer_blobs_sha256, BareImageApplyError,
    BareImageApplyReport,
};
pub use image_plan::{
    parse_single_manifest_bytes, platform_matches, select_manifest_for_current_target,
    select_manifest_for_target, selected_manifest_digest_for_current_target,
    selected_manifest_digest_from_index_bytes, single_manifest, validate_digest_reference,
    BareImagePlan, ImagePlanError,
};
#[cfg(all(feature = "std", not(target_os = "none")))]
pub use init::fork_and_init;
pub use layer_pipeline::{
    apply_layer_chunks, bytes_to_hex, format_digest, sha256_digest_reference, sha256_layer_digest,
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
pub use oci_path::{layer_path_safe, normalize_layer_path};
#[cfg(all(feature = "std", not(target_os = "none")))]
pub use process::{setup_container_child, ContainerConfig};
#[cfg(all(feature = "std", not(target_os = "none")))]
pub use rootfs::setup_rootfs;
pub use rootfs_access::{
    build_launch_plan, resolve_executable, resolve_executable_path, OciDeviceId, OciExecutable,
    OciLaunchPlan, OciRootfs, OciRootfsEntry, OciRootfsEntryKind, OciRootfsError,
};
#[cfg(all(feature = "std", not(target_os = "none")))]
pub use rootless::{generate_gid_map, generate_uid_map};
pub use runtime_config::{BareNamespace, BareNamespaceKind, BareRuntimeConfig};
pub use spec::*;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub use state::{
    container_state_dir, delete_state, fifo_path, load_state, save_state, state_exists,
    state_file_path, ContainerState,
};
#[cfg(all(feature = "std", not(target_os = "none")))]
pub use syscalls::*;
pub use tar_layer::{
    apply_uncompressed_tar_layer, apply_uncompressed_tar_layer_streaming,
    apply_validated_tar_layer, apply_validated_tar_layer_sha256,
    apply_validated_uncompressed_tar_layer, decompress_gzip_layer, decompress_zstd_layer,
    layer_compression, parse_oci_whiteout, validate_and_decode_tar_layer,
    validate_and_decode_tar_layer_sha256, DecodedTarLayer, OciLayerCompression, OciWhiteout,
    TarEntry, TarEntryKind, TarLayerApplyError, TarLayerApplyReport, TarLayerError, TarLayerSink,
    UncompressedTarStream,
};
#[cfg(all(feature = "std", not(target_os = "none")))]
pub use userns::drop_capabilities;
pub use validate::{host_arch, host_os, validate_spec, OciValidationError};

pub use registry::auth::{decode_basic_auth, parse_bearer_auth, RegistryAuth};
#[cfg(any(
    feature = "registry-client",
    all(feature = "std", not(target_os = "none"))
))]
pub use registry::client::RegistryClient;
pub use registry::config::{
    parse_image_config, parse_json_bytes, parse_manifest, parse_single_manifest, HistoryEntry,
    ImageConfig, ImageConfigInner, RootFs,
};
#[cfg(all(feature = "std", not(target_os = "none")))]
pub use registry::dbus_client::SecretClient;
#[cfg(feature = "edgefs")]
pub use registry::edgefs_pull::EdgeFsImagePullReport;
pub use registry::errors::RegistryError;
#[cfg(any(
    feature = "registry-client",
    all(feature = "std", not(target_os = "none"))
))]
pub use registry::image_ref::ImageRef;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub use registry::layer::{
    build_rootfs as registry_build_rootfs, extract_layer, verify_blob_digest,
};
pub use registry::manifest::{
    ImageManifest, LayerDescriptor, ManifestDescriptor, PlatformDescriptor, SingleManifest,
};
pub use registry::oci_spec::{generate_oci_spec, generate_oci_spec_model};
#[cfg(all(feature = "std", not(target_os = "none")))]
pub use registry::pull::{ImagePullReport, PullProgress};

pub const DEFAULT_NAMESPACES: &[(&str, Option<&str>)] = &[
    ("mount", None),
    ("pid", None),
    ("network", None),
    ("ipc", None),
    ("uts", None),
    ("cgroup", None),
];
pub const DEFAULT_MASKED_PATHS: &[&str] = &[
    "/proc/acpi",
    "/proc/kcore",
    "/proc/keys",
    "/proc/latency_stats",
    "/proc/timer_list",
    "/proc/timer_stats",
    "/proc/sched_debug",
    "/proc/scsi",
    "/sys/firmware",
];
pub const DEFAULT_READONLY_PATHS: &[&str] = &[
    "/proc/asound",
    "/proc/bus",
    "/proc/fs",
    "/proc/irq",
    "/proc/sys",
    "/proc/sysrq-trigger",
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

pub fn default_masked_paths() -> alloc::vec::Vec<alloc::string::String> {
    DEFAULT_MASKED_PATHS
        .iter()
        .map(|path| (*path).into())
        .collect()
}

pub fn default_readonly_paths() -> alloc::vec::Vec<alloc::string::String> {
    DEFAULT_READONLY_PATHS
        .iter()
        .map(|path| (*path).into())
        .collect()
}

pub fn namespace_flags(namespaces: &[OciNamespace]) -> i32 {
    namespace_flags_from_types(namespaces.iter().map(|ns| ns.ns_type.as_str()))
}

pub(crate) fn namespace_flags_from_types<'a>(
    namespace_types: impl IntoIterator<Item = &'a str>,
) -> i32 {
    let mut flags = 0;

    #[cfg(all(feature = "std", not(target_os = "none")))]
    {
        for ns_type in namespace_types {
            if let Some(flag) = process::ns_type_to_flag(ns_type) {
                flags |= flag;
            }
        }
    }

    #[cfg(any(not(feature = "std"), target_os = "none"))]
    {
        let _ = namespace_types;
    }

    flags
}
