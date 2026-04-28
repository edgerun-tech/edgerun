use crate::prelude::*;
use std::io;
use std::path::Path;

/// Create a gzip-compressed tar from a directory.
pub(crate) fn create_tar_from_dir(dir: &Path) -> io::Result<Vec<u8>> {
    let mut tar = Vec::new();
    append_tar_dir(&mut tar, dir, Path::new(""))?;
    tar.extend_from_slice(&[0u8; 1024]);
    Ok(gzip_bytes(&tar))
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

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
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
    let name = path_to_tar_bytes(path)?;
    if name.len() > 100 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("tar path too long: {}", path.display()),
        ));
    }
    header[..name.len()].copy_from_slice(&name);

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

fn gzip_bytes(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&[0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 255]);
    out.extend_from_slice(&miniz_oxide::deflate::compress_to_vec(data, 6));
    out.extend_from_slice(&edgerun_encoding::crc32::crc32(data).to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out
}
