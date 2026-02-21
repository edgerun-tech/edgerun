use std::env;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");

    let pkg_version = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".to_string());
    let count = git_stdout(["rev-list", "--count", "HEAD"]);
    let sha = git_stdout(["rev-parse", "--short", "HEAD"]);

    if let (Some(count), Some(sha)) = (count, sha) {
        if !count.is_empty() && !sha.is_empty() {
            let version = format!("{pkg_version}+{count}.g{sha}");
            println!("cargo:rustc-env=TERM_BUILD_VERSION={version}");
        }
    }
}

fn git_stdout(args: [&str; 3]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}
