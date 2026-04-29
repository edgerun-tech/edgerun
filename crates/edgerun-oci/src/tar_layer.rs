//! no_std parser for uncompressed OCI tar layers.

use crate::layer_pipeline::{
    apply_layer_chunks, sha256_layer_digest, LayerApplyReport, LayerDigest, LayerPipelineError,
    LayerSink,
};
use crate::oci_path::{layer_path_safe, normalize_layer_path};
use crate::prelude::*;
use crate::registry::manifest::LayerDescriptor;
pub use crate::tar_compression::{
    decompress_gzip_layer, decompress_zstd_layer, layer_compression, OciLayerCompression,
};
pub use crate::tar_whiteout::{parse_oci_whiteout, OciWhiteout};
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
    Decompress(String),
    UnsupportedMediaType(Option<String>),
}

impl fmt::Display for TarLayerApplyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Layer(error) => write!(f, "invalid layer blob: {error}"),
            Self::Tar(error) => write!(f, "invalid tar layer: {error}"),
            Self::Decompress(error) => write!(f, "layer decompression failed: {error}"),
            Self::UnsupportedMediaType(Some(media_type)) => {
                write!(f, "unsupported tar layer media type: {media_type}")
            }
            Self::UnsupportedMediaType(None) => f.write_str("missing tar layer media type"),
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
pub struct DecodedTarLayer {
    pub layer: LayerApplyReport,
    pub compression: OciLayerCompression,
    pub tar_bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
pub struct TarEntry {
    pub path: String,
    pub kind: TarEntryKind,
    pub size: u64,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub mtime: u64,
    pub dev_major: Option<u32>,
    pub dev_minor: Option<u32>,
    pub link_name: Option<String>,
    pub whiteout: Option<OciWhiteout>,
}

pub trait TarLayerSink {
    fn apply_entry(&mut self, entry: &TarEntry, data: &[u8]) -> Result<(), String>;
}

pub fn apply_validated_tar_layer<'a, I, D, S>(
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
    let decoded = validate_and_decode_tar_layer(descriptor, chunks, digest)?;

    let entries_applied =
        apply_uncompressed_tar_layer(&decoded.tar_bytes, sink).map_err(TarLayerApplyError::Tar)?;

    Ok(TarLayerApplyReport {
        layer: decoded.layer,
        entries_applied,
    })
}

pub fn apply_validated_tar_layer_sha256<'a, I, S>(
    descriptor: &LayerDescriptor,
    chunks: I,
    sink: &mut S,
) -> Result<TarLayerApplyReport, TarLayerApplyError>
where
    I: IntoIterator<Item = &'a [u8]>,
    S: TarLayerSink,
{
    apply_validated_tar_layer(descriptor, chunks, sha256_layer_digest(), sink)
}

pub fn validate_and_decode_tar_layer<'a, I, D>(
    descriptor: &LayerDescriptor,
    chunks: I,
    digest: D,
) -> Result<DecodedTarLayer, TarLayerApplyError>
where
    I: IntoIterator<Item = &'a [u8]>,
    D: LayerDigest,
{
    let mut blob = LayerBytes::default();
    let layer = apply_layer_chunks(descriptor, chunks, digest, &mut blob)
        .map_err(TarLayerApplyError::Layer)?;

    let compression = layer_compression(descriptor.media_type.as_deref());
    let tar_bytes = match compression {
        OciLayerCompression::Uncompressed => blob.bytes,
        OciLayerCompression::Gzip => decompress_gzip_layer(&blob.bytes)?,
        OciLayerCompression::Zstd => decompress_zstd_layer(&blob.bytes)?,
        OciLayerCompression::Unknown => {
            return Err(TarLayerApplyError::UnsupportedMediaType(
                descriptor.media_type.clone(),
            ));
        }
    };

    Ok(DecodedTarLayer {
        layer,
        compression,
        tar_bytes,
    })
}

pub fn validate_and_decode_tar_layer_sha256<'a, I>(
    descriptor: &LayerDescriptor,
    chunks: I,
) -> Result<DecodedTarLayer, TarLayerApplyError>
where
    I: IntoIterator<Item = &'a [u8]>,
{
    validate_and_decode_tar_layer(descriptor, chunks, sha256_layer_digest())
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
    if layer_compression(descriptor.media_type.as_deref()) != OciLayerCompression::Uncompressed {
        return Err(TarLayerApplyError::UnsupportedMediaType(
            descriptor.media_type.clone(),
        ));
    }

    apply_validated_tar_layer(descriptor, chunks, digest, sink)
}

pub fn apply_uncompressed_tar_layer<S: TarLayerSink>(
    data: &[u8],
    sink: &mut S,
) -> Result<usize, TarLayerError> {
    let mut offset = 0usize;
    let mut applied = 0usize;
    let mut global = TarEntryOverrides::default();
    let mut pending = TarEntryOverrides::default();

    while offset < data.len() {
        if data.len() - offset < BLOCK_SIZE {
            return Err(TarLayerError::TruncatedHeader);
        }

        let header = &data[offset..offset + BLOCK_SIZE];
        if is_zero_block(header) {
            break;
        }

        let mut entry = parse_header(header)?;

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
            TarEntryKind::PaxExtended => {
                pending.merge(parse_pax_overrides(payload)?);
            }
            TarEntryKind::PaxGlobal => {
                global.merge(parse_pax_overrides(payload)?);
            }
            TarEntryKind::GnuLongName => {
                pending.path = Some(parse_long_name(payload)?);
            }
            TarEntryKind::GnuLongLink => {
                pending.link_name = Some(parse_long_name(payload)?);
            }
            TarEntryKind::Regular
            | TarEntryKind::Directory
            | TarEntryKind::Symlink
            | TarEntryKind::Hardlink
            | TarEntryKind::Character
            | TarEntryKind::Block
            | TarEntryKind::Fifo => {
                apply_overrides(&mut entry, &global, &pending);
                pending = TarEntryOverrides::default();
                validate_entry_paths(&entry)?;
                sink.apply_entry(&entry, payload)
                    .map_err(TarLayerError::Sink)?;
                applied += 1;
            }
        }

        offset += round_up_to_block(size);
    }

    Ok(applied)
}

pub struct UncompressedTarStream<'a, S: TarLayerSink> {
    sink: &'a mut S,
    buffer: Vec<u8>,
    applied: usize,
    global: TarEntryOverrides,
    pending: TarEntryOverrides,
    finished: bool,
}

impl<'a, S: TarLayerSink> UncompressedTarStream<'a, S> {
    pub fn new(sink: &'a mut S) -> Self {
        Self {
            sink,
            buffer: Vec::new(),
            applied: 0,
            global: TarEntryOverrides::default(),
            pending: TarEntryOverrides::default(),
            finished: false,
        }
    }

    pub fn push(&mut self, chunk: &[u8]) -> Result<usize, TarLayerError> {
        if self.finished {
            if chunk.iter().any(|byte| *byte != 0) {
                return Err(TarLayerError::InvalidHeader(
                    "non-zero data after end-of-archive".into(),
                ));
            }
            return Ok(self.applied);
        }
        self.buffer.extend_from_slice(chunk);
        self.drain_ready_entries()?;
        Ok(self.applied)
    }

    pub fn finish(mut self) -> Result<usize, TarLayerError> {
        self.drain_ready_entries()?;
        if self.finished || self.buffer.is_empty() {
            return Ok(self.applied);
        }
        if self.buffer.len() < BLOCK_SIZE {
            return Err(TarLayerError::TruncatedHeader);
        }
        Err(TarLayerError::TruncatedHeader)
    }

    fn drain_ready_entries(&mut self) -> Result<(), TarLayerError> {
        loop {
            if self.buffer.len() < BLOCK_SIZE {
                return Ok(());
            }

            let header = &self.buffer[..BLOCK_SIZE];
            if is_zero_block(header) {
                self.buffer.drain(..BLOCK_SIZE);
                self.finished = true;
                if self.buffer.iter().any(|byte| *byte != 0) {
                    return Err(TarLayerError::InvalidHeader(
                        "non-zero data after end-of-archive".into(),
                    ));
                }
                self.buffer.clear();
                return Ok(());
            }

            let mut entry = parse_header(header)?;
            let size = entry.size as usize;
            let padded_size = round_up_to_block(size);
            let needed = BLOCK_SIZE
                .checked_add(padded_size)
                .ok_or_else(|| TarLayerError::InvalidHeader("tar entry size overflow".into()))?;
            if self.buffer.len() < needed {
                return Ok(());
            }

            let payload = &self.buffer[BLOCK_SIZE..BLOCK_SIZE + size];
            apply_parsed_entry(
                self.sink,
                &mut entry,
                payload,
                &mut self.global,
                &mut self.pending,
                &mut self.applied,
            )?;
            self.buffer.drain(..needed);
        }
    }
}

pub fn apply_uncompressed_tar_layer_streaming<'a, I, S>(
    chunks: I,
    sink: &mut S,
) -> Result<usize, TarLayerError>
where
    I: IntoIterator<Item = &'a [u8]>,
    S: TarLayerSink,
{
    let mut stream = UncompressedTarStream::new(sink);
    for chunk in chunks {
        stream.push(chunk)?;
    }
    stream.finish()
}

fn apply_parsed_entry<S: TarLayerSink>(
    sink: &mut S,
    entry: &mut TarEntry,
    payload: &[u8],
    global: &mut TarEntryOverrides,
    pending: &mut TarEntryOverrides,
    applied: &mut usize,
) -> Result<(), TarLayerError> {
    match entry.kind {
        TarEntryKind::PaxExtended => {
            pending.merge(parse_pax_overrides(payload)?);
        }
        TarEntryKind::PaxGlobal => {
            global.merge(parse_pax_overrides(payload)?);
        }
        TarEntryKind::GnuLongName => {
            pending.path = Some(parse_long_name(payload)?);
        }
        TarEntryKind::GnuLongLink => {
            pending.link_name = Some(parse_long_name(payload)?);
        }
        TarEntryKind::Regular
        | TarEntryKind::Directory
        | TarEntryKind::Symlink
        | TarEntryKind::Hardlink
        | TarEntryKind::Character
        | TarEntryKind::Block
        | TarEntryKind::Fifo => {
            apply_overrides(entry, global, pending);
            *pending = TarEntryOverrides::default();
            validate_entry_paths(entry)?;
            sink.apply_entry(entry, payload)
                .map_err(TarLayerError::Sink)?;
            *applied = applied.saturating_add(1);
        }
    }
    Ok(())
}

#[derive(Default)]
struct TarEntryOverrides {
    path: Option<String>,
    link_name: Option<String>,
}

impl TarEntryOverrides {
    fn merge(&mut self, other: Self) {
        if other.path.is_some() {
            self.path = other.path;
        }
        if other.link_name.is_some() {
            self.link_name = other.link_name;
        }
    }
}

fn apply_overrides(entry: &mut TarEntry, global: &TarEntryOverrides, pending: &TarEntryOverrides) {
    if let Some(path) = global.path.as_ref() {
        entry.path = path.clone();
    }
    if let Some(link_name) = global.link_name.as_ref() {
        entry.link_name = Some(link_name.clone());
    }
    if let Some(path) = pending.path.as_ref() {
        entry.path = path.clone();
    }
    if let Some(link_name) = pending.link_name.as_ref() {
        entry.link_name = Some(link_name.clone());
    }
    entry.path = normalize_layer_path(&entry.path);
    if matches!(entry.kind, TarEntryKind::Hardlink) {
        if let Some(link_name) = entry.link_name.as_mut() {
            *link_name = normalize_layer_path(link_name);
        }
    }
    entry.whiteout = parse_oci_whiteout(&entry.path);
}

fn validate_entry_paths(entry: &TarEntry) -> Result<(), TarLayerError> {
    if !layer_path_safe(&entry.path) {
        return Err(TarLayerError::InvalidPath(entry.path.clone()));
    }
    if let Some(link_name) = entry.link_name.as_ref() {
        match entry.kind {
            TarEntryKind::Symlink => {
                if !link_target_safe(&entry.path, link_name) {
                    return Err(TarLayerError::InvalidPath(link_name.clone()));
                }
            }
            _ if !layer_path_safe(link_name) => {
                return Err(TarLayerError::InvalidPath(link_name.clone()));
            }
            _ => {}
        }
    }
    Ok(())
}

fn link_target_safe(entry_path: &str, link_name: &str) -> bool {
    let resolved = if link_name.starts_with('/') {
        normalize_layer_path(link_name)
    } else {
        let parent = entry_path
            .rsplit_once('/')
            .map(|(parent, _)| parent)
            .unwrap_or("");
        if parent.is_empty() {
            normalize_layer_path(link_name)
        } else {
            normalize_layer_path(&format!("{parent}/{link_name}"))
        }
    };
    !resolved.is_empty() && layer_path_safe(&resolved)
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
        whiteout: parse_oci_whiteout(&path),
        path,
        kind,
        size: parse_octal(&header[124..136])?,
        mode: parse_octal(&header[100..108])? as u32,
        uid: parse_octal(&header[108..116])? as u32,
        gid: parse_octal(&header[116..124])? as u32,
        mtime: parse_octal(&header[136..148])?,
        dev_major: matches!(kind, TarEntryKind::Character | TarEntryKind::Block)
            .then(|| parse_octal(&header[329..337]).map(|value| value as u32))
            .transpose()?,
        dev_minor: matches!(kind, TarEntryKind::Character | TarEntryKind::Block)
            .then(|| parse_octal(&header[337..345]).map(|value| value as u32))
            .transpose()?,
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

fn parse_pax_overrides(payload: &[u8]) -> Result<TarEntryOverrides, TarLayerError> {
    let mut offset = 0usize;
    let mut overrides = TarEntryOverrides::default();

    while offset < payload.len() {
        let Some(space_offset) = payload[offset..].iter().position(|byte| *byte == b' ') else {
            return Err(TarLayerError::InvalidHeader(
                "invalid pax record length".into(),
            ));
        };
        let length_bytes = &payload[offset..offset + space_offset];
        let record_len = parse_decimal(length_bytes)?;
        if record_len <= space_offset + 1 || offset + record_len > payload.len() {
            return Err(TarLayerError::InvalidHeader(
                "invalid pax record size".into(),
            ));
        }

        let record = &payload[offset + space_offset + 1..offset + record_len];
        let record = record.strip_suffix(b"\n").unwrap_or(record);
        if let Some(eq_offset) = record.iter().position(|byte| *byte == b'=') {
            let key = core::str::from_utf8(&record[..eq_offset])
                .map_err(|_| TarLayerError::InvalidHeader("invalid pax key".into()))?;
            let value = core::str::from_utf8(&record[eq_offset + 1..])
                .map_err(|_| TarLayerError::InvalidHeader("invalid pax value".into()))?;
            match key {
                "path" => overrides.path = Some(value.into()),
                "linkpath" => overrides.link_name = Some(value.into()),
                _ => {}
            }
        }

        offset += record_len;
    }

    Ok(overrides)
}

fn parse_decimal(bytes: &[u8]) -> Result<usize, TarLayerError> {
    let mut value = 0usize;
    if bytes.is_empty() {
        return Err(TarLayerError::InvalidHeader("missing decimal value".into()));
    }
    for byte in bytes {
        match *byte {
            b'0'..=b'9' => {
                value = value
                    .checked_mul(10)
                    .and_then(|value| value.checked_add(usize::from(byte - b'0')))
                    .ok_or_else(|| TarLayerError::InvalidHeader("decimal field overflow".into()))?;
            }
            _ => return Err(TarLayerError::InvalidHeader("invalid decimal value".into())),
        }
    }
    Ok(value)
}

fn parse_long_name(payload: &[u8]) -> Result<String, TarLayerError> {
    let end = payload
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(payload.len());
    core::str::from_utf8(&payload[..end])
        .map(String::from)
        .map_err(|_| TarLayerError::InvalidHeader("invalid GNU long name".into()))
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

#[cfg(all(test, not(target_os = "none")))]
#[path = "../tests/unit_src/src/tar_layer_tests.rs"]
mod tests;
