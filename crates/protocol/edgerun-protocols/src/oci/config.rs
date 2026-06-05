//! OCI image config and manifest JSON field projection.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use super::manifest::{
    ImageIndex, ImageManifest, LayerDescriptor, ManifestDescriptor, PlatformDescriptor,
    SingleManifest,
};

/// Parsed image config.
#[derive(Debug, Clone)]
pub struct ImageConfig {
    pub architecture: Option<String>,
    pub os: Option<String>,
    pub config: Option<ImageConfigInner>,
    pub rootfs: Option<RootFs>,
    pub history: Option<Vec<HistoryEntry>>,
}

/// Inner image config (Cmd, Env, WorkingDir, etc.).
#[derive(Debug, Clone)]
pub struct ImageConfigInner {
    pub user: Option<String>,
    pub env: Option<Vec<String>>,
    pub entrypoint: Option<Vec<String>>,
    pub cmd: Option<Vec<String>>,
    pub working_dir: Option<String>,
    pub exposed_ports: Option<BTreeMap<String, OciObjectPresence>>,
    pub volumes: Option<BTreeMap<String, OciObjectPresence>>,
    pub labels: Option<BTreeMap<String, String>>,
    pub stop_signal: Option<String>,
}

/// Presence marker for OCI object maps such as `Volumes` and `ExposedPorts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OciObjectPresence;

/// Root filesystem info.
#[derive(Debug, Clone)]
pub struct RootFs {
    pub r#type: String,
    pub diff_ids: Vec<String>,
}

/// History entry (build layer info, from OCI image config).
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub created: Option<String>,
    pub created_by: Option<String>,
    pub comment: Option<String>,
    pub empty_layer: Option<bool>,
}

/// Parse an image config from JSON bytes.
pub fn parse_image_config(data: &[u8]) -> Result<ImageConfig, String> {
    parse_image_config_span(data)
}

/// Validate JSON bytes and return the original bytes.
///
/// This replaces the deleted generic JSON object helper. Callers that need OCI
/// data should project directly into typed records instead of inspecting a
/// dynamic object model.
pub fn parse_json_bytes(data: &[u8]) -> Result<&[u8], String> {
    let span = root_object_span(data)?;
    if skip_ws(data, span.end) != data.len() {
        return Err("trailing bytes after JSON document".into());
    }
    Ok(data)
}

/// Parse a raw JSON blob into an ImageManifest (index or single).
pub fn parse_manifest(data: &[u8]) -> Result<ImageManifest, String> {
    let root = root_object_span(data)?;
    if field_span(data, root, &["manifests"])?
        .map(|span| first_array_element_span(data, span).is_ok())
        .unwrap_or(false)
    {
        return parse_image_index_span(data, root).map(ImageManifest::Index);
    }
    parse_single_manifest_span(data, root).map(ImageManifest::Single)
}

/// Parse a raw JSON blob as a single manifest.
pub fn parse_single_manifest(data: &[u8]) -> Result<SingleManifest, String> {
    let root = root_object_span(data)?;
    parse_single_manifest_span(data, root)
}

fn parse_image_config_span(data: &[u8]) -> Result<ImageConfig, String> {
    let root = root_object_span(data)?;
    Ok(ImageConfig {
        architecture: optional_string(data, root, &["architecture"])?,
        os: optional_string(data, root, &["os"])?,
        config: optional_object(data, root, &["config"], parse_image_config_inner)?,
        rootfs: optional_object(data, root, &["rootfs"], parse_rootfs)?,
        history: optional_object_array(data, root, &["history"], parse_history_entry)?,
    })
}

fn parse_image_config_inner(data: &[u8], object: Span) -> Result<ImageConfigInner, String> {
    Ok(ImageConfigInner {
        user: optional_string(data, object, &["User", "user"])?,
        env: optional_string_array(data, object, &["Env", "env"])?,
        entrypoint: optional_string_array(data, object, &["Entrypoint", "entrypoint"])?,
        cmd: optional_string_array(data, object, &["Cmd", "cmd"])?,
        working_dir: optional_string(data, object, &["WorkingDir", "workingDir", "working_dir"])?,
        exposed_ports: optional_presence_map(data, object, &["ExposedPorts", "exposedPorts"])?,
        volumes: optional_presence_map(data, object, &["Volumes", "volumes"])?,
        labels: optional_string_map(data, object, &["Labels", "labels"])?,
        stop_signal: optional_string(data, object, &["StopSignal", "stopSignal", "stop_signal"])?,
    })
}

fn parse_rootfs(data: &[u8], object: Span) -> Result<RootFs, String> {
    Ok(RootFs {
        r#type: required_string(data, object, &["type"])?,
        diff_ids: required_string_array(data, object, &["diff_ids"])?,
    })
}

fn parse_history_entry(data: &[u8], object: Span) -> Result<HistoryEntry, String> {
    Ok(HistoryEntry {
        created: optional_string(data, object, &["created"])?,
        created_by: optional_string(data, object, &["created_by"])?,
        comment: optional_string(data, object, &["comment"])?,
        empty_layer: optional_bool(data, object, &["empty_layer"])?,
    })
}

fn parse_image_index_span(data: &[u8], object: Span) -> Result<ImageIndex, String> {
    Ok(ImageIndex {
        media_type: optional_string(data, object, &["mediaType"])?,
        manifests: required_object_array(data, object, &["manifests"], parse_manifest_descriptor)?,
    })
}

fn parse_single_manifest_span(data: &[u8], object: Span) -> Result<SingleManifest, String> {
    let config = required_value(data, object, &["config"])?;
    let (config_digest, config_size, config_media_type) = if starts_with_ws(data, config, b'{') {
        let config = object_span(data, config.start)?;
        (
            required_string(data, config, &["digest"])?,
            optional_u64(data, config, &["size"])?,
            optional_string(data, config, &["mediaType"])?,
        )
    } else {
        (parse_string_value(data, config)?, None, None)
    };

    Ok(SingleManifest {
        config_digest,
        config_size,
        config_media_type,
        layers: required_object_array(data, object, &["layers"], parse_layer_descriptor)?,
    })
}

fn parse_manifest_descriptor(data: &[u8], object: Span) -> Result<ManifestDescriptor, String> {
    Ok(ManifestDescriptor {
        media_type: optional_string(data, object, &["mediaType"])?,
        digest: required_string(data, object, &["digest"])?,
        size: required_u64(data, object, &["size"])?,
        platform: optional_object(data, object, &["platform"], parse_platform_descriptor)?,
    })
}

fn parse_platform_descriptor(data: &[u8], object: Span) -> Result<PlatformDescriptor, String> {
    Ok(PlatformDescriptor {
        architecture: optional_string(data, object, &["architecture"])?,
        os: optional_string(data, object, &["os"])?,
    })
}

fn parse_layer_descriptor(data: &[u8], object: Span) -> Result<LayerDescriptor, String> {
    Ok(LayerDescriptor {
        media_type: optional_string(data, object, &["mediaType"])?,
        digest: required_string(data, object, &["digest"])?,
        size: required_u64(data, object, &["size"])?,
    })
}

#[derive(Clone, Copy)]
struct Span {
    start: usize,
    end: usize,
}

fn root_object_span(data: &[u8]) -> Result<Span, String> {
    let start = skip_ws(data, 0);
    object_span(data, start)
}

fn object_span(data: &[u8], start: usize) -> Result<Span, String> {
    if data.get(start) != Some(&b'{') {
        return Err("expected JSON object".into());
    }
    value_span(data, start)
}

fn array_span(data: &[u8], start: usize) -> Result<Span, String> {
    if data.get(start) != Some(&b'[') {
        return Err("expected JSON array".into());
    }
    value_span(data, start)
}

fn field_span(data: &[u8], object: Span, names: &[&str]) -> Result<Option<Span>, String> {
    let mut pos = skip_ws(data, object.start + 1);
    if data.get(pos) == Some(&b'}') {
        return Ok(None);
    }
    loop {
        let key_span = string_span(data, pos)?;
        let key = parse_string_value(data, key_span)?;
        pos = skip_ws(data, key_span.end);
        expect_byte(data, pos, b':')?;
        let value_start = skip_ws(data, pos + 1);
        let value = value_span(data, value_start)?;
        if names.iter().any(|name| key == *name) {
            return Ok(Some(value));
        }
        pos = skip_ws(data, value.end);
        match data.get(pos) {
            Some(b',') => pos = skip_ws(data, pos + 1),
            Some(b'}') if pos + 1 == object.end => return Ok(None),
            _ => return Err("expected comma or object end".into()),
        }
    }
}

fn required_value(data: &[u8], object: Span, names: &[&str]) -> Result<Span, String> {
    field_span(data, object, names)?.ok_or_else(|| format!("missing required field `{}`", names[0]))
}

fn optional_object<T>(
    data: &[u8],
    object: Span,
    names: &[&str],
    parse: fn(&[u8], Span) -> Result<T, String>,
) -> Result<Option<T>, String> {
    match field_span(data, object, names)? {
        Some(span) => parse(data, object_span(data, skip_ws(data, span.start))?).map(Some),
        None => Ok(None),
    }
}

fn optional_string(data: &[u8], object: Span, names: &[&str]) -> Result<Option<String>, String> {
    field_span(data, object, names)?
        .map(|span| parse_string_value(data, span))
        .transpose()
}

fn required_string(data: &[u8], object: Span, names: &[&str]) -> Result<String, String> {
    parse_string_value(data, required_value(data, object, names)?)
}

fn optional_bool(data: &[u8], object: Span, names: &[&str]) -> Result<Option<bool>, String> {
    field_span(data, object, names)?
        .map(|span| parse_bool_value(data, span))
        .transpose()
}

fn optional_u64(data: &[u8], object: Span, names: &[&str]) -> Result<Option<u64>, String> {
    field_span(data, object, names)?
        .map(|span| parse_u64_value(data, span))
        .transpose()
}

fn required_u64(data: &[u8], object: Span, names: &[&str]) -> Result<u64, String> {
    parse_u64_value(data, required_value(data, object, names)?)
}

fn optional_string_array(
    data: &[u8],
    object: Span,
    names: &[&str],
) -> Result<Option<Vec<String>>, String> {
    field_span(data, object, names)?
        .map(|span| parse_string_array(data, span))
        .transpose()
}

fn required_string_array(data: &[u8], object: Span, names: &[&str]) -> Result<Vec<String>, String> {
    parse_string_array(data, required_value(data, object, names)?)
}

fn optional_object_array<T>(
    data: &[u8],
    object: Span,
    names: &[&str],
    parse: fn(&[u8], Span) -> Result<T, String>,
) -> Result<Option<Vec<T>>, String> {
    field_span(data, object, names)?
        .map(|span| parse_object_array(data, span, parse))
        .transpose()
}

fn required_object_array<T>(
    data: &[u8],
    object: Span,
    names: &[&str],
    parse: fn(&[u8], Span) -> Result<T, String>,
) -> Result<Vec<T>, String> {
    parse_object_array(data, required_value(data, object, names)?, parse)
}

fn optional_presence_map(
    data: &[u8],
    object: Span,
    names: &[&str],
) -> Result<Option<BTreeMap<String, OciObjectPresence>>, String> {
    field_span(data, object, names)?
        .map(|span| parse_presence_map(data, span))
        .transpose()
}

fn optional_string_map(
    data: &[u8],
    object: Span,
    names: &[&str],
) -> Result<Option<BTreeMap<String, String>>, String> {
    field_span(data, object, names)?
        .map(|span| parse_string_map(data, span))
        .transpose()
}

fn parse_string_array(data: &[u8], span: Span) -> Result<Vec<String>, String> {
    parse_array(data, span, parse_string_value)
}

fn parse_object_array<T>(
    data: &[u8],
    span: Span,
    parse: fn(&[u8], Span) -> Result<T, String>,
) -> Result<Vec<T>, String> {
    parse_array(data, span, |data, span| {
        parse(data, object_span(data, skip_ws(data, span.start))?)
    })
}

fn parse_array<T>(
    data: &[u8],
    span: Span,
    parse: fn(&[u8], Span) -> Result<T, String>,
) -> Result<Vec<T>, String> {
    let array = array_span(data, skip_ws(data, span.start))?;
    let mut values = Vec::new();
    let mut pos = skip_ws(data, array.start + 1);
    if data.get(pos) == Some(&b']') {
        return Ok(values);
    }
    loop {
        let value = value_span(data, pos)?;
        values.push(parse(data, value)?);
        pos = skip_ws(data, value.end);
        match data.get(pos) {
            Some(b',') => pos = skip_ws(data, pos + 1),
            Some(b']') if pos + 1 == array.end => return Ok(values),
            _ => return Err("expected comma or array end".into()),
        }
    }
}

fn first_array_element_span(data: &[u8], span: Span) -> Result<Span, String> {
    let array = array_span(data, skip_ws(data, span.start))?;
    let pos = skip_ws(data, array.start + 1);
    if data.get(pos) == Some(&b']') {
        return Err("empty array".into());
    }
    value_span(data, pos)
}

fn parse_presence_map(
    data: &[u8],
    span: Span,
) -> Result<BTreeMap<String, OciObjectPresence>, String> {
    let object = object_span(data, skip_ws(data, span.start))?;
    let mut map = BTreeMap::new();
    for_each_object_field(data, object, |key, _value| {
        map.insert(key, OciObjectPresence);
        Ok(())
    })?;
    Ok(map)
}

fn parse_string_map(data: &[u8], span: Span) -> Result<BTreeMap<String, String>, String> {
    let object = object_span(data, skip_ws(data, span.start))?;
    let mut map = BTreeMap::new();
    for_each_object_field(data, object, |key, value| {
        map.insert(key, parse_string_value(data, value)?);
        Ok(())
    })?;
    Ok(map)
}

fn for_each_object_field(
    data: &[u8],
    object: Span,
    mut visit: impl FnMut(String, Span) -> Result<(), String>,
) -> Result<(), String> {
    let mut pos = skip_ws(data, object.start + 1);
    if data.get(pos) == Some(&b'}') {
        return Ok(());
    }
    loop {
        let key_span = string_span(data, pos)?;
        let key = parse_string_value(data, key_span)?;
        pos = skip_ws(data, key_span.end);
        expect_byte(data, pos, b':')?;
        let value_start = skip_ws(data, pos + 1);
        let value = value_span(data, value_start)?;
        visit(key, value)?;
        pos = skip_ws(data, value.end);
        match data.get(pos) {
            Some(b',') => pos = skip_ws(data, pos + 1),
            Some(b'}') if pos + 1 == object.end => return Ok(()),
            _ => return Err("expected comma or object end".into()),
        }
    }
}

fn parse_string_value(data: &[u8], span: Span) -> Result<String, String> {
    let span = string_span(data, skip_ws(data, span.start))?;
    let mut out = String::new();
    let mut pos = span.start + 1;
    while pos < span.end - 1 {
        match data[pos] {
            b'\\' => {
                pos += 1;
                let escaped = *data
                    .get(pos)
                    .ok_or_else(|| "bad string escape".to_string())?;
                match escaped {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'b' => out.push('\u{0008}'),
                    b'f' => out.push('\u{000c}'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'u' => {
                        return Err("unicode JSON escapes require json-scalar.wat".into());
                    }
                    _ => return Err("bad string escape".into()),
                }
            }
            byte if byte < 0x20 => return Err("control byte in JSON string".into()),
            byte => out.push(byte as char),
        }
        pos += 1;
    }
    Ok(out)
}

fn parse_bool_value(data: &[u8], span: Span) -> Result<bool, String> {
    let start = skip_ws(data, span.start);
    let end = trim_ws_end(data, span.end);
    match &data[start..end] {
        b"true" => Ok(true),
        b"false" => Ok(false),
        _ => Err("expected JSON boolean".into()),
    }
}

fn parse_u64_value(data: &[u8], span: Span) -> Result<u64, String> {
    let start = skip_ws(data, span.start);
    let end = trim_ws_end(data, span.end);
    let text = core::str::from_utf8(&data[start..end]).map_err(|_| "bad number".to_string())?;
    if text.is_empty()
        || text.starts_with('-')
        || text.bytes().any(|byte| matches!(byte, b'.' | b'e' | b'E'))
    {
        return Err("expected unsigned integer".into());
    }
    text.parse::<u64>()
        .map_err(|_| "invalid unsigned integer".into())
}

fn starts_with_ws(data: &[u8], span: Span, byte: u8) -> bool {
    data.get(skip_ws(data, span.start)) == Some(&byte)
}

fn value_span(data: &[u8], start: usize) -> Result<Span, String> {
    let start = skip_ws(data, start);
    match data.get(start).copied() {
        Some(b'"') => string_span(data, start),
        Some(b'{') => balanced_span(data, start, b'{', b'}'),
        Some(b'[') => balanced_span(data, start, b'[', b']'),
        Some(b'-' | b'0'..=b'9' | b't' | b'f' | b'n') => {
            let mut end = start;
            while let Some(byte) = data.get(end).copied() {
                if matches!(byte, b',' | b'}' | b']') || byte.is_ascii_whitespace() {
                    break;
                }
                end += 1;
            }
            Ok(Span { start, end })
        }
        _ => Err("expected JSON value".into()),
    }
}

fn balanced_span(data: &[u8], start: usize, open: u8, close: u8) -> Result<Span, String> {
    expect_byte(data, start, open)?;
    let mut depth = 0usize;
    let mut pos = start;
    while pos < data.len() {
        match data[pos] {
            b'"' => pos = string_span(data, pos)?.end,
            byte if byte == open => {
                depth += 1;
                pos += 1;
            }
            byte if byte == close => {
                depth -= 1;
                pos += 1;
                if depth == 0 {
                    return Ok(Span { start, end: pos });
                }
            }
            _ => pos += 1,
        }
    }
    Err("unterminated JSON container".into())
}

fn string_span(data: &[u8], start: usize) -> Result<Span, String> {
    expect_byte(data, start, b'"')?;
    let mut pos = start + 1;
    while pos < data.len() {
        match data[pos] {
            b'\\' => pos += 2,
            b'"' => {
                return Ok(Span {
                    start,
                    end: pos + 1,
                });
            }
            _ => pos += 1,
        }
    }
    Err("unterminated JSON string".into())
}

fn expect_byte(data: &[u8], pos: usize, byte: u8) -> Result<(), String> {
    if data.get(pos) == Some(&byte) {
        Ok(())
    } else {
        Err(format!("expected byte `{}`", byte as char))
    }
}

fn skip_ws(data: &[u8], mut pos: usize) -> usize {
    while data.get(pos).is_some_and(u8::is_ascii_whitespace) {
        pos += 1;
    }
    pos
}

fn trim_ws_end(data: &[u8], mut end: usize) -> usize {
    while end > 0 && data[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    end
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn parses_single_manifest() {
        let json = br#"{
            "schemaVersion": 2,
            "config": {"mediaType": "application/vnd.oci.image.config.v1+json", "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "size": 1},
            "layers": [{"mediaType": "application/vnd.oci.image.layer.v1.tar", "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", "size": 2}]
        }"#;
        let manifest = parse_single_manifest(json).unwrap();
        assert_eq!(
            manifest.config_digest,
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        );
        assert_eq!(manifest.layers.len(), 1);
    }

    #[test]
    fn parses_image_config() {
        let json = br#"{
            "architecture": "amd64",
            "os": "linux",
            "rootfs": {"type": "layers", "diff_ids": ["sha256:abc"]},
            "config": {"Cmd": ["/bin/sh"], "Env": ["A=B"], "Volumes": {"/cache": {}}},
            "history": [{"created_by": "test", "empty_layer": true}]
        }"#;
        let config = parse_image_config(json).unwrap();
        assert_eq!(config.architecture.as_deref(), Some("amd64"));
        assert_eq!(config.rootfs.unwrap().diff_ids, vec!["sha256:abc"]);
        assert!(
            config
                .config
                .unwrap()
                .volumes
                .unwrap()
                .contains_key("/cache")
        );
    }
}
