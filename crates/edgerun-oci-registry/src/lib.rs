//! OCI Image Registry Client
//!
//! Pulls images from Docker Hub and any OCI-compliant registry.
//! Handles authentication, manifest resolution, layer downloading,
//! extraction with whiteout handling, and overlay filesystem setup.

use edgerun_core::crypto::sha256;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

// ===========================================================================
// Registry client
// ===========================================================================

/// Authentication credentials for a registry.
#[derive(Clone, Debug)]
pub enum RegistryAuth {
    /// No authentication (public images).
    Anonymous,
    /// Username and password (basic auth or token auth).
    Basic { username: String, password: String },
    /// Pre-baked bearer token.
    Bearer { token: String },
}

/// An OCI image reference (e.g., `docker.io/library/alpine:latest`).
#[derive(Clone, Debug)]
pub struct ImageRef {
    pub registry: String,
    pub repository: String,
    pub tag: String,
}

impl std::str::FromStr for ImageRef {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut registry = "docker.io".to_string();
        let mut rest = s;

        // Check for registry prefix
        if let Some((prefix, remaining)) = s.split_once('/') {
            if prefix.contains('.') || prefix.contains(':') {
                registry = prefix.to_string();
                rest = remaining;
            }
        }

        let (mut repository, tag) = if let Some((repo, tag)) = rest.rsplit_once(':') {
            (repo.to_string(), tag.to_string())
        } else {
            (rest.to_string(), "latest".to_string())
        };

        // Docker Hub special: library/<name> for official images
        if registry == "docker.io" && !repository.contains('/') {
            repository = format!("library/{}", repository);
        }

        Ok(ImageRef { registry, repository, tag })
    }
}

impl std::fmt::Display for ImageRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}:{}", self.registry, self.repository, self.tag)
    }
}

/// The main registry client.
pub struct RegistryClient {
    agent: ureq::Agent,
    auth: RegistryAuth,
    token: Option<String>,
    /// Total bytes downloaded so far (for metering).
    bytes_downloaded: u64,
}

impl RegistryClient {
    /// Create a new registry client with anonymous auth.
    pub fn new() -> Self {
        Self {
            agent: ureq::AgentBuilder::new()
                .timeout(std::time::Duration::from_secs(300))
                .build(),
            auth: RegistryAuth::Anonymous,
            token: None,
            bytes_downloaded: 0,
        }
    }

    /// Total bytes downloaded by this client since creation.
    pub fn bytes_downloaded(&self) -> u64 {
        self.bytes_downloaded
    }

    /// Reset the byte counter.
    pub fn reset_byte_counter(&mut self) {
        self.bytes_downloaded = 0;
    }

    /// Set authentication credentials.
    pub fn with_auth(mut self, auth: RegistryAuth) -> Self {
        self.auth = auth;
        self
    }

    /// Read Docker config.json for credentials.
    pub fn with_docker_config(path: &Path) -> io::Result<Self> {
        let data = fs::read_to_string(path)?;
        let config: DockerConfig = lifegraph_json::from_str(&data)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        if let Some(auths) = config.auths {
            for (_key, auth_entry) in &auths {
                if let Some(auth_str) = &auth_entry.auth {
                    if let Some((username, password)) = decode_basic_auth(auth_str) {
                        return Ok(Self {
                            agent: ureq::AgentBuilder::new()
                                .timeout(std::time::Duration::from_secs(300))
                                .build(),
                            auth: RegistryAuth::Basic { username, password },
                            token: None,
                            bytes_downloaded: 0,
                        });
                    }
                }
            }
        }

        Ok(Self::new())
    }

    /// Ping the registry to verify connectivity.
    pub fn ping(&mut self, registry: &str) -> Result<(), RegistryError> {
        let url = format!("https://{}/v2/", registry);
        let resp = self.agent.get(&url).call();

        match resp {
            Ok(_) => Ok(()),
            Err(ureq::Error::Status(401, resp)) => {
                // Expected for Docker Hub — authenticate
                self.handle_auth_challenge(registry, &resp)?;
                Ok(())
            }
            Err(ureq::Error::Status(code, _)) => {
                Err(RegistryError::HttpStatus(code))
            }
            Err(e) => Err(RegistryError::HttpError(e.to_string())),
        }
    }

    /// Resolve an image reference to its manifest.
    pub fn resolve_manifest(
        &mut self,
        image: &ImageRef,
    ) -> Result<ImageManifest, RegistryError> {
        self.ensure_auth(&image.registry)?;

        let url = format!(
            "https://{}/v2/{}/manifests/{}",
            image.registry, image.repository, image.tag
        );

        let mut req = self.agent.get(&url)
            .set("Accept", "application/vnd.docker.distribution.manifest.v2+json")
            .set("Accept", "application/vnd.oci.image.manifest.v1+json")
            .set("Accept", "application/vnd.docker.distribution.manifest.list.v2+json")
            .set("Accept", "application/vnd.oci.image.index.v1+json");

        if let Some(ref token) = self.token {
            req = req.set("Authorization", &format!("Bearer {}", token));
        }

        match req.call() {
            Ok(resp) => {
                let body: lifegraph_json::Value = lifegraph_json::from_str(&resp.into_string().map_err(|e| RegistryError::HttpError(e.to_string()))?).map_err(|e| RegistryError::ParseError(e.to_string()))?;
                self.parse_manifest(&body)
            }
            Err(ureq::Error::Status(code, resp)) => {
                if code == 401 {
                    self.handle_auth_challenge(&image.registry, &resp)?;
                    // Retry
                    return self.resolve_manifest(image);
                }
                Err(RegistryError::ManifestNotFound(image.tag.clone()))
            }
            Err(e) => Err(RegistryError::HttpError(e.to_string())),
        }
    }

    /// Pull an image to a local bundle directory.
    ///
    /// Returns the path to the OCI bundle (config.json + rootfs/).
    pub fn pull(
        &mut self,
        image: &ImageRef,
        bundle_path: &Path,
        store_path: &Path,
    ) -> Result<PathBuf, RegistryError> {
        // Resolve manifest
        let manifest = self.resolve_manifest(image)?;

        // Get the platform-specific manifest
        let manifest_data = match manifest {
            ImageManifest::Single(m) => m,
            ImageManifest::Index(idx) => {
                // Pick the first manifest for now (would do arch matching in prod)
                if idx.manifests.is_empty() {
                    return Err(RegistryError::NoManifests);
                }
                let m = &idx.manifests[0];
                // Fetch the actual manifest
                self.fetch_manifest_by_digest(&image.registry, &image.repository, &m.digest)?
            }
        };

        // Fetch image config
        let config_blob = self.fetch_blob(&image.registry, &image.repository, &manifest_data.config_digest)?;
        let image_config: ImageConfig = lifegraph_json::from_slice(&config_blob)
            .map_err(|e| RegistryError::ParseError(e.to_string()))?;

        // Create bundle directory
        fs::create_dir_all(bundle_path)?;
        fs::create_dir_all(store_path)?;

        // Create rootfs
        let rootfs = bundle_path.join("rootfs");
        fs::create_dir_all(&rootfs)?;

        // Create overlay dirs
        let overlay_upper = store_path.join("upper");
        let overlay_work = store_path.join("work");
        fs::create_dir_all(&overlay_upper)?;
        fs::create_dir_all(&overlay_work)?;

        // Download and extract layers
        let mut layer_dirs = Vec::new();
        for (i, layer) in manifest_data.layers.iter().enumerate() {
            let layer_dir = store_path.join(format!("layer_{}", i));
            fs::create_dir_all(&layer_dir)?;

            // Download blob
            let blob_path = store_path.join(format!("{}.tar.gz", layer.digest.replace(':', "_")));
            if !blob_path.exists() {
                self.download_blob(&image.registry, &image.repository, &layer.digest, &blob_path)?;
            }

            // Verify digest
            verify_blob_digest(&blob_path, &layer.digest)?;

            // Extract layer
            extract_layer(&blob_path, &layer_dir, layer.media_type.as_deref())?;
            layer_dirs.push(layer_dir);
        }

        // Apply whiteouts in reverse order (top layer first)
        apply_whiteouts(&layer_dirs)?;

        // Build rootfs from layers
        build_rootfs(&layer_dirs, &rootfs)?;

        // Generate OCI config.json
        let oci_spec = generate_oci_spec(&image_config, &rootfs);
        let config_json = lifegraph_json::to_string_pretty(&oci_spec)
            .map_err(|e| RegistryError::ParseError(e.to_string()))?;
        fs::write(bundle_path.join("config.json"), config_json)?;

        Ok(bundle_path.to_path_buf())
    }

    /// Fetch a single blob by digest.
    pub fn fetch_blob(
        &mut self,
        registry: &str,
        repository: &str,
        digest: &str,
    ) -> Result<Vec<u8>, RegistryError> {
        let url = format!(
            "https://{}/v2/{}/blobs/{}",
            registry, repository, digest
        );

        let mut req = self.agent.get(&url);
        if let Some(ref token) = self.token {
            req = req.set("Authorization", &format!("Bearer {}", token));
        }

        match req.call() {
            Ok(resp) => {
                let mut bytes = Vec::new();
                resp.into_reader().read_to_end(&mut bytes)
                    .map_err(|e| RegistryError::IoError(e))?;
                Ok(bytes)
            }
            Err(ureq::Error::Status(401, resp)) => {
                self.handle_auth_challenge(registry, &resp)?;
                self.fetch_blob(registry, repository, digest)
            }
            Err(e) => Err(RegistryError::HttpError(e.to_string())),
        }
    }

    /// Fetch a manifest by digest.
    fn fetch_manifest_by_digest(
        &mut self,
        registry: &str,
        repository: &str,
        digest: &str,
    ) -> Result<SingleManifest, RegistryError> {
        let blob = self.fetch_blob(registry, repository, digest)?;
        let manifest: lifegraph_json::Value = lifegraph_json::from_slice(&blob)
            .map_err(|e| RegistryError::ParseError(e.to_string()))?;

        // Parse layers from the manifest
        let layers = manifest.get("layers")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter().filter_map(|l| {
                    let digest = l.get("digest")?.as_str()?.to_string();
                    let media_type = l.get("mediaType")?.as_str()?.to_string();
                    let size = l.get("size")?.as_u64().unwrap_or(0);
                    Some(LayerDescriptor {
                        digest,
                        media_type: Some(media_type),
                        size,
                    })
                }).collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let config_digest = manifest.get("config")
            .and_then(|c| c.get("digest"))
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();

        Ok(SingleManifest { config_digest, layers })
    }

    /// Download a blob to a local file, tracking bytes for metering.
    fn download_blob(
        &mut self,
        registry: &str,
        repository: &str,
        digest: &str,
        dest: &Path,
    ) -> Result<(), RegistryError> {
        let url = format!(
            "https://{}/v2/{}/blobs/{}",
            registry, repository, digest
        );

        let mut req = self.agent.get(&url);
        if let Some(ref token) = self.token {
            req = req.set("Authorization", &format!("Bearer {}", token));
        }

        match req.call() {
            Ok(resp) => {
                let mut reader = resp.into_reader();
                let mut file = File::create(dest)
                    .map_err(|e| RegistryError::IoError(e))?;
                let n = io::copy(&mut reader, &mut file)
                    .map_err(|e| RegistryError::IoError(e))?;
                // Track bytes for metering
                self.bytes_downloaded += n;
                Ok(())
            }
            Err(ureq::Error::Status(401, resp)) => {
                self.handle_auth_challenge(registry, &resp)?;
                self.download_blob(registry, repository, digest, dest)
            }
            Err(e) => Err(RegistryError::HttpError(e.to_string())),
        }
    }

    /// Handle Docker Registry V2 authentication.
    fn handle_auth_challenge(
        &mut self,
        registry: &str,
        resp: &ureq::Response,
    ) -> Result<(), RegistryError> {
        let www_auth = resp.header("WWW-Authenticate")
            .ok_or(RegistryError::AuthError("No WWW-Authenticate header".into()))?;

        // Parse Bearer realm, service, scope
        let (realm, service, scope) = parse_bearer_auth(www_auth)
            .ok_or_else(|| RegistryError::AuthError("Invalid Bearer challenge".into()))?;

        // Build token request URL
        let mut url = format!("{}?service={}", realm, urlencoding::encode(&service));
        if let Some(scope) = scope {
            url.push_str(&format!("&scope={}", urlencoding::encode(&scope)));
        }

        let mut req = self.agent.get(&url);

        // Add basic auth if available
        if let RegistryAuth::Basic { ref username, ref password } = self.auth {
            let auth_str = format!("{}:{}", username, password);
            let encoded = base64::encode(auth_str.as_bytes());
            req = req.set("Authorization", &format!("Basic {}", encoded));
        }

        match req.call() {
            Ok(resp) => {
                let body: lifegraph_json::Value = lifegraph_json::from_str(
                    &resp.into_string().map_err(|e| RegistryError::HttpError(e.to_string()))?
                ).map_err(|e| RegistryError::ParseError(e.to_string()))?;
                self.token = body.get("token")
                    .or_else(|| body.get("access_token"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                if self.token.is_some() {
                    Ok(())
                } else {
                    Err(RegistryError::AuthError("No token in response".into()))
                }
            }
            Err(e) => Err(RegistryError::HttpError(e.to_string())),
        }
    }

    /// Ensure we're authenticated for the given registry.
    fn ensure_auth(&mut self, registry: &str) -> Result<(), RegistryError> {
        if self.token.is_none() && !matches!(self.auth, RegistryAuth::Anonymous) {
            // Try to ping which may trigger auth
            let _ = self.ping(registry);
        }
        Ok(())
    }

    /// Parse a manifest response into an ImageManifest.
    fn parse_manifest(&self, body: &lifegraph_json::Value) -> Result<ImageManifest, RegistryError> {
        // Check if this is an index/manifest list
        if body.get("manifests").is_some() {
            let manifests: Vec<ManifestDescriptor> = lifegraph_json::from_value(body["manifests"].clone())
                .map_err(|e| RegistryError::ParseError(e.to_string()))?;
            let media_type = body.get("mediaType").and_then(|v| v.as_str()).map(|s| s.to_string());
            return Ok(ImageManifest::Index(ImageIndex {
                media_type,
                manifests,
            }));
        }

        // Single manifest
        let layers: Vec<LayerDescriptor> = body.get("layers")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter().filter_map(|l| {
                    let digest = l.get("digest")?.as_str()?.to_string();
                    let media_type = l.get("mediaType")?.as_str()?.to_string();
                    let size = l.get("size")?.as_u64().unwrap_or(0);
                    Some(LayerDescriptor { digest, media_type: Some(media_type), size })
                }).collect()
            })
            .unwrap_or_default();

        let config_digest = body.get("config")
            .and_then(|c| c.get("digest"))
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();

        Ok(ImageManifest::Single(SingleManifest {
            config_digest,
            layers,
        }))
    }
}

// ===========================================================================
// Manifest types
// ===========================================================================

#[derive(Debug)]
pub enum ImageManifest {
    Single(SingleManifest),
    Index(ImageIndex),
}

#[derive(Debug)]
pub struct SingleManifest {
    pub config_digest: String,
    pub layers: Vec<LayerDescriptor>,
}

#[derive(Debug)]
pub struct ImageIndex {
    pub media_type: Option<String>,
    pub manifests: Vec<ManifestDescriptor>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestDescriptor {
    pub media_type: Option<String>,
    pub digest: String,
    pub size: u64,
    pub platform: Option<PlatformDescriptor>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformDescriptor {
    pub architecture: Option<String>,
    pub os: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerDescriptor {
    pub media_type: Option<String>,
    pub digest: String,
    pub size: u64,
}

// ===========================================================================
// Image config (from the config blob)
// ===========================================================================

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageConfig {
    pub architecture: Option<String>,
    pub os: Option<String>,
    pub config: Option<ImageConfigInner>,
    pub rootfs: Option<RootFs>,
    pub history: Option<Vec<HistoryEntry>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageConfigInner {
    pub user: Option<String>,
    pub env: Option<Vec<String>>,
    pub entrypoint: Option<Vec<String>>,
    pub cmd: Option<Vec<String>>,
    pub working_dir: Option<String>,
    pub exposed_ports: Option<HashMap<String, lifegraph_json::Value>>,
    pub volumes: Option<HashMap<String, lifegraph_json::Value>>,
    pub labels: Option<HashMap<String, String>>,
    pub stop_signal: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RootFs {
    pub r#type: String,
    pub diff_ids: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub created: Option<String>,
    pub created_by: Option<String>,
    pub comment: Option<String>,
    pub empty_layer: Option<bool>,
}

// ===========================================================================
// OCI spec generation from image config
// ===========================================================================

pub fn generate_oci_spec(
    image_config: &ImageConfig,
    rootfs: &Path,
) -> edgerun_oci_runtime::OciSpec {
    use edgerun_oci_runtime::{OciProcess, OciRoot, OciUser, OciLinux, OciNamespace, OciMount};

    let process_config = image_config.config.as_ref();

    let args = process_config
        .and_then(|c| c.entrypoint.as_ref())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .chain(
            process_config.and_then(|c| c.cmd.as_ref()).cloned().unwrap_or_default()
        )
        .collect::<Vec<_>>();

    let env = process_config
        .and_then(|c| c.env.as_ref())
        .cloned()
        .unwrap_or_else(|| vec![
            "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".into(),
        ]);

    let cwd = process_config
        .and_then(|c| c.working_dir.as_ref())
        .cloned()
        .unwrap_or_else(|| "/".into());

    // Parse user
    let (uid, gid) = parse_user(process_config.and_then(|c| c.user.as_ref()));

    // Generate volume mounts
    let volume_mounts: Vec<edgerun_oci_runtime::OciMount> = image_config.config.as_ref()
        .and_then(|c| c.volumes.as_ref())
        .map(|volumes| {
            volumes.keys().map(|dest| {
                edgerun_oci_runtime::OciMount {
                    destination: dest.clone(),
                    mount_type: Some("tmpfs".into()),
                    source: Some("tmpfs".into()),
                    options: Some(vec!["nosuid".into(), "nodev".into(), "noexec".into()]),
                }
            }).collect()
        })
        .unwrap_or_default();

    edgerun_oci_runtime::OciSpec {
        version: "1.0.2".into(),
        process: Some(OciProcess {
            args: if args.is_empty() {
                Some(vec!["/bin/sh".into()])
            } else {
                Some(args)
            },
            env: Some(env),
            cwd: Some(cwd),
            no_new_privileges: Some(true),
            user: Some(OciUser {
                uid: Some(uid),
                gid: Some(gid),
                additional_gids: None,
            }),
            ..Default::default()
        }),
        root: Some(OciRoot {
            path: rootfs.to_str().unwrap_or("/").into(),
            readonly: None,
        }),
        hostname: Some("edgerun".into()),
        linux: Some(OciLinux {
            namespaces: Some(edgerun_oci_runtime::default_namespaces()),
            masked_paths: Some(vec![
                "/proc/acpi".into(), "/proc/kcore".into(), "/proc/keys".into(),
                "/proc/latency_stats".into(), "/proc/timer_list".into(),
                "/proc/timer_stats".into(), "/proc/sched_debug".into(),
                "/proc/scsi".into(), "/sys/firmware".into(),
            ]),
            readonly_paths: Some(vec![
                "/proc/asound".into(), "/proc/bus".into(), "/proc/fs".into(),
                "/proc/irq".into(), "/proc/sys".into(), "/proc/sysrq-trigger".into(),
            ]),
            ..Default::default()
        }),
        mounts: if volume_mounts.is_empty() {
            None
        } else {
            Some(volume_mounts)
        },
    }
}

fn parse_user(user_str: Option<&String>) -> (u32, u32) {
    if let Some(s) = user_str {
        let parts: Vec<&str> = s.split(':').collect();
        let uid = parts[0].parse().unwrap_or(0);
        let gid = if parts.len() > 1 {
            parts[1].parse().unwrap_or(0)
        } else {
            0
        };
        (uid, gid)
    } else {
        (0, 0)
    }
}

// ===========================================================================
// Layer extraction
// ===========================================================================

/// Extract a compressed layer tarball to a directory.
fn extract_layer(
    blob_path: &Path,
    dest: &Path,
    media_type: Option<&str>,
) -> Result<(), RegistryError> {
    let file = File::open(blob_path)
        .map_err(|e| RegistryError::IoError(e))?;

    // Determine compression from media type or file extension
    let is_gzip = media_type.map(|mt| mt.contains("gzip")).unwrap_or(false)
        || blob_path.extension().map(|e| e == "gz").unwrap_or(false);
    let is_zstd = media_type.map(|mt| mt.contains("zstd")).unwrap_or(false);

    if is_zstd {
        let mut decoder = zstd::Decoder::new(file)
            .map_err(|e| RegistryError::IoError(e))?;
        extract_tar(&mut decoder, dest)?;
    } else if is_gzip {
        let mut decoder = flate2::read::GzDecoder::new(file);
        extract_tar(&mut decoder, dest)?;
    } else {
        extract_tar(&mut io::BufReader::new(file), dest)?;
    }

    Ok(())
}

/// Extract a tar stream to a directory.
fn extract_tar<R: Read>(reader: R, dest: &Path) -> Result<(), RegistryError> {
    let mut archive = tar::Archive::new(reader);

    // Extract all entries
    archive.unpack(dest)
        .map_err(|e| RegistryError::IoError(e))?;

    Ok(())
}

/// Apply whiteout files across layers (in reverse order, top layer first).
fn apply_whiteouts(layer_dirs: &[PathBuf]) -> Result<(), RegistryError> {
    // Process layers in reverse order (top layer first)
    for layer_dir in layer_dirs.iter().rev() {
        remove_whiteout_files(layer_dir)?;
    }
    Ok(())
}

/// Remove whiteout files from a directory tree.
fn remove_whiteout_files(dir: &Path) -> Result<(), RegistryError> {
    use std::os::unix::fs::FileTypeExt;

    if !dir.is_dir() {
        return Ok(());
    }

    let entries: Vec<_> = match fs::read_dir(dir) {
        Ok(entries) => entries.filter_map(|e| e.ok()).collect(),
        Err(_) => return Ok(()),
    };

    for entry in entries {
        let path = entry.path();
        let file_name = entry.file_name();

        // AUFS-style whiteout: .wh.<filename>
        if let Some(name) = file_name.to_str() {
            if name == ".wh..wh..opq" {
                // Opaque whiteout — the entire directory is replaced
                // We handle this by marking the directory (would need more complex logic in build_rootfs)
                continue;
            }
            if let Some(rest) = name.strip_prefix(".wh.") {
                // Remove the whiteouted file
                let target = path.parent().unwrap().join(rest);
                if target.exists() {
                    if target.is_dir() {
                        let _ = fs::remove_dir_all(&target);
                    } else {
                        let _ = fs::remove_file(&target);
                    }
                }
                // Remove the whiteout file itself
                let _ = fs::remove_file(&path);
                continue;
            }
        }

        // Overlay-style whiteout: character device 0:0
        if let Ok(metadata) = path.metadata() {
            if metadata.file_type().is_char_device() {
                // Check if it's a 0:0 device (overlay whiteout)
                // This requires checking dev_t
            }
        }

        // Recurse into subdirectories
        if path.is_dir() {
            remove_whiteout_files(&path)?;
        }
    }

    Ok(())
}

/// Build rootfs by merging layers in order (bottom to top).
fn build_rootfs(layer_dirs: &[PathBuf], dest: &Path) -> Result<(), RegistryError> {
    // Copy layers in order (each layer overwrites previous)
    for layer_dir in layer_dirs {
        copy_dir_contents(layer_dir, dest)?;
    }
    Ok(())
}

/// Copy all contents from src to dest, overwriting existing files.
fn copy_dir_contents(src: &Path, dest: &Path) -> Result<(), RegistryError> {
    use std::os::unix::fs::symlink;

    if !src.is_dir() {
        return Ok(());
    }

    let entries: Vec<_> = match fs::read_dir(src) {
        Ok(entries) => entries.filter_map(|e| e.ok()).collect(),
        Err(_) => return Ok(()),
    };

    for entry in entries {
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        let file_name = entry.file_name();

        // Skip whiteout files
        if let Some(name) = file_name.to_str() {
            if name.starts_with(".wh.") {
                continue;
            }
        }

        if src_path.is_dir() {
            let _ = fs::create_dir_all(&dest_path);
            copy_dir_contents(&src_path, &dest_path)?;
        } else if src_path.is_symlink() {
            let target = fs::read_link(&src_path)
                .map_err(|e| RegistryError::IoError(e))?;
            let _ = fs::remove_file(&dest_path);
            let _ = symlink(&target, &dest_path);
        } else {
            let _ = fs::remove_file(&dest_path);
            fs::copy(&src_path, &dest_path)
                .map_err(|e| RegistryError::IoError(e))?;
        }
    }

    Ok(())
}

// ===========================================================================
// Digest verification
// ===========================================================================

fn verify_blob_digest(blob_path: &Path, expected_digest: &str) -> Result<(), RegistryError> {
    let data = fs::read(blob_path)
        .map_err(|e| RegistryError::IoError(e))?;

    let computed_hash = sha256(&data);
    let computed = format!("sha256:{}", edgerun_core::util::bytes_to_hex(&computed_hash));

    if computed != expected_digest {
        return Err(RegistryError::DigestMismatch {
            expected: expected_digest.to_string(),
            computed,
        });
    }

    Ok(())
}

// ===========================================================================
// Auth helpers
// ===========================================================================

fn decode_basic_auth(auth: &str) -> Option<(String, String)> {
    let decoded = base64::decode(auth)
        .ok()?;
    let s = String::from_utf8(decoded).ok()?;
    let (username, password) = s.split_once(':')?;
    Some((username.to_string(), password.to_string()))
}

fn parse_bearer_auth(header: &str) -> Option<(String, String, Option<String>)> {
    // Parse: Bearer realm="https://auth.docker.io/token",service="registry.docker.io",scope="repository:library/alpine:pull"
    if !header.starts_with("Bearer ") && !header.starts_with("bearer ") {
        return None;
    }

    let params = &header[7..];
    let mut realm = None;
    let mut service = None;
    let mut scope = None;

    for part in params.split(',') {
        let part = part.trim();
        if let Some((key, val)) = part.split_once('=') {
            let val = val.trim_matches('"');
            match key {
                "realm" => realm = Some(val.to_string()),
                "service" => service = Some(val.to_string()),
                "scope" => scope = Some(val.to_string()),
                _ => {}
            }
        }
    }

    Some((
        realm?,
        service.unwrap_or_else(|| "registry".into()),
        scope,
    ))
}

// ===========================================================================
// Docker config.json parsing
// ===========================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DockerConfig {
    auths: Option<HashMap<String, DockerAuthEntry>>,
}

#[derive(Debug, Deserialize)]
struct DockerAuthEntry {
    auth: Option<String>,
}

// ===========================================================================
// Errors
// ===========================================================================

#[derive(Debug)]
pub enum RegistryError {
    HttpStatus(u16),
    HttpError(String),
    AuthError(String),
    ManifestNotFound(String),
    NoManifests,
    DigestMismatch { expected: String, computed: String },
    IoError(io::Error),
    ParseError(String),
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryError::HttpStatus(code) => write!(f, "HTTP {}", code),
            RegistryError::HttpError(e) => write!(f, "HTTP error: {}", e),
            RegistryError::AuthError(e) => write!(f, "Auth error: {}", e),
            RegistryError::ManifestNotFound(tag) => write!(f, "Manifest not found: {}", tag),
            RegistryError::NoManifests => write!(f, "No manifests in index"),
            RegistryError::DigestMismatch { expected, computed } => {
                write!(f, "Digest mismatch:\n  expected: {}\n  computed: {}", expected, computed)
            }
            RegistryError::IoError(e) => write!(f, "I/O error: {}", e),
            RegistryError::ParseError(e) => write!(f, "Parse error: {}", e),
        }
    }
}

impl std::error::Error for RegistryError {}

impl From<io::Error> for RegistryError {
    fn from(e: io::Error) -> Self {
        RegistryError::IoError(e)
    }
}

// ===========================================================================
// URL encoding helper (minimal, no external crate)
// ===========================================================================

mod urlencoding {
    pub fn encode(s: &str) -> String {
        let mut out = String::with_capacity(s.len() * 3);
        for b in s.bytes() {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    out.push(b as char);
                }
                _ => {
                    out.push('%');
                    out.push(HEX[(b >> 4) as usize] as char);
                    out.push(HEX[(b & 0xf) as usize] as char);
                }
            }
        }
        out
    }

    const HEX: &[u8; 16] = b"0123456789ABCDEF";
}

// ===========================================================================
// Minimal base64 implementation (standard alphabet, no padding stripping)
// ===========================================================================

mod base64 {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    pub fn encode(data: &[u8]) -> String {
        let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
        let chunks = data.chunks_exact(3);
        let remainder = chunks.remainder();

        for chunk in chunks {
            let b0 = chunk[0] as u32;
            let b1 = chunk[1] as u32;
            let b2 = chunk[2] as u32;
            let triple = (b0 << 16) | (b1 << 8) | b2;
            out.push(ALPHABET[((triple >> 18) & 0x3F) as usize] as char);
            out.push(ALPHABET[((triple >> 12) & 0x3F) as usize] as char);
            out.push(ALPHABET[((triple >> 6) & 0x3F) as usize] as char);
            out.push(ALPHABET[(triple & 0x3F) as usize] as char);
        }

        match remainder.len() {
            1 => {
                let b0 = remainder[0] as u32;
                out.push(ALPHABET[((b0 >> 2) & 0x3F) as usize] as char);
                out.push(ALPHABET[((b0 << 4) & 0x3F) as usize] as char);
                out.push('=');
                out.push('=');
            }
            2 => {
                let b0 = remainder[0] as u32;
                let b1 = remainder[1] as u32;
                let triple = (b0 << 8) | b1;
                out.push(ALPHABET[((triple >> 10) & 0x3F) as usize] as char);
                out.push(ALPHABET[((triple >> 4) & 0x3F) as usize] as char);
                out.push(ALPHABET[((triple << 2) & 0x3F) as usize] as char);
                out.push('=');
            }
            _ => {}
        }

        out
    }

    fn decode_char(c: u8) -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a' + 26) as u32),
            b'0'..=b'9' => Some((c - b'0' + 52) as u32),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }

    pub fn decode(input: &str) -> Result<Vec<u8>, String> {
        let bytes = input.as_bytes();
        let padding = bytes.iter().rev().take_while(|&&b| b == b'=').count();
        let data_len = (bytes.len() / 4) * 3 + match padding {
            0 => 0,
            1 => 2,
            2 => 1,
            _ => return Err("invalid padding".into()),
        };

        let mut out = Vec::with_capacity(data_len);
        let chunks = bytes.chunks_exact(4);

        for chunk in chunks {
            let vals: Result<Vec<u32>, String> = chunk.iter()
                .filter(|&&b| b != b'=')
                .map(|&b| decode_char(b).ok_or_else(|| "invalid base64 char".to_string()))
                .collect();
            let vals = vals?;

            match vals.len() {
                4 => {
                    let triple = (vals[0] << 18) | (vals[1] << 12) | (vals[2] << 6) | vals[3];
                    out.push((triple >> 16) as u8);
                    out.push(((triple >> 8) & 0xFF) as u8);
                    out.push((triple & 0xFF) as u8);
                }
                3 => {
                    let triple = (vals[0] << 10) | (vals[1] << 4) | (vals[2] >> 2);
                    out.push((triple >> 8) as u8);
                    out.push((triple & 0xFF) as u8);
                }
                2 => {
                    let val = (vals[0] << 2) | (vals[1] >> 4);
                    out.push(val as u8);
                }
                _ => return Err("invalid base64 length".into()),
            }
        }

        Ok(out)
    }
}
