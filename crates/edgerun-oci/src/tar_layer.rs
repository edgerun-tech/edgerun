//! no_std parser for uncompressed OCI tar layers.

use crate::layer_pipeline::{
    apply_layer_chunks, sha256_layer_digest, LayerApplyReport, LayerDigest, LayerPipelineError,
    LayerSink,
};
use crate::oci_path::{layer_path_safe, normalize_layer_path};
use crate::prelude::*;
use crate::registry::manifest::LayerDescriptor;
use core::fmt;
use edgerun_encoding::crc32::crc32;

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
pub enum OciLayerCompression {
    Uncompressed,
    Gzip,
    Zstd,
    Unknown,
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

pub fn layer_compression(media_type: Option<&str>) -> OciLayerCompression {
    match media_type {
        Some(
            "application/vnd.oci.image.layer.v1.tar"
            | "application/vnd.oci.image.layer.nondistributable.v1.tar"
            | "application/vnd.docker.image.rootfs.diff.tar",
        ) => OciLayerCompression::Uncompressed,
        Some(
            "application/vnd.oci.image.layer.v1.tar+gzip"
            | "application/vnd.oci.image.layer.nondistributable.v1.tar+gzip"
            | "application/vnd.docker.image.rootfs.diff.tar.gzip",
        ) => OciLayerCompression::Gzip,
        Some(
            "application/vnd.oci.image.layer.v1.tar+zstd"
            | "application/vnd.oci.image.layer.nondistributable.v1.tar+zstd",
        ) => OciLayerCompression::Zstd,
        _ => OciLayerCompression::Unknown,
    }
}

pub fn decompress_gzip_layer(data: &[u8]) -> Result<Vec<u8>, TarLayerApplyError> {
    if data.len() < 18 || data[0] != 0x1f || data[1] != 0x8b || data[2] != 8 {
        return Err(TarLayerApplyError::Decompress("invalid gzip header".into()));
    }

    let flags = data[3];
    if flags & 0xe0 != 0 {
        return Err(TarLayerApplyError::Decompress(
            "reserved gzip flags are set".into(),
        ));
    }

    let mut offset = 10usize;
    if flags & 0x04 != 0 {
        if offset + 2 > data.len() {
            return Err(TarLayerApplyError::Decompress(
                "truncated gzip extra field".into(),
            ));
        }
        let extra_len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
        offset = offset
            .checked_add(2 + extra_len)
            .ok_or_else(|| TarLayerApplyError::Decompress("gzip extra field overflow".into()))?;
    }
    if flags & 0x08 != 0 {
        offset = skip_gzip_zero_terminated(data, offset, "name")?;
    }
    if flags & 0x10 != 0 {
        offset = skip_gzip_zero_terminated(data, offset, "comment")?;
    }
    if flags & 0x02 != 0 {
        offset = offset
            .checked_add(2)
            .ok_or_else(|| TarLayerApplyError::Decompress("gzip header crc overflow".into()))?;
    }
    if offset + 8 > data.len() {
        return Err(TarLayerApplyError::Decompress("truncated gzip body".into()));
    }

    let footer = data.len() - 8;
    let out = miniz_oxide::inflate::decompress_to_vec(&data[offset..footer])
        .map_err(|_| TarLayerApplyError::Decompress("invalid deflate stream".into()))?;
    let expected_crc = u32::from_le_bytes([
        data[footer],
        data[footer + 1],
        data[footer + 2],
        data[footer + 3],
    ]);
    let expected_len = u32::from_le_bytes([
        data[footer + 4],
        data[footer + 5],
        data[footer + 6],
        data[footer + 7],
    ]);

    if expected_crc != crc32(&out) {
        return Err(TarLayerApplyError::Decompress(
            "gzip payload crc mismatch".into(),
        ));
    }
    if expected_len != out.len() as u32 {
        return Err(TarLayerApplyError::Decompress(
            "gzip payload size mismatch".into(),
        ));
    }

    Ok(out)
}

#[cfg(feature = "zstd")]
pub fn decompress_zstd_layer(data: &[u8]) -> Result<Vec<u8>, TarLayerApplyError> {
    use ruzstd::decoding::StreamingDecoder;
    use ruzstd::io::Read;

    let mut decoder = StreamingDecoder::new(data)
        .map_err(|error| TarLayerApplyError::Decompress(format!("invalid zstd frame: {error}")))?;
    let mut out = Vec::new();
    decoder
        .read_to_end(&mut out)
        .map_err(|error| TarLayerApplyError::Decompress(format!("invalid zstd stream: {error}")))?;
    Ok(out)
}

#[cfg(not(feature = "zstd"))]
pub fn decompress_zstd_layer(_data: &[u8]) -> Result<Vec<u8>, TarLayerApplyError> {
    Err(TarLayerApplyError::UnsupportedMediaType(Some(
        "application/vnd.oci.image.layer.v1.tar+zstd".into(),
    )))
}

fn skip_gzip_zero_terminated(
    data: &[u8],
    offset: usize,
    field: &str,
) -> Result<usize, TarLayerApplyError> {
    data[offset..]
        .iter()
        .position(|byte| *byte == 0)
        .and_then(|relative| offset.checked_add(relative + 1))
        .ok_or_else(|| TarLayerApplyError::Decompress(format!("truncated gzip {field} field")))
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

pub fn parse_oci_whiteout(path: &str) -> Option<OciWhiteout> {
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
mod tests {
    use super::*;
    use crate::test_support::{
        real_sha256_digest_for, tar, tar_device_entry, tar_entry, tar_entry_with_mtime,
        test_digest_for, TestDigest, TEST_TAR_BLOCK_SIZE,
    };
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

    fn pax_record(key: &str, value: &str) -> Vec<u8> {
        let body = format!("{key}={value}\n");
        let mut len = body.len() + 2;
        loop {
            let next = body.len() + len.to_string().len() + 1;
            if next == len {
                break format!("{len} {body}").into_bytes();
            }
            len = next;
        }
    }

    fn gzip(bytes: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&[0x1f, 0x8b, 0x08, 0x00, 0, 0, 0, 0, 0x00, 0xff]);
        out.extend_from_slice(&miniz_oxide::deflate::compress_to_vec(bytes, 6));
        out.extend_from_slice(&crc32(bytes).to_le_bytes());
        out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        out
    }

    #[cfg(feature = "zstd")]
    fn zstd(bytes: &[u8]) -> Vec<u8> {
        ruzstd::encoding::compress_to_vec(bytes, ruzstd::encoding::CompressionLevel::Fastest)
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
    fn parses_device_major_minor() {
        let data = tar(vec![tar_device_entry("dev/null", b'3', 1, 3)]);
        let mut sink = CollectSink::default();

        let count = apply_uncompressed_tar_layer(&data, &mut sink).unwrap();

        assert_eq!(count, 1);
        assert_eq!(sink.entries[0].0.kind, TarEntryKind::Character);
        assert_eq!(sink.entries[0].0.dev_major, Some(1));
        assert_eq!(sink.entries[0].0.dev_minor, Some(3));
    }

    #[test]
    fn parses_mtime() {
        let data = tar(vec![tar_entry_with_mtime(
            "etc/hosts",
            b'0',
            b"127.0.0.1\n",
            1_700_000_000,
        )]);
        let mut sink = CollectSink::default();

        let count = apply_uncompressed_tar_layer(&data, &mut sink).unwrap();

        assert_eq!(count, 1);
        assert_eq!(sink.entries[0].0.mtime, 1_700_000_000);
    }

    #[test]
    fn validates_layer_before_applying_tar() {
        let data = tar(vec![tar_entry("bin/app", b'0', b"run")]);
        let descriptor = LayerDescriptor {
            media_type: Some("application/vnd.oci.image.layer.v1.tar".into()),
            digest: test_digest_for(&data),
            size: data.len() as u64,
        };
        let mut sink = CollectSink::default();
        let chunks: [&[u8]; 2] = [&data[..TEST_TAR_BLOCK_SIZE], &data[TEST_TAR_BLOCK_SIZE..]];

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
    fn streams_uncompressed_tar_across_arbitrary_chunks() {
        let data = tar(vec![
            tar_entry("etc/hostname", b'0', b"edge"),
            tar_entry("bin/app", b'0', b"run"),
        ]);
        let chunks: Vec<&[u8]> = data.chunks(137).collect();
        let mut sink = CollectSink::default();

        let count = apply_uncompressed_tar_layer_streaming(chunks, &mut sink).unwrap();

        assert_eq!(count, 2);
        assert_eq!(sink.entries[0].0.path, "etc/hostname");
        assert_eq!(sink.entries[0].1, b"edge");
        assert_eq!(sink.entries[1].0.path, "bin/app");
        assert_eq!(sink.entries[1].1, b"run");
    }

    #[test]
    fn streaming_tar_preserves_pax_overrides() {
        let mut pax = Vec::new();
        pax.extend_from_slice(&pax_record("path", "very/long/path/from/pax"));
        let data = tar(vec![
            tar_entry("pax", b'x', &pax),
            tar_entry("short", b'0', b"body"),
        ]);
        let mut stream_sink = CollectSink::default();
        let chunks: Vec<&[u8]> = data.chunks(211).collect();

        let count = apply_uncompressed_tar_layer_streaming(chunks, &mut stream_sink).unwrap();

        assert_eq!(count, 1);
        assert_eq!(stream_sink.entries[0].0.path, "very/long/path/from/pax");
        assert_eq!(stream_sink.entries[0].1, b"body");
    }

    #[test]
    fn streaming_tar_rejects_truncated_header() {
        let data = vec![0u8; TEST_TAR_BLOCK_SIZE - 1];
        let mut sink = CollectSink::default();

        let error =
            apply_uncompressed_tar_layer_streaming([data.as_slice()], &mut sink).unwrap_err();

        assert_eq!(error, TarLayerError::TruncatedHeader);
    }

    #[test]
    fn validates_layer_with_builtin_sha256_digest() {
        let data = tar(vec![tar_entry("bin/app", b'0', b"run")]);
        let descriptor = LayerDescriptor {
            media_type: Some("application/vnd.oci.image.layer.v1.tar".into()),
            digest: real_sha256_digest_for(&data),
            size: data.len() as u64,
        };
        let mut sink = CollectSink::default();

        let report =
            apply_validated_tar_layer_sha256(&descriptor, [data.as_slice()], &mut sink).unwrap();

        assert_eq!(report.entries_applied, 1);
        assert_eq!(sink.entries[0].0.path, "bin/app");
    }

    #[test]
    fn validates_and_applies_gzip_tar_layer() {
        let tar = tar(vec![tar_entry("bin/app", b'0', b"run")]);
        let data = gzip(&tar);
        let descriptor = LayerDescriptor {
            media_type: Some("application/vnd.oci.image.layer.v1.tar+gzip".into()),
            digest: test_digest_for(&data),
            size: data.len() as u64,
        };
        let mut sink = CollectSink::default();

        let report = apply_validated_tar_layer(
            &descriptor,
            [data.as_slice()],
            TestDigest::default(),
            &mut sink,
        )
        .unwrap();

        assert_eq!(report.entries_applied, 1);
        assert_eq!(report.layer.bytes_written, data.len() as u64);
        assert_eq!(sink.entries[0].0.path, "bin/app");
        assert_eq!(sink.entries[0].1, b"run");
    }

    #[cfg(feature = "zstd")]
    #[test]
    fn validates_and_applies_zstd_tar_layer() {
        let tar = tar(vec![tar_entry("bin/app", b'0', b"run")]);
        let data = zstd(&tar);
        let descriptor = LayerDescriptor {
            media_type: Some("application/vnd.oci.image.layer.v1.tar+zstd".into()),
            digest: test_digest_for(&data),
            size: data.len() as u64,
        };
        let mut sink = CollectSink::default();

        let report = apply_validated_tar_layer(
            &descriptor,
            [data.as_slice()],
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
    fn classifies_layer_media_types() {
        assert_eq!(
            layer_compression(Some("application/vnd.oci.image.layer.v1.tar")),
            OciLayerCompression::Uncompressed
        );
        assert_eq!(
            layer_compression(Some("application/vnd.oci.image.layer.v1.tar+gzip")),
            OciLayerCompression::Gzip
        );
        assert_eq!(
            layer_compression(Some("application/vnd.docker.image.rootfs.diff.tar.gzip")),
            OciLayerCompression::Gzip
        );
        assert_eq!(
            layer_compression(Some("application/vnd.oci.image.layer.v1.tar+zstd")),
            OciLayerCompression::Zstd
        );
        assert_eq!(layer_compression(None), OciLayerCompression::Unknown);
    }

    #[test]
    fn rejects_compressed_layer_media_type_before_tar_apply() {
        let data = tar(vec![tar_entry("bin/app", b'0', b"run")]);
        let descriptor = LayerDescriptor {
            media_type: Some("application/vnd.oci.image.layer.v1.tar+gzip".into()),
            digest: test_digest_for(&data),
            size: data.len() as u64,
        };
        let mut sink = CollectSink::default();

        let error = apply_validated_uncompressed_tar_layer(
            &descriptor,
            [data.as_slice()],
            TestDigest::default(),
            &mut sink,
        )
        .unwrap_err();

        assert_eq!(
            error,
            TarLayerApplyError::UnsupportedMediaType(descriptor.media_type)
        );
        assert!(sink.entries.is_empty());
    }

    #[test]
    fn rejects_gzip_crc_mismatch() {
        let tar = tar(vec![tar_entry("bin/app", b'0', b"run")]);
        let mut data = gzip(&tar);
        let footer = data.len() - 8;
        data[footer] ^= 0xff;

        let error = decompress_gzip_layer(&data).unwrap_err();

        assert!(matches!(error, TarLayerApplyError::Decompress(_)));
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
    fn applies_pax_path_and_linkpath() {
        let mut pax = Vec::new();
        pax.extend_from_slice(&pax_record("path", "very/long/path/from/pax"));
        pax.extend_from_slice(&pax_record("linkpath", "target/from/pax"));
        let data = tar(vec![
            tar_entry("pax", b'x', &pax),
            tar_entry("short-link", b'2', b""),
        ]);
        let mut sink = CollectSink::default();

        apply_uncompressed_tar_layer(&data, &mut sink).unwrap();

        assert_eq!(sink.entries.len(), 1);
        assert_eq!(sink.entries[0].0.path, "very/long/path/from/pax");
        assert_eq!(
            sink.entries[0].0.link_name.as_deref(),
            Some("target/from/pax")
        );
    }

    #[test]
    fn applies_gnu_long_name() {
        let long_path = "long/component/name/that/does/not/fit/in/header";
        let mut long_name = long_path.as_bytes().to_vec();
        long_name.push(0);
        let data = tar(vec![
            tar_entry("././@LongLink", b'L', &long_name),
            tar_entry("short", b'0', b"body"),
        ]);
        let mut sink = CollectSink::default();

        apply_uncompressed_tar_layer(&data, &mut sink).unwrap();

        assert_eq!(sink.entries.len(), 1);
        assert_eq!(sink.entries[0].0.path, long_path);
        assert_eq!(sink.entries[0].1, b"body");
    }

    #[test]
    fn rejects_unsafe_pax_path() {
        let data = tar(vec![
            tar_entry("pax", b'x', &pax_record("path", "../escape")),
            tar_entry("short", b'0', b"bad"),
        ]);
        let mut sink = CollectSink::default();

        let error = apply_uncompressed_tar_layer(&data, &mut sink).unwrap_err();

        assert_eq!(error, TarLayerError::InvalidPath("../escape".into()));
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
