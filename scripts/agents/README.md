# Containerized Agent Workflow (Virtualized View)

Core event/diff tooling is implemented in Go at `tooling/cmd/edgertool`.
The shell scripts in this directory are compatibility entrypoints that delegate to the Go CLI.
For faster execution, build once with `make tool-build` (wrappers will use `out/tooling/edgertool` automatically).

## Launch isolated agent run

```bash
scripts/agents/launch-agent.sh github-lane "implement GitHub integration list/read/create routes"
```

This command:
- creates a virtualized repo snapshot under `out/agents/runs/<run_id>/base` (no `.git`)
- creates a writable candidate workspace under `out/agents/runs/<run_id>/work`
- runs Codex CLI in Docker against that virtual workspace
- emits a proposed diff patch + event under `out/agents/runs/<run_id>/events`

Optional auto storage wiring (fail-closed):

```bash
export EDGERUN_AGENT_STORAGE_AUTOSUBMIT=1
export EDGERUN_AGENT_STORAGE_AUTO_DRY_RUN=1
export EDGERUN_AGENT_STORAGE_DATA_DIR=/home/ken/src/edgerun/out/vfs-ops/storage
export EDGERUN_AGENT_STORAGE_REPO_ID=repo-main
export EDGERUN_AGENT_STORAGE_BRANCH=main
scripts/agents/launch-agent.sh github-lane "implement GitHub integration list/read/create routes"
```

When enabled, `agent-launch` submits the run patch to storage and runs gatekeeper dry-run automatically.

## Canonical Operator Entry

```bash
scripts/agents/run-task.sh github-lane "implement GitHub integration list/read/create routes"
```

This is the canonical entrypoint. It enables storage autosubmit + gate dry-run by default and delegates to `edgertool agent-launch`.

## Context tools (via mcp-syscall)

```bash
scripts/agents/mcp-context.sh pack /workspace/virtual/crates/edgerun-node-manager/src/main.rs
scripts/agents/mcp-context.sh symbols /workspace/virtual/crates/edgerun-node-manager/src/main.rs
scripts/agents/mcp-context.sh refs handle_local_mcp_start
```

Requires `mcp-syscall` reachable at `MCP_SYSCALL_URL` (default `http://127.0.0.1:7047`).

## Run tests on candidate workspace

```bash
scripts/agents/test-executor.sh out/agents/runs/<run_id>/work quick
scripts/agents/test-executor.sh out/agents/runs/<run_id>/work frontend
```

## Accept diff (event-first, no local mutation)

```bash
scripts/agents/apply-accepted-diff.sh out/agents/runs/<run_id>
```

This publishes `edgerun.agents.diff.accepted` to NATS and exits.
If NATS is unavailable, the command fails (fail-closed).

## Apply accepted diff locally (explicit)

```bash
scripts/agents/apply-accepted-diff.sh --apply out/agents/runs/<run_id>
```

## Submit proposal to storage queue

Requires `vfs_operator` from `edgerun-storage` release build.

```bash
scripts/agents/storage-proposal-submit.sh \
  out/agents/runs/<run_id> \
  /home/ken/src/edgerun/out/vfs-ops/storage \
  repo-main \
  main
```

This submits the run patch as a `propose-diff` event with `proposal_id=<run_id>`.

## Gatekeeper dry-run and apply

Requires `proposal_gatekeeper` from `edgerun-storage` release build.

```bash
scripts/agents/storage-proposal-apply.sh \
  /home/ken/src/edgerun/out/vfs-ops/storage \
  repo-main \
  main \
  <proposal_id> \
  --dry-run
```

```bash
scripts/agents/storage-proposal-apply.sh \
  /home/ken/src/edgerun/out/vfs-ops/storage \
  repo-main \
  main \
  <proposal_id>
```

The dry-run performs isolated patch/fmt/check validation without writing apply/reject events.
Use `--repo-root <PATH>` when validating proposals against a different repository root.
Use `--fmt-cmd`, `--check-cmd`, and `--timeout-secs` to override default gatekeeper validation commands.
