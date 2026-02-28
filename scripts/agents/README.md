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

Compatibility mode (legacy branch merge) still works:

```bash
scripts/agents/merge-agent.sh agent/github-lane-20260301093000 main
```
