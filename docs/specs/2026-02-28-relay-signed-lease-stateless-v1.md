# Relay Signed Lease Stateless Mode V1

## Goal
- Remove relay dependency on persistent domain-registration state by introducing signed lease validation.
- Let `os` worker issue short-lived signed lease tokens used as `registration_token` in relay control requests.

## Non-goals
- Replacing pairing/session in-memory runtime cache.
- Full JWT/JWS framework adoption in this slice.
- Node-manager protocol redesign beyond reusing existing `registration_token` field.

## Security and Constraints
- Lease token must be signed by control-plane signer key material in worker secret.
- Relay must verify signature and strict claims (`domain`, `profile_public_key_b64url`, expiry, nonce/jti).
- Lease TTL bounded and fail-closed on invalid/expired tokens.
- No relay state required for domain reservation validation.

## Acceptance Criteria
1. `POST /api/tunnel/reserve-domain` on `os` worker returns:
- deterministic domain,
- signed lease token in `registrationToken`.
2. Relay validates signed lease token for:
- `register-endpoint`,
- `create-pairing-code`.
3. Relay no longer requires a pre-existing in-memory domain reservation for these paths.
4. Existing UI flow can reserve -> issue pairing code without relay reservation state.
5. Frontend checks/build and targeted Cypress specs pass.

## Rollout / Rollback
- Rollout:
1. Add worker lease signing helpers and reserve-domain response changes.
2. Add relay lease verification and replace domain-record gating.
3. Validate via frontend + rust checks.
- Rollback:
1. Revert worker lease issuance and relay lease verification changes.
2. Restore relay domain-record dependency path.
