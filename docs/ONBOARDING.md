# Get Started

This guide starts with a Solana vanity-address workflow, then walks through
local and distributed modes, and finally the limitations/gotchas so you can
map required primitives to expected security outcomes.

Prerequisites for examples:

- Rust toolchain (`cargo`)
- `curl`
- `jq` (for JSON extraction in shell snippets)
- `openssl` (for HMAC signature examples)

## 1) First Workflow: Generate a Solana Vanity Address

This runs vanity search fully local on the client. Seed material never leaves
the client process.

```bash
cargo run -p edgerun-vanity-client -- \
  --mode secure-local \
  --seed-hex 0101010101010101010101010101010101010101010101010101010101010101 \
  --prefix So1 \
  --start-counter 0 \
  --end-counter 1000000 \
  --chunk-attempts 50000
```

## 2) Local vs Distributed Modes

### Local (`secure-local`)

- Seed stays on client.
- No scheduler/worker required.
- Best default for secrecy.

### Distributed (`distributed-insecure`)

- Uses scheduler + worker execution.
- Seed/derivation material is exposed to worker-side execution.
- Demo/testing only unless you add trusted execution + attestation validation.

## 3) Distributed Demo (Optional, Insecure by Design)

Use this only when seed disclosure to workers is acceptable for demo/testing.

### 2.1 Build payload wasm

```bash
cargo build -p edgerun-vanity-payload --target wasm32-unknown-unknown
```

### 2.2 Start scheduler

```bash
cargo run -p edgerun-scheduler
```

### 2.3 Start worker (separate shell)

```bash
EDGERUN_SCHEDULER_URL=http://127.0.0.1:8080 \
EDGERUN_WORKER_RUNTIME_IDS=0000000000000000000000000000000000000000000000000000000000000000 \
cargo run -p edgerun-worker
```

### 2.4 Run distributed vanity client

```bash
cargo run -p edgerun-vanity-client -- \
  --mode distributed-insecure \
  --allow-worker-seed-exposure \
  --scheduler-url http://127.0.0.1:8080 \
  --runtime-id 0000000000000000000000000000000000000000000000000000000000000000 \
  --wasm-path target/wasm32-unknown-unknown/debug/edgerun_vanity_payload.wasm \
  --seed-hex 0101010101010101010101010101010101010101010101010101010101010101 \
  --prefix So1 \
  --start-counter 0 \
  --end-counter 1000000 \
  --chunk-attempts 50000 \
  --escrow-per-job-lamports 1000000 \
  --max-escrow-lamports 20000000
```

## 4) Policy and Session Hardening (Optional)

These endpoints are for control-plane security and auditability. Job creation
does not require these calls.

Scheduler policy routes:

- `POST /v1/session/create`
- `POST /v1/session/rotate`
- `POST /v1/session/invalidate`
- `GET /v1/policy/info`
- `GET /v1/policy/audit`
- `GET /v1/trust/policy/get`
- `POST /v1/trust/policy/set`
- `GET /v1/attestation/policy/get`
- `POST /v1/attestation/policy/set`

### 3.1 Create a policy session

```bash
BASE=http://127.0.0.1:8080
ORIGIN=https://demo.local
SESSION_JSON=$(curl -sS -X POST "$BASE/v1/session/create" \
  -H 'content-type: application/json' \
  -d "{\"bound_origin\":\"$ORIGIN\"}")

TOKEN=$(printf '%s' "$SESSION_JSON" | jq -r .token)
SESSION_KEY=$(printf '%s' "$SESSION_JSON" | jq -r .session_key)
```

### 3.2 Sign helper (HMAC contract)

```bash
sign_req() {
  METHOD="$1"
  PATH_Q="$2"
  BODY="$3"
  NONCE="$4"
  TS="$(date +%s)"
  BODY_HASH="$(printf '%s' "$BODY" | openssl dgst -binary -sha256 | openssl base64 -A | tr '+/' '-_' | tr -d '=')"
  CANONICAL="${METHOD}|${PATH_Q}|${TS}|${NONCE}|${BODY_HASH}"
  SIG="$(printf '%s' "$CANONICAL" | openssl dgst -binary -sha256 -hmac "$SESSION_KEY" | openssl base64 -A | tr '+/' '-_' | tr -d '=')"
  printf '%s\n%s\n%s\n' "$TS" "$NONCE" "$SIG"
}
```

### 3.3 Read policy info

```bash
readarray -t S < <(sign_req GET /v1/policy/info "" nonce-info-1)
curl -sS "$BASE/v1/policy/info" \
  -H "authorization: Bearer $TOKEN" \
  -H "origin: $ORIGIN" \
  -H "x-hwv-ts: ${S[0]}" \
  -H "x-hwv-nonce: ${S[1]}" \
  -H "x-hwv-sig: ${S[2]}" | jq .
```

### 3.4 Set trust policy profile

```bash
BODY='{"profile":"strict"}'
readarray -t S < <(sign_req POST /v1/trust/policy/set "$BODY" nonce-trust-set-1)
curl -sS -X POST "$BASE/v1/trust/policy/set" \
  -H 'content-type: application/json' \
  -H "authorization: Bearer $TOKEN" \
  -H "origin: $ORIGIN" \
  -H "x-hwv-ts: ${S[0]}" \
  -H "x-hwv-nonce: ${S[1]}" \
  -H "x-hwv-sig: ${S[2]}" \
  -d "$BODY" | jq .
```

### 3.5 Set attestation policy

```bash
BODY='{"required":true,"max_age_secs":300,"allowed_measurements":["tee-measurement-a"]}'
readarray -t S < <(sign_req POST /v1/attestation/policy/set "$BODY" nonce-att-set-1)
curl -sS -X POST "$BASE/v1/attestation/policy/set" \
  -H 'content-type: application/json' \
  -H "authorization: Bearer $TOKEN" \
  -H "origin: $ORIGIN" \
  -H "x-hwv-ts: ${S[0]}" \
  -H "x-hwv-nonce: ${S[1]}" \
  -H "x-hwv-sig: ${S[2]}" \
  -d "$BODY" | jq .
```

### 3.6 Rotate or invalidate session

```bash
readarray -t S < <(sign_req POST /v1/session/rotate "" nonce-rotate-1)
NEW=$(curl -sS -X POST "$BASE/v1/session/rotate" \
  -H 'content-type: application/json' \
  -H "authorization: Bearer $TOKEN" \
  -H "origin: $ORIGIN" \
  -H "x-hwv-ts: ${S[0]}" \
  -H "x-hwv-nonce: ${S[1]}" \
  -H "x-hwv-sig: ${S[2]}" \
  -d '{"bound_origin":"'"$ORIGIN"'"}')
TOKEN=$(printf '%s' "$NEW" | jq -r .token)
SESSION_KEY=$(printf '%s' "$NEW" | jq -r .session_key)
```

```bash
readarray -t S < <(sign_req POST /v1/session/invalidate "" nonce-invalidate-1)
curl -sS -X POST "$BASE/v1/session/invalidate" \
  -H 'content-type: application/json' \
  -H "authorization: Bearer $TOKEN" \
  -H "origin: $ORIGIN" \
  -H "x-hwv-ts: ${S[0]}" \
  -H "x-hwv-nonce: ${S[1]}" \
  -H "x-hwv-sig: ${S[2]}" \
  -d '{}' | jq .
```

## 5) Limitations and Gotchas

Distributed vanity search without TEEs is not secret. Splitting generation and
evaluation across workers does not fix this if any worker can reconstruct the
search seed/counter path.

Use the table below to decide what primitives you need for each result.

| Desired Result | Required Primitives |
| --- | --- |
| Basic vanity demo only | `edgerun-vanity-payload` + `edgerun-vanity-client` in `secure-local` mode |
| Distributed orchestration demo | Scheduler + worker + vanity payload (accept seed exposure risk) |
| Controlled policy changes | Policy sessions (HMAC headers), nonce replay protection, audit log |
| Strong secrecy in distributed search | TEE execution + remote attestation verification + measurement allowlist policy |
| Strong integrity of worker claims | Attestation claim verification bound to worker identity and challenge nonce |

## 6) Optional Environment Toggles

Scheduler toggles you can enable as needed:

- `EDGERUN_SCHEDULER_REQUIRE_POLICY_SESSION=true|false`
- `EDGERUN_SCHEDULER_POLICY_SESSION_BOOTSTRAP_TOKEN=<token>`
- `EDGERUN_SCHEDULER_POLICY_SESSION_SHARED=true|false`
- `EDGERUN_SCHEDULER_REQUIRE_RESULT_ATTESTATION=true|false`
- `EDGERUN_SCHEDULER_QUORUM_REQUIRES_ATTESTATION=true|false`
- `EDGERUN_SCHEDULER_REQUIRE_CLIENT_SIGNATURES=true|false`

Start minimal. Add controls only when you need them.
