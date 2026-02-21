// SPDX-License-Identifier: Apache-2.0
use anyhow::{Context, Result};

pub type Bundle = edgerun_types::BundlePayload;

pub fn decode_bundle_from_canonical_bytes(bytes: &[u8]) -> Result<Bundle> {
    edgerun_types::decode_bundle_payload_canonical(bytes)
        .context("bundle must be canonical CBOR payload bytes")
}
