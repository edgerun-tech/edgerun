// SPDX-License-Identifier: GPL-2.0-only
use std::path::PathBuf;

use edgerun_storage::virtual_fs::{
    FsDeltaProposedV1, SourceImportRequestV1, StorageBackedVirtualFs, VfsCursorV1, VfsEventTypeV1,
    VfsModeV1, VfsSourceKindV1, VirtualFsQueryFilter,
};
use prost::Message;

fn usage() {
    eprintln!(
        "Usage:
  vfs_operator import-git --data-dir PATH --repo-id ID --repo-path PATH --git-ref REF [--initiated-by ID]
  vfs_operator propose-diff --data-dir PATH --repo-id ID --branch ID --proposal-id ID --agent-id ID --intent TEXT --diff-file PATH
  vfs_operator list-proposals --data-dir PATH --repo-id ID --branch ID [--limit N] [--cursor N]
  vfs_operator list-events --data-dir PATH --repo-id ID [--branch ID] [--event-type NAME] [--limit N] [--cursor N]
  vfs_operator materialize --data-dir PATH --repo-id ID"
    );
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(cmd) = args.next() else {
        usage();
        std::process::exit(2);
    };

    let rest: Vec<String> = args.collect();
    let res = match cmd.as_str() {
        "import-git" => cmd_import_git(&rest),
        "propose-diff" => cmd_propose_diff(&rest),
        "list-proposals" => cmd_list_proposals(&rest),
        "list-events" => cmd_list_events(&rest),
        "materialize" => cmd_materialize(&rest),
        "--help" | "-h" | "help" => {
            usage();
            Ok(())
        }
        _ => Err(format!("unknown command: {cmd}")),
    };

    if let Err(e) = res {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn parse_opt(args: &[String], key: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == key).map(|w| w[1].clone())
}

fn parse_usize(args: &[String], key: &str, default: usize) -> Result<usize, String> {
    match parse_opt(args, key) {
        Some(v) => v
            .parse::<usize>()
            .map_err(|e| format!("invalid {key}: {e}")),
        None => Ok(default),
    }
}

fn parse_u64(args: &[String], key: &str, default: u64) -> Result<u64, String> {
    match parse_opt(args, key) {
        Some(v) => v.parse::<u64>().map_err(|e| format!("invalid {key}: {e}")),
        None => Ok(default),
    }
}

fn require_opt(args: &[String], key: &str) -> Result<String, String> {
    parse_opt(args, key).ok_or_else(|| format!("missing required {key}"))
}

fn open_writer(args: &[String]) -> Result<StorageBackedVirtualFs, String> {
    let data_dir = PathBuf::from(require_opt(args, "--data-dir")?);
    let repo_id = require_opt(args, "--repo-id")?;
    StorageBackedVirtualFs::open_writer(data_dir, &repo_id).map_err(|e| format!("open writer: {e}"))
}

fn open_reader(args: &[String]) -> Result<StorageBackedVirtualFs, String> {
    let data_dir = PathBuf::from(require_opt(args, "--data-dir")?);
    let repo_id = require_opt(args, "--repo-id")?;
    StorageBackedVirtualFs::open_reader(data_dir, &repo_id).map_err(|e| format!("open reader: {e}"))
}

fn cmd_import_git(args: &[String]) -> Result<(), String> {
    let mut vfs = open_writer(args)?;
    let repo_id = require_opt(args, "--repo-id")?;
    let repo_path = PathBuf::from(require_opt(args, "--repo-path")?);
    let git_ref = require_opt(args, "--git-ref")?;
    let initiated_by = parse_opt(args, "--initiated-by").unwrap_or_else(|| "operator".to_string());

    let req = SourceImportRequestV1 {
        schema_version: 1,
        repo_id,
        source_kind: VfsSourceKindV1::VfsSourceKindGit as i32,
        mode: VfsModeV1::VfsModeCode as i32,
        source_locator: repo_path.display().to_string(),
        source_ref: git_ref.clone(),
        initiated_by: initiated_by.clone(),
    };
    let _ = vfs
        .import_source(&req, 0, 0, vec![0u8; 32], "{}".to_string())
        .map_err(|e| format!("import source event failed: {e}"))?;
    let report = vfs
        .import_git_repo(&repo_path, &git_ref, &initiated_by)
        .map_err(|e| format!("import git repo failed: {e}"))?;

    println!("import_source_kind=git");
    println!("import_mode=code");
    println!("imported_object_count={}", report.imported_object_count);
    println!("imported_bytes={}", report.imported_bytes);
    println!("first_event_offset={}", report.first_event_offset);
    println!("last_event_offset={}", report.last_event_offset);
    println!(
        "root_projection_hash={}",
        hex::encode(report.root_projection_hash)
    );
    Ok(())
}

fn cmd_propose_diff(args: &[String]) -> Result<(), String> {
    let mut vfs = open_writer(args)?;
    let repo_id = require_opt(args, "--repo-id")?;
    let branch = require_opt(args, "--branch")?;
    let proposal_id = require_opt(args, "--proposal-id")?;
    let agent_id = require_opt(args, "--agent-id")?;
    let intent = require_opt(args, "--intent")?;
    let diff_file = PathBuf::from(require_opt(args, "--diff-file")?);
    let diff_unified = std::fs::read(&diff_file)
        .map_err(|e| format!("failed to read diff file {}: {e}", diff_file.display()))?;

    let proposal = FsDeltaProposedV1 {
        schema_version: 1,
        repo_id,
        proposal_id: proposal_id.clone(),
        branch_id: branch.clone(),
        base_cursor: Some(VfsCursorV1 {
            branch_id: branch,
            seq: 0,
            head_event_hash: Vec::new(),
        }),
        agent_id,
        intent,
        diff_unified,
    };

    let offset = vfs
        .propose_delta(&proposal)
        .map_err(|e| format!("propose delta failed: {e}"))?;
    println!("proposal_id={proposal_id}");
    println!("event_offset={offset}");
    Ok(())
}

fn cmd_list_proposals(args: &[String]) -> Result<(), String> {
    let vfs = open_reader(args)?;
    let branch = require_opt(args, "--branch")?;
    let limit = parse_usize(args, "--limit", 200)?;
    let cursor = parse_u64(args, "--cursor", 0)?;

    let rows = vfs
        .query(
            limit,
            cursor,
            VirtualFsQueryFilter {
                event_type: Some(VfsEventTypeV1::VfsEventTypeFsDeltaProposed),
                branch_id: Some(branch),
            },
        )
        .map_err(|e| format!("query proposals failed: {e}"))?;

    for row in &rows.events {
        let env = &row.envelope;
        let p = FsDeltaProposedV1::decode(env.payload.as_slice())
            .map_err(|e| format!("decode proposal payload failed: {e}"))?;
        println!(
            "seq={} proposal_id={} agent_id={} intent={} diff_bytes={}",
            env.seq,
            p.proposal_id,
            p.agent_id,
            sanitize_single_line(&p.intent),
            p.diff_unified.len()
        );
    }
    println!("count={}", rows.events.len());
    if let Some(next) = rows.next_cursor_offset {
        println!("next_cursor_offset={next}");
    }
    Ok(())
}

fn cmd_list_events(args: &[String]) -> Result<(), String> {
    let vfs = open_reader(args)?;
    let branch_id = parse_opt(args, "--branch");
    let limit = parse_usize(args, "--limit", 200)?;
    let cursor = parse_u64(args, "--cursor", 0)?;
    let event_type = match parse_opt(args, "--event-type") {
        Some(name) => Some(parse_event_type_name(&name)?),
        None => None,
    };

    let rows = vfs
        .query(
            limit,
            cursor,
            VirtualFsQueryFilter {
                event_type,
                branch_id,
            },
        )
        .map_err(|e| format!("query events failed: {e}"))?;

    for row in &rows.events {
        println!(
            "seq={} branch={} type={} hash={} payload_bytes={}",
            row.envelope.seq,
            row.envelope.branch_id,
            event_type_name(row.envelope.event_type),
            row.event_hash,
            row.envelope.payload.len()
        );
    }
    println!("count={}", rows.events.len());
    if let Some(next) = rows.next_cursor_offset {
        println!("next_cursor_offset={next}");
    }
    Ok(())
}

fn cmd_materialize(args: &[String]) -> Result<(), String> {
    let vfs = open_reader(args)?;
    let state = vfs
        .materialize()
        .map_err(|e| format!("materialize failed: {e}"))?;
    println!("repo_id={}", state.repo_id);
    println!("source_kind={:?}", state.source_kind);
    println!("mode={:?}", state.mode);
    println!("imported={}", state.imported);
    println!("max_seq={}", state.max_seq);
    println!("branch_count={}", state.branches.len());
    for b in state.branches {
        println!(
            "branch={} head_seq={} proposed={} applied={} rejected={} log_entries={}",
            b.branch_id,
            b.head_seq,
            b.proposed_delta_count,
            b.applied_delta_count,
            b.rejected_delta_count,
            b.log_entry_count
        );
    }
    Ok(())
}

fn parse_event_type_name(name: &str) -> Result<VfsEventTypeV1, String> {
    let n = name.trim().to_ascii_lowercase();
    match n.as_str() {
        "source_imported" => Ok(VfsEventTypeV1::VfsEventTypeSourceImported),
        "blob_stored" => Ok(VfsEventTypeV1::VfsEventTypeBlobStored),
        "branch_created" => Ok(VfsEventTypeV1::VfsEventTypeBranchCreated),
        "branch_head_moved" => Ok(VfsEventTypeV1::VfsEventTypeBranchHeadMoved),
        "fs_delta_proposed" => Ok(VfsEventTypeV1::VfsEventTypeFsDeltaProposed),
        "fs_delta_applied" => Ok(VfsEventTypeV1::VfsEventTypeFsDeltaApplied),
        "fs_delta_rejected" => Ok(VfsEventTypeV1::VfsEventTypeFsDeltaRejected),
        "snapshot_checkpointed" => Ok(VfsEventTypeV1::VfsEventTypeSnapshotCheckpointed),
        "log_entry_appended" => Ok(VfsEventTypeV1::VfsEventTypeLogEntryAppended),
        "partition_declared" => Ok(VfsEventTypeV1::VfsEventTypePartitionDeclared),
        _ => Err(format!("unsupported --event-type value: {name}")),
    }
}

fn event_type_name(raw: i32) -> &'static str {
    match raw {
        x if x == VfsEventTypeV1::VfsEventTypeSourceImported as i32 => "source_imported",
        x if x == VfsEventTypeV1::VfsEventTypeBlobStored as i32 => "blob_stored",
        x if x == VfsEventTypeV1::VfsEventTypeBranchCreated as i32 => "branch_created",
        x if x == VfsEventTypeV1::VfsEventTypeBranchHeadMoved as i32 => "branch_head_moved",
        x if x == VfsEventTypeV1::VfsEventTypeFsDeltaProposed as i32 => "fs_delta_proposed",
        x if x == VfsEventTypeV1::VfsEventTypeFsDeltaApplied as i32 => "fs_delta_applied",
        x if x == VfsEventTypeV1::VfsEventTypeFsDeltaRejected as i32 => "fs_delta_rejected",
        x if x == VfsEventTypeV1::VfsEventTypeSnapshotCheckpointed as i32 => {
            "snapshot_checkpointed"
        }
        x if x == VfsEventTypeV1::VfsEventTypeLogEntryAppended as i32 => "log_entry_appended",
        x if x == VfsEventTypeV1::VfsEventTypePartitionDeclared as i32 => "partition_declared",
        _ => "unknown",
    }
}

fn sanitize_single_line(input: &str) -> String {
    input.replace('\n', " ").replace('\r', " ")
}
