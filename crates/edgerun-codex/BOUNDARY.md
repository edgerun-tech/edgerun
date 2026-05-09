# Lift Boundary

This workspace keeps the parts needed to build a web-facing Codex-like client:

- `codex-api`: Responses, compact, memories, realtime, SSE, and websocket API clients.
- `codex-client`: HTTP transport, auth header abstraction, custom CA handling, compression.
- `codex-model-provider`, `codex-model-provider-info`, `codex-models-manager`: provider metadata, bearer/API-key auth attachment, model catalog loading and refresh.
- `codex-protocol`, `codex-app-server-protocol`, `codex-shell-command`, `codex-tools`, `codex-code-mode`: typed contracts, tool schemas, command parsing, and code-mode tool definitions.
- `codex-apply-patch`: standalone apply-patch parser and filesystem applier.
- `codex-core`: a stub facade over the useful lower layers.

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

## Current Verification

`cargo check --workspace` passes for the lifted workspace.

`cargo tree --workspace --edges normal` does not include the excluded Codex product crates as normal dependencies.
