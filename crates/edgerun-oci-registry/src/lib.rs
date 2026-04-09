//! OCI Image Registry Client
//!
//! Pulls images from any OCI-compliant registry, including
//! Docker Hub, GitHub Container Registry, and self-hosted registries.
//! Handles authentication, manifest resolution, layer downloading,
//! extraction with whiteout handling, and overlay filesystem setup.

mod auth;
mod base64;
mod client;
mod config;
mod errors;
mod layer;
mod manifest;
mod oci_spec;
mod urlencoding;

pub use auth::{decode_basic_auth, parse_bearer_auth, RegistryAuth};
pub use client::{ImageRef, RegistryClient};
pub use config::{HistoryEntry, ImageConfig, ImageConfigInner, RootFs};
pub use errors::RegistryError;
pub use layer::{build_rootfs, extract_layer, verify_blob_digest};
pub use manifest::{
    ImageManifest, LayerDescriptor, ManifestDescriptor, PlatformDescriptor,
    SingleManifest,
};
pub use oci_spec::generate_oci_spec;
