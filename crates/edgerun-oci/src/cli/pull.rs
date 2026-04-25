//! `ert pull` command — pull an image from a registry.

use std::path::PathBuf;

use crate::cli::GlobalOpts;
use crate::cli::resolve_registry_auth;
use crate::ImageRef;
use crate::RegistryClient;

pub fn cmd_pull(_opts: &GlobalOpts, args: &[String]) -> std::io::Result<()> {
    let (image_ref, images_dir, store_path) = parse_pull_args(args)?;

    let image: ImageRef = image_ref
        .parse()
        .map_err(|e: String| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    let auth = resolve_registry_auth(&image.registry)?;
    let mut client = RegistryClient::new().with_auth(auth);

    let bundle_path = images_dir.join(&image.repository).join(&image.tag);

    println!("Pulling {}...", image_ref);

    let rt = edgerun_rt::Runtime::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| std::io::Error::other(e.to_string()))?;

    let image_clone = image.clone();
    let bundle_clone = bundle_path.clone();
    let store_clone = store_path.clone();
    let result =
        rt.block_on(async move { client.pull(&image_clone, &bundle_clone, &store_clone).await });

    match result {
        Ok(path) => {
            println!("Pulled {} to {}", image_ref, path.display());
            Ok(())
        }
        Err(e) => Err(std::io::Error::other(format!("pull failed: {}", e))),
    }
}

fn parse_pull_args(args: &[String]) -> std::io::Result<(String, PathBuf, PathBuf)> {
    let default_images = PathBuf::from("/var/lib/edgerun/images");
    let default_store = PathBuf::from("/var/lib/edgerun/store");

    let mut images_dir = default_images;
    let mut store_path = default_store;
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
