use std::io::{self, Read};

use edgerun_ui_core::initial_setup::file_store::PasswordRootFileStore;

fn main() {
    let args = parse_args();
    if args.setup_password_stdin || args.unlock_password_stdin {
        let password = match read_password_stdin() {
            Ok(password) => password,
            Err(error) => {
                eprintln!("edgerun-node-ui: {error}");
                std::process::exit(1);
            }
        };
        let store = PasswordRootFileStore::new(args.config);
        let result = if args.setup_password_stdin {
            store.create(&password).map(|status| {
                format!(
                    "password root configured: {} byte envelope",
                    status.envelope_len
                )
            })
        } else {
            store
                .unlock(&password)
                .map(|_| "password root unlocked".to_string())
        };
        match result {
            Ok(summary) => {
                println!("{summary}");
                return;
            }
            Err(error) => {
                eprintln!("edgerun-node-ui: {error:?}");
                std::process::exit(1);
            }
        }
    }
    if let Err(error) =
        edgerun_node::ui_sdl::run_window_for_frames_with_root(args.frames, args.config)
    {
        eprintln!("edgerun-node-ui: {error}");
        std::process::exit(1);
    }
}

struct Args {
    frames: Option<u32>,
    config: std::path::PathBuf,
    setup_password_stdin: bool,
    unlock_password_stdin: bool,
}

fn parse_args() -> Args {
    let mut frames = None;
    let mut config = std::env::var_os("EDGERUN_NODE_UI_ROOT")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(std::path::PathBuf::from)
                .map(|home| home.join(".edgerun").join("node-ui"))
        })
        .unwrap_or_else(|| std::path::PathBuf::from("node-ui-data"));
    let mut args = std::env::args().skip(1);
    let mut setup_password_stdin = false;
    let mut unlock_password_stdin = false;
    while let Some(arg) = args.next() {
        if arg == "--frames" {
            frames = args.next().and_then(|value| value.parse().ok());
        } else if arg == "--config" {
            if let Some(value) = args.next() {
                config = std::path::PathBuf::from(value);
            }
        } else if arg == "--setup-password-stdin" {
            setup_password_stdin = true;
        } else if arg == "--unlock-password-stdin" {
            unlock_password_stdin = true;
        }
    }
    Args {
        frames,
        config,
        setup_password_stdin,
        unlock_password_stdin,
    }
}

fn read_password_stdin() -> Result<String, String> {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|err| format!("read password from stdin failed: {err}"))?;
    while input.ends_with('\n') || input.ends_with('\r') {
        input.pop();
    }
    Ok(input)
}
