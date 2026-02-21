# edgerun-runtime-web

Browser wrapper for `edgerun-runtime` using `wasm-bindgen`.

## Build

From repo root:

```bash
bash scripts/build-runtime-web.sh
```

Or from frontend:

```bash
bun run runtime:web:build
```

Skip frontend mirroring:

```bash
bash scripts/build-runtime-web.sh --no-sync
```

This writes the generated package to:

- Canonical crate output: `crates/edgerun-runtime-web/pkg-web/`
- Frontend mirror (if `frontend/` exists): `frontend/public/wasm/edgerun-runtime-web/`

## JS/TS usage

```ts
const runtime = await import("/wasm/edgerun-runtime-web/edgerun_runtime_web.js");

const digest = runtime.execute_bundle_payload_bytes_for_runtime_and_abi_digest_strict(
  bundlePayloadBytes,
  expectedRuntimeIdHex,
  expectedAbiVersion,
);
```

Exported functions:
- `validate_wasm_module(wasmBytes)`
- `execute_bundle_payload_bytes_strict(bundlePayloadBytes)`
- `execute_bundle_payload_bytes_for_runtime_and_abi_digest_strict(bundlePayloadBytes, expectedRuntimeIdHex, expectedAbiVersion)`
