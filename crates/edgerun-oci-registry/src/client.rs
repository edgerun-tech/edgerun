//! Registry client — HTTP operations using std networking.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::auth::{parse_bearer_auth, RegistryAuth};
use crate::config::{
    parse_image_config, parse_json_bytes, parse_manifest,
    parse_single_manifest,
};
use crate::errors::RegistryError;
use crate::layer::{
    apply_whiteouts, build_rootfs, extract_layer, verify_blob_digest,
};
use crate::manifest::{ImageManifest, SingleManifest};
use crate::oci_spec::generate_oci_spec;
use crate::urlencoding;

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

        if registry == "docker.io" && !repository.contains('/') {
            repository = format!("library/{}", repository);
        }

        Ok(ImageRef {
            registry,
            repository,
            tag,
        })
    }
}

impl std::fmt::Display for ImageRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}:{}", self.registry, self.repository, self.tag)
    }
}

/// The main registry client.
pub struct RegistryClient {
    auth: RegistryAuth,
    token: Option<String>,
    bytes_downloaded: u64,
}

impl RegistryClient {
    /// Create a new registry client with anonymous auth.
    pub fn new() -> Self {
        Self {
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

    /// Read registry config.json for credentials.
    /// Supports the standard OCI/Docker-compatible config.json format.
    pub fn with_registry_config(path: &Path) -> io::Result<Self> {
        let auth = crate::auth::load_registry_auth(path)?;
        Ok(Self {
            auth,
            token: None,
            bytes_downloaded: 0,
        })
    }

    /// Use the edgerun secret service for registry credentials.
    ///
    /// Credentials are looked up in the secret service backend under
    /// `{namespace}/{registry_host}`. The stored secret should be in
    /// `username:password` format.
    ///
    /// The `registry_host` parameter (e.g. `docker.io`, `ghcr.io`) is used
    /// to select the correct credential. If set to `None`, the host will
    /// be resolved from the image reference when pulling.
    pub fn with_secret_service_auth(
        data_root: &Path,
        namespace: &str,
        registry_host: &str,
    ) -> Self {
        Self {
            auth: RegistryAuth::FromSecretService {
                data_root: data_root.to_path_buf(),
                namespace: namespace.into(),
                registry_host: registry_host.into(),
            },
            token: None,
            bytes_downloaded: 0,
        }
    }

    /// Perform an HTTPS GET request with auth handling.
    fn authenticated_get(
        &mut self,
        registry: &str,
        path: &str,
        extra_headers: &[(&str, &str)],
    ) -> Result<Vec<u8>, RegistryError> {
        let result = self.do_get(registry, path, extra_headers)?;

        match result {
            GetResult::Success(body) => Ok(body),
            GetResult::Unauthorized(resp_headers) => {
                let www_auth = resp_headers
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case("WWW-Authenticate"))
                    .map(|(_, v)| v.as_str())
                    .ok_or_else(|| {
                        RegistryError::AuthError("No WWW-Authenticate header".into())
                    })?;

                self.handle_auth_challenge(registry, www_auth)?;
                // Retry
                let result2 = self.do_get(registry, path, extra_headers)?;
                match result2 {
                    GetResult::Success(body) => Ok(body),
                    GetResult::Unauthorized(_) => Err(RegistryError::AuthError(
                        "Still unauthorized after token refresh".into(),
                    )),
                }
            }
        }
    }

    /// Perform a raw GET request.
    fn do_get(
        &self,
        host: &str,
        path: &str,
        extra_headers: &[(&str, &str)],
    ) -> Result<GetResult, RegistryError> {
        let mut request = format!(
            "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n",
            path, host
        );

        for &(key, val) in extra_headers {
            request.push_str(&format!("{}: {}\r\n", key, val));
        }

        if let Some(ref token) = self.token {
            request.push_str(&format!("Authorization: Bearer {}\r\n", token));
        }

        request.push_str("\r\n");

        let addr = format!("{}:443", host);
        let addrs: Vec<_> = addr
            .to_socket_addrs()
            .map_err(|e| RegistryError::HttpError(e.to_string()))?
            .collect();
        if addrs.is_empty() {
            return Err(RegistryError::HttpError("No addresses resolved".into()));
        }

        let mut stream =
            TcpStream::connect_timeout(&addrs[0], Duration::from_secs(30))
                .map_err(|e| RegistryError::HttpError(e.to_string()))?;
        stream
            .set_read_timeout(Some(Duration::from_secs(300)))
            .map_err(|e| RegistryError::HttpError(e.to_string()))?;

        stream
            .write_all(request.as_bytes())
            .map_err(|e| RegistryError::HttpError(e.to_string()))?;
        stream
            .flush()
            .map_err(|e| RegistryError::HttpError(e.to_string()))?;

        let mut reader = BufReader::new(stream);
        let mut status_line = String::new();
        reader
            .read_line(&mut status_line)
            .map_err(|e| RegistryError::HttpError(e.to_string()))?;

        let parts: Vec<&str> = status_line.split_whitespace().collect();
        let status_code = if parts.len() >= 2 {
            parts[1].parse::<u16>().unwrap_or(0)
        } else {
            0
        };

        let mut headers: Vec<(String, String)> = Vec::new();
        let mut content_length: Option<usize> = None;
        loop {
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .map_err(|e| RegistryError::HttpError(e.to_string()))?;
            let line = line.trim();
            if line.is_empty() {
                break;
            }
            if let Some((key, val)) = line.split_once(": ") {
                if key.eq_ignore_ascii_case("Content-Length") {
                    content_length = val.parse::<usize>().ok();
                }
                headers.push((key.to_string(), val.to_string()));
            }
        }

        if status_code == 401 {
            return Ok(GetResult::Unauthorized(headers));
        }

        let mut body = Vec::new();
        if let Some(len) = content_length {
            let mut limited = reader.take(len as u64);
            limited
                .read_to_end(&mut body)
                .map_err(|e| RegistryError::HttpError(e.to_string()))?;
        } else {
            reader
                .read_to_end(&mut body)
                .map_err(|e| RegistryError::HttpError(e.to_string()))?;
        }

        if status_code >= 400 {
            return Err(RegistryError::HttpStatus(status_code));
        }

        Ok(GetResult::Success(body))
    }

    /// Ping the registry to verify connectivity.
    pub fn ping(&mut self, registry: &str) -> Result<(), RegistryError> {
        let path = "/v2/".to_string();
        let result = self.do_get(registry, &path, &[])?;
        match result {
            GetResult::Success(_) => Ok(()),
            GetResult::Unauthorized(resp_headers) => {
                let www_auth = resp_headers
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case("WWW-Authenticate"))
                    .map(|(_, v)| v.as_str())
                    .ok_or_else(|| {
                        RegistryError::AuthError("No WWW-Authenticate header".into())
                    })?;
                self.handle_auth_challenge(registry, www_auth)?;
                Ok(())
            }
        }
    }

    /// Resolve an image reference to its manifest.
    pub fn resolve_manifest(
        &mut self,
        image: &ImageRef,
    ) -> Result<ImageManifest, RegistryError> {
        self.ensure_auth(&image.registry)?;

        let url = format!("/v2/{}/manifests/{}", image.repository, image.tag);
        let headers = [
            (
                "Accept",
                "application/vnd.docker.distribution.manifest.v2+json",
            ),
            ("Accept", "application/vnd.oci.image.manifest.v1+json"),
            (
                "Accept",
                "application/vnd.docker.distribution.manifest.list.v2+json",
            ),
            ("Accept", "application/vnd.oci.image.index.v1+json"),
        ];

        let body = self.authenticated_get(&image.registry, &url, &headers)?;

        parse_manifest(&body).map_err(|e| RegistryError::ParseError(e))
    }

    /// Fetch a manifest by digest.
    pub fn fetch_manifest_by_digest(
        &mut self,
        registry: &str,
        repository: &str,
        digest: &str,
    ) -> Result<SingleManifest, RegistryError> {
        let body = self.fetch_blob(registry, repository, digest)?;
        parse_single_manifest(&body)
            .map_err(|e| RegistryError::ParseError(e))
    }

    /// Fetch a single blob by digest.
    pub fn fetch_blob(
        &mut self,
        registry: &str,
        repository: &str,
        digest: &str,
    ) -> Result<Vec<u8>, RegistryError> {
        let url = format!("/v2/{}/blobs/{}", repository, digest);
        self.authenticated_get(registry, &url, &[])
    }

    /// Pull an image to a local bundle directory.
    pub fn pull(
        &mut self,
        image: &ImageRef,
        bundle_path: &Path,
        store_path: &Path,
    ) -> Result<PathBuf, RegistryError> {
        let manifest = self.resolve_manifest(image)?;

        let manifest_data = match manifest {
            ImageManifest::Single(m) => m,
            ImageManifest::Index(idx) => {
                if idx.manifests.is_empty() {
                    return Err(RegistryError::NoManifests);
                }
                let m = &idx.manifests[0];
                self.fetch_manifest_by_digest(
                    &image.registry,
                    &image.repository,
                    &m.digest,
                )?
            }
        };

        let config_blob = self.fetch_blob(
            &image.registry,
            &image.repository,
            &manifest_data.config_digest,
        )?;
        let image_config = parse_image_config(&config_blob)
            .map_err(|e| RegistryError::ParseError(e.to_string()))?;

        std::fs::create_dir_all(bundle_path)?;
        std::fs::create_dir_all(store_path)?;

        let rootfs = bundle_path.join("rootfs");
        std::fs::create_dir_all(&rootfs)?;

        let overlay_upper = store_path.join("upper");
        let overlay_work = store_path.join("work");
        std::fs::create_dir_all(&overlay_upper)?;
        std::fs::create_dir_all(&overlay_work)?;

        let mut layer_dirs = Vec::new();
        for (i, layer) in manifest_data.layers.iter().enumerate() {
            let layer_dir = store_path.join(format!("layer_{}", i));
            std::fs::create_dir_all(&layer_dir)?;

            let blob_path =
                store_path.join(format!("{}.tar.gz", layer.digest.replace(':', "_")));
            if !blob_path.exists() {
                self.download_blob(
                    &image.registry,
                    &image.repository,
                    &layer.digest,
                    &blob_path,
                )?;
            }

            verify_blob_digest(&blob_path, &layer.digest)?;
            extract_layer(&blob_path, &layer_dir, layer.media_type.as_deref())?;
            layer_dirs.push(layer_dir);
        }

        apply_whiteouts(&layer_dirs)?;
        build_rootfs(&layer_dirs, &rootfs)?;

        let config_json = generate_oci_spec(&image_config, &rootfs);
        std::fs::write(bundle_path.join("config.json"), config_json)?;

        Ok(bundle_path.to_path_buf())
    }

    /// Download a blob to a local file, tracking bytes.
    fn download_blob(
        &mut self,
        registry: &str,
        repository: &str,
        digest: &str,
        dest: &Path,
    ) -> Result<(), RegistryError> {
        let url = format!("/v2/{}/blobs/{}", repository, digest);
        let body = self.authenticated_get(registry, &url, &[])?;

        let n = body.len() as u64;
        self.bytes_downloaded += n;

        std::fs::write(dest, &body).map_err(|e| RegistryError::IoError(e))
    }

    /// Handle OCI Registry V2 authentication (Bearer token exchange).
    fn handle_auth_challenge(
        &mut self,
        _registry: &str,
        www_auth: &str,
    ) -> Result<(), RegistryError> {
        let (realm, service, scope) = parse_bearer_auth(www_auth)
            .ok_or_else(|| {
                RegistryError::AuthError("Invalid Bearer challenge".into())
            })?;

        let mut url =
            format!("{}?service={}", realm, urlencoding::encode(&service));
        if let Some(sc) = scope {
            url.push_str(&format!("&scope={}", urlencoding::encode(&sc)));
        }

        // Parse host from realm URL
        let host = realm
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .split('/')
            .next()
            .unwrap_or(&realm);

        let mut request = format!(
            "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n",
            url, host
        );

        // Resolve credentials based on auth type
        let creds = match &self.auth {
            RegistryAuth::Basic { username, password } => {
                Some((username.clone(), password.clone()))
            }
            RegistryAuth::FromSecretService { data_root, namespace, registry_host } => {
                crate::auth::resolve_from_secret_service(data_root, namespace, registry_host)
            }
            _ => None,
        };

        if let Some((username, password)) = creds {
            let auth_str = format!("{}:{}", username, password);
            let encoded = crate::base64::encode(auth_str.as_bytes());
            request.push_str(&format!("Authorization: Basic {}\r\n", encoded));
        }

        request.push_str("\r\n");

        let addr = format!("{}:443", host);
        let addrs: Vec<_> = addr
            .to_socket_addrs()
            .map_err(|e| RegistryError::HttpError(e.to_string()))?
            .collect();
        if addrs.is_empty() {
            return Err(RegistryError::HttpError("No addresses resolved".into()));
        }

        let mut stream =
            TcpStream::connect_timeout(&addrs[0], Duration::from_secs(30))
                .map_err(|e| RegistryError::HttpError(e.to_string()))?;
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .map_err(|e| RegistryError::HttpError(e.to_string()))?;

        stream
            .write_all(request.as_bytes())
            .map_err(|e| RegistryError::HttpError(e.to_string()))?;
        stream
            .flush()
            .map_err(|e| RegistryError::HttpError(e.to_string()))?;

        let mut reader = BufReader::new(stream);
        let mut status_line = String::new();
        reader
            .read_line(&mut status_line)
            .map_err(|e| RegistryError::HttpError(e.to_string()))?;

        loop {
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .map_err(|e| RegistryError::HttpError(e.to_string()))?;
            if line.trim().is_empty() {
                break;
            }
        }

        let mut body = Vec::new();
        reader
            .read_to_end(&mut body)
            .map_err(|e| RegistryError::HttpError(e.to_string()))?;

        let value = parse_json_bytes(&body)
            .map_err(|e| RegistryError::ParseError(e))?;

        // Find token or access_token
        self.token = if let edgerun_json::JsonValue::Object(fields) = &value {
            fields
                .iter()
                .find(|(k, _)| k == "token" || k == "access_token")
                .and_then(|(_, v)| {
                    if let edgerun_json::JsonValue::String(s) = v {
                        Some(s.clone())
                    } else {
                        None
                    }
                })
        } else {
            None
        };

        if self.token.is_some() {
            Ok(())
        } else {
            Err(RegistryError::AuthError("No token in response".into()))
        }
    }

    /// Ensure we're authenticated for the given registry.
    fn ensure_auth(&mut self, registry: &str) -> Result<(), RegistryError> {
        if self.token.is_none() && !matches!(self.auth, RegistryAuth::Anonymous) {
            let _ = self.ping(registry);
        }
        Ok(())
    }
}

/// Result of an HTTP GET: either body bytes or 401 with response headers.
enum GetResult {
    Success(Vec<u8>),
    Unauthorized(Vec<(String, String)>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tmp_root() -> PathBuf {
        static C: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = C.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!("oci_client_test_{}_{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn new_client_is_anonymous() {
        let client = RegistryClient::new();
        assert!(matches!(client.auth, RegistryAuth::Anonymous));
    }

    #[test]
    fn with_auth_sets_credentials() {
        let client = RegistryClient::new().with_auth(RegistryAuth::Basic {
            username: "user".into(),
            password: "pass".into(),
        });
        if let RegistryAuth::Basic { username, password } = client.auth {
            assert_eq!(username, "user");
            assert_eq!(password, "pass");
        } else { panic!("expected Basic"); }
    }

    #[test]
    fn with_secret_service_auth_sets_variant() {
        let root = tmp_root();
        let client = RegistryClient::with_secret_service_auth(&root, "registry", "docker.io");
        if let RegistryAuth::FromSecretService { data_root, namespace, registry_host } = client.auth {
            assert_eq!(data_root, root);
            assert_eq!(namespace, "registry");
            assert_eq!(registry_host, "docker.io");
        } else { panic!("expected FromSecretService"); }
    }

    #[test]
    fn image_ref_parsing() {
        let img: ImageRef = "docker.io/library/alpine:latest".parse().unwrap();
        assert_eq!(img.registry, "docker.io");
        assert_eq!(img.repository, "library/alpine");
        assert_eq!(img.tag, "latest");
    }

    #[test]
    fn image_ref_default_tag() {
        let img: ImageRef = "myregistry/myrepo".parse().unwrap();
        assert_eq!(img.tag, "latest");
    }

    #[test]
    fn image_ref_docker_hub_library() {
        let img: ImageRef = "alpine:3.18".parse().unwrap();
        assert_eq!(img.registry, "docker.io");
        assert_eq!(img.repository, "library/alpine");
        assert_eq!(img.tag, "3.18");
    }

    #[test]
    fn image_ref_display() {
        let img = ImageRef {
            registry: "ghcr.io".into(),
            repository: "owner/repo".into(),
            tag: "v1".into(),
        };
        assert_eq!(img.to_string(), "ghcr.io/owner/repo:v1");
    }
}
