//! OCI Registry V2 client — async, using `edgerun_http::HttpClient`.

use crate::prelude::*;
use crate::BareImagePlan;
use core::{fmt, str::FromStr};
#[cfg(all(feature = "std", not(target_os = "none")))]
use std::io;
#[cfg(all(feature = "std", not(target_os = "none")))]
use std::path::Path;

use edgerun_http::{HttpClient, Request, Response};

use super::auth::{parse_bearer_auth, RegistryAuth};
use super::config::{parse_image_config, parse_json_bytes, parse_manifest, parse_single_manifest};
use super::errors::RegistryError;
use super::manifest::{ImageManifest, SingleManifest};
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
    pub(crate) async fn authenticated_put(
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
    pub(crate) async fn authenticated_post(
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
    pub(crate) async fn ensure_auth(&mut self, registry: &str) -> Result<(), RegistryError> {
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
    ) -> Result<std::path::PathBuf, RegistryError> {
        super::pull::pull(self, image, bundle_path, store_path).await
    }

    /// Pull an image to a local bundle directory and report coarse progress.
    #[cfg(all(feature = "std", not(target_os = "none")))]
    pub async fn pull_with_progress<F>(
        &mut self,
        image: &ImageRef,
        bundle_path: &Path,
        store_path: &Path,
        progress: F,
    ) -> Result<super::pull::ImagePullReport, RegistryError>
    where
        F: FnMut(super::pull::PullProgress),
    {
        super::pull::pull_with_progress(self, image, bundle_path, store_path, progress).await
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
        super::bundle_push::push(self, image, bundle_path).await
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

#[cfg(all(test, feature = "std", not(target_os = "none")))]
#[path = "../../tests/unit_src/src/registry/client_tests.rs"]
mod tests;
