//! edgerun Node Daemon (edgerund)
//!
//! Runs a single-writer stream node with mesh networking,
//! command processing, and capability discovery.
//!
//! ## Usage
//! ```text
//! edgerund init --config node.yaml --software          # Dev-only: in-memory key
//! edgerund status --config node.yaml                   # Show node identity
//! ```
//!
//! ## Security
//! The node's private key NEVER leaves secure hardware. The config only stores
//! the public key (NodeID) and a reference to the hardware key handle.
//! No `.key` file is ever written.

use std::env;
use std::path::PathBuf;

use crate::init_cmd::{
    cmd_init, cmd_init_encrypted, cmd_init_provisioned, cmd_provision, cmd_unlock,
};
use crate::status_cmd::cmd_status;

/// Parsed CLI arguments.
pub enum Command {
    Init {
        config: PathBuf,
        name: Option<String>,
        software: bool,
    },
    InitEncrypted {
        config: PathBuf,
        key_file: PathBuf,
        name: Option<String>,
        passphrase: Option<String>,
    },
    InitProvisioned {
        config: PathBuf,
        name: Option<String>,
        controller: Option<String>,
    },
    Provision {
        config: PathBuf,
        pin: String,
        password: Option<String>,
        target_addr: Option<String>,
    },
    Unlock {
        config: PathBuf,
        password: Option<String>,
        target_addr: Option<String>,
    },
    Status {
        config: PathBuf,
    },
}

pub fn parse_args() -> Result<Command, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        return Err("Usage: edgerund <command> [options]\n\nCommands:\n  init    Generate node identity\n  status  Show node identity\n  help    Show this help".to_string());
    }
    let cmd = args[0].as_str();
    match cmd {
        "init" => {
            let mut config = PathBuf::from("node.yaml");
            let mut name = None;
            let mut software = false;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--config" => { i += 1; config = PathBuf::from(&args[i]); }
                    "--name" => { i += 1; name = Some(args[i].clone()); }
                    "--software" => { software = true; }
                    "--help" | "-h" => {
                        return Err("Usage: edgerund init [--config path] [--name name] [--software]".into());
                    }
                    other => return Err(format!("unknown option: {}", other)),
                }
                i += 1;
            }
            Ok(Command::Init { config, name, software })
        }
        "init-encrypted" => {
            let mut config = PathBuf::from("node.yaml");
            let mut key_file = PathBuf::from("node.key.enc");
            let mut name = None;
            let mut passphrase = None;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--config" => { i += 1; config = PathBuf::from(&args[i]); }
                    "--key-file" => { i += 1; key_file = PathBuf::from(&args[i]); }
                    "--name" => { i += 1; name = Some(args[i].clone()); }
                    "--passphrase" => { i += 1; passphrase = Some(args[i].clone()); }
                    "--help" | "-h" => {
                        return Err("Usage: edgerund init-encrypted [--config path] [--key-file path] [--name name] [--passphrase phrase]".into());
                    }
                    other => return Err(format!("unknown option: {}", other)),
                }
                i += 1;
            }
            Ok(Command::InitEncrypted { config, key_file, name, passphrase })
        }
        "init-provisioned" => {
            let mut config = PathBuf::from("node.yaml");
            let mut name = None;
            let mut controller = None;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--config" => { i += 1; config = PathBuf::from(&args[i]); }
                    "--name" => { i += 1; name = Some(args[i].clone()); }
                    "--controller" => { i += 1; controller = Some(args[i].clone()); }
                    "--help" | "-h" => {
                        return Err("Usage: edgerund init-provisioned [--config path] [--name name] [--controller node-id]".into());
                    }
                    other => return Err(format!("unknown option: {}", other)),
                }
                i += 1;
            }
            Ok(Command::InitProvisioned { config, name, controller })
        }
        "provision" => {
            let mut config = PathBuf::from("node.yaml");
            let mut pin = String::new();
            let mut password = None;
            let mut target_addr = None;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--config" => { i += 1; config = PathBuf::from(&args[i]); }
                    "--pin" => { i += 1; pin = args[i].clone(); }
                    "--password" => { i += 1; password = Some(args[i].clone()); }
                    "--target" => { i += 1; target_addr = Some(args[i].clone()); }
                    "--help" | "-h" => {
                        return Err("Usage: edgerund provision --config path --pin PIN [--password pass] [--target addr]".into());
                    }
                    other => return Err(format!("unknown option: {}", other)),
                }
                i += 1;
            }
            if pin.is_empty() {
                return Err("error: --pin is required".into());
            }
            Ok(Command::Provision { config, pin, password, target_addr })
        }
        "unlock" => {
            let mut config = PathBuf::from("node.yaml");
            let mut password = None;
            let mut target_addr = None;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--config" => { i += 1; config = PathBuf::from(&args[i]); }
                    "--password" => { i += 1; password = Some(args[i].clone()); }
                    "--target" => { i += 1; target_addr = Some(args[i].clone()); }
                    "--help" | "-h" => {
                        return Err("Usage: edgerund unlock [--config path] [--password pass] [--target addr]".into());
                    }
                    other => return Err(format!("unknown option: {}", other)),
                }
                i += 1;
            }
            Ok(Command::Unlock { config, password, target_addr })
        }
        "status" => {
            let mut config = PathBuf::from("node.yaml");
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--config" => { i += 1; config = PathBuf::from(&args[i]); }
                    "--help" | "-h" => {
                        return Err("Usage: edgerund status [--config path]".into());
                    }
                    other => return Err(format!("unknown option: {}", other)),
                }
                i += 1;
            }
            Ok(Command::Status { config })
        }
        "help" | "--help" | "-h" => {
            Err("edgerun Node Daemon\n\nCommands:\n  init    Generate node identity\n  status  Show node identity".into())
        }
        other => Err(format!("unknown command: {}", other)),
    }
}

pub fn main() {
    let cmd = match parse_args() {
        Ok(c) => c,
        Err(msg) => {
            eprintln!("{}", msg);
            std::process::exit(1);
        }
    };
    match cmd {
        Command::Init {
            config,
            name,
            software,
        } => {
            cmd_init(&config, name, software);
        }
        Command::InitEncrypted {
            config,
            key_file,
            name,
            passphrase,
        } => {
            cmd_init_encrypted(&config, &key_file, name, passphrase);
        }
        Command::InitProvisioned {
            config,
            name,
            controller,
        } => {
            cmd_init_provisioned(&config, name, controller);
        }
        Command::Provision {
            config,
            pin,
            password,
            target_addr,
        } => {
            cmd_provision(&config, &pin, password, target_addr);
        }
        Command::Unlock {
            config,
            password,
            target_addr,
        } => {
            cmd_unlock(&config, password, target_addr);
        }
        Command::Status { config } => {
            cmd_status(&config);
        }
    }
}
