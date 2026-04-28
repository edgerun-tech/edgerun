//! Local OCI bundle pull workflow.

use crate::prelude::*;
use std::path::{Path, PathBuf};

use super::client::RegistryClient;
use super::config::parse_image_config;
use super::errors::RegistryError;
use super::image_ref::ImageRef;
use super::layer::{apply_whiteouts, build_rootfs, extract_layer, verify_blob_digest};
use super::manifest::{ImageManifest, LayerDescriptor, SingleManifest};
use super::oci_spec::generate_oci_spec;

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
            client
                .fetch_manifest_by_digest(&image.registry, &image.repository, &best.digest)
                .await
        }
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

        verify_blob_digest(&blob_path, &layer.digest)?;

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

    std::fs::write(dest, &body).map_err(RegistryError::IoError)?;
    Ok(bytes)
}

fn layer_extension(layer: &LayerDescriptor) -> &'static str {
    match layer.media_type.as_deref() {
        Some(media_type) if media_type.contains("zstd") => "tar.zst",
        Some(media_type) if media_type.contains("gzip") || media_type.contains("tar") => "tar.gz",
        Some(media_type) if media_type.contains("oci") => "tar",
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
}
