# Lift Boundary

This workspace keeps the parts needed to build a web-facing Codex-like client while separating portable logic from native transport adapters.

## Portable layers

These crates are expected to build with `default-features = false` for browser/WASM-style runtimes:

- `codex-client`: transport-neutral request/response types, `HttpTransport`, retry helpers, SSE byte-stream helpers, and lightweight request telemetry traits.
- `codex-api`: generic Responses, compact, memories, models, SSE, and typed API clients that run over any `HttpTransport`.
- `codex-model-provider`, `codex-model-provider-info`, `codex-models-manager`: provider metadata, bearer/API-key auth attachment, model catalog loading, and refresh logic using injected HTTP transport.
- `codex-protocol`, `codex-app-server-protocol`, `codex-shell-command`, `codex-tools`, `codex-code-mode`: typed contracts, tool schemas, command parsing, and code-mode tool definitions.
- `codex-apply-patch`: standalone apply-patch parser and filesystem applier.
- `codex-core`: a lightweight facade over the useful lower layers.

## Native transport layer

Native transport remains the default feature set for compatibility, but it is optional.

The `native-transport` feature enables:

- `codex-client::ReqwestTransport`, `CodexHttpClient`, custom CA helpers, rustls provider setup, and ChatGPT Cloudflare cookie support.
- `codex-api` websocket/realtime clients and local file upload helpers.
- `codex-model-provider::create_model_provider`, which constructs a default native HTTP transport.
- `codex-core` default native facade behavior.

Browser/WASM callers should disable default features and inject a fetch-backed `HttpTransport`:

```toml
codex-client = { workspace = true, default-features = false }
codex-api = { workspace = true, default-features = false }
codex-model-provider = { workspace = true, default-features = false }
codex-core = { workspace = true, default-features = false }
```

Then call `create_model_provider_with_transport(...)` instead of the native-only `create_model_provider(...)`.

The original `codex-core` implementation remains in `core/src/lib.rs` and related files as reference material, but it is not compiled. The compiled entrypoint is `core/src/stub_lib.rs`.

## Deliberately Excluded

These product/platform integrations are outside the lifted boundary:

- persisted login and token refresh
- telemetry, analytics, feedback, rollout tracing
- TUI and terminal integrations
- MCP runtime and app/plugin installers
- hooks, skills, memories, state DB, thread store
- sandboxing, exec server, network proxy, Windows sandbox
- config schema generation and product config layering

## Verification targets

Native compatibility check:

```bash
cargo check --workspace
```

Portable browser-facing checks:

```bash
cargo check -p codex-client --no-default-features
cargo check -p codex-api --no-default-features
cargo check -p codex-model-provider --no-default-features
cargo check -p codex-core --no-default-features
```

Dependency sanity check:

```bash
cargo tree -p codex-core --no-default-features --edges normal
```

The no-default-features tree should not include native reqwest transport, rustls native roots, native websocket transport, or local filesystem upload helpers.
