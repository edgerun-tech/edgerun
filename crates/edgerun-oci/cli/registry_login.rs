//! `ert registry login` — store registry bearer tokens via secret service.

use crate::prelude::*;
use std::io;

use crate::SecretClient;
use crate::cli::{GlobalOpts, invalid_input, parse_cli_args, required_positional};
use edgerun_clap::{Arg, Command};

pub fn cmd_login(_opts: &GlobalOpts, args: &[String]) -> io::Result<()> {
    const USAGE: &str = "Usage: ert registry login <registry> --token token";
    let matches = parse_cli_args(
        Command::new("registry login").arg(Arg::new("token").long("token")),
        args,
        USAGE,
    )?;
    if matches.positional_count() > 1 {
        return Err(invalid_input(USAGE));
    }

    let registry = required_positional(&matches, 0, USAGE)?;
    let token = matches
        .get_one::<String>("token")
        .ok_or_else(|| invalid_input(USAGE))?;

    println!("Storing registry token for {}...", registry);

    let mut client = SecretClient::connect()?;
    client.open_session()?;
    client.unlock()?;
    client.store_registry_credential(registry, token.as_bytes())?;

    println!("Token stored for {}", registry);
    Ok(())
}
