# Dependency Sovereignty

Edgerun's target architecture is a `no_std`-first, Edgerun-owned library stack with no default dependency on crates.io or vendored third-party source.

This does not mean every third-party API disappears immediately. The migration strategy is:

1. Keep compatibility crate names where useful for existing call sites.
2. Replace the package behind those names with `edgerun-*` implementations.
3. Move reusable logic toward `no_std` + optional `alloc`.
4. Keep native, filesystem, network, TLS root, and process-global behavior behind explicit features.
5. Turn inventory checks from report-only into strict CI once debt is small enough to block regressions.

## Policy

Reusable Edgerun crates should depend on:

- Rust language crates: `core`, `alloc`, and narrowly justified `std` features.
- Edgerun-owned crates: `edgerun-*`.
- Lifted Codex crates inside the Codex workspace: `codex-*`.

Everything else is technical debt unless it is explicitly allowed by `scripts/dependency-sovereignty.py`.

## Replacement pattern

When replacing a dependency, prefer this pattern:

```toml
zeroize = { package = "edgerun-zeroize", path = "../edgerun-zeroize", default-features = false }
```

That keeps source imports stable while changing package ownership. After call sites are stable, APIs can be renamed from compatibility names to Edgerun names.

## Completed direct replacements

### `zeroize` → `edgerun-zeroize`

`zeroize` is now represented by `crates/utility/edgerun-zeroize`, a small `no_std` crate with optional `alloc` support.

Direct replacements completed:

- `crates/utility/edgerun-rustls-pki-types`
- `crates/utility/edgerun-rusttls`

Remaining related debt:

- Nested vendored manifests under `crates/utility/edgerun-crypto/vendor/*` may still reference upstream `zeroize` internally. Those trees should either be deleted after their code is fully internalized or patched to use `edgerun-zeroize` as part of the crypto cleanup.

### `subtle` → `edgerun-subtle`

`subtle` is now represented by `crates/utility/edgerun-subtle`.

Direct replacements completed:

- `crates/utility/edgerun-rusttls`

Implementation note: `edgerun-subtle` currently exposes the already-internalized implementation from `edgerun-crypto/src/subtle.rs` to avoid duplicating constant-time primitive code. The intended final shape is to make `edgerun-subtle` the canonical implementation and have `edgerun-crypto` consume it directly.

## CI status

`python3 scripts/dependency-sovereignty.py` runs in CI in report mode. It should become strict when the current findings are reduced to an intentional, reviewed allowlist.

Strict target:

```bash
python3 scripts/dependency-sovereignty.py --strict
```

## Prioritized replacement order

Replace small, foundational crates first:

1. `zeroize` → `edgerun-zeroize`.
2. `subtle` → `edgerun-subtle` constant-time primitives.
3. `once_cell` → `edgerun-once` / `edgerun-sync` minimal once-cell primitives.
4. `base64` / PEM helpers → `edgerun-encoding`.
5. `http` → finish moving `edgerun-http` away from re-exporting upstream `http`.
6. `tungstenite` → replace facade with Edgerun WebSocket protocol implementation.
7. `rustls-webpki` / `ring` → split TLS validation and crypto provider into Edgerun-owned modules.

Large crates such as TLS, WebSocket, ICU, schema generation, tree-sitter, and V8 should not be blindly copied. Keep them feature-gated while building minimal Edgerun-native subsets that match actual product requirements.
