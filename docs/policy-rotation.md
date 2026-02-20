# Policy Rotation Runbook

This runbook covers scheduler/worker policy key rotation with overlap support.

## Policy model

- Scheduler signs assignment policy metadata with:
  - `policy_key_id`
  - `policy_version`
  - `policy_valid_after_unix_s`
  - `policy_valid_until_unix_s`
- Worker verifies assignment signatures against allowed tuples:
  - primary: `(EDGERUN_WORKER_POLICY_KEY_ID, EDGERUN_WORKER_POLICY_VERSION, EDGERUN_WORKER_POLICY_VERIFY_KEY_HEX)`
  - optional next: `(EDGERUN_WORKER_POLICY_KEY_ID_NEXT, EDGERUN_WORKER_POLICY_VERSION_NEXT, EDGERUN_WORKER_POLICY_VERIFY_KEY_HEX_NEXT)`

## Scheduler settings

- `EDGERUN_SCHEDULER_POLICY_SIGNING_KEY_HEX`
- `EDGERUN_SCHEDULER_POLICY_KEY_ID`
- `EDGERUN_SCHEDULER_POLICY_VERSION`
- `EDGERUN_SCHEDULER_POLICY_TTL_SECS`

Inspect active scheduler policy:

```bash
curl -s http://127.0.0.1:8080/v1/policy/info | jq
```

## Worker settings

- Primary:
  - `EDGERUN_WORKER_POLICY_KEY_ID`
  - `EDGERUN_WORKER_POLICY_VERSION`
  - `EDGERUN_WORKER_POLICY_VERIFY_KEY_HEX`
- Next (optional overlap):
  - `EDGERUN_WORKER_POLICY_KEY_ID_NEXT`
  - `EDGERUN_WORKER_POLICY_VERSION_NEXT`
  - `EDGERUN_WORKER_POLICY_VERIFY_KEY_HEX_NEXT`
- Window tolerance:
  - `EDGERUN_WORKER_POLICY_CLOCK_SKEW_SECS`

## Rotation procedure

1. Prepare next key material:
   - generate new scheduler signing key seed
   - derive next verifying pubkey for workers
2. Roll out worker overlap config first:
   - keep current primary values
   - set `*_NEXT` values to next tuple
3. Rotate scheduler:
   - set scheduler signing key + `policy_key_id` + `policy_version` to next tuple
4. Verify acceptance:
   - run `./edgerun test rotation`
5. Cutover completion:
   - promote next tuple to worker primary
   - remove worker `*_NEXT` values
6. Confirm rejection of old tuples:
   - rotated assignments should still pass
   - old tuple assignments should fail policy verification

## Rollback

If failures spike during cutover:

1. Restore previous scheduler key/version.
2. Keep both worker primary+next tuples enabled.
3. Re-run `./edgerun test rotation`.
4. Investigate scheduler `/v1/policy/info` and worker `assignment_policy_verify` failures.

## Validation commands

```bash
./edgerun test rotation
./edgerun test integration
./edgerun test e2e
```
