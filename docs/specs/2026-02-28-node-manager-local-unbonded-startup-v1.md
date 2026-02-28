# 2026-02-28 Node Manager Local-Unbonded Startup V1

## Goal
- Allow `edgerun-node-manager run` to start in local-first mode without requiring `owner_pubkey`/bonding.
- Keep local bridge (`127.0.0.1:7777`) and local Docker/Swarm status APIs available even when cloud onboarding is not configured.

## Non-Goals
- Removing TPM requirement.
- Removing bonding/register/init flows when owner context is available.
- Redesigning cloud runtime image policy flow.

## Security / Constraints
- Preserve fail-closed behavior for TPM signer initialization.
- Preserve existing cloud bootstrap path when owner context exists.
- Do not introduce fallback-only insecure modes; local-unbonded mode is explicit and limited to local control-plane functions.

## Acceptance Criteria
1. `edgerun-node-manager run` does not exit with `owner_pubkey is required for first-boot bonding` when unbonded.
2. Local bridge starts and remains reachable in unbonded mode.
3. Cloud bootstrap/runtime-image/heartbeat calls run only when owner context exists.
4. Existing bonded path behavior remains unchanged.

## Rollout / Rollback
- Rollout: ship node-manager update and rebuild host binaries used by compose image.
- Rollback: revert this spec’s implementation commit to restore strict owner-gated startup behavior.
