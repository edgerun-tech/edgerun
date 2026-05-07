//! OCI image layer media type and whiteout rules.

use alloc::format;
use alloc::string::String;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OciLayerCompression {
    Uncompressed,
    Gzip,
    Zstd,
    Unknown,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OciWhiteout {
    RemovePath(String),
    OpaqueDirectory(String),
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

#[cfg(test)]
mod tests {
    use super::*;

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
            layer_compression(Some("application/vnd.oci.image.layer.v1.tar+zstd")),
            OciLayerCompression::Zstd
        );
        assert_eq!(layer_compression(None), OciLayerCompression::Unknown);
    }

    #[test]
    fn parses_whiteout_entries() {
        assert_eq!(
            parse_oci_whiteout("etc/.wh.hosts"),
            Some(OciWhiteout::RemovePath("etc/hosts".into()))
        );
        assert_eq!(
            parse_oci_whiteout("var/lib/.wh..wh..opq"),
            Some(OciWhiteout::OpaqueDirectory("var/lib".into()))
        );
        assert_eq!(parse_oci_whiteout("regular/file"), None);
    }
}
