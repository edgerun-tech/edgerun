# Intent UI Integration Secrets Encrypted Profile V1

## Goal
- Ensure integration secrets are stored encrypted within the user profile when running in persistent profile mode.
- Eliminate plain localStorage persistence for integration secrets in profile mode.

## Non-goals
- Server-side secret escrow.
- Changing credential vault behavior outside integrations token mirroring.
- Requiring network services for profile encryption/decryption.

## Security and Constraints
- Profile secrets use the same encrypted profile blob boundary as profile onboarding.
- Secrets are only writable when a decrypted profile context is active.
- In profile mode, integration secret reads must come from encrypted profile state.
- Ephemeral mode may continue local browser behavior without persistent encrypted profile.
- If runtime claims profile mode but decrypted profile-secret context is unavailable, integration linking must not silently drop tokens.
- In that degraded state, token persistence may temporarily fall back to local browser storage until encrypted context is re-established.

## Acceptance Criteria
1. Profile create/load hydrates decrypted integration secret map into runtime context.
2. Integration store reads/writes token secrets through encrypted profile context in profile mode.
3. When encrypted profile context is available, profile mode does not rely on plain localStorage token values for integration connectivity.
4. When encrypted profile context is unavailable, integration token writes/reads use deterministic fallback behavior so linked integrations remain functional and no token is silently discarded.
5. Tailscale API/auth keys follow encrypted profile secret path in profile mode.
6. Frontend check/build and targeted Cypress integration specs pass.

## Rollout / Rollback
- Rollout: additive profile secret context + integration store migration.
- Rollback: revert profile secret context wiring and restore previous localStorage token path.
