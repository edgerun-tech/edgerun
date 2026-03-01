# 2026-03-01 Codex Session Persistence And Resume v1

## Goal
Make Codex assistant sessions reliably resumable across requests and browser reloads by returning real session/thread identifiers from backend execution and consistently hydrating/switching session state in Intent UI.

## Non-goals
- No change to assistant integration gating (device + integration availability stays unchanged).
- No change to provider set or onboarding requirements.
- No change to transport endpoint shape (`/api/assistant` request/response contract remains compatible).

## Security / Constraint Requirements
- Keep fail-closed behavior for Codex execution errors.
- Preserve deterministic local backend call path (`/api/assistant` -> `/v1/local/assistant`).
- Do not require browser access to host filesystem; session import stays within existing persisted app state.

## Acceptance Criteria
1. Backend uses Codex JSON events to capture the active thread/session id for each request.
2. Backend resumes Codex execution when a prior thread/session id is supplied.
3. Backend response returns stable `sessionId` and `threadId` values for frontend persistence.
4. Frontend normalizes missing `sessionId`/`threadId` fields so switching and follow-up requests always carry a resumable id.
5. Cypress coverage proves:
   - sessions persist across reload,
   - previously stored sessions hydrate/import into runtime state,
   - switching sessions changes the thread used for follow-up assistant calls.

## Rollout / Rollback
- Rollout:
  - Update node-manager assistant execution logic (resume + thread extraction).
  - Update Intent UI session id normalization and debug session-switch helper.
  - Add Cypress persistence/resume coverage.
- Rollback:
  - Revert this change set to restore prior echo-only session behavior.
