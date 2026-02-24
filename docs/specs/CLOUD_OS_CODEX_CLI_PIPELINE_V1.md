# CLOUD_OS_CODEX_CLI_PIPELINE_V1

## Goal
Wire Cloud OS text input to execute `codex` CLI commands and show concrete command results in the existing result panel flow.

## Non-Goals
- Replacing existing LLM/MCP intent flow.
- Adding streaming token UI in this iteration.
- Changing auth setup (assume operator already completed `codex login`).

## Security and Constraints
- Preserve append-only event system behavior already added elsewhere.
- Execute CLI only through an explicit input prefix (`/codex ...`) to avoid accidental shell execution.
- Enforce execution timeout and bounded stdout/stderr capture.
- Return structured JSON with exit code, stdout, stderr, and duration.

## Acceptance Criteria
1. IntentBar supports `/codex <prompt>` input.
2. Backend API route executes `codex exec` with provided prompt.
3. Results appear in Cloud OS result history with `log-viewer` rendering.
4. On failures/timeouts, user sees clear error output and non-zero exit code.
5. Cloud OS lint/build pass.

## Rollout / Rollback
- Rollout: additive API route + IntentBar branch.
- Rollback: remove `/codex` branch and API route; leave existing intent flow unchanged.
