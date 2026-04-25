//! `ert push` command — push an image to a registry.

use std::path::PathBuf;

use crate::cli::GlobalOpts;
use crate::ImageRef;
use crate::RegistryClient;
use crate::SecretClient;

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

fn resolve_registry_auth(registry: &str) -> std::io::Result<crate::RegistryAuth> {
    let mut secret_client = match SecretClient::connect() {
        Ok(c) => c,
        Err(_) => return Ok(crate::RegistryAuth::Anonymous),
    };

    if secret_client.open_session().is_err() {
        return Ok(crate::RegistryAuth::Anonymous);
    }

    match secret_client.get_registry_credential(registry) {
        Ok(secret_bytes) => {
            let secret_str = String::from_utf8(secret_bytes).map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "invalid credential encoding",
                )
            })?;
            let (username, password) = secret_str.split_once(':').ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "credential must be username:password",
                )
            })?;
            Ok(crate::RegistryAuth::Basic {
                username: username.to_string(),
                password: password.to_string(),
            })
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(crate::RegistryAuth::Anonymous),
        Err(_) => Ok(crate::RegistryAuth::Anonymous),
    }
}
