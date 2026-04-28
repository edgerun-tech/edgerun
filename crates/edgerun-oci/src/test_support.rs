use crate::image_plan::BareImagePlan;
use crate::layer_pipeline::{format_digest, LayerDigest};
use crate::prelude::*;
use crate::registry::manifest::LayerDescriptor;
use crate::runtime_config::BareRuntimeConfig;
use crate::spec::{OciProcess, OciRoot, OciSpec};

pub const TEST_TAR_BLOCK_SIZE: usize = 512;

#[derive(Default)]
pub struct TestDigest {
    bytes: Vec<u8>,
}

impl LayerDigest for TestDigest {
    fn algorithm(&self) -> &'static str {
        "sha256"
    }

    fn update(&mut self, chunk: &[u8]) {
        self.bytes.extend_from_slice(chunk);
    }

    fn finish(self) -> Vec<u8> {
        let mut out = [0u8; 32];
        for (index, byte) in self.bytes.iter().enumerate() {
            out[index % 32] = out[index % 32].wrapping_add(*byte);
        }
        out.to_vec()
    }
}

pub fn test_digest_for(bytes: &[u8]) -> String {
    let mut digest = TestDigest::default();
    digest.update(bytes);
    format_digest(digest.algorithm(), &digest.finish())
}

pub fn real_sha256_digest_for(bytes: &[u8]) -> String {
    format_digest("sha256", &edgerun_crypto::sha256(bytes))
}

pub fn layer_descriptor(data: &[u8]) -> LayerDescriptor {
    layer_descriptor_with_media_type(data, "application/vnd.oci.image.layer.v1.tar")
}

pub fn real_sha256_layer_descriptor(data: &[u8]) -> LayerDescriptor {
    LayerDescriptor {
        media_type: Some("application/vnd.oci.image.layer.v1.tar".into()),
        digest: real_sha256_digest_for(data),
        size: data.len() as u64,
    }
}

pub fn layer_descriptor_with_media_type(data: &[u8], media_type: &str) -> LayerDescriptor {
    LayerDescriptor {
        media_type: Some(media_type.into()),
        digest: test_digest_for(data),
        size: data.len() as u64,
    }
}

pub fn bare_image_plan_for_layers(layers: Vec<LayerDescriptor>) -> BareImagePlan {
    bare_image_plan_for_layers_and_diff_ids(layers, Vec::new())
}

pub fn bare_image_plan_for_layers_and_diff_ids(
    layers: Vec<LayerDescriptor>,
    diff_ids: Vec<String>,
) -> BareImagePlan {
    BareImagePlan {
        config_digest: "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
            .into(),
        layers,
        diff_ids,
        image_os: Some("linux".into()),
        image_arch: Some("amd64".into()),
        runtime: BareRuntimeConfig::from_spec(&OciSpec {
            version: "1.0.2".into(),
            platform: None,
            process: Some(OciProcess {
                args: Some(vec!["/init".into()]),
                ..Default::default()
            }),
            root: Some(OciRoot {
                path: "/rootfs".into(),
                readonly: Some(false),
            }),
            hostname: None,
            domainname: None,
            linux: None,
            mounts: None,
            annotations: None,
        })
        .unwrap(),
    }
}

pub fn tar_entry(path: &str, kind: u8, body: &[u8]) -> Vec<u8> {
    tar_entry_with_mtime(path, kind, body, 0)
}

pub fn tar_entry_with_mtime(path: &str, kind: u8, body: &[u8], mtime: u64) -> Vec<u8> {
    let mut header = [0u8; TEST_TAR_BLOCK_SIZE];
    write_field(&mut header[0..100], path.as_bytes());
    write_octal(&mut header[100..108], 0o644);
    write_octal(&mut header[108..116], 0);
    write_octal(&mut header[116..124], 0);
    write_octal(&mut header[124..136], body.len() as u64);
    write_octal(&mut header[136..148], mtime);
    for byte in &mut header[148..156] {
        *byte = b' ';
    }
    header[156] = kind;
    write_field(&mut header[257..263], b"ustar");
    write_field(&mut header[263..265], b"00");
    let checksum: u64 = header.iter().map(|byte| u64::from(*byte)).sum();
    write_octal(&mut header[148..156], checksum);

    let mut out = header.to_vec();
    out.extend_from_slice(body);
    out.resize(out.len() + (round_up_to_block(body.len()) - body.len()), 0);
    out
}

pub fn tar_device_entry(path: &str, kind: u8, major: u32, minor: u32) -> Vec<u8> {
    let mut entry = tar_entry(path, kind, &[]);
    for byte in &mut entry[148..156] {
        *byte = b' ';
    }
    write_octal(&mut entry[329..337], major.into());
    write_octal(&mut entry[337..345], minor.into());
    let checksum: u64 = entry[..TEST_TAR_BLOCK_SIZE]
        .iter()
        .map(|byte| u64::from(*byte))
        .sum();
    write_octal(&mut entry[148..156], checksum);
    entry
}

pub fn tar(entries: Vec<Vec<u8>>) -> Vec<u8> {
    let mut out = Vec::new();
    for entry in entries {
        out.extend_from_slice(&entry);
    }
    out.extend_from_slice(&[0u8; TEST_TAR_BLOCK_SIZE]);
    out.extend_from_slice(&[0u8; TEST_TAR_BLOCK_SIZE]);
    out
}

pub fn write_field(field: &mut [u8], value: &[u8]) {
    let len = value.len().min(field.len());
    field[..len].copy_from_slice(&value[..len]);
}

pub fn write_octal(field: &mut [u8], value: u64) {
    for byte in field.iter_mut() {
        *byte = 0;
    }
    let s = format!("{:0width$o}", value, width = field.len() - 1);
    write_field(field, s.as_bytes());
}

pub fn round_up_to_block(size: usize) -> usize {
    let remainder = size % TEST_TAR_BLOCK_SIZE;
    if remainder == 0 {
        size
    } else {
        size + (TEST_TAR_BLOCK_SIZE - remainder)
    }
}
