//! OCI image/distribution specification data model and parsers.
//!
//! This module owns OCI JSON bytes and host-independent descriptor selection.
//! It does not pull registries, unpack layers, mount root filesystems, or start
//! containers.

pub mod config;
pub mod image_ref;
pub mod layer;
pub mod linux_catalog;
pub mod manifest;
pub mod runtime_spec;
mod runtime_spec_json;
pub mod validation;

pub use config::{
    HistoryEntry, ImageConfig, ImageConfigInner, RootFs, parse_image_config, parse_json_bytes,
    parse_manifest, parse_single_manifest,
};
pub use image_ref::ImageRef;
pub use layer::{OciLayerCompression, OciWhiteout, layer_compression, parse_oci_whiteout};
pub use manifest::{
    ImageIndex, ImageManifest, LayerDescriptor, ManifestDescriptor, PlatformDescriptor,
    SingleManifest, platform_matches, select_manifest_for_target,
    selected_manifest_digest_from_index_bytes, single_manifest, validate_digest_reference,
};
pub use runtime_spec::{
    OciBox, OciCapabilities, OciHook, OciHooks, OciIdMapping, OciIoPriority, OciLinux,
    OciLinuxBlockIO, OciLinuxCpu, OciLinuxDevice, OciLinuxDeviceCgroup, OciLinuxHugepageLimit,
    OciLinuxIntelRdt, OciLinuxMemory, OciLinuxNetwork, OciLinuxNetworkPriority, OciLinuxPids,
    OciLinuxResources, OciLinuxSeccomp, OciLinuxThrottleDevice, OciLinuxWeightDevice, OciMount,
    OciNamespace, OciPlatform, OciProcess, OciRlimit, OciRoot, OciSchedDeadline, OciScheduler,
    OciSeccompAction, OciSeccompArg, OciSeccompSyscallEntry, OciSpec, OciUser, parse_oci_process,
    parse_oci_spec,
};
pub use validation::{
    DEFAULT_ARGS, DEFAULT_ENV, OciValidationError, default_process_args, default_process_env,
    validate_spec,
};
