// SPDX-License-Identifier: Apache-2.0
use std::ffi::OsString;
use std::path::Path;
use std::process::Stdio;

use anyhow::{anyhow, Context, Result};

pub(crate) fn run_program_sync(
    label: &str,
    program: &str,
    args: &[&str],
    cwd: &Path,
    allow_missing: bool,
) -> Result<()> {
    run_program_sync_with_env(label, program, args, cwd, allow_missing, &[])
}

pub(crate) fn run_program_sync_with_env(
    label: &str,
    program: &str,
    args: &[&str],
    cwd: &Path,
    allow_missing: bool,
    envs: &[(OsString, OsString)],
) -> Result<()> {
    let display = if args.is_empty() {
        program.to_string()
    } else {
        format!("{program} {}", args.join(" "))
    };
    println!("==> {label}");
    println!("$ {display}");

    let mut command = std::process::Command::new(program);
    command
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    for (k, v) in envs {
        command.env(k, v);
    }
    let status = command.status();

    match status {
        Ok(status) => {
            if status.success() {
                Ok(())
            } else {
                Err(anyhow!(
                    "step '{}' failed with exit status {:?}",
                    label,
                    status.code()
                ))
            }
        }
        Err(err) if allow_missing && err.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("[warn] optional command missing: {program}");
            Ok(())
        }
        Err(err) => Err(err).with_context(|| format!("failed to launch: {display}")),
    }
}

pub(crate) fn run_program_sync_owned(
    label: &str,
    program: &str,
    args: &[String],
    cwd: &Path,
    allow_missing: bool,
) -> Result<()> {
    let borrowed: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    run_program_sync_with_env(label, program, &borrowed, cwd, allow_missing, &[])
}

pub(crate) fn run_program_capture_sync_owned(
    label: &str,
    program: &str,
    args: &[String],
    cwd: &Path,
    envs: &[(OsString, OsString)],
) -> Result<String> {
    let display = if args.is_empty() {
        program.to_string()
    } else {
        format!("{program} {}", args.join(" "))
    };
    println!("==> {label}");
    println!("$ {display}");

    let mut command = std::process::Command::new(program);
    command
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in envs {
        command.env(k, v);
    }
    let output = command
        .output()
        .with_context(|| format!("failed to launch: {display}"))?;

    let mut text = String::new();
    text.push_str(&String::from_utf8_lossy(&output.stdout));
    if !output.stdout.is_empty() && !output.stderr.is_empty() {
        text.push('\n');
    }
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    print!("{text}");

    if output.status.success() {
        Ok(text)
    } else {
        Err(anyhow!(
            "step '{}' failed with exit status {:?}",
            label,
            output.status.code()
        ))
    }
}

pub(crate) fn command_exists(cmd: &str) -> bool {
    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(cmd);
        if candidate.is_file() {
            return true;
        }
    }
    false
}
