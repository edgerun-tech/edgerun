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
#[cfg(feature = "edgefs")]
use crate::image_apply::{
    apply_bare_image_layer_blob_sha256, validate_bare_image_layer_set, BareImageApplyReport,
};
#[cfg(feature = "edgefs")]
use crate::layer_pipeline::{
    format_digest, sha256_layer_digest, validate_layer_descriptor, LayerApplyReport, LayerDigest,
};
#[cfg(feature = "edgefs")]
use crate::tar_layer::{
    layer_compression, OciLayerCompression, TarLayerApplyReport, TarLayerSink,
    UncompressedTarStream,
};
#[cfg(feature = "edgefs")]
use edgerun_edgefs::EdgeFs;
#[cfg(feature = "edgefs")]
use edgerun_storage::BlockStorage;

#[derive(Debug, Clone)]
struct RegistryTokenResponse {
    token: Option<String>,
}

edgerun_json::impl_json_struct! {
    RegistryTokenResponse {
        required {}
        optional { token: ["token", "access_token"] => String }
    }
}

#[derive(Debug, Clone)]
struct PushManifestConfig {
    media_type: String,
    digest: String,
    size: usize,
}

#[derive(Debug, Clone)]
struct PushManifestLayer {
    media_type: String,
    digest: String,
    size: usize,
}

#[derive(Debug, Clone)]
struct PushManifest {
    schema_version: u32,
    media_type: String,
    config: PushManifestConfig,
    layers: Vec<PushManifestLayer>,
}

edgerun_json::impl_json_struct! {
    PushManifestConfig {
        required {
            media_type: "mediaType" => String,
            digest: "digest" => String,
            size: "size" => usize,
        }
        optional {}
    }
}

edgerun_json::impl_json_struct! {
    PushManifestLayer {
        required {
            media_type: "mediaType" => String,
            digest: "digest" => String,
            size: "size" => usize,
        }
        optional {}
    }
}

edgerun_json::impl_json_struct! {
    PushManifest {
        required {
            schema_version: "schemaVersion" => u32,
            media_type: "mediaType" => String,
            config: "config" => PushManifestConfig,
            layers: "layers" => Vec<PushManifestLayer>,
        }
        optional {}
    }
}

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
    insecure_http: bool,
}

#[cfg(feature = "edgefs")]
#[derive(Debug, Clone)]
pub struct EdgeFsImagePullReport {
    pub plan: BareImagePlan,
    pub apply: BareImageApplyReport,
    pub bytes_downloaded: u64,
}

#[cfg(all(feature = "std", not(target_os = "none")))]
#[derive(Debug, Clone)]
pub struct ImagePullReport {
    pub path: PathBuf,
    pub bytes_downloaded: u64,
    pub layers: usize,
}

#[cfg(all(feature = "std", not(target_os = "none")))]
#[derive(Debug, Clone)]
pub enum PullProgress {
    Resolving {
        image: String,
    },
    ManifestResolved {
        layers: usize,
        config_digest: String,
    },
    FetchingConfig {
        digest: String,
    },
    ConfigFetched {
        bytes: u64,
    },
    LayerCached {
        index: usize,
        total: usize,
        digest: String,
    },
    LayerDownloading {
        index: usize,
        total: usize,
        digest: String,
        size: u64,
    },
    LayerDownloaded {
        index: usize,
        total: usize,
        digest: String,
        bytes: u64,
    },
    LayerExtracting {
        index: usize,
        total: usize,
        digest: String,
    },
    LayerExtracted {
        index: usize,
        total: usize,
        digest: String,
    },
    ApplyingWhiteouts {
        layers: usize,
    },
    BuildingRootfs {
        path: PathBuf,
    },
    WritingConfig {
        path: PathBuf,
    },
}

impl RegistryClient {
    /// Create a new registry client with anonymous auth.
    pub fn new() -> Self {
        Self {
            auth: RegistryAuth::Anonymous,
            token: None,
            bytes_downloaded: 0,
            insecure_http: false,
        }
    }

    /// Use plain HTTP for registries explicitly trusted by the caller.
    pub fn insecure_http(mut self) -> Self {
        self.insecure_http = true;
        self
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
            insecure_http: false,
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
            insecure_http: false,
        }
    }

    fn registry_url(&self, registry: &str, path: &str) -> String {
        let scheme = if self.insecure_http { "http" } else { "https" };
        let registry = registry_api_host(registry);
        format!("{}://{}{}", scheme, registry, path)
    }

    /// Perform a GET request with auth handling.
    async fn authenticated_get(
        &mut self,
        registry: &str,
        path: &str,
        extra_headers: &[(&str, &str)],
    ) -> Result<Vec<u8>, RegistryError> {
        let url = self.registry_url(registry, path);
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

        let client = registry_http_client().no_redirects();
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
            self.response_body_following_redirects(&client, resp2).await
        } else if resp.status().as_u16() >= 400 {
            Err(RegistryError::HttpError(format!(
                "GET {url} returned HTTP {}",
                resp.status().as_u16()
            )))
        } else {
            self.response_body_following_redirects(&client, resp).await
        }
    }

    async fn response_body_following_redirects(
        &mut self,
        client: &HttpClient,
        mut resp: Response,
    ) -> Result<Vec<u8>, RegistryError> {
        let mut redirects = 0u8;
        while (300..400).contains(&resp.status().as_u16()) {
            if redirects >= 5 {
                return Err(RegistryError::HttpError(
                    "too many registry redirects".into(),
                ));
            }
            let location = resp
                .headers()
                .get("location")
                .or_else(|| resp.headers().get("Location"))
                .map(|value| value.as_str().to_string())
                .ok_or_else(|| {
                    RegistryError::HttpError("registry redirect without Location".into())
                })?;
            let request = Request::builder()
                .method(edgerun_http::Method::GET)
                .uri(&location)
                .build()?;
            resp = client
                .execute(&request)
                .await
                .map_err(|e| RegistryError::HttpError(e.to_string()))?;
            if resp.status().as_u16() >= 400 {
                return Err(RegistryError::HttpError(format!(
                    "GET redirect {location} returned HTTP {}",
                    resp.status().as_u16()
                )));
            }
            redirects = redirects.saturating_add(1);
        }
        if resp.status().as_u16() >= 400 {
            return Err(RegistryError::HttpError(format!(
                "registry GET after redirect returned HTTP {}",
                resp.status().as_u16()
            )));
        }
        Ok(self.count_body(resp.body()))
    }

    fn count_body(&mut self, body: &[u8]) -> Vec<u8> {
        self.bytes_downloaded = self.bytes_downloaded.saturating_add(body.len() as u64);
        body.to_vec()
    }

    /// Perform an authenticated PUT request.
    async fn authenticated_put(
        &mut self,
        registry: &str,
        path: &str,
        body: &[u8],
        extra_headers: &[(&str, &str)],
    ) -> Result<Vec<u8>, RegistryError> {
        let url = self.registry_url(registry, path);
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
        let client = registry_http_client().no_redirects();
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
        let url = self.registry_url(registry, path);
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
        let client = registry_http_client().no_redirects();
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
        let client = registry_http_client();
        let resp = client
            .execute(&request)
            .await
            .map_err(|e| RegistryError::HttpError(e.to_string()))?;

        let value = parse_json_bytes(resp.body()).map_err(|error| {
            RegistryError::ParseError(format!(
                "token response parse failed: status {}, body {} bytes: {}",
                resp.status().as_u16(),
                resp.body().len(),
                error
            ))
        })?;

        self.token = edgerun_json::from_json_value::<RegistryTokenResponse>(value)
            .ok()
            .and_then(|response| response.token);

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
        let url = self.registry_url(registry, path);
        let mut builder = Request::builder()
            .method(edgerun_http::Method::GET)
            .uri(&url);
        if let Some(ref token) = self.token {
            builder = builder.header("Authorization", &format!("Bearer {}", token));
        }
        let request = builder.build()?;
        let client = registry_http_client().no_redirects();
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

        let path = format!("/v2/{}/manifests/{}", image.repository, image.tag);
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
        let body = self
            .authenticated_get(&image.registry, path.as_str(), &headers)
            .await?;
        parse_manifest(&body).map_err(|error| {
            RegistryError::ParseError(format!(
                "manifest parse failed for {}: body {} bytes: {}",
                image,
                body.len(),
                error
            ))
        })
    }

    /// Fetch a manifest by digest.
    pub async fn fetch_manifest_by_digest(
        &mut self,
        registry: &str,
        repository: &str,
        digest: &str,
    ) -> Result<SingleManifest, RegistryError> {
        let path = format!("/v2/{}/manifests/{}", repository, digest);
        let body = self
            .authenticated_get(
                registry,
                &path,
                &[
                    (
                        "Accept",
                        "application/vnd.docker.distribution.manifest.v2+json",
                    ),
                    ("Accept", "application/vnd.oci.image.manifest.v1+json"),
                ],
            )
            .await?;
        parse_single_manifest(&body).map_err(|error| {
            RegistryError::ParseError(format!(
                "manifest digest parse failed for {}/{}@{}: body {} bytes: {}",
                registry,
                repository,
                digest,
                body.len(),
                error
            ))
        })
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
        let image_config = parse_image_config(&config_blob).map_err(|error| {
            RegistryError::ParseError(format!(
                "image config parse failed for {}: body {} bytes: {}",
                manifest_data.config_digest,
                config_blob.len(),
                error
            ))
        })?;

        let plan = BareImagePlan::from_manifest_config(&manifest_data, &image_config, rootfs)
            .map_err(|error| RegistryError::ParseError(error.to_string()))?;
        plan.validate_descriptors()
            .map_err(|error| RegistryError::ParseError(error.to_string()))?;
        Ok(plan)
    }

    /// Pull an image through `edgerun-http` and apply its rootfs layers into EdgeFS.
    ///
    /// The registry manifest/config and all layer blobs are fetched with the
    /// same authenticated registry path used by [`Self::fetch_bare_image_plan`].
    /// Layers are then validated, decompressed, whiteouts are applied, and final
    /// file contents are written into the provided encrypted EdgeFS instance.
    #[cfg(feature = "edgefs")]
    pub async fn pull_into_edgefs<S: BlockStorage>(
        &mut self,
        image: &ImageRef,
        rootfs: &str,
        fs: &mut EdgeFs<S>,
    ) -> Result<EdgeFsImagePullReport, RegistryError> {
        let plan = self.fetch_bare_image_plan(image, rootfs).await?;
        validate_bare_image_layer_set(&plan)
            .map_err(|error| RegistryError::ParseError(error.to_string()))?;

        let mut layer_reports = Vec::with_capacity(plan.layers.len());
        let mut entries_applied = 0usize;
        for (index, layer) in plan.layers.iter().enumerate() {
            let (report, entries) = if layer_compression(layer.media_type.as_deref())
                == OciLayerCompression::Uncompressed
            {
                self.fetch_uncompressed_layer_into_edgefs(
                    &image.registry,
                    &image.repository,
                    &plan,
                    index,
                    fs,
                )
                .await?
            } else {
                let blob = self
                    .fetch_blob(&image.registry, &image.repository, &layer.digest)
                    .await?;
                apply_bare_image_layer_blob_sha256(&plan, index, &blob, fs)
                    .map_err(|error| RegistryError::ParseError(error.to_string()))?
            };
            entries_applied = entries_applied.saturating_add(entries);
            layer_reports.push(report);
        }

        let apply = BareImageApplyReport {
            layers_applied: layer_reports.len(),
            entries_applied,
            layer_reports,
        };

        Ok(EdgeFsImagePullReport {
            plan,
            apply,
            bytes_downloaded: self.bytes_downloaded,
        })
    }

    #[cfg(feature = "edgefs")]
    async fn fetch_uncompressed_layer_into_edgefs<S: BlockStorage>(
        &mut self,
        registry: &str,
        repository: &str,
        plan: &BareImagePlan,
        index: usize,
        fs: &mut EdgeFs<S>,
    ) -> Result<(TarLayerApplyReport, usize), RegistryError> {
        let descriptor = plan
            .layers
            .get(index)
            .ok_or_else(|| RegistryError::ParseError("layer index out of range".into()))?;
        validate_layer_descriptor(descriptor, "sha256")
            .map_err(|error| RegistryError::ParseError(error.to_string()))?;

        let path = format!("/v2/{}/blobs/{}", repository, descriptor.digest);
        let url = self.registry_url(registry, &path);
        let mut builder = Request::builder()
            .method(edgerun_http::Method::GET)
            .uri(&url);
        if let Some(ref token) = self.token {
            builder = builder.header("Authorization", &format!("Bearer {}", token));
        }
        let request = builder.build()?;

        let mut digest = sha256_layer_digest();
        let mut bytes_written = 0u64;
        let mut stream = UncompressedTarStream::new(fs);
        let response = HttpClient::new()
            .version(edgerun_http::HttpVersion::Http1)
            .no_redirects()
            .no_decompress()
            .execute_http1_body_chunks(&request, |chunk| {
                bytes_written = bytes_written.saturating_add(chunk.len() as u64);
                digest.update(chunk);
                stream
                    .push(chunk)
                    .map(|_| ())
                    .map_err(|error| edgerun_http::Error::InvalidResponse(error.to_string()))
            })
            .await
            .map_err(|error| RegistryError::HttpError(error.to_string()))?;

        if response.status().as_u16() >= 400 {
            return Err(RegistryError::HttpStatus(response.status().as_u16()));
        }
        self.bytes_downloaded = self.bytes_downloaded.saturating_add(bytes_written);
        if bytes_written != descriptor.size {
            return Err(RegistryError::ParseError(format!(
                "layer size mismatch: expected {}, got {}",
                descriptor.size, bytes_written
            )));
        }
        let actual_digest = format_digest(digest.algorithm(), &digest.finish());
        if actual_digest != descriptor.digest {
            return Err(RegistryError::DigestMismatch {
                expected: descriptor.digest.clone(),
                computed: actual_digest,
            });
        }
        if let Some(expected_diff_id) = plan.diff_ids.get(index) {
            if expected_diff_id != &descriptor.digest {
                return Err(RegistryError::ParseError(format!(
                    "uncompressed diff_id mismatch for layer {index}: expected {expected_diff_id}, got {}",
                    descriptor.digest
                )));
            }
        }

        let entries_applied = stream
            .finish()
            .map_err(|error| RegistryError::ParseError(error.to_string()))?;
        let layer = LayerApplyReport {
            digest: descriptor.digest.clone(),
            bytes_written,
            media_type: descriptor.media_type.clone(),
        };
        Ok((
            TarLayerApplyReport {
                layer,
                entries_applied,
            },
            entries_applied,
        ))
    }

    /// Pull an image to a local bundle directory.
    #[cfg(all(feature = "std", not(target_os = "none")))]
    pub async fn pull(
        &mut self,
        image: &ImageRef,
        bundle_path: &Path,
        store_path: &Path,
    ) -> Result<PathBuf, RegistryError> {
        self.pull_with_progress(image, bundle_path, store_path, |_| {})
            .await
            .map(|report| report.path)
    }

    /// Pull an image to a local bundle directory and report coarse progress.
    #[cfg(all(feature = "std", not(target_os = "none")))]
    pub async fn pull_with_progress<F>(
        &mut self,
        image: &ImageRef,
        bundle_path: &Path,
        store_path: &Path,
        mut progress: F,
    ) -> Result<ImagePullReport, RegistryError>
    where
        F: FnMut(PullProgress),
    {
        self.bytes_downloaded = 0;
        progress(PullProgress::Resolving {
            image: image.to_string(),
        });
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

        let total_layers = manifest_data.layers.len();
        progress(PullProgress::ManifestResolved {
            layers: total_layers,
            config_digest: manifest_data.config_digest.clone(),
        });
        progress(PullProgress::FetchingConfig {
            digest: manifest_data.config_digest.clone(),
        });
        let config_blob = self
            .fetch_blob(
                &image.registry,
                &image.repository,
                &manifest_data.config_digest,
            )
            .await?;
        progress(PullProgress::ConfigFetched {
            bytes: config_blob.len() as u64,
        });
        let image_config = parse_image_config(&config_blob).map_err(|error| {
            RegistryError::ParseError(format!(
                "image config parse failed for {}: body {} bytes: {}",
                manifest_data.config_digest,
                config_blob.len(),
                error
            ))
        })?;

        std::fs::create_dir_all(bundle_path)?;
        std::fs::create_dir_all(store_path)?;

        let rootfs = bundle_path.join("rootfs");

        let cache_dir = store_path.join("cache");
        std::fs::create_dir_all(&cache_dir)?;

        let mut layer_dirs = Vec::new();
        for (offset, layer) in manifest_data.layers.iter().enumerate() {
            let index = offset + 1;
            let cache_key = layer.digest.replace(':', "_");
            let cached_layer = cache_dir.join(&cache_key);

            if cached_layer.is_dir() {
                progress(PullProgress::LayerCached {
                    index,
                    total: total_layers,
                    digest: layer.digest.clone(),
                });
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
                progress(PullProgress::LayerDownloading {
                    index,
                    total: total_layers,
                    digest: layer.digest.clone(),
                    size: layer.size,
                });
                let bytes = self
                    .download_blob(
                        &image.registry,
                        &image.repository,
                        &layer.digest,
                        &blob_path,
                    )
                    .await?;
                progress(PullProgress::LayerDownloaded {
                    index,
                    total: total_layers,
                    digest: layer.digest.clone(),
                    bytes,
                });
            }

            verify_blob_digest(&blob_path, &layer.digest)?;

            std::fs::create_dir_all(&cached_layer)?;
            progress(PullProgress::LayerExtracting {
                index,
                total: total_layers,
                digest: layer.digest.clone(),
            });
            extract_layer(&blob_path, &cached_layer, layer.media_type.as_deref())?;
            progress(PullProgress::LayerExtracted {
                index,
                total: total_layers,
                digest: layer.digest.clone(),
            });
            layer_dirs.push(cached_layer);
        }

        progress(PullProgress::ApplyingWhiteouts {
            layers: layer_dirs.len(),
        });
        apply_whiteouts(&layer_dirs)?;
        progress(PullProgress::BuildingRootfs {
            path: rootfs.clone(),
        });
        if rootfs.exists() {
            std::fs::remove_dir_all(&rootfs).map_err(RegistryError::IoError)?;
        }
        std::fs::create_dir_all(&rootfs).map_err(RegistryError::IoError)?;
        build_rootfs(&layer_dirs, &rootfs)?;

        let config_json = generate_oci_spec(&image_config, rootfs.to_str().unwrap_or("/"));
        let config_path = bundle_path.join("config.json");
        progress(PullProgress::WritingConfig {
            path: config_path.clone(),
        });
        std::fs::write(config_path, config_json)?;

        Ok(ImagePullReport {
            path: bundle_path.to_path_buf(),
            bytes_downloaded: self.bytes_downloaded,
            layers: total_layers,
        })
    }

    /// Download a blob to a local file, tracking bytes.
    #[cfg(all(feature = "std", not(target_os = "none")))]
    async fn download_blob(
        &mut self,
        registry: &str,
        repository: &str,
        digest: &str,
        dest: &Path,
    ) -> Result<u64, RegistryError> {
        let url = format!("/v2/{}/blobs/{}", repository, digest);
        let body = self.authenticated_get(registry, &url, &[]).await?;
        let bytes = body.len() as u64;

        std::fs::write(dest, &body).map_err(RegistryError::IoError)?;
        Ok(bytes)
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

        let manifest = edgerun_json::to_json_string(&PushManifest {
            schema_version: 2,
            media_type: "application/vnd.oci.image.manifest.v1+json".into(),
            config: PushManifestConfig {
                media_type: "application/vnd.oci.image.config.v1+json".into(),
                digest: config_digest.clone(),
                size: config_blob.len(),
            },
            layers: vec![PushManifestLayer {
                media_type: "application/vnd.oci.image.layer.v1.tar+gzip".into(),
                digest: layer_digest.clone(),
                size: layer_data.len(),
            }],
        })
        .map_err(|error| RegistryError::ParseError(error.to_string()))?;

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

fn registry_http_client() -> HttpClient {
    HttpClient::new().version(edgerun_http::HttpVersion::Http1)
}

fn registry_api_host(registry: &str) -> &str {
    if registry == "docker.io" {
        "registry-1.docker.io"
    } else {
        registry
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
    out.extend_from_slice(&edgerun_encoding::crc32::crc32(data).to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out
}

#[cfg(all(test, feature = "std", not(target_os = "none")))]
#[path = "../../tests/unit_src/src/registry/client_tests.rs"]
mod tests;
