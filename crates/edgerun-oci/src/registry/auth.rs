//! Registry authorization helpers.

use crate::prelude::*;
use alloc::string::{String, ToString};

#[cfg(all(feature = "std", not(target_os = "none")))]
use std::path::PathBuf;

/// Authorization material for a registry.
#[derive(Clone, Debug)]
pub enum RegistryAuth {
    /// No authorization (public images).
    Anonymous,
    /// Pre-baked bearer token.
    Bearer { token: String },
    /// Resolve credentials from the edgerun secret service.
    #[cfg(all(feature = "std", not(target_os = "none")))]
    FromSecretService {
        data_root: PathBuf,
        namespace: String,
        registry_host: String,
    },
}

/// Read an EdgeRun registry token file.
///
/// The file content is treated as a bearer token. Docker-compatible
/// username/shared-secret auth files are intentionally unsupported.
#[cfg(all(feature = "std", not(target_os = "none")))]
pub fn load_registry_auth(path: &std::path::Path) -> std::io::Result<RegistryAuth> {
    let token = std::fs::read_to_string(path)?.trim().to_string();
    if token.is_empty() {
        Ok(RegistryAuth::Anonymous)
    } else {
        Ok(RegistryAuth::Bearer { token })
    }
}

/// Resolve a bearer token from the secret service for a given registry host.
/// Returns `None` if no credential is found.
#[cfg(all(feature = "std", not(target_os = "none")))]
pub fn resolve_from_secret_service(
    data_root: &std::path::Path,
    namespace: &str,
    registry_host: &str,
) -> Option<String> {
    // Build the collection path
    let coll = format!("/org/freedesktop/secrets/collections/{}", namespace);

    // Use the backend to look up the credential
    let backend = edgerun_secret_service::Backend::new_noop(data_root.to_path_buf()).ok()?;
    let (secret_bytes, _meta) = backend.get(&coll, registry_host).ok()??;

    let token = String::from_utf8(secret_bytes).ok()?;
    let token = token.trim().to_string();
    (!token.is_empty()).then_some(token)
}
