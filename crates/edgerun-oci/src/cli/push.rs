//! `ert push` command — push an image to a registry.

use std::path::PathBuf;

use crate::cli::GlobalOpts;
use crate::cli::resolve_registry_auth;
use crate::ImageRef;
use crate::RegistryClient;

pub fn cmd_push(_opts: &GlobalOpts, args: &[String]) -> std::io::Result<()> {
    let (image_ref, images_dir) = parse_push_args(args)?;

    let image: ImageRef = image_ref
        .parse()
        .map_err(|e: String| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    let auth = resolve_registry_auth(&image.registry)?;
    let mut client = RegistryClient::new().with_auth(auth);

    let bundle_path = images_dir.join(&image.repository).join(&image.tag);

    println!("Pushing {}...", image_ref);

    let rt = edgerun_rt::Runtime::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| std::io::Error::other(e.to_string()))?;

    let image_clone = image.clone();
    let bundle_clone = bundle_path.clone();
    let result = rt.block_on(async move { client.push(&image_clone, &bundle_clone).await });

    match result {
        Ok(()) => {
            println!("Pushed {}", image_ref);
            Ok(())
        }
        Err(e) => Err(std::io::Error::other(format!("push failed: {}", e))),
    }
}

fn parse_push_args(args: &[String]) -> std::io::Result<(String, PathBuf)> {
    let default_images = PathBuf::from("/var/lib/edgerun/images");
    let mut images_dir = default_images;
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
            "Usage: ert push [--images-dir DIR] <image>",
        )
    })?;

    Ok((image, images_dir))
}
