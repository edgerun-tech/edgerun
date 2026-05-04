use crate::artifact::ArtifactWriter;
use anyhow::Result;
use std::process::Command;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

pub fn collect(artifacts: &mut ArtifactWriter) -> Result<()> {
    let mut info = String::new();

    let output = Command::new("uname").arg("-a").output()?;
    let machine = String::from_utf8_lossy(&output.stdout);
    info.push_str(&format!("Machine: {}", machine));

    let output = Command::new("cat").arg("/proc/version").output()?;
    let version = String::from_utf8_lossy(&output.stdout);
    info.push_str(&format!("OS/kernel: {}", version));

    let output = Command::new("rustc").arg("--version").output()?;
    let rustc = String::from_utf8_lossy(&output.stdout);
    info.push_str(&format!("Rust version: {}", rustc));

    info.push_str("Target: x86_64-unknown-linux-musl\n");

    let output = Command::new("git").arg("rev-parse").arg("HEAD").output()?;
    let commit = String::from_utf8_lossy(&output.stdout);
    info.push_str(&format!("Commit: {}\n", commit.trim()));

    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let secs = duration.as_secs();
    info.push_str(&format!("Date: unix timestamp {}\n", secs));

    artifacts.write("system-info.txt", &info)?;
    Ok(())
}
