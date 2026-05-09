//! `ert push` command — push an image to a registry.

use crate::prelude::*;
use std::path::PathBuf;

use crate::ImageRef;
use crate::RegistryClient;
use crate::cli::{
    GlobalOpts, default_images_dir, invalid_input, parse_cli_args, required_positional,
    resolve_registry_auth,
};
use edgerun_clap::{Arg, Command};

pub fn cmd_push(_opts: &GlobalOpts, args: &[String]) -> std::io::Result<()> {
    let (image_ref, images_dir) = parse_push_args(args)?;

    let image: ImageRef = image_ref
        .parse()
        .map_err(|e: String| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    let auth = resolve_registry_auth(&image.registry)?;
    let mut client = RegistryClient::new().with_auth(auth);

    let bundle_path = images_dir.join(&image.repository).join(&image.tag);

    println!("Pushing {}...", image_ref);

    let rt = edgerun_node::rt::Runtime::new_multi_thread()
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
    const USAGE: &str = "Usage: ert push [--images-dir DIR] <image>";
    let matches = parse_cli_args(
        Command::new("push").arg(Arg::new("images-dir").long("images-dir")),
        args,
        USAGE,
    )?;
    if matches.positional_count() > 1 {
        return Err(invalid_input(USAGE));
    }
    let image = required_positional(&matches, 0, USAGE)?.to_string();
    let images_dir = matches
        .get_one::<PathBuf>("images-dir")
        .unwrap_or_else(default_images_dir);
    Ok((image, images_dir))
}
