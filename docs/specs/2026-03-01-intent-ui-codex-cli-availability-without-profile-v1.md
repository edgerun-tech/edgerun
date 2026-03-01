# 2026-03-01 Intent UI Codex CLI Availability Without Profile v1

## Goal
Allow IntentBar/Codex assistant execution when the Codex CLI integration is linked and a device is online, even if profile mode is not active.

## Non-goals
- No change to other integrations' profile requirements.
- No change to node/device connectivity gating.
- No change to assistant transport (`/api/assistant`) or provider selection semantics.

## Security / Constraint Requirements
- Fail closed when no online device is available.
- Keep existing "must link integration" gate.
- Keep deterministic integration lifecycle behavior in both main-thread and worker lifecycle implementations.

## Acceptance Criteria
1. `codex_cli` reports `available=true` when connected and device-ready, regardless of profile session state.
2. `codex_cli` reports `Connected device required` when linked but no device is online.
3. Assistant call path (`openCodexResponse`) no longer blocks on profile-only availability for linked Codex CLI.
4. Cypress coverage proves Codex assistant can execute with linked `codex_cli` in non-profile runtime.

## Rollout / Rollback
- Rollout: update integration lifecycle availability logic in both shared source and integrations worker.
- Rollback: revert lifecycle availability logic and the new Cypress case in one commit.
