# Edgerun Storage Operator Guide (Multi-Agent Code Editing)

## What This Operates
This runbook is for event-log-backed code editing with multiple agents proposing diffs.

Core flow:
1. Import a repo into VFS storage.
2. Agents submit proposed unified diffs (`FsDeltaProposedV1`).
3. Operator inspects proposal queue.
4. Gatekeeper validates diffs in an isolated git worktree (`fmt` + `check`).
5. Valid diffs are applied (`FsDeltaAppliedV1`); invalid diffs are rejected (`FsDeltaRejectedV1`).
6. Snapshot checkpoints are emitted automatically as applied diffs accumulate.

## Build Release Binaries
From `crates/edgerun-storage`:

```bash
cargo build --release -p edgerun-storage
```

Binaries used by this guide:
- `/var/cache/build/rust/target/release/vfs_operator`
- `/var/cache/build/rust/target/release/proposal_gatekeeper`
- `/var/cache/build/rust/target/release/proposal_batch_gatekeeper`
- `/var/cache/build/rust/target/release/docker_log_ingest`
- `/var/cache/build/rust/target/release/docker_log_driver`

## Operator Environment
Set once per session:

```bash
export DATA_DIR=/home/ken/src/edgerun/out/vfs-ops/storage
export REPO_ID=repo-main
export BRANCH=main
export REPO_ROOT=/home/ken/src/edgerun
mkdir -p "$DATA_DIR"
```

## 1) Import Git Repo Into Storage
This creates source import events and blob events for current repo state:

```bash
/var/cache/build/rust/target/release/vfs_operator import-git \
  --data-dir "$DATA_DIR" \
  --repo-id "$REPO_ID" \
  --repo-path "$REPO_ROOT" \
  --git-ref HEAD \
  --initiated-by operator
```

Quick sanity check:

```bash
/var/cache/build/rust/target/release/vfs_operator materialize \
  --data-dir "$DATA_DIR" \
  --repo-id "$REPO_ID"
```

## 2) Agents Submit Diff Proposals
Each agent submits a unified patch file.

Example patch file creation:

```bash
cat > /tmp/agent-a.patch <<'PATCH'
diff --git a/README.mdx b/README.mdx
--- a/README.mdx
+++ b/README.mdx
@@ -1 +1,2 @@
 # edgerun-storage
+agent-a edit
PATCH
```

Submit proposal event:

```bash
/var/cache/build/rust/target/release/vfs_operator propose-diff \
  --data-dir "$DATA_DIR" \
  --repo-id "$REPO_ID" \
  --branch "$BRANCH" \
  --proposal-id agent-a-001 \
  --agent-id agent-a \
  --intent "README update" \
  --diff-file /tmp/agent-a.patch
```

Repeat per agent with unique `--proposal-id`.

## 3) Inspect Proposal Queue
List queued proposals for a branch:

```bash
/var/cache/build/rust/target/release/vfs_operator list-proposals \
  --data-dir "$DATA_DIR" \
  --repo-id "$REPO_ID" \
  --branch "$BRANCH" \
  --limit 200
```

Track applied/rejected/snapshot events:

```bash
/var/cache/build/rust/target/release/vfs_operator list-events \
  --data-dir "$DATA_DIR" \
  --repo-id "$REPO_ID" \
  --branch "$BRANCH" \
  --event-type fs_delta_applied \
  --limit 200

/var/cache/build/rust/target/release/vfs_operator list-events \
  --data-dir "$DATA_DIR" \
  --repo-id "$REPO_ID" \
  --branch "$BRANCH" \
  --event-type fs_delta_rejected \
  --limit 200

/var/cache/build/rust/target/release/vfs_operator list-events \
  --data-dir "$DATA_DIR" \
  --repo-id "$REPO_ID" \
  --branch "$BRANCH" \
  --event-type snapshot_checkpointed \
  --limit 200
```

## 4) Validate + Apply One Proposal (Dry Run First)
Dry run (no apply/reject events written):

```bash
/var/cache/build/rust/target/release/proposal_gatekeeper \
  --data-dir "$DATA_DIR" \
  --repo-id "$REPO_ID" \
  --branch "$BRANCH" \
  --proposal-id agent-a-001 \
  --repo-root "$REPO_ROOT" \
  --fmt-cmd "cargo fmt --all" \
  --check-cmd "cargo check -p edgerun-storage" \
  --timeout-secs 300 \
  --dry-run
```

Apply for real:

```bash
/var/cache/build/rust/target/release/proposal_gatekeeper \
  --data-dir "$DATA_DIR" \
  --repo-id "$REPO_ID" \
  --branch "$BRANCH" \
  --proposal-id agent-a-001 \
  --repo-root "$REPO_ROOT" \
  --fmt-cmd "cargo fmt --all" \
  --check-cmd "cargo check -p edgerun-storage" \
  --timeout-secs 300
```

Behavior:
- patch apply fails -> appends `FsDeltaRejectedV1`
- format/check fails -> appends `FsDeltaRejectedV1`
- format mutates diff -> gatekeeper appends formatted proposal and applies that proposal ID

## 5) Validate + Apply Batches
Dry run batch:

```bash
/var/cache/build/rust/target/release/proposal_batch_gatekeeper \
  --data-dir "$DATA_DIR" \
  --repo-id "$REPO_ID" \
  --branch "$BRANCH" \
  --proposal-ids "agent-a-001,agent-b-004,agent-c-002" \
  --repo-root "$REPO_ROOT" \
  --fmt-cmd "cargo fmt --all" \
  --check-cmd "cargo check -p edgerun-storage" \
  --timeout-secs 300 \
  --dry-run
```

Apply batch:

```bash
/var/cache/build/rust/target/release/proposal_batch_gatekeeper \
  --data-dir "$DATA_DIR" \
  --repo-id "$REPO_ID" \
  --branch "$BRANCH" \
  --proposal-ids "agent-a-001,agent-b-004,agent-c-002" \
  --repo-root "$REPO_ROOT" \
  --fmt-cmd "cargo fmt --all" \
  --check-cmd "cargo check -p edgerun-storage" \
  --timeout-secs 300
```

Batch semantics:
- proposals are replayed in listed order
- each proposal must apply/format/check successfully in sequence
- first failure rejects that proposal and exits
- prior applied proposals remain applied (append-only event log)

## 6) Snapshot Cadence
`StorageBackedVirtualFs` auto-checkpoints snapshots based on applied diff count.

Current policy field:
- `auto_checkpoint_every_applied`

Meaning:
- `0` disables automatic snapshot checkpoints
- `N > 0` emits `SnapshotCheckpointedV1` every `N` applied diffs

Monitor checkpoints with `list-events --event-type snapshot_checkpointed`.

## 7) Throughput/Latency Simulation (10 Agents)
Synthetic load test:

```bash
cargo run --release --bin simulate_10_agents -- 10 10
cargo run --release --bin simulate_10_agents -- 10 50
cargo run --release --bin simulate_10_agents -- 10 100
```

Printed metrics:
- `simulation_edits_per_sec`
- `latency_p50_ms`, `latency_p95_ms`, `latency_p99_ms`
- `events_proposed`, `events_applied`

## 8) Docker Logger Plugin (Non-JSON)
`docker_log_ingest` accepts stdin lines with this frame:

`container_id|container_name|stream|ts_unix_ms|message`

Example:

```bash
cat <<'LOGS' | /var/cache/build/rust/target/release/docker_log_ingest \
  --data-dir "$DATA_DIR" \
  --repo-id "$REPO_ID" \
  --branch "$BRANCH" \
  --declared-by docker-plugin \
  --partition-prefix docker \
  --batch-lines 1000 \
  --ensure-log-source
cid-a|api|stdout|1709251200001|boot ok
cid-a|api|stdout|1709251200002|listening :8081
cid-b|worker|stderr|1709251200003|retrying job
LOGS
```

Then inspect storage-backed docker log events:

```bash
/var/cache/build/rust/target/release/vfs_operator list-events \
  --data-dir "$DATA_DIR" \
  --repo-id "$REPO_ID" \
  --branch "$BRANCH" \
  --event-type log_entry_appended \
  --limit 200
```

Batching guidance:
- Prefer larger `--batch-lines` for throughput (for example `500` to `2000`).
- Storage now appends each ingest batch into one sealed segment, so larger batches reduce segment churn.
- Tradeoff: larger batches increase per-batch latency but raise total lines/sec.

### Docker Log-Driver Endpoint (Unix Socket)
`docker_log_driver` exposes Docker log-driver plugin endpoints over Unix socket:
- `POST /Plugin.Activate`
- `POST /LogDriver.Capabilities`
- `POST /LogDriver.StartLogging`
- `POST /LogDriver.StopLogging`

Run:

```bash
mkdir -p /run/edgerun
/var/cache/build/rust/target/release/docker_log_driver \
  --data-dir "$DATA_DIR" \
  --repo-id "$REPO_ID" \
  --branch "$BRANCH" \
  --socket-path /run/edgerun/docker-log-driver.sock \
  --partition-prefix docker \
  --batch-lines 1000 \
  --ensure-log-source
```

Quick protocol check:

```bash
curl --unix-socket /run/edgerun/docker-log-driver.sock \
  -X POST http://localhost/Plugin.Activate
```

Notes:
- Docker log-driver control payloads are JSON (Docker protocol requirement).
- Log stream payloads are decoded from Docker-style length-delimited protobuf entries with line fallback mode.

### Docker Managed Plugin Packaging
Build plugin bundle (`config.json` + `rootfs`) under `out/`:

```bash
cd /home/ken/src/edgerun/crates/edgerun-storage
./tools/build_docker_log_driver_plugin.sh
```

Create and enable plugin:

```bash
sudo mkdir -p /var/lib/edgerun/docker-log-driver /run/docker/plugins
sudo docker plugin rm -f edgerun/docker-log-driver:local || true
sudo docker plugin create edgerun/docker-log-driver:local /home/ken/src/edgerun/out/docker-log-driver-plugin
sudo docker plugin set edgerun/docker-log-driver:local \
  DATA_DIR=/var/lib/edgerun/docker-log-driver \
  REPO_ID=docker-logs \
  BRANCH=main \
  BATCH_LINES=1000 \
  MAX_STREAM_BUFFER_BYTES=8388608
sudo docker plugin enable edgerun/docker-log-driver:local
```

Use as log-driver for a container:

```bash
docker run --rm \
  --log-driver=edgerun/docker-log-driver:local \
  alpine sh -c 'echo hello-from-plugin'
```

Disable/remove:

```bash
sudo docker plugin disable edgerun/docker-log-driver:local
sudo docker plugin rm edgerun/docker-log-driver:local
```

## 9) Daily Verification Checklist
From `crates/edgerun-storage`:

```bash
cargo fmt --all
cargo check -p edgerun-storage
cargo test -p edgerun-storage
cargo clippy -p edgerun-storage --lib -- -D warnings
```
