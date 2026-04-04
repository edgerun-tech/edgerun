# Lifegraph v0 Validator Matrix

This matrix maps each validator or routine to:

- its input surface
- deterministic outputs
- mandatory checks
- minimal mandatory vector set

## 1. Canonicalization routines

| Routine | Inputs | Outputs | Mandatory checks | Mandatory cases |
|---|---|---|---|---|
| `canonicalize_signable(record)` | semantic record, family version | canonical bytes | null handling, repeated order, oneof handling, nested messages, NFC text normalization | canonical/01, 02, 03, 04, 05 |
| `canonicalize_full(record)` | semantic record incl. signature | canonical bytes | full vs signable distinction, signature slot placement | canonical/05 |
| `hash_record(record)` | signable canonical bytes, domain tag | digest | correct domain separation, exact hash bytes | canonical/05 plus family-specific derived vectors |
| `derive_object_id()` | canonicalization id, canonical bytes | object_id | object identity independent of storage wrapper | object/01 |

## 2. Stream validator

### Validator
`validate_stream_append(ctx, event)`

### Inputs
- local stream head
- existing stream events or genesis
- stream writer identity
- candidate event

### Outputs
- verdict
- reason_code
- derived `event_hash`
- optional `head_after`

### Mandatory checks
- supported envelope version
- genesis constraints
- signature valid under stream writer
- exact prev hash linkage
- contiguous sequence
- duplicate exact event detection
- fork rejection
- defer on missing predecessor

### Mandatory cases
- `stream/01-genesis-valid`
- `stream/02-append-valid`
- `stream/03-gap-defer`
- `stream/04-fork-reject`

## 3. Delegation validator

### Validator
`validate_delegation_chain(ctx, issuer, action, scope, chain)`

### Inputs
- issuer identity
- requested action and scope
- delegation chain
- local controllers or other trust roots
- revocations
- assurance policy

### Outputs
- verdict
- reason_code
- authority basis (`direct` or `delegated`)
- effective capability summary

### Mandatory checks
- direct authority recognition
- chain signature verification
- chain continuity
- attenuation
- non-delegable parent enforcement
- revocation application
- root trust recognition
- scope match to requested action

### Mandatory cases
- `delegation/01-direct-controller`
- `delegation/02-single-delegation`
- `delegation/03-chain-attenuation-reject`
- `delegation/04-revoked-delegation-reject`

## 4. Command validator

### Validator
`validate_command(ctx, command)`

### Inputs
- local node identity
- replay cache
- local authority state
- local policy
- incoming command

### Outputs
- verdict
- reason_code
- derived `command_hash`
- optional `decision` (`committed` or `rejected`)
- optional resulting replay cache entry

### Mandatory checks
- structural validity
- signature validity
- target node match
- time window validity
- duplicate same-hash handling
- command_id collision handling
- authority validation
- policy validation
- commit-before-execute invariant

### Mandatory cases
- `command/01-commit-success`
- `command/02-reject-wrong-target`
- `command/03-duplicate-command`
- `command/04-command-id-collision`

## 5. Control validator

### Validator
`validate_control_change(ctx, command)`

### Inputs
- current controller-set projection
- control policy projection
- control-change command
- optional proof-of-possession evidence

### Outputs
- verdict
- reason_code
- resulting control-state delta
- resulting controller set on success

### Mandatory checks
- authorization under current control state
- target identity validity
- post-state simulation
- no empty controller set unless policy explicitly allows
- safe transfer add-verify-remove logic

### Mandatory cases
- `control/01-add-controller`
- `control/02-remove-last-controller-reject`
- `control/03-safe-transfer-add-verify-remove`

## 6. Snapshot validator

### Validator
`validate_snapshot(ctx, snapshot, deltas?)`

### Inputs
- snapshot descriptor
- snapshot payload object
- producer trust policy
- local base heads/checkpoints
- optional deltas

### Outputs
- verdict
- reason_code
- acceptance class
- updated view on success

### Mandatory checks
- descriptor signature
- base compatibility
- trusted vs cache-only producer distinction
- payload object validation
- delta continuity
- rejection on conflicting base
- defer on gap

### Mandatory cases
- `snapshot/01-trusted-snapshot-accept`
- `snapshot/02-stale-snapshot-accept-stale`
- `snapshot/03-delta-gap-defer`

## 7. Object validator

### Validator
`validate_object_retrieval(ctx, target, representation)`

### Inputs
- object ref or representation ref
- logical object descriptor
- representation header
- representation bytes or chunks
- local access state

### Outputs
- verdict
- reason_code
- validation level (`representation_valid_only` or `logical_object_valid`)
- derived `object_id` or `representation_digest`

### Mandatory checks
- representation digest validity
- manifest/chunk reconstruction
- access check before decryption
- transform reversal
- logical object identity derivation
- object id mismatch rejection

### Mandatory cases
- `object/01-raw-object-valid`
- `object/02-encrypted-representation-valid-no-access`

## 8. Result comparison policy

For every validator case, the harness must compare:

- verdict
- reason_code when applicable
- all derived fields declared in `expected.yaml`
- all post-state fields declared in `expected.yaml`

Fields not declared in `expected.yaml` are ignored for pass/fail.

## 9. Mandatory invariant coverage

The full matrix must cover these invariants:

- single-writer stream integrity
- append-only event ordering
- command authority is local to the target node
- no privilege expansion across delegation chains
- durable control changes require node commitment
- snapshots never override stream authority
- logical object identity is distinct from storage representation
