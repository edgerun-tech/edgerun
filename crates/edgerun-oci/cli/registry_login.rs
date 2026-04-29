//! `ert registry login` — authenticate and store credentials via secret service.

use crate::prelude::*;
use std::io::{self, Write};

use crate::cli::{invalid_input, parse_cli_args, required_positional, GlobalOpts};
use crate::SecretClient;
use edgerun_clap::{Arg, Command};

pub fn cmd_login(_opts: &GlobalOpts, args: &[String]) -> io::Result<()> {
    const USAGE: &str = "Usage: ert registry login <registry> [-u username] [-p password]";
    let matches = parse_cli_args(
        Command::new("registry login")
            .arg(Arg::new("username").short('u').long("username"))
            .arg(Arg::new("password").short('p').long("password")),
        args,
        USAGE,
    )?;
    if matches.positional_count() > 1 {
        return Err(invalid_input(USAGE));
    }

    let registry = required_positional(&matches, 0, USAGE)?;
    let username = matches.get_one::<String>("username");
    let password = matches.get_one::<String>("password");
    let (user, pass) = if let (Some(u), Some(p)) = (username, password) {
        (u, p)
    } else {
        prompt_credentials(registry)?
    };

    println!(
        "Storing credential for {} (biometric verification required)...",
        registry
    );

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
