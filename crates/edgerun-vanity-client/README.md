# edgerun-vanity-client

Client-side orchestrator for escrow-bounded vanity search on edgerun.

It supports two modes:

- `secure-local` (default): no seed material leaves client.
- `distributed-insecure`: submits chunked jobs to scheduler (workers can derive key material from submitted seed).

## Build

```bash
cargo build -p edgerun-vanity-client
```

## Run (secure, default)

```bash
cargo run -p edgerun-vanity-client -- \
  --mode secure-local \
  --seed-hex 0101010101010101010101010101010101010101010101010101010101010101 \
  --prefix So1 \
  --start-counter 0 \
  --end-counter 1000000 \
  --chunk-attempts 50000
```

## Run (distributed, insecure)

```bash
cargo run -p edgerun-vanity-client -- \
  --mode distributed-insecure \
  --allow-worker-seed-exposure \
  --scheduler-url http://127.0.0.1:8080 \
  --runtime-id 0000000000000000000000000000000000000000000000000000000000000000 \
  --wasm-path /var/cache/build/rust/target/wasm32-unknown-unknown/debug/edgerun_vanity_payload.wasm \
  --seed-hex 0101010101010101010101010101010101010101010101010101010101010101 \
  --prefix So1 \
  --start-counter 0 \
  --end-counter 1000000 \
  --chunk-attempts 50000 \
  --escrow-per-job-lamports 1000000 \
  --max-escrow-lamports 20000000
```
