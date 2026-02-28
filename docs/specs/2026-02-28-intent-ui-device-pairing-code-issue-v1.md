# Intent UI Device Pairing Code Issuance V1

## Goal
- Let users issue a relay pairing code directly from the Devices connect block.
- Auto-fill the generated pairing code into the Linux connect script.

## Non-goals
- Full domain reservation UX in this slice.
- Server-side credential vaulting or long-lived secret storage changes.

## Security and Constraints
- Relay API calls from browser must go through same-origin worker proxy endpoint.
- Proxy accepts only required inputs (`domain`, `registrationToken`, optional `ttlSeconds`) and forwards protobuf to relay.
- Fail closed: invalid/missing fields return explicit errors; no silent fallback.
- Keep no port `8080` usage.

## Acceptance Criteria
1. Devices connect block includes domain + registration token inputs and an `Issue pairing code` action.
2. Worker exposes `POST /api/tunnel/create-pairing-code` and returns JSON output from relay protobuf response.
3. Successful issuance auto-populates pairing code input and Linux script output.
4. UI surfaces issuance status/error and expiry information.
5. Cypress test verifies issuance action with mocked API response updates pairing code/script.

## Rollout / Rollback
- Rollout:
1. Add worker API proxy endpoint.
2. Add devices-panel issuance controls and storage.
3. Add/update Cypress test.
- Rollback:
1. Remove worker API route.
2. Remove devices-panel issuance controls.
3. Revert Cypress updates.
