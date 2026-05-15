# Direct Dependencies

Normal dependencies from the lifted workspace after pruning and the first Edgerun replacements:

| Crate | Direct normal dependencies |
| --- | --- |
| `codex-app-server-protocol` | `anyhow`, `codex-protocol`, `codex-shell-command`, `edgerun-clap`, `inventory`, `rmcp`, `schemars`, `serde`, `serde_json`, `serde_with`, `strum_macros`, `thiserror`, `tracing`, `uuid` |
| `codex-protocol` | `chardetng`, `chrono`, `edgerun-encoding`, `edgerun-glob`, `encoding_rs`, `icu_decimal`, `icu_locale_core`, `icu_provider`, `quick-xml`, `reqwest`, `schemars`, `serde`, `serde_json`, `serde_with`, `strum`, `strum_macros`, `sys-locale`, `thiserror`, `tokio`, `tracing`, `uuid` |
| `codex-shell-command` | `codex-protocol`, `edgerun-encoding`, `edgerun-json`, `edgerun-shlex`, `edgerun-url`, `regex`, `tree-sitter`, `tree-sitter-bash` |
| `codex-apply-patch` | `anyhow`, `async-trait`, `similar`, `thiserror`, `tokio`, `tree-sitter`, `tree-sitter-bash` |
| `codex-code-mode` | `async-channel`, `async-trait`, `codex-protocol`, `deno_core_icudata`, `serde`, `serde_json`, `tokio`, `tokio-util`, `tracing`, `v8` |
| `codex-api` | `async-channel`, `async-trait`, `bytes`, `chrono`, `codex-client`, `codex-protocol`, `edgerun-encoding`, `edgerun-url`, `eventsource-stream`, `futures`, `http`, `reqwest`, `serde`, `serde_json`, `thiserror`, `tokio`, `tokio-tungstenite`, `tokio-util`, `tracing`, `tungstenite` |
| `codex-client` | `async-trait`, `bytes`, `eventsource-stream`, `futures`, `http`, `opentelemetry`, `rand`, `reqwest`, `rustls`, `rustls-native-certs`, `rustls-pki-types`, `serde`, `serde_json`, `thiserror`, `tokio`, `tracing`, `tracing-opentelemetry`, `zstd` |
| `codex-core` | `codex-api`, `codex-apply-patch`, `codex-client`, optional `codex-model-provider`, optional `codex-model-provider-info`, optional `codex-models-manager`, `codex-protocol`, `codex-shell-command`, `codex-tools`, `edgerun-json`, optional native `edgerun-reqwest` |
| `codex-model-provider` | `async-trait`, `codex-api`, `codex-client`, `codex-model-provider-info`, `codex-models-manager`, `codex-protocol`, `codex-response-debug-context`, `http`, `reqwest`, `tokio`, `tracing` |
| `codex-model-provider-info` | `codex-api`, `codex-app-server-protocol`, `codex-protocol`, `http`, `schemars`, `serde` |
| `codex-models-manager` | `async-trait`, `chrono`, `codex-app-server-protocol`, `codex-protocol`, `serde`, `serde_json`, `tokio`, `tracing` |
| `codex-response-debug-context` | `codex-api`, `edgerun-encoding`, `edgerun-json`, `http` |
| `codex-tools` | `codex-app-server-protocol`, `codex-code-mode`, `codex-protocol`, `rmcp`, `serde`, `serde_json`, `tracing` |

Current direct registry dependency edges: 107.

`cargo tree --workspace --edges normal` currently has no normal dependency on the excluded product crates listed in `BOUNDARY.md`.
