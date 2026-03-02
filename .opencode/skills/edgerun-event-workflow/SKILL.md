---
name: edgerun-event-workflow
description: Operate on edgerun using storage-native proposal events instead of direct repository edits
license: Apache-2.0
compatibility: opencode
metadata:
  audience: agents
  repository: edgerun
---

## What I do

- Enforce event-first development for `edgerun`.
- Use `edgerun_syscall` MCP tools for code changes, preferring `apply_patch` proposal flow.
- Keep local repository files as working material, not as the source of truth.
- Treat `edgerun-storage` event log as canonical state for proposed, applied, and rejected changes.

## When to use me

Use this skill for any coding task in `/home/ken/src/edgerun` where the agent needs to change code, review change state, or explain workflow.

## Core operating rules

- Submit code changes as diffs through `edgerun_syscall.apply_patch`.
- Do not rely on direct file mutation as the primary path.
- Keep protected-root submit-only behavior enabled in MCP runtime.
- Query state from storage events and proposals before making new edits.
- Use deterministic, minimal diffs and converge by proposal IDs.

## Required environment model

- MCP server name: `edgerun_syscall`
- MCP endpoint: `http://127.0.0.1:7047/sse`
- Storage data dir must be outside the repository checkout.
- Recommended repo and branch IDs:
  - `repo_id=edgerun`
  - `branch=main`

## Standard workflow

1. Read current event state (`list-events`, `list-proposals`) before editing.
2. Create a focused unified diff for the intended change.
3. Submit via `edgerun_syscall.apply_patch`.
4. Record proposal ID and verify it appears in storage queue.
5. Validate via gatekeeper flow (dry run first, then apply when requested).
6. Re-check event stream for applied or rejected outcome.

## Verification checklist

- Proposal appears in `fs_delta_proposed` events.
- No direct-edit fallback was used for protected repo paths.
- Applied/rejected result is visible in event stream.
- Any reported status references proposal IDs and event evidence.

## Anti-patterns to avoid

- Treating git commit history as the primary orchestration channel.
- Writing changes directly and backfilling events later.
- Storing storage state inside the repository working tree.
- Skipping proposal state checks before new edits.
