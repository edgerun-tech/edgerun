// SPDX-License-Identifier: GPL-2.0-only
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use edgerun_storage::virtual_fs::{
    FsDeltaAppliedV1, FsDeltaProposedV1, FsDeltaRejectedV1, StorageBackedVirtualFs,
};

fn usage() {
    eprintln!(
        "Usage: proposal_batch_gatekeeper --data-dir PATH --repo-id ID --branch ID --proposal-ids id1,id2 --repo-root PATH [--fmt-cmd \"cargo fmt --all\"] [--check-cmd \"cargo check --workspace\"] [--timeout-secs N] [--dry-run]"
    );
}

fn main() {
    let mut data_dir: Option<PathBuf> = None;
    let mut repo_id: Option<String> = None;
    let mut branch_id: Option<String> = None;
    let mut proposal_ids_csv: Option<String> = None;
    let mut repo_root: Option<PathBuf> = None;
    let mut fmt_cmd: String = "cargo fmt --all".to_string();
    let mut check_cmd: String = "cargo check --workspace".to_string();
    let mut timeout_secs: u64 = 300;
    let mut dry_run = false;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--data-dir" => data_dir = args.next().map(PathBuf::from),
            "--repo-id" => repo_id = args.next(),
            "--branch" => branch_id = args.next(),
            "--proposal-ids" => proposal_ids_csv = args.next(),
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
    let Some(proposal_ids_csv) = proposal_ids_csv else {
        usage();
        std::process::exit(2);
    };
    let Some(repo_root) = repo_root else {
        usage();
        std::process::exit(2);
    };

    let proposal_ids: Vec<String> = proposal_ids_csv
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect();
    if proposal_ids.is_empty() {
        eprintln!("--proposal-ids must contain at least one id");
        std::process::exit(2);
    }

    let mut vfs = match StorageBackedVirtualFs::open_writer(data_dir, &repo_id) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("failed to open vfs: {e}");
            std::process::exit(1);
        }
    };

    let mut proposals = Vec::with_capacity(proposal_ids.len());
    for pid in &proposal_ids {
        let Some(p) = (match vfs.find_proposal(&branch_id, pid) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("failed to read proposal {pid}: {e}");
                std::process::exit(1);
            }
        }) else {
            eprintln!("proposal not found: branch={branch_id} proposal_id={pid}");
            std::process::exit(1);
        };
        proposals.push(p);
    }

    let worktree_path = match make_temp_worktree_path(&repo_root) {
        Ok(p) => p,
        Err(e) => {
            reject_and_exit(
                &mut vfs,
                &repo_id,
                &branch_id,
                &proposal_ids[0],
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
            &proposal_ids[0],
            &format!("failed to create detached worktree: {e}"),
        );
    }

    let result = run_batch(
        &mut vfs,
        &repo_id,
        &branch_id,
        &proposals,
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
        Ok(applied_ids) => {
            if dry_run {
                println!(
                    "dry-run accept: proposals would be applied: {}",
                    applied_ids.join(",")
                );
            } else {
                println!("batch accepted and applied: {}", applied_ids.join(","));
            }
        }
        Err((failed_id, reason)) => {
            if dry_run {
                eprintln!("dry-run reject on {failed_id}: {reason}");
                std::process::exit(1);
            }
            reject_and_exit(&mut vfs, &repo_id, &branch_id, &failed_id, &reason);
        }
    }
}

fn run_batch(
    vfs: &mut StorageBackedVirtualFs,
    repo_id: &str,
    branch_id: &str,
    proposals: &[FsDeltaProposedV1],
    _repo_root: &Path,
    worktree_path: &Path,
    fmt_cmd: &str,
    check_cmd: &str,
    timeout: Duration,
    dry_run: bool,
) -> Result<Vec<String>, (String, String)> {
    let _ = run_git(worktree_path, &["config", "user.email", "gatekeeper@local"]);
    let _ = run_git(
        worktree_path,
        &["config", "user.name", "Proposal Gatekeeper"],
    );

    let mut step_commits: Vec<(String, [u8; 32], String)> = Vec::new();

    for proposal in proposals {
        let patch_path = worktree_path.join(format!(".proposal-{}.patch", proposal.proposal_id));
        if let Err(e) = std::fs::write(&patch_path, &proposal.diff_unified) {
            return Err((
                proposal.proposal_id.clone(),
                format!("failed to write patch file: {e}"),
            ));
        }

        if let Err(e) = run_git(
            worktree_path,
            &[
                "apply",
                "--whitespace=nowarn",
                patch_path.to_string_lossy().as_ref(),
            ],
        ) {
            return Err((
                proposal.proposal_id.clone(),
                format!("patch apply failed: {e}"),
            ));
        }

        if let Err(e) = run_cmd_line_with_timeout(worktree_path, fmt_cmd, timeout) {
            return Err((
                proposal.proposal_id.clone(),
                format!("format command failed: {e}"),
            ));
        }
        if let Err(e) = run_cmd_line_with_timeout(worktree_path, check_cmd, timeout) {
            return Err((
                proposal.proposal_id.clone(),
                format!("check command failed: {e}"),
            ));
        }

        if let Err(e) = run_git(worktree_path, &["add", "-A"]) {
            return Err((proposal.proposal_id.clone(), format!("git add failed: {e}")));
        }
        if let Err(e) = run_git(
            worktree_path,
            &[
                "commit",
                "--allow-empty",
                "-m",
                &format!("gate-step:{}", proposal.proposal_id),
            ],
        ) {
            return Err((
                proposal.proposal_id.clone(),
                format!("git commit failed: {e}"),
            ));
        }
        let commit_hash = match run_git_bytes(worktree_path, &["rev-parse", "HEAD"]) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).trim().to_string(),
            Err(e) => {
                return Err((
                    proposal.proposal_id.clone(),
                    format!("failed to capture step commit hash: {e}"),
                ))
            }
        };

        let mut original_hash = [0u8; 32];
        original_hash.copy_from_slice(&blake3::hash(&proposal.diff_unified).as_bytes()[..32]);
        step_commits.push((proposal.proposal_id.clone(), original_hash, commit_hash));
    }

    let mut applied_ids = Vec::with_capacity(step_commits.len());
    for (proposal_id, original_hash, commit_hash) in step_commits {
        let step_patch = match run_git_bytes(
            worktree_path,
            &["show", "--binary", "--format=", &commit_hash],
        ) {
            Ok(bytes) => bytes,
            Err(e) => {
                return Err((
                    proposal_id,
                    format!("failed to read step patch from commit {commit_hash}: {e}"),
                ))
            }
        };
        let formatted_hash = blake3::hash(&step_patch);

        let winning_id = if formatted_hash.as_bytes() != &original_hash {
            let formatted_id = format!("{}-fmt-{}", proposal_id, now_unix_ms());
            let base_cursor = vfs
                .find_proposal(branch_id, &proposal_id)
                .ok()
                .flatten()
                .and_then(|p| p.base_cursor);
            let formatted = FsDeltaProposedV1 {
                schema_version: 1,
                repo_id: repo_id.to_string(),
                proposal_id: formatted_id.clone(),
                branch_id: branch_id.to_string(),
                base_cursor,
                agent_id: "proposal-batch-gatekeeper".to_string(),
                intent: format!("formatted+validated from {proposal_id}"),
                diff_unified: step_patch,
            };
            if !dry_run {
                if let Err(e) = vfs.propose_delta(&formatted) {
                    return Err((
                        proposal_id,
                        format!("failed to append formatted proposal event: {e}"),
                    ));
                }
            }
            formatted_id
        } else {
            proposal_id
        };

        if !dry_run {
            let applied = FsDeltaAppliedV1 {
                schema_version: 1,
                repo_id: repo_id.to_string(),
                proposal_id: winning_id.clone(),
                branch_id: branch_id.to_string(),
                base_cursor: None,
                resulting_cursor: None,
                applied_by: "proposal-batch-gatekeeper".to_string(),
            };
            if let Err(e) = vfs.apply_delta(&applied) {
                return Err((winning_id, format!("failed to append applied event: {e}")));
            }
        }

        applied_ids.push(winning_id);
    }

    Ok(applied_ids)
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
        rejected_by: "proposal-batch-gatekeeper".to_string(),
        reason: reason.to_string(),
        conflict_paths: Vec::new(),
    };
    let _ = vfs.reject_delta(&rejected);
    eprintln!("proposal rejected ({proposal_id}): {reason}");
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

fn make_temp_worktree_path(repo_root: &Path) -> Result<PathBuf, std::io::Error> {
    let ts = now_unix_ms();
    let pid = std::process::id();
    let dir = repo_root
        .join("..")
        .join(format!(".edgerun-proposal-batch-gate-{pid}-{ts}"));
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
