use crate::prelude::*;
use core::fmt::Write as _;

const OCI_MANIFEST_MEDIA_TYPE: &str = "application/vnd.oci.image.manifest.v1+json";
const OCI_CONFIG_MEDIA_TYPE: &str = "application/vnd.oci.image.config.v1+json";
const OCI_GZIP_LAYER_MEDIA_TYPE: &str = "application/vnd.oci.image.layer.v1.tar+gzip";

pub(crate) fn push_manifest_json(
    config_digest: String,
    config_size: usize,
    layer_digest: String,
    layer_size: usize,
) -> String {
    let mut out = String::new();
    out.push_str("{\"schemaVersion\":2,\"mediaType\":");
    write_json_string(&mut out, OCI_MANIFEST_MEDIA_TYPE);
    out.push_str(",\"config\":{\"mediaType\":");
    write_json_string(&mut out, OCI_CONFIG_MEDIA_TYPE);
    out.push_str(",\"digest\":");
    write_json_string(&mut out, &config_digest);
    out.push_str(",\"size\":");
    write!(&mut out, "{config_size}").expect("writing to String cannot fail");
    out.push_str("},\"layers\":[{\"mediaType\":");
    write_json_string(&mut out, OCI_GZIP_LAYER_MEDIA_TYPE);
    out.push_str(",\"digest\":");
    write_json_string(&mut out, &layer_digest);
    out.push_str(",\"size\":");
    write!(&mut out, "{layer_size}").expect("writing to String cannot fail");
    out.push_str("}]}");
    out
}

fn write_json_string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch <= '\u{1f}' => {
                write!(out, "\\u{:04x}", ch as u32).expect("writing to String cannot fail");
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
}
