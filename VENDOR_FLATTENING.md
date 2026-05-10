# Vendor Flattening

Goal: remove hidden vendored dependency trees and make every dependency explicit, owned, and replaceable.

## Rules

1. `vendor/cargo/*` is the only temporary vendor root while migration is in progress.
2. Nested `*/vendor/*` trees are not allowed as a final state.
3. Each nested vendor tree must become one of:
   - deleted, if the code has already been internalized or is unused;
   - promoted to `crates/utility/edgerun-*`, if it is still needed;
   - replaced by a smaller Edgerun-native implementation.
4. Promoted crates must be `no_std` by default when possible, with `alloc`/`std` behind features.
5. Compatibility dependency names may be preserved temporarily with Cargo package aliases, for example:

```toml
zeroize = { package = "edgerun-zeroize", path = "../edgerun-zeroize", default-features = false }
```

## Inventory commands

Report nested vendor trees:

```bash
python3 scripts/vendor-flattening-inventory.py
```

Fail on nested vendor trees once the current debt is removed:

```bash
python3 scripts/vendor-flattening-inventory.py --strict
```

Report all external dependency escape hatches:

```bash
python3 scripts/dependency-sovereignty.py
```

## Current known nested vendor debt

### `crates/utility/edgerun-crypto/vendor/num-bigint-dig-0.8.6`

Status: flatten/delete candidate.

Observation: `edgerun-crypto` already contains internalized big integer source under `crates/utility/edgerun-crypto/src/num_bigint`. The nested vendor manifest is not referenced by `edgerun-crypto/build.rs` and should be verified as dead before deletion.

Decision path:

1. Run repository search for `vendor/num-bigint-dig`, `num-bigint-dig-0.8.6`, and path references into the nested tree.
2. Run `cargo check -p edgerun-crypto --features rsa,alloc,zeroize` before deletion.
3. Delete the nested vendor directory if no source/build/test path depends on it.
4. Run the crypto checks again.

### Nested vendor dependencies under `edgerun-crypto/vendor/*`

Status: inventory required.

The `vendor-flattening-inventory.py` script should be treated as source of truth for the exact list. Each nested vendored package should get one tracker entry before removal or promotion.

## Completed flattening/replacements

### `zeroize`

Status: direct dependency replacement complete for the first two consumers.

Created `crates/utility/edgerun-zeroize`:

- `no_std` by default
- optional `alloc`
- volatile wipe plus compiler fence
- compatibility trait name `Zeroize`
- `Zeroizing<T>` wrapper

Replaced direct vendored `zeroize` usage in:

- `crates/utility/edgerun-rustls-pki-types`
- `crates/utility/edgerun-rusttls`

Remaining related debt: nested vendored manifests under `edgerun-crypto/vendor/*` may still refer to upstream `zeroize`; those should disappear when the nested vendor tree is deleted or promoted.
