# 2026-03-01 Compose and Verify Consolidation V1

## Goal and non-goals
- Goal:
  - Remove legacy compatibility branches that are no longer part of active framework routing operations.
  - Consolidate duplicate tunnel startup logic in the node-manager compose helper script.
  - Keep all operator-facing commands and behavior intact.
- Non-goals:
  - Change compose service topology.
  - Remove existing command aliases (`up-dev`, `up-tunnel`).

## Security and constraint requirements
- Keep preflight checks for tunnel config/credentials before tunnel startup.
- Preserve local bridge validation behavior in verify flows.
- Do not reduce validation coverage in `verify-local-stack.sh`.

## Acceptance criteria
1. `scripts/node-manager-compose.sh` no longer duplicates tunnel preflight/start logic across commands.
2. `scripts/verify-local-stack.sh` checks only current framework caddy container path (no stale `osdev` fallback).
3. `scripts/node-manager-compose.sh up-tunnel-verify` remains functional.
4. Script syntax checks pass.
5. Local stack verify command passes on active environment.

## Rollout and rollback
- Rollout:
  - Extract shared shell helper functions for tunnel preflight/start.
  - Remove legacy container-name branch from verify script.
- Rollback:
  - Revert script edits to restore prior duplicated command handlers and fallback checks.
