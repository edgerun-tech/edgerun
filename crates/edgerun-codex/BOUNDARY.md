# Lift Boundary

This workspace keeps the parts needed to build a web-facing Codex-like client while separating portable logic from native transport adapters.

## Portable layers

These crates are expected to build with `default-features = false` for browser/WASM-style runtimes:

- `codex-client`: transport-neutral request/response types, `HttpTransport`, retry helpers, SSE byte-stream helpers, and lightweight request telemetry traits.
- `codex-api`: generic Responses, compact, memories, models, SSE, and typed API clients that run over any `HttpTransport`.
- `codex-model-provider`, `codex-model-provider-info`, `codex-models-manager`: optional provider metadata, bearer/API-key auth attachment, model catalog loading, and refresh logic using injected HTTP transport.
- `codex-protocol`, `codex-app-server-protocol`, `codex-shell-command`, `codex-tools`, `codex-code-mode`: typed contracts, tool schemas, command parsing, and code-mode tool definitions.
- `codex-apply-patch`: standalone apply-patch parser and filesystem applier.
- `codex-core`: a lightweight facade over the useful lower layers.

The compiled `codex-core` facade now includes a transport-injected
`ModelClient` for the critical turn path:

- `ModelClient::stream_turn(...)` returns the typed `ResponseStream` from
  `codex-api`.
- `ModelClient::collect_turn(...)` drains the stream into final output text,
  reasoning deltas, completed items, token usage, server model, and turn end
  state.
- Native callers can use `ModelClient::new_native(...)` with the
  `native-transport` feature.
- Browser/WASM callers use `ModelClient::new(...)` with an `Arc<dyn
  HttpTransport>` backed by `fetch`.

This preserves model streaming, tools, reasoning controls, verbosity, JSON
schema output, compression, request metadata, session/thread headers, provider
auth, and provider retry policy without pulling in the product TUI, SQLite log
store, telemetry exporters, plugin scanning, or terminal/runtime state.

## Native transport layer

Native transport remains the default feature set for compatibility, but it is optional.

The `native-transport` feature enables:

- `codex-client::ReqwestTransport` and `CodexHttpClient` through the lighter `codex-client/reqwest-transport` feature.
- native request execution for callers that pass `Provider` and `SharedAuthProvider` directly.

The heavier `codex-client/native-transport` and `codex-api/native-transport`
features remain available for custom CA helpers, rustls provider setup,
ChatGPT Cloudflare cookie support, websocket/realtime clients, and local file
upload helpers.

The `model-provider` feature additionally enables:

- `codex-model-provider::create_model_provider`, which constructs a default native HTTP transport.
- `codex-core::ModelClient::from_model_provider(...)`.

Browser/WASM callers should disable default features and inject a fetch-backed `HttpTransport`:

```toml
codex-client = { workspace = true, default-features = false }
codex-api = { workspace = true, default-features = false }
codex-model-provider = { workspace = true, default-features = false, optional = true }
codex-core = { workspace = true, default-features = false }
```

Then call `create_model_provider_with_transport(...)` instead of the native-only `create_model_provider(...)`.

The original `codex-core` implementation remains in `core/src/lib.rs` and related files as reference material, but it is not compiled. The compiled entrypoint is `core/src/stub_lib.rs`, which exposes the portable client facade above.

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
