use edgerun_error::Context;
use std::fs;
use std::path::Path;
use std::time::Duration;

pub async fn wait_for_pid_file(path: &Path) -> edgerun_error::Result<String> {
    let pid = edgerun_tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if let Ok(contents) = fs::read_to_string(path) {
                let trimmed = contents.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
            edgerun_tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .context("timed out waiting for pid file")?;

    Ok(pid)
}

pub fn process_is_alive(pid: &str) -> edgerun_error::Result<bool> {
    let status = std::process::Command::new("kill")
        .args(["-0", pid])
        .status()
        .context("failed to probe process liveness with kill -0")?;
    Ok(status.success())
}

async fn wait_for_process_exit_inner(pid: String) -> edgerun_error::Result<()> {
    loop {
        if !process_is_alive(&pid)? {
            return Ok(());
        }
        edgerun_tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

pub async fn wait_for_process_exit(pid: &str) -> edgerun_error::Result<()> {
    let pid = pid.to_string();
    edgerun_tokio::time::timeout(Duration::from_secs(2), wait_for_process_exit_inner(pid))
        .await
        .context("timed out waiting for process to exit")??;

    Ok(())
}
