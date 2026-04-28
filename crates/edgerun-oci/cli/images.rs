//! Local image listing command.

use crate::prelude::*;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::cli::{default_images_dir, inline_value, invalid_input, CliArgs};

pub fn cmd_images(_opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    let (images_dir, json) = parse_images_args(args)?;
    let mut images = Vec::new();
    collect_images(&images_dir, &images_dir, &mut images)?;
    images.sort_by(|a, b| a.repository.cmp(&b.repository).then(a.tag.cmp(&b.tag)));

    if json {
        print_images_json(&images);
    } else {
        print_images_table(&images);
    }
    Ok(())
}

#[derive(Debug)]
struct LocalImage {
    repository: String,
    tag: String,
    path: PathBuf,
    size: u64,
}

fn parse_images_args(args: &[String]) -> io::Result<(PathBuf, bool)> {
    let mut images_dir = default_images_dir();
    let mut json = false;
    let mut args = CliArgs::new(args);
    while let Some(arg) = args.next() {
        match arg {
            "--images-dir" => {
                images_dir = PathBuf::from(args.value("--images-dir requires a path")?);
            }
            "--format" => {
                json = args.value("--format requires a value")? == "json";
            }
            arg if inline_value(arg, "--images-dir").is_some() => {
                images_dir = PathBuf::from(inline_value(arg, "--images-dir").unwrap());
            }
            arg if inline_value(arg, "--format").is_some() => {
                json = inline_value(arg, "--format").unwrap() == "json";
            }
            "--format=json" | "--json" => {
                json = true;
            }
            _ => {
                return Err(invalid_input(
                    "Usage: ert images [--images-dir DIR] [--format json]",
                ));
            }
        }
    }
    Ok((images_dir, json))
}

fn collect_images(root: &Path, dir: &Path, out: &mut Vec<LocalImage>) -> io::Result<()> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if path.join("config.json").is_file() {
            if let Some(image) = image_from_bundle(root, &path)? {
                out.push(image);
            }
        } else {
            collect_images(root, &path, out)?;
        }
    }
    Ok(())
}

fn image_from_bundle(root: &Path, bundle: &Path) -> io::Result<Option<LocalImage>> {
    let rel = match bundle.strip_prefix(root) {
        Ok(rel) => rel,
        Err(_) => return Ok(None),
    };
    let parts = rel
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    if parts.len() < 2 {
        return Ok(None);
    }
    let tag = parts.last().cloned().unwrap_or_default();
    let repository = parts[..parts.len() - 1].join("/");
    let size = dir_size(bundle)?;
    Ok(Some(LocalImage {
        repository,
        tag,
        path: bundle.to_path_buf(),
        size,
    }))
}

fn dir_size(path: &Path) -> io::Result<u64> {
    let mut total = 0u64;
    let Ok(entries) = fs::read_dir(path) else {
        return Ok(0);
    };
    for entry in entries {
        let entry = entry?;
        let meta = fs::symlink_metadata(entry.path())?;
        if meta.is_dir() {
            total += dir_size(&entry.path())?;
        } else {
            total += meta.len();
        }
    }
    Ok(total)
}

fn print_images_table(images: &[LocalImage]) {
    println!("{:<36} {:<16} {:<12} PATH", "REPOSITORY", "TAG", "SIZE");
    println!("{:-<96}", "");
    for image in images {
        println!(
            "{:<36} {:<16} {:<12} {}",
            image.repository,
            image.tag,
            format_bytes(image.size),
            image.path.display()
        );
    }
}

fn print_images_json(images: &[LocalImage]) {
    let entries = images
        .iter()
        .map(|image| {
            let path = image.path.to_string_lossy();
            edgerun_json::json!({
                "repository": image.repository.as_str(),
                "tag": image.tag.as_str(),
                "size": image.size,
                "path": path.as_ref()
            })
        })
        .collect::<Vec<_>>();
    let output = edgerun_json::JsonValue::Array(entries);
    println!("{}", edgerun_json::to_string(&output).unwrap_or_default());
}

fn format_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = KIB * 1024;
    const GIB: u64 = MIB * 1024;
    if bytes >= GIB {
        format!("{:.1}GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1}MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1}KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes}B")
    }
}
