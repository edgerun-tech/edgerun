//! `ert registry logout` — remove registry credentials.

use crate::cli::GlobalOpts;
use crate::SecretClient;

pub fn cmd_logout(_opts: &GlobalOpts, args: &[String]) -> std::io::Result<()> {
    if args.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Usage: ert registry logout <registry> [--all]",
        ));
    }

    if args.iter().any(|a| a == "--all") {
        return logout_all();
    }

    let registry = &args[0];
    println!("Removing credential for {}...", registry);

    let mut client = SecretClient::connect()?;
    client.open_session()?;
    client.unlock()?;

    let deleted = client.delete_registry_credential(registry)?;

    if deleted {
        println!("Logged out from {}", registry);
    } else {
        println!("No credential found for {}", registry);
    }

    Ok(())
}

fn logout_all() -> std::io::Result<()> {
    println!("Removing all registry credentials...");

    let mut client = SecretClient::connect()?;
    client.open_session()?;
    client.unlock()?;

    let known_registries = [
        "docker.io",
        "ghcr.io",
        "gcr.io",
        "registry.gitlab.com",
        "quay.io",
        "mcr.microsoft.com",
        "public.ecr.aws",
    ];

    let mut count = 0;
    for registry in &known_registries {
        match client.delete_registry_credential(registry) {
            Ok(true) => {
                println!("  Removed: {}", registry);
                count += 1;
            }
            Ok(false) => {}
            Err(_) => {}
        }
    }

    if count == 0 {
        println!("No stored registry credentials found.");
    } else {
        println!("Removed {} credential(s).", count);
    }

    Ok(())
}
