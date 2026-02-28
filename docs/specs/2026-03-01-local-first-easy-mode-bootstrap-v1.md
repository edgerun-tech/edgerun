# Local-First Easy Mode Bootstrap V1

## Goal
Provide a single deterministic operator workflow for local-first Edgerun bring-up so day-to-day iteration does not require remembering multiple compose/profile/service commands.

## Non-Goals
- No runtime architecture rewrite in this change.
- No changes to scheduling semantics, intent contracts, or executor behavior.
- No cloud relay/pairing restoration.

## Security and Constraints
- Local bridge remains fail-closed: frontend must use local bridge endpoint or show explicit connection error.
- No new service binds to port `8080`.
- Keep Docker socket and TPM/device access model unchanged for now.
- Preserve existing tunnel/caddy optionality for remote browser access.

## Acceptance Criteria
1. Operator can run a single command for local dev stack bring-up (node manager + tunnel profile services).
2. Operator can run one command to inspect health/status (compose state + local bridge probe).
3. Operator can tail logs by service with one command.
4. Obsolete relay/pair command path is removed from primary compose operator script.
5. Workflow is documented in script usage text and wired to Make targets.

## Rollout
- Update `scripts/node-manager-compose.sh` with easy-mode commands.
- Add Makefile aliases for the easy-mode commands.
- Keep existing commands compatible where possible (`up`, `up-tunnel`, `down`, `logs`).

## Rollback
- Revert `scripts/node-manager-compose.sh` and `Makefile` to previous command set.
- Existing compose files and service definitions remain unchanged, so rollback is low risk.
