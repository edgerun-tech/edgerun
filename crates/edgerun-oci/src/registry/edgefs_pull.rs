use crate::prelude::*;

use super::client::RegistryClient;
use super::errors::RegistryError;
use super::image_ref::ImageRef;
use crate::image_apply::{
    apply_bare_image_layer_blob_sha256, validate_bare_image_layer_set, BareImageApplyReport,
};
use crate::layer_pipeline::{
    format_digest, sha256_layer_digest, validate_layer_descriptor, LayerApplyReport, LayerDigest,
};
use crate::tar_layer::{
    layer_compression, OciLayerCompression, TarLayerApplyReport, TarLayerSink,
    UncompressedTarStream,
};
use crate::BareImagePlan;
use edgerun_edgefs::EdgeFs;
use edgerun_http::{HttpClient, Request};
use edgerun_storage::BlockStorage;

#[derive(Debug, Clone)]
pub struct EdgeFsImagePullReport {
    pub plan: BareImagePlan,
    pub apply: BareImageApplyReport,
    pub bytes_downloaded: u64,
}

impl RegistryClient {
    /// Pull an image through `edgerun-http` and apply its rootfs layers into EdgeFS.
    ///
    /// The registry manifest/config and all layer blobs are fetched with the
    /// same authenticated registry path used by [`Self::fetch_bare_image_plan`].
    /// Layers are then validated, decompressed, whiteouts are applied, and final
    /// file contents are written into the provided encrypted EdgeFS instance.
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
}
