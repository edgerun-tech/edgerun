# edgerun-vanity-payload

Deterministic Solana vanity search payload for edgerun workers.

Security warning: this payload requires seed material in job input. Any worker executing the job can derive the same keyspace.

Use this payload for local search or insecure/disclosure-accepted demos. For a
production secrecy story, keep derivation local or run worker-side search only
inside trusted execution environments with attestation verification.

## What it does

- Derives deterministic ed25519 keypairs from `(seed, counter)`.
- Computes Solana address (base58 pubkey) for each counter.
- Searches a counter range `[start_counter, start_counter + max_attempts)` for a prefix match.
- Returns only `(counter, pubkey, address)` for the match, so worker output does not include private key bytes.

## Input format

Binary payload:

1. `u8 version` (`1`)
2. `[u8; 32] seed`
3. `u64 start_counter` (little-endian)
4. `u64 max_attempts` (little-endian)
5. `u8 prefix_len`
6. `[u8; prefix_len] prefix` (UTF-8 base58 prefix)

## Output format

Binary payload:

1. `u8 version` (`1`)
2. `u8 status` (`0=found`, `1=not_found`, `2=error`)
3. `u64 counter` (`u64::MAX` for not found/error)
4. `[u8; 32] pubkey` (zeroed for not found/error)
5. `u8 data_len`
6. `[u8; data_len] data` (`address` for found, error code for error)

## Build

```bash
cargo test -p edgerun-vanity-payload
cargo build -p edgerun-vanity-payload --target wasm32-unknown-unknown
```

For end-to-end usage examples, see `docs/ONBOARDING.md`.
