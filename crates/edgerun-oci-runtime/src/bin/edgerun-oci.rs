//! OCI-compatible CLI wrapper for edgerun-oci-runtime.
//!
//! Implements the commands needed by the `opencontainers/runtime-tools` conformance suite:
//! - `create` — Set up container, clone namespaces, prepare rootfs, fork but don't exec
//! - `start` — Resume exec the container process
//! - `state` — Output container state JSON to stdout
//! - `kill` — Send signal to container process
//! - `delete` — Stop and cleanup container state

use edgerun_oci_runtime::cli::{self, parse_args, print_usage};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        print_usage();
        std::process::exit(1);
    }

    let Some((opts, command, cmd_args)) = parse_args(&args) else {
        print_usage();
        std::process::exit(1);
    };

    let result = match command.as_str() {
        "create" => cli::cmd_create(&opts, &cmd_args),
        "start" => cli::cmd_start(&opts, &cmd_args),
        "state" => cli::cmd_state(&opts, &cmd_args),
        "kill" => cli::cmd_kill(&opts, &cmd_args),
        "delete" => cli::cmd_delete(&opts, &cmd_args),
        _ => {
            eprintln!("Unknown command: {}", command);
            print_usage();
            std::process::exit(127);
        }
    };

    if let Err(e) = result {
        eprintln!("edgerun-oci: {}: {}", command, e);
        std::process::exit(1);
    }
}
