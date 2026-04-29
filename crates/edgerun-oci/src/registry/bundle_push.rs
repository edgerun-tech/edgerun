//! Local OCI bundle push workflow.

use crate::prelude::*;
use std::io;
use std::path::Path;

use super::client::RegistryClient;
use super::errors::RegistryError;
use super::image_ref::ImageRef;
use super::push_manifest::push_manifest_json;
use super::tar_push::create_tar_from_dir;
use crate::layer_pipeline::sha256_digest_reference;
use edgerun_encoding::percent::{
    percent_encode, percent_encode_colon_pair, percent_encode_path_segments,
};
use edgerun_http::Response;

const OCI_MANIFEST_MEDIA_TYPE: &str = "application/vnd.oci.image.manifest.v1+json";

pub(crate) async fn push(
    client: &mut RegistryClient,
    image: &ImageRef,
    bundle_path: &Path,
) -> Result<(), RegistryError> {
    client.ensure_auth(&image.registry).await?;

    let rootfs = bundle_path.join("rootfs");
    if !rootfs.is_dir() {
        return Err(RegistryError::IoError(io::Error::new(
            io::ErrorKind::NotFound,
            format!("rootfs not found: {}", rootfs.display()),
        )));
    }

    let config_path = bundle_path.join("config.json");
    let config_json = std::fs::read_to_string(&config_path).map_err(RegistryError::IoError)?;

    let config_blob = config_json.as_bytes().to_vec();
    let config_digest = sha256_digest_reference(&config_blob);
    push_blob_raw(
        client,
        &image.registry,
        &image.repository,
        &config_blob,
        &config_digest,
    )
    .await?;

    let layer_data = create_tar_from_dir(&rootfs)?;
    let layer_digest = sha256_digest_reference(&layer_data);
    push_blob_raw(
        client,
        &image.registry,
        &image.repository,
        &layer_data,
        &layer_digest,
    )
    .await?;

    let manifest = push_manifest_json(
        config_digest.clone(),
        config_blob.len(),
        layer_digest.clone(),
        layer_data.len(),
    )
    .map_err(|error| RegistryError::ParseError(error.to_string()))?;

    push_manifest(
        client,
        &image.registry,
        &image.repository,
        manifest.as_bytes(),
        &image.tag,
    )
    .await?;
    let manifest_digest = sha256_digest_reference(manifest.as_bytes());
    push_manifest_by_digest(
        client,
        &image.registry,
        &image.repository,
        manifest.as_bytes(),
        &manifest_digest,
    )
    .await
}

async fn push_blob_raw(
    client: &mut RegistryClient,
    registry: &str,
    repository: &str,
    data: &[u8],
    digest: &str,
) -> Result<(), RegistryError> {
    let init_path = format!(
        "/v2/{}/blobs/uploads/",
        percent_encode_path_segments(repository)
    );
    let upload = client
        .authenticated_post_response(registry, &init_path, &[], &[])
        .await?;
    let upload_path = upload_location_path(&upload, repository, &init_path)?;
    let put_path = format!(
        "{}{}digest={}",
        upload_path,
        if upload_path.contains('?') { "&" } else { "?" },
        percent_encode(digest)
    );
    client
        .authenticated_put(
            registry,
            &put_path,
            data,
            &[("Content-Type", "application/octet-stream")],
        )
        .await?;
    Ok(())
}

fn upload_location_path(
    response: &Response,
    repository: &str,
    base_path: &str,
) -> Result<String, RegistryError> {
    let location = response
        .headers()
        .get("location")
        .or_else(|| response.headers().get("Location"))
        .map(|value| value.as_str())
        .ok_or_else(|| RegistryError::HttpError("blob upload response missing Location".into()))?;

    if location.starts_with('/') {
        return Ok(location.to_string());
    }

    if let Some(without_scheme) = location
        .strip_prefix("http://")
        .or_else(|| location.strip_prefix("https://"))
    {
        if let Some(path_start) = without_scheme.find('/') {
            return Ok(without_scheme[path_start..].to_string());
        }
    }

    if location.contains('/') {
        return Ok(format!("/{}", location.trim_start_matches('/')));
    }

    if let Some((upload_id, query)) = location.split_once('?') {
        return Ok(format!(
            "{}{}?{}",
            base_path,
            percent_encode(upload_id),
            query
        ));
    }

    Ok(format!(
        "/v2/{}/blobs/uploads/{}",
        percent_encode_path_segments(repository),
        percent_encode(location)
    ))
}

async fn push_manifest(
    client: &mut RegistryClient,
    registry: &str,
    repository: &str,
    manifest: &[u8],
    tag: &str,
) -> Result<(), RegistryError> {
    let path = format!(
        "/v2/{}/manifests/{}",
        percent_encode_path_segments(repository),
        percent_encode_colon_pair(tag)
    );
    client
        .authenticated_put(
            registry,
            &path,
            manifest,
            &[("Content-Type", OCI_MANIFEST_MEDIA_TYPE)],
        )
        .await?;
    Ok(())
}

async fn push_manifest_by_digest(
    client: &mut RegistryClient,
    registry: &str,
    repository: &str,
    manifest: &[u8],
    digest: &str,
) -> Result<(), RegistryError> {
    let path = format!(
        "/v2/{}/manifests/{}",
        percent_encode_path_segments(repository),
        percent_encode_colon_pair(digest)
    );
    client
        .authenticated_put(
            registry,
            &path,
            manifest,
            &[("Content-Type", OCI_MANIFEST_MEDIA_TYPE)],
        )
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_http::{HeaderMap, Response, StatusCode};

    fn upload_response(location: &str) -> Response {
        let mut headers = HeaderMap::new();
        headers.insert("Location", location).unwrap();
        Response::from_parts(StatusCode::new(202).unwrap(), headers, Vec::new())
    }

    #[test]
    fn upload_location_path_uses_registry_location_header() {
        let response = upload_response("/v2/owner/repo/blobs/uploads/uuid-123?_state=opaque");

        let path =
            upload_location_path(&response, "owner/repo", "/v2/owner/repo/blobs/uploads/").unwrap();

        assert_eq!(path, "/v2/owner/repo/blobs/uploads/uuid-123?_state=opaque");
    }

    #[test]
    fn upload_location_path_strips_absolute_registry_url() {
        let response =
            upload_response("https://registry.example/v2/owner/repo/blobs/uploads/uuid-123");

        let path =
            upload_location_path(&response, "owner/repo", "/v2/owner/repo/blobs/uploads/").unwrap();

        assert_eq!(path, "/v2/owner/repo/blobs/uploads/uuid-123");
    }

    #[test]
    fn upload_location_path_resolves_relative_upload_id_with_query() {
        let response = upload_response("uuid 123?_state=opaque");

        let path =
            upload_location_path(&response, "owner/repo", "/v2/owner/repo/blobs/uploads/").unwrap();

        assert_eq!(
            path,
            "/v2/owner/repo/blobs/uploads/uuid%20123?_state=opaque"
        );
    }

    #[test]
    fn upload_location_path_resolves_query_only_relative_location() {
        let response = upload_response("?_state=opaque");

        let path =
            upload_location_path(&response, "owner/repo", "/v2/owner/repo/blobs/uploads/").unwrap();

        assert_eq!(path, "/v2/owner/repo/blobs/uploads/?_state=opaque");
    }
}
