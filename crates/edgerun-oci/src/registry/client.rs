//! OCI Registry V2 client — async, using `edgerun_http::HttpClient`.

use std::io;
use std::path::{Path, PathBuf};

use edgerun_http::{HttpClient, Request, Response, HeaderMap};

use super::auth::{parse_bearer_auth, RegistryAuth};
use super::config::{
    parse_image_config, parse_json_bytes, parse_manifest,
    parse_single_manifest,
};
use super::errors::RegistryError;
use super::layer::{
    apply_whiteouts, build_rootfs, extract_layer, verify_blob_digest,
};
use super::manifest::{ImageManifest, SingleManifest};
use super::oci_spec::generate_oci_spec;
use super::urlencoding;

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

        // Detect registry: if the first path component contains '.', ':', or is 'localhost'
        if let Some((prefix, remaining)) = s.split_once('/') {
            if prefix.contains('.') || prefix.contains(':') || prefix == "localhost" {
                registry = prefix.to_string();
                rest = remaining;
            }
        }

        // Split tag/digest from repository — handle both tag (:) and digest (@) references
        // Digest references take precedence: repo@sha256:abc...
        let (mut repository, tag) = if let Some((repo, _digest)) = rest.rsplit_once('@') {
            // Digest reference — store digest in tag field for downstream use
            // Repository must not contain ':' which would be confused with tag
            (repo.to_string(), String::new())
        } else if let Some((repo, tag)) = rest.rsplit_once(':') {
            // Check if the ':' is part of a registry port (e.g., "myregistry:5000")
            // If repo contains ':', it's likely a port number, not a tag
            if repo.contains(':') {
                (rest.to_string(), "latest".to_string())
            } else {
                (repo.to_string(), tag.to_string())
            }
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
    pub fn with_registry_config(path: &Path) -> io::Result<Self> {
        let auth = super::auth::load_registry_auth(path)?;
        Ok(Self {
            auth,
            token: None,
            bytes_downloaded: 0,
        })
    }

    /// Use the edgerun secret service for registry credentials.
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
    async fn authenticated_get(
        &mut self,
        registry: &str,
        path: &str,
        extra_headers: &[(&str, &str)],
    ) -> Result<Vec<u8>, RegistryError> {
        let url = format!("https://{}{}", registry, path);
        let mut builder = Request::builder()
            .method(edgerun_http::Method::GET)
            .uri(&url);
        for &(k, v) in extra_headers {
            builder = builder.header(k, v);
        }
        if let Some(ref token) = self.token {
            builder = builder.header("Authorization", &format!("Bearer {}", token));
        }
        let request = builder.build()?;

        let client = HttpClient::new().no_redirects();
        let resp = client.execute(&request).await.map_err(|e| RegistryError::HttpError(e.to_string()))?;

        if resp.status().as_u16() == 401 {
            let www_auth = resp.headers().get("www-authenticate")
                .or_else(|| resp.headers().get("WWW-Authenticate"))
                .map(|v| v.as_str())
                .ok_or_else(|| RegistryError::AuthError("No WWW-Authenticate header".into()))?;
            self.handle_auth_challenge(registry, www_auth).await?;
            // Retry
            let mut builder2 = Request::builder()
                .method(edgerun_http::Method::GET)
                .uri(&url);
            for &(k, v) in extra_headers {
                builder2 = builder2.header(k, v);
            }
            if let Some(ref token) = self.token {
                builder2 = builder2.header("Authorization", &format!("Bearer {}", token));
            }
            let request2 = builder2.build()?;
            let resp2 = client.execute(&request2).await.map_err(|e| RegistryError::HttpError(e.to_string()))?;
            if resp2.status().as_u16() >= 400 {
                return Err(RegistryError::HttpStatus(resp2.status().as_u16()));
            }
            Ok(resp2.body().to_vec())
        } else if resp.status().as_u16() >= 400 {
            Err(RegistryError::HttpStatus(resp.status().as_u16()))
        } else {
            Ok(resp.body().to_vec())
        }
    }

    /// Perform an authenticated PUT request.
    async fn authenticated_put(
        &mut self,
        registry: &str,
        path: &str,
        body: &[u8],
        extra_headers: &[(&str, &str)],
    ) -> Result<Vec<u8>, RegistryError> {
        let url = format!("https://{}{}", registry, path);
        let result = self.do_put(&url, body, extra_headers).await?;
        if result.status().as_u16() == 401 {
            let www_auth = result.headers().get("www-authenticate")
                .or_else(|| result.headers().get("WWW-Authenticate"))
                .map(|v| v.as_str())
                .ok_or_else(|| RegistryError::AuthError("No WWW-Authenticate header".into()))?;
            self.handle_auth_challenge(registry, www_auth).await?;
            let result2 = self.do_put(&url, body, extra_headers).await?;
            if result2.status().as_u16() >= 400 {
                return Err(RegistryError::HttpStatus(result2.status().as_u16()));
            }
            Ok(result2.body().to_vec())
        } else if result.status().as_u16() >= 400 {
            Err(RegistryError::HttpStatus(result.status().as_u16()))
        } else {
            Ok(result.body().to_vec())
        }
    }

    async fn do_put(&self, url: &str, body: &[u8], extra_headers: &[(&str, &str)]) -> Result<Response, RegistryError> {
        let mut builder = Request::builder()
            .method(edgerun_http::Method::PUT)
            .uri(url)
            .body(body.to_vec());
        for &(k, v) in extra_headers {
            builder = builder.header(k, v);
        }
        if let Some(ref token) = self.token {
            builder = builder.header("Authorization", &format!("Bearer {}", token));
        }
        let request = builder.build()?;
        let client = HttpClient::new().no_redirects();
        client.execute(&request).await.map_err(|e| RegistryError::HttpError(e.to_string()))
    }

    /// Perform an authenticated POST request.
    async fn authenticated_post(
        &mut self,
        registry: &str,
        path: &str,
        body: &[u8],
        extra_headers: &[(&str, &str)],
    ) -> Result<Vec<u8>, RegistryError> {
        let url = format!("https://{}{}", registry, path);
        let result = self.do_post(&url, body, extra_headers).await?;
        if result.status().as_u16() == 401 {
            let www_auth = result.headers().get("www-authenticate")
                .or_else(|| result.headers().get("WWW-Authenticate"))
                .map(|v| v.as_str())
                .ok_or_else(|| RegistryError::AuthError("No WWW-Authenticate header".into()))?;
            self.handle_auth_challenge(registry, www_auth).await?;
            let result2 = self.do_post(&url, body, extra_headers).await?;
            if result2.status().as_u16() >= 400 {
                return Err(RegistryError::HttpStatus(result2.status().as_u16()));
            }
            Ok(result2.body().to_vec())
        } else if result.status().as_u16() >= 400 {
            Err(RegistryError::HttpStatus(result.status().as_u16()))
        } else {
            Ok(result.body().to_vec())
        }
    }

    async fn do_post(&self, url: &str, body: &[u8], extra_headers: &[(&str, &str)]) -> Result<Response, RegistryError> {
        let mut builder = Request::builder()
            .method(edgerun_http::Method::POST)
            .uri(url)
            .body(body.to_vec());
        for &(k, v) in extra_headers {
            builder = builder.header(k, v);
        }
        if let Some(ref token) = self.token {
            builder = builder.header("Authorization", &format!("Bearer {}", token));
        }
        let request = builder.build()?;
        let client = HttpClient::new().no_redirects();
        client.execute(&request).await.map_err(|e| RegistryError::HttpError(e.to_string()))
    }

    /// Handle OCI Registry V2 authentication (Bearer token exchange).
    async fn handle_auth_challenge(
        &mut self,
        _registry: &str,
        www_auth: &str,
    ) -> Result<(), RegistryError> {
        let (realm, service, scope) = parse_bearer_auth(www_auth)
            .ok_or_else(|| RegistryError::AuthError("Invalid Bearer challenge".into()))?;

        let mut url = format!("{}?service={}", realm, urlencoding::encode(&service));
        if let Some(sc) = scope {
            url.push_str(&format!("&scope={}", urlencoding::encode(&sc)));
        }

        let mut builder = Request::builder()
            .method(edgerun_http::Method::GET)
            .uri(&url);

        // Resolve credentials based on auth type
        let creds = match &self.auth {
            RegistryAuth::Basic { username, password } => {
                Some((username.clone(), password.clone()))
            }
            RegistryAuth::FromSecretService { data_root, namespace, registry_host } => {
                super::auth::resolve_from_secret_service(data_root, namespace, registry_host)
            }
            _ => None,
        };

        if let Some((username, password)) = creds {
            let auth_str = format!("{}:{}", username, password);
            let encoded = super::base64::encode(auth_str.as_bytes());
            builder = builder.header("Authorization", &format!("Basic {}", encoded));
        }

        let request = builder.build()?;
        let client = HttpClient::new();
        let resp = client.execute(&request).await.map_err(|e| RegistryError::HttpError(e.to_string()))?;

        let value = parse_json_bytes(resp.body())
            .map_err(|e| RegistryError::ParseError(e))?;

        self.token = if let edgerun_json::JsonValue::Object(fields) = &value {
            fields.iter()
                .find(|(k, _)| k == "token" || k == "access_token")
                .and_then(|(_, v)| {
                    if let edgerun_json::JsonValue::String(s) = v { Some(s.clone()) } else { None }
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
    async fn ensure_auth(&mut self, registry: &str) -> Result<(), RegistryError> {
        if self.token.is_none() && !matches!(self.auth, RegistryAuth::Anonymous) {
            self.ping(registry).await?;
        }
        Ok(())
    }

    /// Ping the registry to verify connectivity and obtain auth challenge.
    pub async fn ping(&mut self, registry: &str) -> Result<(), RegistryError> {
        let resp = self.do_get_raw(registry, "/v2/").await?;
        // 200 = no auth needed, 401 = auth needed (success for token flow), 4xx/5xx = error
        if resp.status().as_u16() >= 400 && resp.status().as_u16() != 401 {
            return Err(RegistryError::HttpStatus(resp.status().as_u16()));
        }
        Ok(())
    }

    async fn do_get_raw(&mut self, registry: &str, path: &str) -> Result<Response, RegistryError> {
        let url = format!("https://{}{}", registry, path);
        let mut builder = Request::builder()
            .method(edgerun_http::Method::GET)
            .uri(&url);
        if let Some(ref token) = self.token {
            builder = builder.header("Authorization", &format!("Bearer {}", token));
        }
        let request = builder.build()?;
        let client = HttpClient::new().no_redirects();
        client.execute(&request).await.map_err(|e| RegistryError::HttpError(e.to_string()))
    }

    /// Resolve an image reference to its manifest.
    pub async fn resolve_manifest(&mut self, image: &ImageRef) -> Result<ImageManifest, RegistryError> {
        self.ensure_auth(&image.registry).await?;

        let url = format!("https://{}/v2/{}/manifests/{}", image.registry, image.repository, image.tag);
        let headers = [
            ("Accept", "application/vnd.docker.distribution.manifest.v2+json"),
            ("Accept", "application/vnd.oci.image.manifest.v1+json"),
            ("Accept", "application/vnd.docker.distribution.manifest.list.v2+json"),
            ("Accept", "application/vnd.oci.image.index.v1+json"),
        ];
        let mut builder = Request::builder()
            .method(edgerun_http::Method::GET)
            .uri(&url);
        if let Some(ref token) = self.token {
            builder = builder.header("Authorization", &format!("Bearer {}", token));
        }
        for (k, v) in &headers {
            builder = builder.header(k, v);
        }
        let request = builder.build()?;
        let client = HttpClient::new().no_redirects();
        let resp = client.execute(&request).await.map_err(|e| RegistryError::HttpError(e.to_string()))?;

        if resp.status().as_u16() == 401 {
            let www_auth = resp.headers().get("www-authenticate")
                .or_else(|| resp.headers().get("WWW-Authenticate"))
                .map(|v| v.as_str())
                .ok_or_else(|| RegistryError::AuthError("No WWW-Authenticate header".into()))?;
            self.handle_auth_challenge(&image.registry, www_auth).await?;
            // Retry
            let mut builder2 = Request::builder()
                .method(edgerun_http::Method::GET)
                .uri(&url);
            if let Some(ref token) = self.token {
                builder2 = builder2.header("Authorization", &format!("Bearer {}", token));
            }
            for (k, v) in &headers {
                builder2 = builder2.header(k, v);
            }
            let request2 = builder2.build()?;
            let resp2 = client.execute(&request2).await.map_err(|e| RegistryError::HttpError(e.to_string()))?;
            parse_manifest(resp2.body()).map_err(|e| RegistryError::ParseError(e))
        } else if resp.status().as_u16() >= 400 {
            Err(RegistryError::HttpStatus(resp.status().as_u16()))
        } else {
            parse_manifest(resp.body()).map_err(|e| RegistryError::ParseError(e))
        }
    }

    /// Fetch a manifest by digest.
    pub async fn fetch_manifest_by_digest(
        &mut self,
        registry: &str,
        repository: &str,
        digest: &str,
    ) -> Result<SingleManifest, RegistryError> {
        let body = self.fetch_blob(registry, repository, digest).await?;
        parse_single_manifest(&body).map_err(|e| RegistryError::ParseError(e))
    }

    /// Fetch a single blob by digest.
    pub async fn fetch_blob(
        &mut self,
        registry: &str,
        repository: &str,
        digest: &str,
    ) -> Result<Vec<u8>, RegistryError> {
        let url = format!("/v2/{}/blobs/{}", repository, digest);
        self.authenticated_get(registry, &url, &[]).await
    }

    /// Pull an image to a local bundle directory.
    pub async fn pull(
        &mut self,
        image: &ImageRef,
        bundle_path: &Path,
        store_path: &Path,
    ) -> Result<PathBuf, RegistryError> {
        let manifest = self.resolve_manifest(image).await?;

        let manifest_data = match manifest {
            ImageManifest::Single(m) => m,
            ImageManifest::Index(idx) => {
                if idx.manifests.is_empty() {
                    return Err(RegistryError::NoManifests);
                }
                // Select manifest matching current platform (linux/amd64 preferred, fallback to first).
                let current_arch = std::env::consts::ARCH;
                let current_os = std::env::consts::OS;
                let best = idx.manifests.iter()
                    .find(|m| {
                        m.platform.as_ref().map(|p| {
                            p.architecture.as_deref() == Some(current_arch) &&
                            p.os.as_deref() == Some(current_os)
                        }).unwrap_or(false)
                    })
                    .unwrap_or(&idx.manifests[0]);
                self.fetch_manifest_by_digest(
                    &image.registry,
                    &image.repository,
                    &best.digest,
                ).await?
            }
        };

        let config_blob = self.fetch_blob(
            &image.registry,
            &image.repository,
            &manifest_data.config_digest,
        ).await?;
        let image_config = parse_image_config(&config_blob)
            .map_err(|e| RegistryError::ParseError(e.to_string()))?;

        std::fs::create_dir_all(bundle_path)?;
        std::fs::create_dir_all(store_path)?;

        let rootfs = bundle_path.join("rootfs");
        std::fs::create_dir_all(&rootfs)?;

        let cache_dir = store_path.join("cache");
        std::fs::create_dir_all(&cache_dir)?;

        let mut layer_dirs = Vec::new();
        for layer in manifest_data.layers.iter() {
            let cache_key = layer.digest.replace(':', "_");
            let cached_layer = cache_dir.join(&cache_key);

            if cached_layer.is_dir() {
                layer_dirs.push(cached_layer);
                continue;
            }

            let ext = match layer.media_type.as_deref() {
                Some(mt) if mt.contains("zstd") => "tar.zst",
                Some(mt) if mt.contains("gzip") || mt.contains("tar") => "tar.gz",
                Some(mt) if mt.contains("oci") && !mt.contains("gzip") && !mt.contains("zstd") => "tar",
                _ => "tar.gz",
            };
            let blob_path = store_path.join(format!("{}.{}", &cache_key, ext));
            if !blob_path.exists() {
                self.download_blob(
                    &image.registry,
                    &image.repository,
                    &layer.digest,
                    &blob_path,
                ).await?;
            }

            verify_blob_digest(&blob_path, &layer.digest)?;

            std::fs::create_dir_all(&cached_layer)?;
            extract_layer(&blob_path, &cached_layer, layer.media_type.as_deref())?;
            layer_dirs.push(cached_layer);
        }

        apply_whiteouts(&layer_dirs)?;
        build_rootfs(&layer_dirs, &rootfs)?;

        let config_json = generate_oci_spec(&image_config, &rootfs);
        std::fs::write(bundle_path.join("config.json"), config_json)?;

        Ok(bundle_path.to_path_buf())
    }

    /// Download a blob to a local file, tracking bytes.
    async fn download_blob(
        &mut self,
        registry: &str,
        repository: &str,
        digest: &str,
        dest: &Path,
    ) -> Result<(), RegistryError> {
        let url = format!("/v2/{}/blobs/{}", repository, digest);
        let body = self.authenticated_get(registry, &url, &[]).await?;

        let n = body.len() as u64;
        self.bytes_downloaded += n;

        std::fs::write(dest, &body).map_err(|e| RegistryError::IoError(e))
    }

    // ------------------------------------------------------------------
    // Push
    // ------------------------------------------------------------------

    /// Push a local bundle to a registry.
    pub async fn push(&mut self, image: &ImageRef, bundle_path: &Path) -> Result<(), RegistryError> {
        self.ensure_auth(&image.registry).await?;

        let rootfs = bundle_path.join("rootfs");
        if !rootfs.is_dir() {
            return Err(RegistryError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("rootfs not found: {}", rootfs.display()),
            )));
        }

        let config_path = bundle_path.join("config.json");
        let config_json = std::fs::read_to_string(&config_path)
            .map_err(RegistryError::IoError)?;

        let config_blob = config_json.as_bytes().to_vec();
        let config_digest = format!("sha256:{}", hex_digest(&config_blob));
        self.push_blob_raw(&image.registry, &image.repository, &config_blob, &config_digest).await?;

        let layer_data = create_tar_from_dir(&rootfs)?;
        let layer_digest = format!("sha256:{}", hex_digest(&layer_data));
        self.push_blob_raw(&image.registry, &image.repository, &layer_data, &layer_digest).await?;

        let manifest = format!(
            r#"{{"schemaVersion":2,"mediaType":"application/vnd.oci.image.manifest.v1+json","config":{{"mediaType":"application/vnd.oci.image.config.v1+json","digest":"{}","size":{}}},"layers":[{{"mediaType":"application/vnd.oci.image.layer.v1.tar+gzip","digest":"{}","size":{}}}]}}"#,
            config_digest, config_blob.len(), layer_digest, layer_data.len(),
        );

        self.push_manifest(&image.registry, &image.repository, manifest.as_bytes(), &image.tag).await?;
        let manifest_digest = hex_digest(manifest.as_bytes());
        self.push_manifest_by_digest(&image.registry, &image.repository, manifest.as_bytes(), &format!("sha256:{}", manifest_digest)).await?;

        Ok(())
    }

    /// Push a blob using monolithic upload.
    async fn push_blob_raw(&mut self, registry: &str, repository: &str, data: &[u8], digest: &str) -> Result<(), RegistryError> {
        let init_path = format!("/v2/{}/blobs/uploads/", repository);
        let _ = self.authenticated_post(registry, &init_path, &[], &[]).await?;

        let upload_path = format!("/v2/{}/blobs/uploads/{}", repository, super::urlencoding::encode(digest));
        let put_path = format!("{}?digest={}", upload_path, super::urlencoding::encode(digest));
        self.authenticated_put(registry, &put_path, data, &[("Content-Type", "application/octet-stream")]).await?;
        Ok(())
    }

    /// Push a manifest with a tag.
    async fn push_manifest(&mut self, registry: &str, repository: &str, manifest: &[u8], tag: &str) -> Result<(), RegistryError> {
        let path = format!("/v2/{}/manifests/{}", repository, tag);
        self.authenticated_put(registry, &path, manifest, &[
            ("Content-Type", "application/vnd.oci.image.manifest.v1+json"),
        ]).await?;
        Ok(())
    }

    /// Push a manifest by digest.
    async fn push_manifest_by_digest(&mut self, registry: &str, repository: &str, manifest: &[u8], digest: &str) -> Result<(), RegistryError> {
        let path = format!("/v2/{}/manifests/{}", repository, digest);
        self.authenticated_put(registry, &path, manifest, &[
            ("Content-Type", "application/vnd.oci.image.manifest.v1+json"),
        ]).await?;
        Ok(())
    }
}

/// Compute SHA-256 hex digest.
fn hex_digest(data: &[u8]) -> String {
    let hash = edgerun_crypto::sha256(data);
    let mut hex = String::with_capacity(64);
    for b in &hash { hex.push_str(&format!("{:02x}", b)); }
    hex
}

/// Create a gzip-compressed tar from a directory.
fn create_tar_from_dir(dir: &Path) -> std::io::Result<Vec<u8>> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    let mut builder = tar::Builder::new(&mut encoder);
    builder.append_dir_all(".", dir)?;
    builder.finish()?;
    drop(builder);
    Ok(encoder.finish()?)
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
