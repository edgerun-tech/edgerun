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
    let init_path = format!("/v2/{}/blobs/uploads/", repository);
    let _ = client
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

async fn push_manifest(
    client: &mut RegistryClient,
    registry: &str,
    repository: &str,
    manifest: &[u8],
    tag: &str,
) -> Result<(), RegistryError> {
    let path = format!("/v2/{}/manifests/{}", repository, tag);
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
    let path = format!("/v2/{}/manifests/{}", repository, digest);
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
