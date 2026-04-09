//! edgerun-secret-service — A Freedesktop Secret Service compatible D-Bus daemon.
//!
//! Listens on a Unix domain socket and implements the
//! `org.freedesktop.Secret.Service` API for secure credential storage.
//!
//! Usage:
//!   edgerun-secret-service [SOCKET_PATH] [DATA_ROOT]
//!
//! Defaults:
//!   SOCKET_PATH: /run/edgerun/secret.sock
//!   DATA_ROOT:  ~/.local/share/edgerun/secrets

use edgerun_secret_service::dbus_server::Server;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let socket_path = args.get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/run/edgerun/secret.sock"));

    let data_root = args.get(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs_data_root().unwrap_or_else(|| PathBuf::from("/tmp/edgerun-secrets"))
        });

    eprintln!("edgerun-secret-service v0.1");
    eprintln!("  socket: {}", socket_path.display());
    eprintln!("  data:   {}", data_root.display());

    // Ensure data root exists
    if let Err(e) = std::fs::create_dir_all(&data_root) {
        eprintln!("failed to create data root {}: {}", data_root.display(), e);
        std::process::exit(1);
    }

    match Server::bind(&socket_path, data_root) {
        Ok(mut server) => {
            eprintln!("ready — accepting connections");
            loop {
                if let Err(e) = server.accept_once() {
                    eprintln!("connection error: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("failed to bind to {}: {}", socket_path.display(), e);
            std::process::exit(1);
        }
    }
}

/// Determine the default data root directory.
fn dirs_data_root() -> Option<PathBuf> {
    if let Ok(data_home) = std::env::var("XDG_DATA_HOME") {
        return Some(PathBuf::from(data_home).join("edgerun/secrets"));
    }
    if let Ok(home) = std::env::var("HOME") {
        return Some(PathBuf::from(home).join(".local/share/edgerun/secrets"));
    }
    None
}
