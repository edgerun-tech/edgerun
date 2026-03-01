// SPDX-License-Identifier: GPL-2.0-only
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};
use std::time::{SystemTime, UNIX_EPOCH};

use edgerun_storage::virtual_fs::{
    FsDeltaAppliedV1, FsDeltaProposedV1, FsDeltaRejectedV1, StorageBackedVirtualFs,
};

fn usage() {
    eprintln!(
        "Usage:
  proposal_gatekeeper --data-dir PATH --repo-id ID --branch ID --proposal-id ID --repo-root PATH [--fmt-cmd \"cargo fmt --all\"] [--check-cmd \"cargo check --workspace\"] [--timeout-secs N] [--dry-run]
  proposal_gatekeeper --data-dir PATH --repo-id ID --branch ID --repo-root PATH --diff-file PATH --agent-id ID --intent TEXT [--proposal-id ID] [--submit-only] [--fmt-cmd \"cargo fmt --all\"] [--check-cmd \"cargo check --workspace\"] [--timeout-secs N] [--dry-run]"
    );
}

fn main() {
    let mut data_dir: Option<PathBuf> = None;
    let mut repo_id: Option<String> = None;
    let mut branch_id: Option<String> = None;
    let mut proposal_id: Option<String> = None;
    let mut agent_id: Option<String> = None;
    let mut intent: Option<String> = None;
    let mut diff_file: Option<PathBuf> = None;
    let mut repo_root: Option<PathBuf> = None;
    let mut fmt_cmd: String = "cargo fmt --all".to_string();
    let mut check_cmd: String = "cargo check --workspace".to_string();
    let mut timeout_secs: u64 = 300;
    let mut dry_run = false;
    let mut submit_only = false;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--data-dir" => data_dir = args.next().map(PathBuf::from),
            "--repo-id" => repo_id = args.next(),
            "--branch" => branch_id = args.next(),
            "--proposal-id" => proposal_id = args.next(),
            "--agent-id" => agent_id = args.next(),
            "--intent" => intent = args.next(),
            "--diff-file" => diff_file = args.next().map(PathBuf::from),
            "--repo-root" => repo_root = args.next().map(PathBuf::from),
            "--fmt-cmd" => fmt_cmd = args.next().unwrap_or_else(|| "cargo fmt --all".to_string()),
            "--check-cmd" => {
                check_cmd = args
                    .next()
                    .unwrap_or_else(|| "cargo check --workspace".to_string())
            }
            "--timeout-secs" => {
                let raw = args.next().unwrap_or_else(|| "300".to_string());
                timeout_secs = raw.parse::<u64>().unwrap_or(300).max(1);
            }
            "--dry-run" => dry_run = true,
            "--submit-only" => submit_only = true,
            "--help" | "-h" => {
                usage();
                return;
            }
            _ => {
                eprintln!("unknown arg: {arg}");
                usage();
                std::process::exit(2);
            }
        }
    }

    let Some(data_dir) = data_dir else {
        usage();
        std::process::exit(2);
    };
    let Some(repo_id) = repo_id else {
        usage();
        std::process::exit(2);
    };
    let Some(branch_id) = branch_id else {
        usage();
        std::process::exit(2);
    };
    let Some(repo_root) = repo_root else {
        usage();
        std::process::exit(2);
    };

    let mut vfs = match StorageBackedVirtualFs::open_writer(data_dir, &repo_id) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("failed to open vfs: {e}");
            std::process::exit(1);
        }
    };

    let ingest_mode = diff_file.is_some();
    let (proposal_id, proposal) = if ingest_mode {
        let Some(diff_path) = diff_file else {
            usage();
            std::process::exit(2);
        };
        let Some(agent_id) = agent_id else {
            usage();
            std::process::exit(2);
        };
        let Some(intent) = intent else {
            usage();
            std::process::exit(2);
        };
        let proposal_id = proposal_id.unwrap_or_else(|| format!("agent-{}", now_unix_ms()));
        let diff_unified = match std::fs::read(&diff_path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("failed to read diff file {}: {e}", diff_path.display());
                std::process::exit(1);
            }
        };
        let proposal = FsDeltaProposedV1 {
            schema_version: 1,
            repo_id: repo_id.clone(),
            proposal_id: proposal_id.clone(),
            branch_id: branch_id.clone(),
            base_cursor: None,
            agent_id,
            intent,
            diff_unified,
        };
        if let Err(e) = vfs.propose_delta(&proposal) {
            eprintln!("failed to append proposal event: {e}");
            std::process::exit(1);
        }
        println!("proposal_id={proposal_id}");
        if submit_only {
            println!("submitted_only=true");
            return;
        }
        (proposal_id, proposal)
    } else {
        let Some(proposal_id) = proposal_id else {
            usage();
            std::process::exit(2);
        };
        let Some(proposal) = (match vfs.find_proposal(&branch_id, &proposal_id) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("failed to read proposal: {e}");
                std::process::exit(1);
            }
        }) else {
            eprintln!("proposal not found: branch={branch_id} proposal_id={proposal_id}");
            std::process::exit(1);
        };
        (proposal_id, proposal)
    };

    let worktree_path = match make_temp_worktree_path(&repo_root) {
        Ok(p) => p,
        Err(e) => {
            reject_and_exit(
                &mut vfs,
                &repo_id,
                &branch_id,
                &proposal_id,
                &format!("failed to create temp path: {e}"),
            );
        }
    };

    if let Err(e) = run_git(
        &repo_root,
        &[
            "worktree",
            "add",
            "--detach",
            worktree_path.to_string_lossy().as_ref(),
            "HEAD",
        ],
    ) {
        reject_and_exit(
            &mut vfs,
            &repo_id,
            &branch_id,
            &proposal_id,
            &format!("failed to create detached worktree: {e}"),
        );
    }
    if let Err(e) = sync_workspace_into_worktree(&repo_root, &worktree_path) {
        reject_and_exit(
            &mut vfs,
            &repo_id,
            &branch_id,
            &proposal_id,
            &format!("failed to sync workspace into detached worktree: {e}"),
        );
    }

    let result = run_gate(
        &mut vfs,
        &repo_id,
        &branch_id,
        &proposal,
        &proposal_id,
        &repo_root,
        &worktree_path,
        &fmt_cmd,
        &check_cmd,
        Duration::from_secs(timeout_secs),
        dry_run,
    );

    let _ = run_git(
        &repo_root,
        &[
            "worktree",
            "remove",
            "--force",
            worktree_path.to_string_lossy().as_ref(),
        ],
    );

    match result {
        Ok(applied_id) => {
            if dry_run {
                println!("dry-run accept: proposal would be applied as {applied_id}");
            } else {
                println!("proposal accepted and applied: {applied_id}");
            }
        }
        Err(reason) => {
            if dry_run {
                eprintln!("dry-run reject: {reason}");
                std::process::exit(1);
            }
            reject_and_exit(&mut vfs, &repo_id, &branch_id, &proposal_id, &reason);
        }
    }
}

fn run_gate(
    vfs: &mut StorageBackedVirtualFs,
    repo_id: &str,
    branch_id: &str,
    proposal: &FsDeltaProposedV1,
    original_proposal_id: &str,
    _repo_root: &Path,
    worktree_path: &Path,
    fmt_cmd: &str,
    check_cmd: &str,
    timeout: Duration,
    dry_run: bool,
) -> Result<String, String> {
    let patch_path = worktree_path.join(".proposal.patch");
    std::fs::write(&patch_path, &proposal.diff_unified)
        .map_err(|e| format!("failed to write patch file: {e}"))?;

    run_git(
        worktree_path,
        &[
            "apply",
            "--whitespace=nowarn",
            patch_path.to_string_lossy().as_ref(),
        ],
    )
    .map_err(|e| format!("patch apply failed: {e}"))?;

    run_cmd_line_with_timeout(worktree_path, fmt_cmd, timeout)
        .map_err(|e| format!("format command failed: {e}"))?;
    run_cmd_line_with_timeout(worktree_path, check_cmd, timeout)
        .map_err(|e| format!("check command failed: {e}"))?;

    let formatted_diff = run_git_bytes(worktree_path, &["diff", "--binary", "--no-ext-diff"])
        .map_err(|e| format!("failed to read formatted diff: {e}"))?;

    let chosen_proposal_id = if formatted_diff != proposal.diff_unified {
        let formatted_id = format!("{}-fmt-{}", original_proposal_id, now_unix_ms());
        let formatted = FsDeltaProposedV1 {
            schema_version: 1,
            repo_id: repo_id.to_string(),
            proposal_id: formatted_id.clone(),
            branch_id: branch_id.to_string(),
            base_cursor: proposal.base_cursor.clone(),
            agent_id: "proposal-gatekeeper".to_string(),
            intent: format!("formatted+validated from {original_proposal_id}"),
            diff_unified: formatted_diff,
        };
        if !dry_run {
            vfs.propose_delta(&formatted)
                .map_err(|e| format!("failed to append formatted proposal event: {e}"))?;
        }
        formatted_id
    } else {
        original_proposal_id.to_string()
    };

    if dry_run {
        return Ok(chosen_proposal_id);
    }

    let applied = FsDeltaAppliedV1 {
        schema_version: 1,
        repo_id: repo_id.to_string(),
        proposal_id: chosen_proposal_id.clone(),
        branch_id: branch_id.to_string(),
        base_cursor: proposal.base_cursor.clone(),
        resulting_cursor: None,
        applied_by: "proposal-gatekeeper".to_string(),
    };
    vfs.apply_delta(&applied)
        .map_err(|e| format!("failed to append applied event: {e}"))?;

    Ok(chosen_proposal_id)
}

fn reject_and_exit(
    vfs: &mut StorageBackedVirtualFs,
    repo_id: &str,
    branch_id: &str,
    proposal_id: &str,
    reason: &str,
) -> ! {
    let rejected = FsDeltaRejectedV1 {
        schema_version: 1,
        repo_id: repo_id.to_string(),
        proposal_id: proposal_id.to_string(),
        branch_id: branch_id.to_string(),
        rejected_by: "proposal-gatekeeper".to_string(),
        reason: reason.to_string(),
        conflict_paths: Vec::new(),
    };
    let _ = vfs.reject_delta(&rejected);
    eprintln!("proposal rejected: {reason}");
    std::process::exit(1);
}

fn run_git(root: &Path, args: &[&str]) -> Result<(), String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

fn run_git_bytes(root: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(out.stdout)
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

fn run_cmd_line_with_timeout(root: &Path, cmdline: &str, timeout: Duration) -> Result<(), String> {
    let parts: Vec<String> = cmdline
        .split_whitespace()
        .map(ToString::to_string)
        .collect();
    if parts.is_empty() {
        return Err("empty command".to_string());
    }
    let bin = &parts[0];
    let args: Vec<&str> = parts.iter().skip(1).map(String::as_str).collect();
    run_cmd_timeout(root, bin, &args, timeout)
}

fn run_cmd_timeout(root: &Path, bin: &str, args: &[&str], timeout: Duration) -> Result<(), String> {
    let mut child = Command::new(bin)
        .current_dir(root)
        .args(args)
        .spawn()
        .map_err(|e| e.to_string())?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    return Ok(());
                }
                return Err(format!(
                    "command exited with status {status}: {bin} {}",
                    args.join(" ")
                ));
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "timed out after {}s: {bin} {}",
                        timeout.as_secs(),
                        args.join(" ")
                    ));
                }
                thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return Err(e.to_string()),
        }
    }
}

fn sync_workspace_into_worktree(repo_root: &Path, worktree_path: &Path) -> Result<(), String> {
    let out = Command::new("rsync")
        .arg("-a")
        .arg("--delete")
        .arg("--exclude")
        .arg(".git/")
        .arg(format!("{}/", repo_root.display()))
        .arg(format!("{}/", worktree_path.display()))
        .output()
        .map_err(|e| format!("failed to spawn rsync: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

fn make_temp_worktree_path(repo_root: &Path) -> Result<PathBuf, std::io::Error> {
    let ts = now_unix_ms();
    let pid = std::process::id();
    let dir = repo_root
        .join("..")
        .join(format!(".edgerun-proposal-gate-{pid}-{ts}"));
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
