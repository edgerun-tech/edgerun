use crate::prelude::*;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

/// Create a gzip-compressed tar from a directory.
pub(crate) fn create_tar_from_dir(dir: &Path) -> io::Result<Vec<u8>> {
    let mut tar = Vec::new();
    append_tar_dir(&mut tar, dir, Path::new(""))?;
    tar.extend_from_slice(&[0u8; 1024]);
    gzip_bytes(&tar)
}

fn append_tar_dir(out: &mut Vec<u8>, dir: &Path, rel: &Path) -> io::Result<()> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    if !rel.as_os_str().is_empty() {
        let metadata = std::fs::symlink_metadata(dir)?;
        append_tar_header(
            out,
            rel,
            b'5',
            0,
            metadata.permissions().mode(),
            metadata.uid(),
            metadata.gid(),
            metadata.mtime().max(0) as u64,
            None,
        )?;
    }

    let mut entries = std::fs::read_dir(dir)?.collect::<io::Result<Vec<_>>>()?;
    entries.sort_by(|left, right| {
        left.file_name()
            .as_bytes()
            .cmp(right.file_name().as_bytes())
    });

    for entry in entries {
        let path = entry.path();
        let child_rel = rel.join(entry.file_name());
        let metadata = std::fs::symlink_metadata(&path)?;
        let file_type = metadata.file_type();

        if file_type.is_dir() {
            append_tar_dir(out, &path, &child_rel)?;
        } else if file_type.is_symlink() {
            let target = std::fs::read_link(&path)?;
            append_tar_header(
                out,
                &child_rel,
                b'2',
                0,
                metadata.permissions().mode(),
                metadata.uid(),
                metadata.gid(),
                metadata.mtime().max(0) as u64,
                Some(&target),
            )?;
        } else if file_type.is_file() {
            let data = std::fs::read(&path)?;
            append_tar_header(
                out,
                &child_rel,
                b'0',
                data.len() as u64,
                metadata.permissions().mode(),
                metadata.uid(),
                metadata.gid(),
                metadata.mtime().max(0) as u64,
                None,
            )?;
            out.extend_from_slice(&data);
            let padding = (512 - (data.len() % 512)) % 512;
            out.extend(std::iter::repeat(0).take(padding));
        }
    }

    Ok(())
}

fn append_tar_header(
    out: &mut Vec<u8>,
    path: &Path,
    kind: u8,
    size: u64,
    mode: u32,
    uid: u32,
    gid: u32,
    mtime: u64,
    link_name: Option<&Path>,
) -> io::Result<()> {
    let mut header = [0u8; 512];
    let (name, prefix) = split_ustar_path(path)?;
    header[..name.len()].copy_from_slice(&name);
    if let Some(prefix) = prefix {
        header[345..345 + prefix.len()].copy_from_slice(&prefix);
    }

    write_octal(&mut header[100..108], mode as u64)?;
    write_octal(&mut header[108..116], uid as u64)?;
    write_octal(&mut header[116..124], gid as u64)?;
    write_octal(&mut header[124..136], size)?;
    write_octal(&mut header[136..148], mtime)?;
    header[148..156].fill(b' ');
    header[156] = kind;

    if let Some(link_name) = link_name {
        let link = path_to_tar_bytes(link_name)?;
        if link.len() > 100 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("tar link target too long: {}", link_name.display()),
            ));
        }
        header[157..157 + link.len()].copy_from_slice(&link);
    }

    header[257..263].copy_from_slice(b"ustar\0");
    header[263..265].copy_from_slice(b"00");

    let checksum = header.iter().map(|byte| *byte as u32).sum::<u32>();
    write_checksum(&mut header[148..156], checksum);
    out.extend_from_slice(&header);
    Ok(())
}

fn split_ustar_path(path: &Path) -> io::Result<(Vec<u8>, Option<Vec<u8>>)> {
    let name = path_to_tar_bytes(path)?;
    if name.len() <= 100 {
        return Ok((name, None));
    }

    let split = name
        .iter()
        .enumerate()
        .filter(|(_, byte)| **byte == b'/')
        .map(|(index, _)| index)
        .find(|index| *index <= 155 && name.len().saturating_sub(index + 1) <= 100)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("tar path too long: {}", path.display()),
            )
        })?;

    let prefix = &name[..split];
    let suffix = &name[split + 1..];
    if prefix.is_empty() || suffix.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid tar path: {}", path.display()),
        ));
    }
    Ok((suffix.to_vec(), Some(prefix.to_vec())))
}

fn path_to_tar_bytes(path: &Path) -> io::Result<Vec<u8>> {
    let path = path.to_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("non-utf8 tar path: {}", path.display()),
        )
    })?;
    Ok(path.as_bytes().to_vec())
}

fn write_octal(field: &mut [u8], value: u64) -> io::Result<()> {
    let encoded = format!("{:0width$o}\0", value, width = field.len() - 1);
    if encoded.len() > field.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "tar numeric field overflow",
        ));
    }
    field.copy_from_slice(encoded.as_bytes());
    Ok(())
}

fn write_checksum(field: &mut [u8], value: u32) {
    let encoded = format!("{:06o}\0 ", value);
    field.copy_from_slice(encoded.as_bytes());
}

#[cfg(feature = "gzip")]
fn gzip_bytes(data: &[u8]) -> io::Result<Vec<u8>> {
    let mut out = Vec::new();
    out.extend_from_slice(&[0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 255]);
    out.extend_from_slice(&miniz_oxide::deflate::compress_to_vec(data, 6));
    out.extend_from_slice(&edgerun_encoding::crc32::crc32(data).to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    Ok(out)
}

#[cfg(not(feature = "gzip"))]
fn gzip_bytes(_data: &[u8]) -> io::Result<Vec<u8>> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "OCI gzip layer support is disabled",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "gzip")]
    use crate::tar_layer::{apply_uncompressed_tar_layer, decompress_gzip_layer};
    use crate::tar_layer::{TarEntry, TarLayerSink};
    use std::path::PathBuf;

    #[derive(Default)]
    struct CollectSink {
        entries: Vec<TarEntry>,
    }

    impl TarLayerSink for CollectSink {
        fn apply_entry(&mut self, entry: &TarEntry, _data: &[u8]) -> Result<(), String> {
            self.entries.push(entry.clone());
            Ok(())
        }
    }

    fn tmp_dir(name: &str) -> PathBuf {
        static C: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = C.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!(
            "edgerun-oci-tar-push-{name}-{}-{n}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[cfg(feature = "gzip")]
    fn pushed_entries(dir: &Path) -> Vec<TarEntry> {
        let gzip = create_tar_from_dir(dir).unwrap();
        let tar = decompress_gzip_layer(&gzip).unwrap();
        let mut sink = CollectSink::default();
        apply_uncompressed_tar_layer(&tar, &mut sink).unwrap();
        sink.entries
    }

    #[cfg(feature = "gzip")]
    #[test]
    fn create_tar_from_dir_orders_entries_deterministically() {
        let root = tmp_dir("order");
        std::fs::write(root.join("z"), b"z").unwrap();
        std::fs::write(root.join("a"), b"a").unwrap();
        std::fs::create_dir_all(root.join("m")).unwrap();
        std::fs::write(root.join("m/b"), b"b").unwrap();

        let paths = pushed_entries(&root)
            .into_iter()
            .map(|entry| entry.path)
            .collect::<Vec<_>>();

        assert_eq!(paths, vec!["a", "m", "m/b", "z"]);
        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(feature = "gzip")]
    #[test]
    fn create_tar_from_dir_supports_ustar_prefix_paths() {
        let root = tmp_dir("long");
        let dir_name = "a".repeat(90);
        let file_name = "b".repeat(60);
        let dir = root.join(&dir_name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(&file_name), b"long").unwrap();

        let paths = pushed_entries(&root)
            .into_iter()
            .map(|entry| entry.path)
            .collect::<Vec<_>>();

        assert!(paths.contains(&format!("{dir_name}/{file_name}")));
        let _ = std::fs::remove_dir_all(root);
    }
}
