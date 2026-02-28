# Intent UI Profile Bootstrap Gate V1

## Goal
- Add a startup gate in Intent UI (`os.edgerun.tech`) with three explicit entry options:
  - create profile,
  - load profile,
  - proceed without profile (ephemeral session).
- Keep authentication factors (WebAuthn/YubiKey) separate from profile storage backends.
- Ensure ephemeral mode works without login and clearly gates persistent capabilities.

## Non-goals
- No backend sync adapter implementation for Google Drive/Git in this change.
- No server-side profile authority/canonical state implementation.
- No replacement of existing credentials vault workflows.

## Security and constraints
- Profile operations are fail-closed: create/load aborts on auth, crypto, or parse errors.
- Persistent profile mode requires WebAuthn verification before unlock.
- Profile payload is encrypted client-side before storing/exporting.
- Storage backend selection does not imply auth provider coupling.
- Ephemeral mode stores state only in `sessionStorage` and must not silently become persistent.

## Acceptance criteria
- Intent UI shows bootstrap gate on first launch with create/load/ephemeral options.
- `Proceed without profile` sets ephemeral session mode and dismisses gate.
- `Create profile`:
  - enrolls a WebAuthn credential,
  - encrypts a profile blob,
  - stores profile blob via selected backend strategy.
- `Load profile`:
  - reads blob from selected backend strategy,
  - verifies WebAuthn credential,
  - decrypts and activates persistent profile session.
- UI displays active session mode (`ephemeral` vs `profile`) after gate resolution.
- In `ephemeral` mode, profile-gated windows (credentials/device/integration surfaces) do not expose full controls and show a lock panel with an explicit profile unlock action.
- Frontend checks pass (`bun run check`, `bun run build`) and a Cypress test covers first-run gate behavior.

## Rollout and rollback
- Rollout: additive to Intent UI startup sequence; default behavior remains local-first.
- Rollback: remove gate component/store wiring and restore direct startup path.
