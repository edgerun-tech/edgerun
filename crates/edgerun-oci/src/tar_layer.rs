//! no_std parser for uncompressed OCI tar layers.

use crate::layer_pipeline::{
    apply_layer_chunks, LayerApplyReport, LayerDigest, LayerPipelineError, LayerSink,
};
use crate::prelude::*;
use crate::registry::manifest::LayerDescriptor;
use core::fmt;

const BLOCK_SIZE: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TarLayerError {
    TruncatedHeader,
    TruncatedEntry {
        path: String,
        expected: u64,
        actual: usize,
    },
    InvalidHeader(String),
    InvalidPath(String),
    UnsupportedEntry {
        path: String,
        kind: u8,
    },
    Sink(String),
}

impl fmt::Display for TarLayerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TruncatedHeader => f.write_str("truncated tar header"),
            Self::TruncatedEntry {
                path,
                expected,
                actual,
            } => write!(
                f,
                "truncated tar entry {path}: expected {expected} bytes, got {actual}"
            ),
            Self::InvalidHeader(error) => write!(f, "invalid tar header: {error}"),
            Self::InvalidPath(path) => write!(f, "unsafe tar path: {path}"),
            Self::UnsupportedEntry { path, kind } => {
                write!(f, "unsupported tar entry kind {kind:?} at {path}")
            }
            Self::Sink(error) => write!(f, "tar sink failed: {error}"),
        }
    }
}

impl core::error::Error for TarLayerError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TarLayerApplyError {
    Layer(LayerPipelineError),
    Tar(TarLayerError),
}

impl fmt::Display for TarLayerApplyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Layer(error) => write!(f, "invalid layer blob: {error}"),
            Self::Tar(error) => write!(f, "invalid tar layer: {error}"),
        }
    }
}

impl core::error::Error for TarLayerApplyError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TarLayerApplyReport {
    pub layer: LayerApplyReport,
    pub entries_applied: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TarEntryKind {
    Regular,
    Directory,
    Symlink,
    Hardlink,
    Character,
    Block,
    Fifo,
    PaxExtended,
    PaxGlobal,
    GnuLongName,
    GnuLongLink,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OciWhiteout {
    RemovePath(String),
    OpaqueDirectory(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TarEntry {
    pub path: String,
    pub kind: TarEntryKind,
    pub size: u64,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub link_name: Option<String>,
    pub whiteout: Option<OciWhiteout>,
}

pub trait TarLayerSink {
    fn apply_entry(&mut self, entry: &TarEntry, data: &[u8]) -> Result<(), String>;
}

pub fn apply_validated_uncompressed_tar_layer<'a, I, D, S>(
    descriptor: &LayerDescriptor,
    chunks: I,
    digest: D,
    sink: &mut S,
) -> Result<TarLayerApplyReport, TarLayerApplyError>
where
    I: IntoIterator<Item = &'a [u8]>,
    D: LayerDigest,
    S: TarLayerSink,
{
    let mut blob = LayerBytes::default();
    let layer = apply_layer_chunks(descriptor, chunks, digest, &mut blob)
        .map_err(TarLayerApplyError::Layer)?;
    let entries_applied =
        apply_uncompressed_tar_layer(&blob.bytes, sink).map_err(TarLayerApplyError::Tar)?;

    Ok(TarLayerApplyReport {
        layer,
        entries_applied,
    })
}

pub fn apply_uncompressed_tar_layer<S: TarLayerSink>(
    data: &[u8],
    sink: &mut S,
) -> Result<usize, TarLayerError> {
    let mut offset = 0usize;
    let mut applied = 0usize;

    while offset < data.len() {
        if data.len() - offset < BLOCK_SIZE {
            return Err(TarLayerError::TruncatedHeader);
        }

        let header = &data[offset..offset + BLOCK_SIZE];
        if is_zero_block(header) {
            break;
        }

        let entry = parse_header(header)?;
        if !path_safe(&entry.path) {
            return Err(TarLayerError::InvalidPath(entry.path));
        }
        if let Some(link_name) = entry.link_name.as_ref() {
            if !path_safe(link_name) {
                return Err(TarLayerError::InvalidPath(link_name.clone()));
            }
        }

        offset += BLOCK_SIZE;
        let size = entry.size as usize;
        if data.len() - offset < size {
            return Err(TarLayerError::TruncatedEntry {
                path: entry.path,
                expected: size as u64,
                actual: data.len().saturating_sub(offset),
            });
        }

        let payload = &data[offset..offset + size];
        match entry.kind {
            TarEntryKind::Regular
            | TarEntryKind::Directory
            | TarEntryKind::Symlink
            | TarEntryKind::Hardlink
            | TarEntryKind::Character
            | TarEntryKind::Block
            | TarEntryKind::Fifo => {
                sink.apply_entry(&entry, payload)
                    .map_err(TarLayerError::Sink)?;
                applied += 1;
            }
            TarEntryKind::PaxExtended
            | TarEntryKind::PaxGlobal
            | TarEntryKind::GnuLongName
            | TarEntryKind::GnuLongLink => {
                return Err(TarLayerError::UnsupportedEntry {
                    path: entry.path,
                    kind: header[156],
                });
            }
        }

        offset += round_up_to_block(size);
    }

    Ok(applied)
}

#[derive(Default)]
struct LayerBytes {
    bytes: Vec<u8>,
}

impl LayerSink for LayerBytes {
    fn write_chunk(&mut self, _descriptor: &LayerDescriptor, chunk: &[u8]) -> Result<(), String> {
        self.bytes.extend_from_slice(chunk);
        Ok(())
    }
}

fn parse_header(header: &[u8]) -> Result<TarEntry, TarLayerError> {
    verify_checksum(header)?;

    let name = parse_string(&header[0..100]);
    if name.is_empty() {
        return Err(TarLayerError::InvalidHeader("missing entry name".into()));
    }
    let prefix = parse_string(&header[345..500]);
    let path = if prefix.is_empty() {
        name
    } else {
        format!("{prefix}/{name}")
    };
    let kind = parse_kind(header[156], &path)?;
    let link_name = {
        let link = parse_string(&header[157..257]);
        (!link.is_empty()).then_some(link)
    };

    Ok(TarEntry {
        whiteout: parse_whiteout(&path),
        path,
        kind,
        size: parse_octal(&header[124..136])?,
        mode: parse_octal(&header[100..108])? as u32,
        uid: parse_octal(&header[108..116])? as u32,
        gid: parse_octal(&header[116..124])? as u32,
        link_name,
    })
}

fn parse_kind(kind: u8, path: &str) -> Result<TarEntryKind, TarLayerError> {
    match kind {
        0 | b'0' => Ok(TarEntryKind::Regular),
        b'1' => Ok(TarEntryKind::Hardlink),
        b'2' => Ok(TarEntryKind::Symlink),
        b'3' => Ok(TarEntryKind::Character),
        b'4' => Ok(TarEntryKind::Block),
        b'5' => Ok(TarEntryKind::Directory),
        b'6' => Ok(TarEntryKind::Fifo),
        b'x' => Ok(TarEntryKind::PaxExtended),
        b'g' => Ok(TarEntryKind::PaxGlobal),
        b'L' => Ok(TarEntryKind::GnuLongName),
        b'K' => Ok(TarEntryKind::GnuLongLink),
        other => Err(TarLayerError::UnsupportedEntry {
            path: path.into(),
            kind: other,
        }),
    }
}

fn parse_whiteout(path: &str) -> Option<OciWhiteout> {
    let (parent, name) = path.rsplit_once('/').unwrap_or(("", path));
    if name == ".wh..wh..opq" {
        return Some(OciWhiteout::OpaqueDirectory(parent.into()));
    }
    name.strip_prefix(".wh.").map(|target| {
        let target_path = if parent.is_empty() {
            target.into()
        } else {
            format!("{parent}/{target}")
        };
        OciWhiteout::RemovePath(target_path)
    })
}

fn parse_string(field: &[u8]) -> String {
    let end = field
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(field.len());
    let bytes = &field[..end];
    core::str::from_utf8(bytes).unwrap_or("").trim().into()
}

fn parse_octal(field: &[u8]) -> Result<u64, TarLayerError> {
    let mut value = 0u64;
    let mut saw_digit = false;
    for byte in field {
        match *byte {
            0 | b' ' => {
                if saw_digit {
                    break;
                }
            }
            b'0'..=b'7' => {
                saw_digit = true;
                value = value
                    .checked_mul(8)
                    .and_then(|value| value.checked_add(u64::from(byte - b'0')))
                    .ok_or_else(|| TarLayerError::InvalidHeader("octal field overflow".into()))?;
            }
            _ => return Err(TarLayerError::InvalidHeader("invalid octal field".into())),
        }
    }
    Ok(value)
}

fn verify_checksum(header: &[u8]) -> Result<(), TarLayerError> {
    let expected = parse_octal(&header[148..156])?;
    let actual: u64 = header
        .iter()
        .enumerate()
        .map(|(index, byte)| {
            if (148..156).contains(&index) {
                u64::from(b' ')
            } else {
                u64::from(*byte)
            }
        })
        .sum();
    if expected == actual {
        Ok(())
    } else {
        Err(TarLayerError::InvalidHeader(format!(
            "checksum mismatch: expected {expected}, got {actual}"
        )))
    }
}

fn path_safe(path: &str) -> bool {
    if path.is_empty() || path.starts_with('/') || path.contains('\0') {
        return false;
    }
    let mut depth = 0usize;
    for component in path.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                if depth == 0 {
                    return false;
                }
                depth -= 1;
            }
            _ => depth += 1,
        }
    }
    true
}

fn is_zero_block(block: &[u8]) -> bool {
    block.iter().all(|byte| *byte == 0)
}

fn round_up_to_block(size: usize) -> usize {
    let remainder = size % BLOCK_SIZE;
    if remainder == 0 {
        size
    } else {
        size + (BLOCK_SIZE - remainder)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    #[derive(Default)]
    struct CollectSink {
        entries: Vec<(TarEntry, Vec<u8>)>,
    }

    impl TarLayerSink for CollectSink {
        fn apply_entry(&mut self, entry: &TarEntry, data: &[u8]) -> Result<(), String> {
            self.entries.push((entry.clone(), data.to_vec()));
            Ok(())
        }
    }

    #[derive(Default)]
    struct TestDigest {
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

    fn tar_entry(path: &str, kind: u8, body: &[u8]) -> Vec<u8> {
        let mut header = [0u8; BLOCK_SIZE];
        write_field(&mut header[0..100], path.as_bytes());
        write_octal(&mut header[100..108], 0o644);
        write_octal(&mut header[108..116], 0);
        write_octal(&mut header[116..124], 0);
        write_octal(&mut header[124..136], body.len() as u64);
        write_octal(&mut header[136..148], 0);
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

    fn tar(entries: Vec<Vec<u8>>) -> Vec<u8> {
        let mut out = Vec::new();
        for entry in entries {
            out.extend_from_slice(&entry);
        }
        out.extend_from_slice(&[0u8; BLOCK_SIZE]);
        out.extend_from_slice(&[0u8; BLOCK_SIZE]);
        out
    }

    fn write_field(field: &mut [u8], value: &[u8]) {
        let len = value.len().min(field.len());
        field[..len].copy_from_slice(&value[..len]);
    }

    fn write_octal(field: &mut [u8], value: u64) {
        for byte in field.iter_mut() {
            *byte = 0;
        }
        let s = format!("{:0width$o}", value, width = field.len() - 1);
        write_field(field, s.as_bytes());
    }

    fn digest_for(bytes: &[u8]) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut digest = TestDigest::default();
        digest.update(bytes);
        let bytes = digest.finish();
        let mut out = String::from("sha256:");
        for byte in bytes {
            out.push(HEX[(byte >> 4) as usize] as char);
            out.push(HEX[(byte & 0x0f) as usize] as char);
        }
        out
    }

    #[test]
    fn parses_regular_file() {
        let data = tar(vec![tar_entry("etc/hosts", b'0', b"127.0.0.1\n")]);
        let mut sink = CollectSink::default();

        let count = apply_uncompressed_tar_layer(&data, &mut sink).unwrap();

        assert_eq!(count, 1);
        assert_eq!(sink.entries[0].0.path, "etc/hosts");
        assert_eq!(sink.entries[0].0.kind, TarEntryKind::Regular);
        assert_eq!(sink.entries[0].1, b"127.0.0.1\n");
    }

    #[test]
    fn validates_layer_before_applying_tar() {
        let data = tar(vec![tar_entry("bin/app", b'0', b"run")]);
        let descriptor = LayerDescriptor {
            media_type: Some("application/vnd.oci.image.layer.v1.tar".into()),
            digest: digest_for(&data),
            size: data.len() as u64,
        };
        let mut sink = CollectSink::default();
        let chunks: [&[u8]; 2] = [&data[..512], &data[512..]];

        let report = apply_validated_uncompressed_tar_layer(
            &descriptor,
            chunks,
            TestDigest::default(),
            &mut sink,
        )
        .unwrap();

        assert_eq!(report.entries_applied, 1);
        assert_eq!(report.layer.bytes_written, data.len() as u64);
        assert_eq!(sink.entries[0].0.path, "bin/app");
        assert_eq!(sink.entries[0].1, b"run");
    }

    #[test]
    fn detects_whiteouts() {
        let data = tar(vec![
            tar_entry("etc/.wh.shadow", b'0', b""),
            tar_entry("var/.wh..wh..opq", b'0', b""),
        ]);
        let mut sink = CollectSink::default();

        apply_uncompressed_tar_layer(&data, &mut sink).unwrap();

        assert_eq!(
            sink.entries[0].0.whiteout,
            Some(OciWhiteout::RemovePath("etc/shadow".into()))
        );
        assert_eq!(
            sink.entries[1].0.whiteout,
            Some(OciWhiteout::OpaqueDirectory("var".into()))
        );
    }

    #[test]
    fn rejects_parent_escape() {
        let data = tar(vec![tar_entry("../escape", b'0', b"bad")]);
        let mut sink = CollectSink::default();

        let error = apply_uncompressed_tar_layer(&data, &mut sink).unwrap_err();

        assert_eq!(error, TarLayerError::InvalidPath("../escape".into()));
    }

    #[test]
    fn rejects_bad_checksum() {
        let mut data = tar(vec![tar_entry("file", b'0', b"ok")]);
        data[0] = b'X';
        let mut sink = CollectSink::default();

        let error = apply_uncompressed_tar_layer(&data, &mut sink).unwrap_err();

        assert!(matches!(error, TarLayerError::InvalidHeader(_)));
    }
}
