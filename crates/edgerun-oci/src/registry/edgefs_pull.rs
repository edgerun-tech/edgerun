use crate::prelude::*;

use super::client::RegistryClient;
use super::errors::RegistryError;
use super::image_ref::ImageRef;
use crate::image_apply::{
    apply_bare_image_layer_blob_sha256, validate_bare_image_layer_set, BareImageApplyReport,
};
use crate::BareImagePlan;
use edgerun_edgefs::EdgeFs;
use edgerun_storage::BlockStorage;

#[derive(Debug, Clone)]
pub struct EdgeFsImagePullReport {
    pub plan: BareImagePlan,
    pub apply: BareImageApplyReport,
    pub bytes_downloaded: u64,
}

impl RegistryClient {
    /// Pull an image through the node HTTP client and apply its rootfs layers into EdgeFS.
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
            let blob = self
                .fetch_blob(&image.registry, &image.repository, &layer.digest)
                .await?;
            let (report, entries) = apply_bare_image_layer_blob_sha256(&plan, index, &blob, fs)
                .map_err(|error| RegistryError::ParseError(error.to_string()))?;
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
}
