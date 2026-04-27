//! OCI Registry V2 client — async, using `edgerun_http::HttpClient`.

use crate::prelude::*;
use crate::BareImagePlan;
use core::{fmt, str::FromStr};
#[cfg(all(feature = "std", not(target_os = "none")))]
use std::io;
#[cfg(all(feature = "std", not(target_os = "none")))]
use std::path::{Path, PathBuf};

use edgerun_http::{HttpClient, Request, Response};

use super::auth::{parse_bearer_auth, RegistryAuth};
use super::config::{parse_image_config, parse_json_bytes, parse_manifest, parse_single_manifest};
use super::errors::RegistryError;
#[cfg(all(feature = "std", not(target_os = "none")))]
use super::layer::{apply_whiteouts, build_rootfs, extract_layer, verify_blob_digest};
use super::manifest::{ImageManifest, SingleManifest};
#[cfg(all(feature = "std", not(target_os = "none")))]
use super::oci_spec::generate_oci_spec;
use super::urlencoding;

/// An OCI image reference (e.g., `docker.io/library/alpine:latest`).
#[derive(Clone, Debug)]
pub struct ImageRef {
    pub registry: String,
    pub repository: String,
    pub tag: String,
}

impl FromStr for ImageRef {
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

impl fmt::Display for ImageRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
    #[cfg(all(feature = "std", not(target_os = "none")))]
    pub fn with_registry_config(path: &Path) -> io::Result<Self> {
        let auth = super::auth::load_registry_auth(path)?;
        Ok(Self {
            auth,
            token: None,
            bytes_downloaded: 0,
        })
    }

    /// Use the edgerun secret service for registry credentials.
    #[cfg(all(feature = "std", not(target_os = "none")))]
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
        let resp = client
            .execute(&request)
            .await
            .map_err(|e| RegistryError::HttpError(e.to_string()))?;

        if resp.status().as_u16() == 401 {
            let www_auth = resp
                .headers()
                .get("www-authenticate")
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
            let resp2 = client
                .execute(&request2)
                .await
                .map_err(|e| RegistryError::HttpError(e.to_string()))?;
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
            let www_auth = result
                .headers()
                .get("www-authenticate")
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

    async fn do_put(
        &self,
        url: &str,
        body: &[u8],
        extra_headers: &[(&str, &str)],
    ) -> Result<Response, RegistryError> {
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
        client
            .execute(&request)
            .await
            .map_err(|e| RegistryError::HttpError(e.to_string()))
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
            let www_auth = result
                .headers()
                .get("www-authenticate")
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

    async fn do_post(
        &self,
        url: &str,
        body: &[u8],
        extra_headers: &[(&str, &str)],
    ) -> Result<Response, RegistryError> {
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
        client
            .execute(&request)
            .await
            .map_err(|e| RegistryError::HttpError(e.to_string()))
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
            #[cfg(all(feature = "std", not(target_os = "none")))]
            RegistryAuth::FromSecretService {
                data_root,
                namespace,
                registry_host,
            } => super::auth::resolve_from_secret_service(data_root, namespace, registry_host),
            _ => None,
        };

        if let Some((username, password)) = creds {
            let auth_str = format!("{}:{}", username, password);
            let encoded = edgerun_encoding::base64::standard_encode(auth_str.as_bytes());
            builder = builder.header("Authorization", &format!("Basic {}", encoded));
        }

        let request = builder.build()?;
        let client = HttpClient::new();
        let resp = client
            .execute(&request)
            .await
            .map_err(|e| RegistryError::HttpError(e.to_string()))?;

        let value = parse_json_bytes(resp.body()).map_err(RegistryError::ParseError)?;

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
        client
            .execute(&request)
            .await
            .map_err(|e| RegistryError::HttpError(e.to_string()))
    }

    /// Resolve an image reference to its manifest.
    pub async fn resolve_manifest(
        &mut self,
        image: &ImageRef,
    ) -> Result<ImageManifest, RegistryError> {
        self.ensure_auth(&image.registry).await?;

        let url = format!(
            "https://{}/v2/{}/manifests/{}",
            image.registry, image.repository, image.tag
        );
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
        let resp = client
            .execute(&request)
            .await
            .map_err(|e| RegistryError::HttpError(e.to_string()))?;

        if resp.status().as_u16() == 401 {
            let www_auth = resp
                .headers()
                .get("www-authenticate")
                .or_else(|| resp.headers().get("WWW-Authenticate"))
                .map(|v| v.as_str())
                .ok_or_else(|| RegistryError::AuthError("No WWW-Authenticate header".into()))?;
            self.handle_auth_challenge(&image.registry, www_auth)
                .await?;
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
            let resp2 = client
                .execute(&request2)
                .await
                .map_err(|e| RegistryError::HttpError(e.to_string()))?;
            parse_manifest(resp2.body()).map_err(RegistryError::ParseError)
        } else if resp.status().as_u16() >= 400 {
            Err(RegistryError::HttpStatus(resp.status().as_u16()))
        } else {
            parse_manifest(resp.body()).map_err(RegistryError::ParseError)
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
        parse_single_manifest(&body).map_err(RegistryError::ParseError)
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

    /// Fetch manifest/config metadata and build a no_std runtime image plan.
    ///
    /// Layer contents are not downloaded here. Call [`Self::fetch_blob`] for
    /// each digest in `plan.layers`, then apply them through the bare layer
    /// pipeline.
    pub async fn fetch_bare_image_plan(
        &mut self,
        image: &ImageRef,
        rootfs: &str,
    ) -> Result<BareImagePlan, RegistryError> {
        let manifest = self.resolve_manifest(image).await?;
        let manifest_data = match manifest {
            ImageManifest::Single(manifest) => manifest,
            ImageManifest::Index(index) => {
                if index.manifests.is_empty() {
                    return Err(RegistryError::NoManifests);
                }
                let selected = crate::select_manifest_for_current_target(&index)
                    .unwrap_or(&index.manifests[0]);
                self.fetch_manifest_by_digest(&image.registry, &image.repository, &selected.digest)
                    .await?
            }
        };

        let config_blob = self
            .fetch_blob(
                &image.registry,
                &image.repository,
                &manifest_data.config_digest,
            )
            .await?;
        let image_config = parse_image_config(&config_blob)
            .map_err(|error| RegistryError::ParseError(error.to_string()))?;

        let plan = BareImagePlan::from_manifest_config(&manifest_data, &image_config, rootfs)
            .map_err(|error| RegistryError::ParseError(error.to_string()))?;
        plan.validate_descriptors()
            .map_err(|error| RegistryError::ParseError(error.to_string()))?;
        Ok(plan)
    }

    /// Pull an image to a local bundle directory.
    #[cfg(all(feature = "std", not(target_os = "none")))]
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
                let current_arch = crate::host_arch();
                let current_os = crate::host_os();
                let best = idx
                    .manifests
                    .iter()
                    .find(|m| {
                        m.platform
                            .as_ref()
                            .map(|p| {
                                p.architecture.as_deref() == Some(current_arch)
                                    && p.os.as_deref() == Some(current_os)
                            })
                            .unwrap_or(false)
                    })
                    .unwrap_or(&idx.manifests[0]);
                self.fetch_manifest_by_digest(&image.registry, &image.repository, &best.digest)
                    .await?
            }
        };

        let config_blob = self
            .fetch_blob(
                &image.registry,
                &image.repository,
                &manifest_data.config_digest,
            )
            .await?;
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
                Some(mt) if mt.contains("oci") && !mt.contains("gzip") && !mt.contains("zstd") => {
                    "tar"
                }
                _ => "tar.gz",
            };
            let blob_path = store_path.join(format!("{}.{}", &cache_key, ext));
            if !blob_path.exists() {
                self.download_blob(
                    &image.registry,
                    &image.repository,
                    &layer.digest,
                    &blob_path,
                )
                .await?;
            }

            verify_blob_digest(&blob_path, &layer.digest)?;

            std::fs::create_dir_all(&cached_layer)?;
            extract_layer(&blob_path, &cached_layer, layer.media_type.as_deref())?;
            layer_dirs.push(cached_layer);
        }

        apply_whiteouts(&layer_dirs)?;
        build_rootfs(&layer_dirs, &rootfs)?;

        let config_json = generate_oci_spec(&image_config, rootfs.to_str().unwrap_or("/"));
        std::fs::write(bundle_path.join("config.json"), config_json)?;

        Ok(bundle_path.to_path_buf())
    }

    /// Download a blob to a local file, tracking bytes.
    #[cfg(all(feature = "std", not(target_os = "none")))]
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

        std::fs::write(dest, &body).map_err(RegistryError::IoError)
    }

    // ------------------------------------------------------------------
    // Push
    // ------------------------------------------------------------------

    /// Push a local bundle to a registry.
    #[cfg(all(feature = "std", not(target_os = "none")))]
    pub async fn push(
        &mut self,
        image: &ImageRef,
        bundle_path: &Path,
    ) -> Result<(), RegistryError> {
        self.ensure_auth(&image.registry).await?;

        let rootfs = bundle_path.join("rootfs");
        if !rootfs.is_dir() {
            return Err(RegistryError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("rootfs not found: {}", rootfs.display()),
            )));
        }

        let config_path = bundle_path.join("config.json");
        let config_json = std::fs::read_to_string(&config_path).map_err(RegistryError::IoError)?;

        let config_blob = config_json.as_bytes().to_vec();
        let config_digest = format!("sha256:{}", hex_digest(&config_blob));
        self.push_blob_raw(
            &image.registry,
            &image.repository,
            &config_blob,
            &config_digest,
        )
        .await?;

        let layer_data = create_tar_from_dir(&rootfs)?;
        let layer_digest = format!("sha256:{}", hex_digest(&layer_data));
        self.push_blob_raw(
            &image.registry,
            &image.repository,
            &layer_data,
            &layer_digest,
        )
        .await?;

        let manifest = format!(
            r#"{{"schemaVersion":2,"mediaType":"application/vnd.oci.image.manifest.v1+json","config":{{"mediaType":"application/vnd.oci.image.config.v1+json","digest":"{}","size":{}}},"layers":[{{"mediaType":"application/vnd.oci.image.layer.v1.tar+gzip","digest":"{}","size":{}}}]}}"#,
            config_digest,
            config_blob.len(),
            layer_digest,
            layer_data.len(),
        );

        self.push_manifest(
            &image.registry,
            &image.repository,
            manifest.as_bytes(),
            &image.tag,
        )
        .await?;
        let manifest_digest = hex_digest(manifest.as_bytes());
        self.push_manifest_by_digest(
            &image.registry,
            &image.repository,
            manifest.as_bytes(),
            &format!("sha256:{}", manifest_digest),
        )
        .await?;

        Ok(())
    }

    /// Push a blob using monolithic upload.
    async fn push_blob_raw(
        &mut self,
        registry: &str,
        repository: &str,
        data: &[u8],
        digest: &str,
    ) -> Result<(), RegistryError> {
        let init_path = format!("/v2/{}/blobs/uploads/", repository);
        let _ = self
            .authenticated_post(registry, &init_path, &[], &[])
            .await?;

        let upload_path = format!(
            "/v2/{}/blobs/uploads/{}",
            repository,
            super::urlencoding::encode(digest)
        );
        let put_path = format!(
            "{}?digest={}",
            upload_path,
            super::urlencoding::encode(digest)
        );
        self.authenticated_put(
            registry,
            &put_path,
            data,
            &[("Content-Type", "application/octet-stream")],
        )
        .await?;
        Ok(())
    }

    /// Push a manifest with a tag.
    async fn push_manifest(
        &mut self,
        registry: &str,
        repository: &str,
        manifest: &[u8],
        tag: &str,
    ) -> Result<(), RegistryError> {
        let path = format!("/v2/{}/manifests/{}", repository, tag);
        self.authenticated_put(
            registry,
            &path,
            manifest,
            &[("Content-Type", "application/vnd.oci.image.manifest.v1+json")],
        )
        .await?;
        Ok(())
    }

    /// Push a manifest by digest.
    async fn push_manifest_by_digest(
        &mut self,
        registry: &str,
        repository: &str,
        manifest: &[u8],
        digest: &str,
    ) -> Result<(), RegistryError> {
        let path = format!("/v2/{}/manifests/{}", repository, digest);
        self.authenticated_put(
            registry,
            &path,
            manifest,
            &[("Content-Type", "application/vnd.oci.image.manifest.v1+json")],
        )
        .await?;
        Ok(())
    }
}

/// Compute SHA-256 hex digest.
#[cfg(all(feature = "std", not(target_os = "none")))]
fn hex_digest(data: &[u8]) -> String {
    let hash = edgerun_crypto::sha256(data);
    edgerun_encoding::hex::bytes_to_hex(&hash)
}

/// Create a gzip-compressed tar from a directory.
#[cfg(all(feature = "std", not(target_os = "none")))]
fn create_tar_from_dir(dir: &Path) -> std::io::Result<Vec<u8>> {
    let mut tar = Vec::new();
    append_tar_dir(&mut tar, dir, Path::new(""))?;
    tar.extend_from_slice(&[0u8; 1024]);
    Ok(gzip_bytes(&tar))
}

#[cfg(all(feature = "std", not(target_os = "none")))]
fn append_tar_dir(out: &mut Vec<u8>, dir: &Path, rel: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    if !rel.as_os_str().is_empty() {
        let metadata = std::fs::symlink_metadata(dir)?;
        append_tar_header(
            out,
            rel,
            b'5',
            0,
            metadata.permissions().mode(),
            metadata.uid(),
            metadata.gid(),
            metadata.mtime().max(0) as u64,
            None,
        )?;
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let child_rel = rel.join(entry.file_name());
        let metadata = std::fs::symlink_metadata(&path)?;
        let file_type = metadata.file_type();

        if file_type.is_dir() {
            append_tar_dir(out, &path, &child_rel)?;
        } else if file_type.is_symlink() {
            let target = std::fs::read_link(&path)?;
            append_tar_header(
                out,
                &child_rel,
                b'2',
                0,
                metadata.permissions().mode(),
                metadata.uid(),
                metadata.gid(),
                metadata.mtime().max(0) as u64,
                Some(&target),
            )?;
        } else if file_type.is_file() {
            let data = std::fs::read(&path)?;
            append_tar_header(
                out,
                &child_rel,
                b'0',
                data.len() as u64,
                metadata.permissions().mode(),
                metadata.uid(),
                metadata.gid(),
                metadata.mtime().max(0) as u64,
                None,
            )?;
            out.extend_from_slice(&data);
            let padding = (512 - (data.len() % 512)) % 512;
            out.extend(std::iter::repeat(0).take(padding));
        }
    }

    Ok(())
}

#[cfg(all(feature = "std", not(target_os = "none")))]
fn append_tar_header(
    out: &mut Vec<u8>,
    path: &Path,
    kind: u8,
    size: u64,
    mode: u32,
    uid: u32,
    gid: u32,
    mtime: u64,
    link_name: Option<&Path>,
) -> std::io::Result<()> {
    let mut header = [0u8; 512];
    let name = path_to_tar_bytes(path)?;
    if name.len() > 100 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("tar path too long: {}", path.display()),
        ));
    }
    header[..name.len()].copy_from_slice(&name);

    write_octal(&mut header[100..108], mode as u64)?;
    write_octal(&mut header[108..116], uid as u64)?;
    write_octal(&mut header[116..124], gid as u64)?;
    write_octal(&mut header[124..136], size)?;
    write_octal(&mut header[136..148], mtime)?;
    header[148..156].fill(b' ');
    header[156] = kind;

    if let Some(link_name) = link_name {
        let link = path_to_tar_bytes(link_name)?;
        if link.len() > 100 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("tar link target too long: {}", link_name.display()),
            ));
        }
        header[157..157 + link.len()].copy_from_slice(&link);
    }

    header[257..263].copy_from_slice(b"ustar\0");
    header[263..265].copy_from_slice(b"00");

    let checksum = header.iter().map(|byte| *byte as u32).sum::<u32>();
    write_checksum(&mut header[148..156], checksum)?;
    out.extend_from_slice(&header);
    Ok(())
}

#[cfg(all(feature = "std", not(target_os = "none")))]
fn path_to_tar_bytes(path: &Path) -> std::io::Result<Vec<u8>> {
    let path = path.to_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("non-utf8 tar path: {}", path.display()),
        )
    })?;
    Ok(path.as_bytes().to_vec())
}

#[cfg(all(feature = "std", not(target_os = "none")))]
fn write_octal(field: &mut [u8], value: u64) -> std::io::Result<()> {
    let encoded = format!("{:0width$o}\0", value, width = field.len() - 1);
    if encoded.len() > field.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "tar numeric field overflow",
        ));
    }
    field.copy_from_slice(encoded.as_bytes());
    Ok(())
}

#[cfg(all(feature = "std", not(target_os = "none")))]
fn write_checksum(field: &mut [u8], value: u32) -> std::io::Result<()> {
    let encoded = format!("{:06o}\0 ", value);
    field.copy_from_slice(encoded.as_bytes());
    Ok(())
}

#[cfg(all(feature = "std", not(target_os = "none")))]
fn gzip_bytes(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&[0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 255]);
    out.extend_from_slice(&miniz_oxide::deflate::compress_to_vec(data, 6));
    out.extend_from_slice(&crc32(data).to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out
}

#[cfg(all(feature = "std", not(target_os = "none")))]
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for byte in data {
        crc ^= *byte as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

#[cfg(all(test, feature = "std", not(target_os = "none")))]
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
        } else {
            panic!("expected Basic");
        }
    }

    #[test]
    fn with_secret_service_auth_sets_variant() {
        let root = tmp_root();
        let client = RegistryClient::with_secret_service_auth(&root, "registry", "docker.io");
        if let RegistryAuth::FromSecretService {
            data_root,
            namespace,
            registry_host,
        } = client.auth
        {
            assert_eq!(data_root, root);
            assert_eq!(namespace, "registry");
            assert_eq!(registry_host, "docker.io");
        } else {
            panic!("expected FromSecretService");
        }
    }

    #[test]
    fn create_tar_from_dir_roundtrips_through_layer_extractor() {
        let tmp = tmp_root();
        let src = tmp.join("src");
        let dest = tmp.join("dest");
        std::fs::create_dir_all(src.join("nested")).unwrap();
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(src.join("nested/file.txt"), b"hello from tar").unwrap();

        let layer = create_tar_from_dir(&src).unwrap();
        let blob = tmp.join("layer.tar.gz");
        std::fs::write(&blob, layer).unwrap();

        crate::registry::layer::extract_layer(
            &blob,
            &dest,
            Some("application/vnd.oci.image.layer.v1.tar+gzip"),
        )
        .unwrap();

        assert_eq!(
            std::fs::read_to_string(dest.join("nested/file.txt")).unwrap(),
            "hello from tar"
        );
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
