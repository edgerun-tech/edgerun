//! Local OCI bundle pull workflow.

use crate::prelude::*;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::client::RegistryClient;
use super::config::parse_image_config;
use super::errors::RegistryError;
use super::image_ref::ImageRef;
use super::layer::{apply_whiteouts, build_rootfs, extract_layer, verify_blob_digest};
use super::manifest::{ImageManifest, LayerDescriptor, SingleManifest};
use super::oci_spec::generate_oci_spec;
use crate::{sha256_digest_reference, validate_digest_reference};

#[derive(Debug, Clone)]
pub struct ImagePullReport {
    pub path: PathBuf,
    pub bytes_downloaded: u64,
    pub layers: usize,
}

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

pub(crate) async fn pull(
    client: &mut RegistryClient,
    image: &ImageRef,
    bundle_path: &Path,
    store_path: &Path,
) -> Result<PathBuf, RegistryError> {
    pull_with_progress(client, image, bundle_path, store_path, |_| {})
        .await
        .map(|report| report.path)
}

pub(crate) async fn pull_with_progress<F>(
    client: &mut RegistryClient,
    image: &ImageRef,
    bundle_path: &Path,
    store_path: &Path,
    mut progress: F,
) -> Result<ImagePullReport, RegistryError>
where
    F: FnMut(PullProgress),
{
    client.reset_byte_counter();
    progress(PullProgress::Resolving {
        image: image.to_string(),
    });
    let manifest = client.resolve_manifest(image).await?;
    let manifest_data = select_manifest(client, image, manifest).await?;
    validate_manifest_descriptors(&manifest_data)?;

    let total_layers = manifest_data.layers.len();
    progress(PullProgress::ManifestResolved {
        layers: total_layers,
        config_digest: manifest_data.config_digest.clone(),
    });
    progress(PullProgress::FetchingConfig {
        digest: manifest_data.config_digest.clone(),
    });
    let config_blob = client
        .fetch_blob(
            &image.registry,
            &image.repository,
            &manifest_data.config_digest,
        )
        .await?;
    verify_descriptor_bytes(
        &config_blob,
        &manifest_data.config_digest,
        manifest_data.config_size,
    )?;
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

    let layer_dirs = fetch_and_extract_layers(
        client,
        image,
        store_path,
        &cache_dir,
        &manifest_data,
        total_layers,
        &mut progress,
    )
    .await?;

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
        bytes_downloaded: client.bytes_downloaded(),
        layers: total_layers,
    })
}

async fn select_manifest(
    client: &mut RegistryClient,
    image: &ImageRef,
    manifest: ImageManifest,
) -> Result<SingleManifest, RegistryError> {
    match manifest {
        ImageManifest::Single(manifest) => Ok(manifest),
        ImageManifest::Index(index) => {
            if index.manifests.is_empty() {
                return Err(RegistryError::NoManifests);
            }
            let best = crate::select_manifest_for_current_target(&index).ok_or_else(|| {
                RegistryError::ParseError(format!(
                    "no manifest found for platform {}/{}",
                    crate::host_os(),
                    crate::host_arch()
                ))
            })?;
            validate_registry_digest("index.manifests[].digest", &best.digest)?;
            client
                .fetch_manifest_by_digest(&image.registry, &image.repository, &best.digest)
                .await
        }
    }
}

fn validate_manifest_descriptors(manifest: &SingleManifest) -> Result<(), RegistryError> {
    validate_registry_digest("manifest.config.digest", &manifest.config_digest)?;
    for (index, layer) in manifest.layers.iter().enumerate() {
        validate_registry_digest(format!("manifest.layers[{index}].digest"), &layer.digest)?;
    }
    Ok(())
}

fn validate_registry_digest(field: impl Into<String>, digest: &str) -> Result<(), RegistryError> {
    if validate_digest_reference(digest) {
        Ok(())
    } else {
        Err(RegistryError::ParseError(format!(
            "invalid digest in {}: {}",
            field.into(),
            digest
        )))
    }
}

async fn fetch_and_extract_layers<F>(
    client: &mut RegistryClient,
    image: &ImageRef,
    store_path: &Path,
    cache_dir: &Path,
    manifest: &SingleManifest,
    total_layers: usize,
    progress: &mut F,
) -> Result<Vec<PathBuf>, RegistryError>
where
    F: FnMut(PullProgress),
{
    let mut layer_dirs = Vec::new();
    for (offset, layer) in manifest.layers.iter().enumerate() {
        let index = offset + 1;
        let cache_key = layer.digest.replace(':', "_");
        let cached_layer = cache_dir.join(&cache_key);
        let cache_marker = cache_dir.join(format!("{cache_key}.complete"));

        if cached_layer.is_dir() && layer_cache_complete(&cache_marker, &layer.digest) {
            progress(PullProgress::LayerCached {
                index,
                total: total_layers,
                digest: layer.digest.clone(),
            });
            layer_dirs.push(cached_layer);
            continue;
        }
        if cached_layer.exists() {
            std::fs::remove_dir_all(&cached_layer).map_err(RegistryError::IoError)?;
        }
        let _ = std::fs::remove_file(&cache_marker);

        let blob_path = store_path.join(format!("{}.{}", &cache_key, layer_extension(layer)));
        if blob_path.exists() && !cached_blob_valid(&blob_path, &layer.digest, Some(layer.size)) {
            std::fs::remove_file(&blob_path).map_err(RegistryError::IoError)?;
        }
        if !blob_path.exists() {
            progress(PullProgress::LayerDownloading {
                index,
                total: total_layers,
                digest: layer.digest.clone(),
                size: layer.size,
            });
            let bytes = download_blob(
                client,
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

        verify_descriptor_file(&blob_path, &layer.digest, Some(layer.size))?;

        std::fs::create_dir_all(&cached_layer)?;
        progress(PullProgress::LayerExtracting {
            index,
            total: total_layers,
            digest: layer.digest.clone(),
        });
        extract_layer(&blob_path, &cached_layer, layer.media_type.as_deref())?;
        std::fs::write(&cache_marker, &layer.digest).map_err(RegistryError::IoError)?;
        progress(PullProgress::LayerExtracted {
            index,
            total: total_layers,
            digest: layer.digest.clone(),
        });
        layer_dirs.push(cached_layer);
    }
    Ok(layer_dirs)
}

fn cached_blob_valid(blob_path: &Path, digest: &str, expected_size: Option<u64>) -> bool {
    verify_descriptor_file(blob_path, digest, expected_size).is_ok()
}

fn verify_descriptor_file(
    path: &Path,
    expected_digest: &str,
    expected_size: Option<u64>,
) -> Result<(), RegistryError> {
    if let Some(size) = expected_size {
        let actual = std::fs::metadata(path)
            .map_err(RegistryError::IoError)?
            .len();
        verify_descriptor_size(expected_digest, size, actual)?;
    }

    verify_blob_digest(path, expected_digest)
}

fn verify_descriptor_bytes(
    data: &[u8],
    expected_digest: &str,
    expected_size: Option<u64>,
) -> Result<(), RegistryError> {
    if let Some(size) = expected_size {
        verify_descriptor_size(expected_digest, size, data.len() as u64)?;
    }

    let computed = sha256_digest_reference(data);
    if computed != expected_digest {
        return Err(RegistryError::DigestMismatch {
            expected: expected_digest.to_string(),
            computed,
        });
    }
    Ok(())
}

fn verify_descriptor_size(
    expected_digest: &str,
    expected_size: u64,
    actual_size: u64,
) -> Result<(), RegistryError> {
    if actual_size == expected_size {
        Ok(())
    } else {
        Err(RegistryError::DescriptorSizeMismatch {
            digest: expected_digest.to_string(),
            expected: expected_size,
            actual: actual_size,
        })
    }
}

fn layer_cache_complete(marker: &Path, digest: &str) -> bool {
    std::fs::read_to_string(marker)
        .map(|value| value.trim() == digest)
        .unwrap_or(false)
}

async fn download_blob(
    client: &mut RegistryClient,
    registry: &str,
    repository: &str,
    digest: &str,
    dest: &Path,
) -> Result<u64, RegistryError> {
    let body = client.fetch_blob(registry, repository, digest).await?;
    let bytes = body.len() as u64;

    atomic_write(dest, &body)?;
    Ok(bytes)
}

fn atomic_write(dest: &Path, data: &[u8]) -> Result<(), RegistryError> {
    let tmp = temporary_write_path(dest);
    if let Some(parent) = tmp.parent() {
        std::fs::create_dir_all(parent).map_err(RegistryError::IoError)?;
    }
    let write_result = (|| {
        let mut file = File::create(&tmp)?;
        file.write_all(data)?;
        file.sync_all()?;
        std::fs::rename(&tmp, dest)?;
        Ok::<(), std::io::Error>(())
    })();

    if let Err(error) = write_result {
        let _ = std::fs::remove_file(&tmp);
        return Err(RegistryError::IoError(error));
    }
    Ok(())
}

fn temporary_write_path(dest: &Path) -> PathBuf {
    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let file_name = dest
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("blob");
    dest.with_file_name(format!(
        ".{file_name}.tmp.{}.{}",
        std::process::id(),
        counter
    ))
}

fn layer_extension(layer: &LayerDescriptor) -> &'static str {
    match layer.media_type.as_deref() {
        Some(media_type) if media_type.contains("zstd") => "tar.zst",
        Some(media_type) if media_type.contains("gzip") => "tar.gz",
        Some(media_type) if media_type.contains("tar") => "tar",
        _ => "tar.gz",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_root() -> PathBuf {
        static C: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = C.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!("oci_pull_test_{}_{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn layer_cache_requires_matching_completion_marker() {
        let root = tmp_root();
        let marker = root.join("sha256_layer.complete");

        assert!(!layer_cache_complete(&marker, "sha256:layer"));

        std::fs::write(&marker, "sha256:other\n").unwrap();
        assert!(!layer_cache_complete(&marker, "sha256:layer"));

        std::fs::write(&marker, "sha256:layer\n").unwrap();
        assert!(layer_cache_complete(&marker, "sha256:layer"));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn cached_blob_valid_rejects_corrupt_cached_blob() {
        let root = tmp_root();
        let blob = root.join("layer.tar");
        let data = b"cached layer";
        let digest = crate::sha256_digest_reference(data);
        std::fs::write(&blob, data).unwrap();

        assert!(cached_blob_valid(&blob, &digest, Some(data.len() as u64)));

        std::fs::write(&blob, b"corrupt").unwrap();
        assert!(!cached_blob_valid(&blob, &digest, Some(data.len() as u64)));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn cached_blob_valid_rejects_size_mismatch() {
        let root = tmp_root();
        let blob = root.join("layer.tar");
        let data = b"cached layer";
        let digest = crate::sha256_digest_reference(data);
        std::fs::write(&blob, data).unwrap();

        assert!(!cached_blob_valid(
            &blob,
            &digest,
            Some(data.len() as u64 + 1)
        ));

        let _ = std::fs::remove_dir_all(root);
    }

    fn manifest_with_digests(config_digest: &str, layer_digest: &str) -> SingleManifest {
        SingleManifest {
            config_digest: config_digest.into(),
            config_size: Some(1),
            config_media_type: Some("application/vnd.oci.image.config.v1+json".into()),
            layers: vec![LayerDescriptor {
                media_type: Some("application/vnd.oci.image.layer.v1.tar".into()),
                digest: layer_digest.into(),
                size: 1,
            }],
        }
    }

    #[test]
    fn validate_manifest_descriptors_rejects_invalid_config_digest() {
        let layer = "sha256:1111111111111111111111111111111111111111111111111111111111111111";
        let manifest = manifest_with_digests("sha256:not-hex", layer);

        let error = validate_manifest_descriptors(&manifest).unwrap_err();

        assert!(matches!(error, RegistryError::ParseError(_)));
        assert!(error.to_string().contains("manifest.config.digest"));
    }

    #[test]
    fn validate_manifest_descriptors_rejects_invalid_layer_digest() {
        let config = "sha256:2222222222222222222222222222222222222222222222222222222222222222";
        let manifest = manifest_with_digests(config, "sha256:not-hex");

        let error = validate_manifest_descriptors(&manifest).unwrap_err();

        assert!(matches!(error, RegistryError::ParseError(_)));
        assert!(error.to_string().contains("manifest.layers[0].digest"));
    }

    #[test]
    fn verify_descriptor_bytes_checks_digest_and_size() {
        let data = b"config bytes";
        let digest = crate::sha256_digest_reference(data);

        assert!(verify_descriptor_bytes(data, &digest, Some(data.len() as u64)).is_ok());

        assert!(matches!(
            verify_descriptor_bytes(b"corrupt", &digest, Some(data.len() as u64)),
            Err(RegistryError::DescriptorSizeMismatch { .. })
        ));

        assert!(matches!(
            verify_descriptor_bytes(b"corrupt", &digest, Some(7)),
            Err(RegistryError::DigestMismatch { .. })
        ));
    }

    #[test]
    fn atomic_write_replaces_dest_and_cleans_temp_name() {
        let root = tmp_root();
        let dest = root.join("layer.tar.gz");
        std::fs::write(&dest, b"old").unwrap();

        atomic_write(&dest, b"new").unwrap();

        assert_eq!(std::fs::read(&dest).unwrap(), b"new");
        let temp_files = std::fs::read_dir(&root)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .map(|name| name.contains(".tmp."))
                    .unwrap_or(false)
            })
            .count();
        assert_eq!(temp_files, 0);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn layer_extension_distinguishes_uncompressed_tar() {
        let digest = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let uncompressed = LayerDescriptor {
            media_type: Some("application/vnd.oci.image.layer.v1.tar".into()),
            digest: digest.into(),
            size: 0,
        };
        let gzip = LayerDescriptor {
            media_type: Some("application/vnd.oci.image.layer.v1.tar+gzip".into()),
            digest: digest.into(),
            size: 0,
        };

        assert_eq!(layer_extension(&uncompressed), "tar");
        assert_eq!(layer_extension(&gzip), "tar.gz");
    }
}
