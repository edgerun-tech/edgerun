# EdgeRun Runtime ABI Compatibility Policy

Last updated: 2026-02-20

## ABI window

Runtime bundle ABI compatibility is defined by `edgerun-types` constants:

- `BUNDLE_ABI_CURRENT = 2`
- `BUNDLE_ABI_MIN_SUPPORTED = 1`

The runtime accepts bundle ABI versions in the inclusive range `[1, 2]`.

## N-1 policy

For each runtime release, the runtime must accept:

- Current ABI (`N`)
- Previous ABI (`N-1`)

For the current release:

- `N = 2`
- `N-1 = 1`

Versions below `BUNDLE_ABI_MIN_SUPPORTED` and above `BUNDLE_ABI_CURRENT` are rejected at bundle decode time.

## Rollout rules

When introducing ABI `N+1`:

1. Increase `BUNDLE_ABI_CURRENT` to `N+1`.
2. Keep `BUNDLE_ABI_MIN_SUPPORTED` at `N` for one full release window.
3. Ensure runtime tests cover both `N+1` and `N`.
4. After deprecation window, raise `BUNDLE_ABI_MIN_SUPPORTED` to `N+1`.

## Enforcement points

- Decode gate: `edgerun_types::decode_bundle_payload_canonical`.
- Runtime strict API policy checks:
  - `execute_bundle_payload_bytes_for_runtime_and_abi_strict`
- Runtime tests:
  - accepts `N` and `N-1`
  - rejects unsupported ABI versions
