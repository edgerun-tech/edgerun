//! OCI Registry V2 client — async, using `edgerun_http::HttpClient`.

use crate::prelude::*;
use crate::BareImagePlan;
#[cfg(all(feature = "std", not(target_os = "none")))]
use std::io;
#[cfg(all(feature = "std", not(target_os = "none")))]
use std::path::Path;

use edgerun_http::{HttpClient, Request, Response};

use super::auth::RegistryAuth;
use super::config::{parse_image_config, parse_json_bytes, parse_manifest, parse_single_manifest};
use super::errors::RegistryError;
use super::image_ref::ImageRef;
use super::manifest::{ImageManifest, SingleManifest};
use super::trust::ImageTrustPolicy;
use edgerun_encoding::percent::{
    percent_encode, percent_encode_colon_pair, percent_encode_path_segments,
};
use edgerun_http::auth::parse_bearer_auth;

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

/// The main registry client.
pub struct RegistryClient {
    pub(crate) auth: RegistryAuth,
    pub(crate) token: Option<String>,
    pub(crate) bytes_downloaded: u64,
    pub(crate) insecure_http: bool,
    pub(crate) trust_policy: ImageTrustPolicy,
}

impl RegistryClient {
    /// Create a new registry client with anonymous auth.
    pub fn new() -> Self {
        Self {
            auth: RegistryAuth::Anonymous,
            token: None,
            bytes_downloaded: 0,
            insecure_http: false,
            trust_policy: ImageTrustPolicy::default(),
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

    pub fn with_image_trust_policy(mut self, trust_policy: ImageTrustPolicy) -> Self {
        self.trust_policy = trust_policy;
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
            trust_policy: ImageTrustPolicy::default(),
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
            trust_policy: ImageTrustPolicy::default(),
        }
    }

    pub(crate) fn registry_url(&self, registry: &str, path: &str) -> String {
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
            self.response_body_following_redirects(&client, &url, resp2)
                .await
        } else if resp.status().as_u16() >= 400 {
            Err(RegistryError::HttpError(format!(
                "GET {url} returned HTTP {}",
                resp.status().as_u16()
            )))
        } else {
            self.response_body_following_redirects(&client, &url, resp)
                .await
        }
    }

    async fn response_body_following_redirects(
        &mut self,
        client: &HttpClient,
        start_url: &str,
        mut resp: Response,
    ) -> Result<Vec<u8>, RegistryError> {
        let mut redirects = 0u8;
        let mut current_url = start_url.to_string();
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
            let redirect_url = resolve_redirect_location(&current_url, &location)?;
            let request = Request::builder()
                .method(edgerun_http::Method::GET)
                .uri(&redirect_url)
                .build()?;
            resp = client
                .execute(&request)
                .await
                .map_err(|e| RegistryError::HttpError(e.to_string()))?;
            if resp.status().as_u16() >= 400 {
                return Err(RegistryError::HttpError(format!(
                    "GET redirect {redirect_url} returned HTTP {}",
                    resp.status().as_u16()
                )));
            }
            current_url = redirect_url;
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

    #[cfg(test)]
    fn test_resolve_redirect_location(
        current_url: &str,
        location: &str,
    ) -> Result<String, RegistryError> {
        resolve_redirect_location(current_url, location)
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
        Ok(self
            .authenticated_post_response(registry, path, body, extra_headers)
            .await?
            .body()
            .to_vec())
    }

    pub(crate) async fn authenticated_post_response(
        &mut self,
        registry: &str,
        path: &str,
        body: &[u8],
        extra_headers: &[(&str, &str)],
    ) -> Result<Response, RegistryError> {
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
            Ok(result2)
        } else if result.status().as_u16() >= 400 {
            Err(RegistryError::HttpStatus(result.status().as_u16()))
        } else {
            Ok(result)
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

        let mut url = format!("{}?service={}", realm, percent_encode(&service));
        if let Some(sc) = scope {
            url.push_str(&format!("&scope={}", percent_encode(&sc)));
        }

        let mut builder = Request::builder()
            .method(edgerun_http::Method::GET)
            .uri(&url);

        let token = match &self.auth {
            RegistryAuth::Bearer { token } => Some(token.clone()),
            #[cfg(all(feature = "std", not(target_os = "none")))]
            RegistryAuth::FromSecretService {
                data_root,
                namespace,
                registry_host,
            } => Some(
                super::auth::resolve_from_secret_service(data_root, namespace, registry_host)
                    .ok_or_else(|| {
                        RegistryError::AuthError(format!(
                            "no usable secret-service credentials for registry {registry_host}"
                        ))
                    })?,
            ),
            _ => None,
        };

        if let Some(token) = token {
            builder = builder.header("Authorization", &format!("Bearer {}", token));
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
        if resp.status().as_u16() == 401 {
            let www_auth = resp
                .headers()
                .get("www-authenticate")
                .or_else(|| resp.headers().get("WWW-Authenticate"))
                .map(|v| v.as_str())
                .ok_or_else(|| RegistryError::AuthError("No WWW-Authenticate header".into()))?;
            if matches!(self.auth, RegistryAuth::Anonymous) {
                parse_bearer_auth(www_auth)
                    .ok_or_else(|| RegistryError::AuthError("Invalid Bearer challenge".into()))?;
                return Ok(());
            }
            self.handle_auth_challenge(registry, www_auth).await?;
            return Ok(());
        }
        // 200 = no auth needed, 401 = token challenge handled above, 4xx/5xx = error.
        if resp.status().as_u16() >= 400 {
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

        let path = format!(
            "/v2/{}/manifests/{}",
            percent_encode_path_segments(&image.repository),
            percent_encode_colon_pair(image.reference())
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
        let path = format!(
            "/v2/{}/manifests/{}",
            percent_encode_path_segments(repository),
            percent_encode_colon_pair(digest)
        );
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
        let url = format!(
            "/v2/{}/blobs/{}",
            percent_encode_path_segments(repository),
            percent_encode_colon_pair(digest)
        );
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
        self.trust_policy.enforce(image)?;
        let manifest = self.resolve_manifest(image).await?;
        let manifest_data = match manifest {
            ImageManifest::Single(manifest) => manifest,
            ImageManifest::Index(index) => {
                if index.manifests.is_empty() {
                    return Err(RegistryError::NoManifests);
                }
                let selected =
                    crate::select_manifest_for_current_target(&index).ok_or_else(|| {
                        RegistryError::ParseError(format!(
                            "no manifest found for platform {}/{}",
                            crate::host_os(),
                            crate::host_arch()
                        ))
                    })?;
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

fn resolve_redirect_location(current_url: &str, location: &str) -> Result<String, RegistryError> {
    if location.starts_with("http://") || location.starts_with("https://") {
        return Ok(location.to_string());
    }

    let (scheme, rest) = current_url
        .split_once("://")
        .ok_or_else(|| RegistryError::HttpError(format!("invalid redirect base: {current_url}")))?;
    let (authority, path) = rest
        .split_once('/')
        .map(|(authority, path)| (authority, format!("/{path}")))
        .unwrap_or((rest, "/".to_string()));

    if authority.is_empty() {
        return Err(RegistryError::HttpError(format!(
            "invalid redirect base: {current_url}"
        )));
    }

    if location.starts_with('/') {
        return Ok(format!("{scheme}://{authority}{location}"));
    }

    let base_dir = path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("");
    let joined = if base_dir.is_empty() {
        format!("/{location}")
    } else {
        format!("{base_dir}/{location}")
    };
    Ok(format!("{scheme}://{authority}{joined}"))
}
