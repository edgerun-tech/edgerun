//! `ert registry login` — authenticate and store credentials via secret service.

use std::io::{self, Write};

use crate::cli::GlobalOpts;
use crate::SecretClient;

pub fn cmd_login(_opts: &GlobalOpts, args: &[String]) -> io::Result<()> {
    if args.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Usage: ert registry login <registry> [-u username] [-p password]",
        ));
    }

    let registry = &args[0];

    let mut username: Option<String> = None;
    let mut password: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-u" | "--username" => {
                if i + 1 < args.len() {
                    username = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, "-u requires a value"));
                }
            }
            "-p" | "--password" => {
                if i + 1 < args.len() {
                    password = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, "-p requires a value"));
                }
            }
            _ => {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("unknown flag: {}", args[i])));
            }
        }
    }

    let (user, pass) = if let (Some(u), Some(p)) = (username, password) {
        (u, p)
    } else {
        prompt_credentials(registry)?
    };

    println!("Storing credential for {} (biometric verification required)...", registry);

    let mut client = SecretClient::connect()?;
    client.open_session()?;
    client.unlock()?;
    client.store_registry_credential(registry, format!("{}:{}", user, pass).as_bytes())?;

    println!("Login successful for {}", registry);
    Ok(())
}

fn prompt_credentials(registry: &str) -> io::Result<(String, String)> {
    print!("Username for {}: ", registry);
    io::stdout().flush()?;

    let mut username = String::new();
    io::stdin().read_line(&mut username)?;
    let username = username.trim().to_string();

    print!("Password: ");
    io::stdout().flush()?;

    let mut password = String::new();
    io::stdin().read_line(&mut password)?;
    let password = password.trim().to_string();

    Ok((username, password))
}
