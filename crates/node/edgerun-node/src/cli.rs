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

use crate::features_cmd::{cmd_features, cmd_self_test, cmd_self_test_child};
use crate::init_cmd::{cmd_init, cmd_init_encrypted, cmd_init_provisioned, cmd_provision};
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
    },
    InitProvisioned {
        config: PathBuf,
        name: Option<String>,
        controller: Option<String>,
    },
    Provision {
        config: PathBuf,
        pin: String,
        target_addr: Option<String>,
    },
    Status {
        config: PathBuf,
    },
    Features,
    SelfTest,
    SelfTestChild,
}

pub fn parse_args() -> Result<Command, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        return Err(help_text());
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
                    "--config" => {
                        i += 1;
                        config = PathBuf::from(&args[i]);
                    }
                    "--name" => {
                        i += 1;
                        name = Some(args[i].clone());
                    }
                    "--software" => {
                        software = true;
                    }
                    "--help" | "-h" => {
                        return Err(
                            "Usage: edgerund init [--config path] [--name name] [--software]"
                                .into(),
                        );
                    }
                    other => return Err(format!("unknown option: {}", other)),
                }
                i += 1;
            }
            Ok(Command::Init {
                config,
                name,
                software,
            })
        }
        "init-encrypted" => {
            let mut config = PathBuf::from("node.yaml");
            let mut key_file = PathBuf::from("node.key.enc");
            let mut name = None;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--config" => {
                        i += 1;
                        config = PathBuf::from(&args[i]);
                    }
                    "--key-file" => {
                        i += 1;
                        key_file = PathBuf::from(&args[i]);
                    }
                    "--name" => {
                        i += 1;
                        name = Some(args[i].clone());
                    }
                    "--help" | "-h" => {
                        return Err("Usage: edgerund init-encrypted [--config path] [--key-file path] [--name name]".into());
                    }
                    other => return Err(format!("unknown option: {}", other)),
                }
                i += 1;
            }
            Ok(Command::InitEncrypted {
                config,
                key_file,
                name,
            })
        }
        "init-provisioned" => {
            let mut config = PathBuf::from("node.yaml");
            let mut name = None;
            let mut controller = None;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--config" => {
                        i += 1;
                        config = PathBuf::from(&args[i]);
                    }
                    "--name" => {
                        i += 1;
                        name = Some(args[i].clone());
                    }
                    "--controller" => {
                        i += 1;
                        controller = Some(args[i].clone());
                    }
                    "--help" | "-h" => {
                        return Err("Usage: edgerund init-provisioned [--config path] [--name name] [--controller node-id]".into());
                    }
                    other => return Err(format!("unknown option: {}", other)),
                }
                i += 1;
            }
            Ok(Command::InitProvisioned {
                config,
                name,
                controller,
            })
        }
        "provision" => {
            let mut config = PathBuf::from("node.yaml");
            let mut pin = String::new();
            let mut target_addr = None;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--config" => {
                        i += 1;
                        config = PathBuf::from(&args[i]);
                    }
                    "--pin" => {
                        i += 1;
                        pin = args[i].clone();
                    }
                    "--target" => {
                        i += 1;
                        target_addr = Some(args[i].clone());
                    }
                    "--help" | "-h" => {
                        return Err(
                            "Usage: edgerund provision --config path --pin PIN [--target addr]"
                                .into(),
                        );
                    }
                    other => return Err(format!("unknown option: {}", other)),
                }
                i += 1;
            }
            if pin.is_empty() {
                return Err("error: --pin is required".into());
            }
            Ok(Command::Provision {
                config,
                pin,
                target_addr,
            })
        }
        "status" => {
            let mut config = PathBuf::from("node.yaml");
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--config" => {
                        i += 1;
                        config = PathBuf::from(&args[i]);
                    }
                    "--help" | "-h" => {
                        return Err("Usage: edgerund status [--config path]".into());
                    }
                    other => return Err(format!("unknown option: {}", other)),
                }
                i += 1;
            }
            Ok(Command::Status { config })
        }
        "features" | "capabilities" => Ok(Command::Features),
        "self-test" => Ok(Command::SelfTest),
        "self-test-child" => Ok(Command::SelfTestChild),
        "help" | "--help" | "-h" => Err(help_text()),
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
        } => {
            cmd_init_encrypted(&config, &key_file, name);
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
            target_addr,
        } => {
            cmd_provision(&config, &pin, target_addr);
        }
        Command::Status { config } => {
            cmd_status(&config);
        }
        Command::Features => {
            cmd_features();
        }
        Command::SelfTest => {
            cmd_self_test();
        }
        Command::SelfTestChild => {
            cmd_self_test_child();
        }
    }
}

fn help_text() -> String {
    "edgerun Node Daemon\n\nCommands:\n  init              Generate node identity\n  init-encrypted    Generate encrypted software identity\n  init-provisioned  Generate provisioned node identity\n  provision         Provision a node with a controller\n  status            Show node identity\n  features          Show compiled-in features/capabilities\n  self-test         Copy this executable and test the copy\n  help              Show this help".into()
}
