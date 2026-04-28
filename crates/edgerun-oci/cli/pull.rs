//! `ert pull` command — pull an image from a registry.

use crate::prelude::*;
use std::path::PathBuf;

use crate::cli::{
    default_images_dir, default_store_dir, inline_value, invalid_input, resolve_registry_auth,
    CliArgs, GlobalOpts,
};
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

pub(crate) fn print_pull_progress(event: PullProgress) {
    match event {
        PullProgress::Resolving { image } => {
            eprintln!("  -> resolving {image}");
        }
        PullProgress::ManifestResolved {
            layers,
            config_digest,
        } => {
            eprintln!(
                "  ok manifest: {} layer{}, config {}",
                layers,
                if layers == 1 { "" } else { "s" },
                short_digest(&config_digest)
            );
        }
        PullProgress::FetchingConfig { digest } => {
            eprintln!("  -> config {}", short_digest(&digest));
        }
        PullProgress::ConfigFetched { bytes } => {
            eprintln!("  ok config: {}", format_bytes(bytes));
        }
        PullProgress::LayerCached {
            index,
            total,
            digest,
        } => {
            eprintln!(
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
            eprintln!(
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
            eprintln!(
                "  ok layer {index}/{total}: {} downloaded",
                format_bytes(bytes)
            );
            eprintln!("     {}", short_digest(&digest));
        }
        PullProgress::LayerExtracting {
            index,
            total,
            digest,
        } => {
            eprintln!(
                "  -> layer {index}/{total}: extracting {}",
                short_digest(&digest)
            );
        }
        PullProgress::LayerExtracted {
            index,
            total,
            digest,
        } => {
            eprintln!(
                "  ok layer {index}/{total}: extracted {}",
                short_digest(&digest)
            );
        }
        PullProgress::ApplyingWhiteouts { layers } => {
            eprintln!(
                "  -> applying whiteouts across {layers} layer{}",
                if layers == 1 { "" } else { "s" }
            );
        }
        PullProgress::BuildingRootfs { path } => {
            eprintln!("  -> building rootfs at {}", path.display());
        }
        PullProgress::WritingConfig { path } => {
            eprintln!("  -> writing {}", path.display());
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

    let mut args = CliArgs::new(args);
    while let Some(arg) = args.next() {
        match arg {
            "--images-dir" => {
                images_dir = PathBuf::from(args.value("--images-dir requires a path")?);
            }
            "--store" => {
                store_path = PathBuf::from(args.value("--store requires a path")?);
            }
            arg if inline_value(arg, "--images-dir").is_some() => {
                images_dir = PathBuf::from(inline_value(arg, "--images-dir").unwrap());
            }
            arg if inline_value(arg, "--store").is_some() => {
                store_path = PathBuf::from(inline_value(arg, "--store").unwrap());
            }
            arg if !arg.starts_with('-') => {
                image = Some(arg.to_string());
            }
            _ => {
                return Err(invalid_input(format!("unknown flag: {}", arg)));
            }
        }
    }

    let image = image
        .ok_or_else(|| invalid_input("Usage: ert pull [--images-dir DIR] [--store DIR] <image>"))?;

    Ok((image, images_dir, store_path))
}
