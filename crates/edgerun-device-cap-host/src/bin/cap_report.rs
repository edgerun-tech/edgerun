// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

fn main() {
    let mut details = false;
    let mut output: Option<PathBuf> = None;
    for arg in std::env::args().skip(1) {
        if arg == "--details" {
            details = true;
            continue;
        }
        output = Some(PathBuf::from(arg));
    }

    let default = if details {
        PathBuf::from("out/bench/capability_report_details.pb")
    } else {
        PathBuf::from("out/bench/capability_report.pb")
    };
    let output = output.unwrap_or(default);
    if let Some(parent) = output.parent() {
        if let Err(err) = std::fs::create_dir_all(parent) {
            eprintln!(
                "failed to create output directory {}: {err}",
                parent.display()
            );
            std::process::exit(1);
        }
    }

    let payload = if details {
        let report = edgerun_device_cap_host::probe_capabilities_with_host_details();
        edgerun_device_cap_host::proto::encode_capability_report_with_details(&report)
    } else {
        let report = edgerun_device_cap_host::probe_capabilities_with_host();
        edgerun_device_cap_host::proto::encode_capability_report(&report)
    };

    match payload {
        Ok(bytes) => match std::fs::write(&output, bytes) {
            Ok(_) => println!("capability report written: {}", output.display()),
            Err(err) => {
                eprintln!("failed to write {}: {err}", output.display());
                std::process::exit(1);
            }
        },
        Err(err) => {
            eprintln!("capability report encode failed: {err}");
            std::process::exit(1);
        }
    }
}
