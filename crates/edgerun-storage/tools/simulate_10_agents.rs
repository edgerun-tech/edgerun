// SPDX-License-Identifier: GPL-2.0-only
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use edgerun_storage::virtual_fs::{
    FsDeltaAppliedV1, FsDeltaProposedV1, SourceImportRequestV1, StorageBackedVirtualFs,
    VfsCursorV1, VfsEventTypeV1, VfsModeV1, VfsSourceKindV1, VirtualFsQueryFilter,
};

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn percentile_ms(samples: &[u128], pct: f64) -> u128 {
    if samples.is_empty() {
        return 0;
    }
    let mut v = samples.to_vec();
    v.sort_unstable();
    let idx = ((v.len() as f64 - 1.0) * pct).round() as usize;
    v[idx]
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let agents: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(10);
    let edits_per_agent: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(200);

    let repo_root = PathBuf::from("/home/ken/src/edgerun");
    let out_root = repo_root.join("out").join("sim-10-agents");
    let storage_dir = out_root.join("storage");
    let _ = std::fs::remove_dir_all(&out_root);
    std::fs::create_dir_all(&storage_dir).expect("create storage dir");

    let repo_id = format!("sim-repo-{}", now_unix_ms());
    let branch_id = "main".to_string();

    let mut init = StorageBackedVirtualFs::open_writer(storage_dir.clone(), &repo_id)
        .expect("open init writer");
    let req = SourceImportRequestV1 {
        schema_version: 1,
        repo_id: repo_id.clone(),
        source_kind: VfsSourceKindV1::VfsSourceKindFsSnapshot as i32,
        mode: VfsModeV1::VfsModeCode as i32,
        source_locator: "sim://seed".to_string(),
        source_ref: "seed".to_string(),
        initiated_by: "simulator".to_string(),
    };
    let _ = init
        .import_source(&req, 0, 0, vec![0u8; 32], "{}".to_string())
        .expect("import source");
    drop(init);

    let writer = Arc::new(Mutex::new(
        StorageBackedVirtualFs::open_writer(storage_dir.clone(), &repo_id)
            .expect("open shared writer"),
    ));
    let latencies_ms = Arc::new(Mutex::new(Vec::<u128>::new()));

    let started = Instant::now();
    let mut joins = Vec::new();

    for agent_idx in 0..agents {
        let writer = Arc::clone(&writer);
        let latencies = Arc::clone(&latencies_ms);
        let branch_id = branch_id.clone();
        let repo_id = repo_id.clone();

        joins.push(thread::spawn(move || {
            for edit_idx in 0..edits_per_agent {
                let t0 = Instant::now();
                let proposal_id = format!("agent-{agent_idx}-edit-{edit_idx}");
                let diff = format!(
                    "diff --git a/src/agent_{agent_idx}.rs b/src/agent_{agent_idx}.rs\n--- a/src/agent_{agent_idx}.rs\n+++ b/src/agent_{agent_idx}.rs\n@@ -0,0 +1,1 @@\n+// edit {edit_idx} by agent {agent_idx}\n"
                )
                .into_bytes();

                let proposal = FsDeltaProposedV1 {
                    schema_version: 1,
                    repo_id: repo_id.clone(),
                    proposal_id: proposal_id.clone(),
                    branch_id: branch_id.clone(),
                    base_cursor: Some(VfsCursorV1 {
                        branch_id: branch_id.clone(),
                        seq: 0,
                        head_event_hash: Vec::new(),
                    }),
                    agent_id: format!("agent-{agent_idx}"),
                    intent: "simulated edit".to_string(),
                    diff_unified: diff,
                };

                let applied = FsDeltaAppliedV1 {
                    schema_version: 1,
                    repo_id: repo_id.clone(),
                    proposal_id,
                    branch_id: branch_id.clone(),
                    base_cursor: None,
                    resulting_cursor: None,
                    applied_by: "sim-gate".to_string(),
                };

                {
                    let mut w = writer.lock().expect("writer lock");
                    w.propose_delta(&proposal).expect("propose");
                    w.apply_delta(&applied).expect("apply");
                }

                let elapsed = t0.elapsed().as_millis();
                latencies.lock().expect("lat lock").push(elapsed);
            }
        }));
    }

    for j in joins {
        j.join().expect("join agent");
    }

    let total_elapsed = started.elapsed();

    let reader =
        StorageBackedVirtualFs::open_reader(storage_dir.clone(), &repo_id).expect("open reader");

    let proposed = reader
        .query(
            1_000_000,
            0,
            VirtualFsQueryFilter {
                event_type: Some(VfsEventTypeV1::VfsEventTypeFsDeltaProposed),
                branch_id: Some(branch_id.clone()),
            },
        )
        .expect("query proposed")
        .events
        .len();

    let applied = reader
        .query(
            1_000_000,
            0,
            VirtualFsQueryFilter {
                event_type: Some(VfsEventTypeV1::VfsEventTypeFsDeltaApplied),
                branch_id: Some(branch_id.clone()),
            },
        )
        .expect("query applied")
        .events
        .len();

    let lats = latencies_ms.lock().expect("latencies lock").clone();
    let total_edits = agents * edits_per_agent;
    let elapsed_secs = total_elapsed.as_secs_f64();
    let edits_per_sec = if elapsed_secs > 0.0 {
        total_edits as f64 / elapsed_secs
    } else {
        0.0
    };

    let p50 = percentile_ms(&lats, 0.50);
    let p95 = percentile_ms(&lats, 0.95);
    let p99 = percentile_ms(&lats, 0.99);

    println!("simulation_agents={agents}");
    println!("simulation_edits_per_agent={edits_per_agent}");
    println!("simulation_total_edits={total_edits}");
    println!("simulation_elapsed_ms={}", total_elapsed.as_millis());
    println!("simulation_edits_per_sec={:.2}", edits_per_sec);
    println!("latency_p50_ms={p50}");
    println!("latency_p95_ms={p95}");
    println!("latency_p99_ms={p99}");
    println!("events_proposed={proposed}");
    println!("events_applied={applied}");
    println!("output_storage_dir={}", storage_dir.display());

    // Small cooldown so logs are readable if running repeatedly.
    thread::sleep(Duration::from_millis(50));
}
