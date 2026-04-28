//! `ert push` command — push an image to a registry.

use crate::prelude::*;
use std::path::PathBuf;

use crate::cli::{
    default_images_dir, inline_value, invalid_input, resolve_registry_auth, CliArgs, GlobalOpts,
};
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
    let mut images_dir = default_images_dir();
    let mut image: Option<String> = None;

    let mut args = CliArgs::new(args);
    while let Some(arg) = args.next() {
        match arg {
            "--images-dir" => {
                images_dir = PathBuf::from(args.value("--images-dir requires a path")?);
            }
            arg if inline_value(arg, "--images-dir").is_some() => {
                images_dir = PathBuf::from(inline_value(arg, "--images-dir").unwrap());
            }
            arg if !arg.starts_with('-') => {
                image = Some(arg.to_string());
            }
            _ => {
                return Err(invalid_input(format!("unknown flag: {}", arg)));
            }
        }
    }

    let image = image.ok_or_else(|| invalid_input("Usage: ert push [--images-dir DIR] <image>"))?;

    Ok((image, images_dir))
}
