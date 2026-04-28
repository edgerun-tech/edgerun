//! `ert pull` command — pull an image from a registry.

use crate::prelude::*;
use std::path::PathBuf;

use crate::cli::{default_images_dir, default_store_dir, resolve_registry_auth, GlobalOpts};
use crate::ImageRef;
use crate::{PullProgress, RegistryClient};

pub fn cmd_pull(_opts: &GlobalOpts, args: &[String]) -> std::io::Result<()> {
    let (image_ref, images_dir, store_path) = parse_pull_args(args)?;

    let image: ImageRef = image_ref
        .parse()
        .map_err(|e: String| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    let auth = resolve_registry_auth(&image.registry)?;
    let mut client = RegistryClient::new().with_auth(auth);

    let bundle_path = images_dir.join(&image.repository).join(&image.tag);

    println!("Pulling {image_ref}");
    println!("  images: {}", images_dir.display());
    println!("  store:  {}", store_path.display());

    let rt = edgerun_rt::Runtime::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| std::io::Error::other(e.to_string()))?;

    let image_clone = image.clone();
    let bundle_clone = bundle_path.clone();
    let store_clone = store_path.clone();
    let result = rt.block_on(async move {
        client
            .pull_with_progress(&image_clone, &bundle_clone, &store_clone, |event| {
                print_pull_progress(event);
            })
            .await
    });

    match result {
        Ok(report) => {
            println!(
                "Done: {} -> {} ({} downloaded, {} layer{})",
                image_ref,
                report.path.display(),
                format_bytes(report.bytes_downloaded),
                report.layers,
                if report.layers == 1 { "" } else { "s" }
            );
            Ok(())
        }
        Err(e) => Err(std::io::Error::other(format!("pull failed: {}", e))),
    }
}

fn print_pull_progress(event: PullProgress) {
    match event {
        PullProgress::Resolving { image } => {
            println!("  -> resolving {image}");
        }
        PullProgress::ManifestResolved {
            layers,
            config_digest,
        } => {
            println!(
                "  ok manifest: {} layer{}, config {}",
                layers,
                if layers == 1 { "" } else { "s" },
                short_digest(&config_digest)
            );
        }
        PullProgress::FetchingConfig { digest } => {
            println!("  -> config {}", short_digest(&digest));
        }
        PullProgress::ConfigFetched { bytes } => {
            println!("  ok config: {}", format_bytes(bytes));
        }
        PullProgress::LayerCached {
            index,
            total,
            digest,
        } => {
            println!(
                "  ok layer {index}/{total}: {} already cached",
                short_digest(&digest)
            );
        }
        PullProgress::LayerDownloading {
            index,
            total,
            digest,
            size,
        } => {
            println!(
                "  -> layer {index}/{total}: {} ({})",
                short_digest(&digest),
                format_bytes(size)
            );
        }
        PullProgress::LayerDownloaded {
            index,
            total,
            digest,
            bytes,
        } => {
            println!(
                "  ok layer {index}/{total}: {} downloaded",
                format_bytes(bytes)
            );
            println!("     {}", short_digest(&digest));
        }
        PullProgress::LayerExtracting {
            index,
            total,
            digest,
        } => {
            println!(
                "  -> layer {index}/{total}: extracting {}",
                short_digest(&digest)
            );
        }
        PullProgress::LayerExtracted {
            index,
            total,
            digest,
        } => {
            println!(
                "  ok layer {index}/{total}: extracted {}",
                short_digest(&digest)
            );
        }
        PullProgress::ApplyingWhiteouts { layers } => {
            println!(
                "  -> applying whiteouts across {layers} layer{}",
                if layers == 1 { "" } else { "s" }
            );
        }
        PullProgress::BuildingRootfs { path } => {
            println!("  -> building rootfs at {}", path.display());
        }
        PullProgress::WritingConfig { path } => {
            println!("  -> writing {}", path.display());
        }
    }
}

fn short_digest(digest: &str) -> String {
    if let Some((algorithm, value)) = digest.split_once(':') {
        let short_len = value.len().min(12);
        format!("{algorithm}:{}", &value[..short_len])
    } else {
        digest.chars().take(18).collect()
    }
}

fn format_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = KIB * 1024;
    const GIB: u64 = MIB * 1024;

    if bytes >= GIB {
        format!("{:.1} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes} B")
    }
}

fn parse_pull_args(args: &[String]) -> std::io::Result<(String, PathBuf, PathBuf)> {
    let mut images_dir = default_images_dir();
    let mut store_path = default_store_dir();
    let mut image: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--images-dir" => {
                if i + 1 < args.len() {
                    images_dir = PathBuf::from(&args[i + 1]);
                    i += 2;
                } else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--images-dir requires a path",
                    ));
                }
            }
            "--store" => {
                if i + 1 < args.len() {
                    store_path = PathBuf::from(&args[i + 1]);
                    i += 2;
                } else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--store requires a path",
                    ));
                }
            }
            _ if !args[i].starts_with('-') => {
                image = Some(args[i].clone());
                i += 1;
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("unknown flag: {}", args[i]),
                ));
            }
        }
    }

    let image = image.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Usage: ert pull [--images-dir DIR] [--store DIR] <image>",
        )
    })?;

    Ok((image, images_dir, store_path))
}
