---
title: Build your first Edgerun app
date: 2026-04-30
author: Ken
summary: Step-by-step instructions for creating and loading a browser app module in Edgerun.
tags: [edgerun, wasm, capabilities, tutorial]
---
# Build your first Edgerun app

This is the practical track for developers who want to extend Edgerun without
building a separate deployment layer.

Edgerun browser apps are Wasm modules loaded into the dashboard workspace. The
current host contract is small and inspectable.

## 1) Host contract at a glance

The runtime expects a module to provide:

- an optional exported function `edgerun_app_start`, and
- imports for command/query/capability/storage operations.

From the browser side, modules are loaded from the configured module URL and can
request capabilities before they become interactive.

## 2) Create a Rust app crate

Use a normal Rust library crate and target Wasm:

```bash
cd /path/to/your/apps
cargo new my-edgerun-app --lib
cd my-edgerun-app
rustup target add wasm32-unknown-unknown
```

Add your `Cargo.toml` entry:

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
```

Keep the implementation focused: write to the host via imported functions and
render only through browser-side DOM if you opt-in to JS glue.

## 3) Minimal Rust skeleton

```rust
#[link(wasm_import_module = "edgerun_host")]
extern "C" {
    fn log(ptr: *const u8, len: usize);
}

#[no_mangle]
pub extern "C" fn edgerun_app_start() {
    let message = "edgerun app started";
    unsafe {
        log(message.as_ptr(), message.len());
    }
}
```

This is intentionally tiny so you can verify imports and runtime wiring before
adding feature surface.

To call queries or commands, keep the ABI import signatures the same and pass JSON
bytes as UTF-8:

```rust
#[link(wasm_import_module = "edgerun_host")]
extern "C" {
    fn query(ptr: *const u8, len: usize) -> i32;
    fn command(ptr: *const u8, len: usize) -> i32;
    fn storage_get(ptr: *const u8, len: usize) -> i32;
    fn storage_put(ptr: *const u8, len: usize) -> i32;
    fn capability_request(ptr: *const u8, len: usize) -> i32;
}
```

Return values are currently acknowledged by the host and exposed through browser
events for your debugging workflow.

## 4) Build and publish the module

```bash
cargo build --target wasm32-unknown-unknown --release
```

Deploy the `.wasm` artifact to a URL the server can read, for example
`/modules/my-edgerun-app.wasm`.

## 5) Register it through config

`BrowserApp` is the config shape used by the server.

```yaml
kind: BrowserApp
spec:
  app_id: edgerun.sample
  title: Sample App
  module:
    url: /modules/my-edgerun-app.wasm
    sha256: 0123...deadbeef
  surfaces: [build-log]
  required_capabilities:
    - selector: git://edgerun.tech/*
      operations: [query]
      constraints: []
  optional_capabilities:
    - selector: vfs://browser/*
      operations: [read, write]
      constraints: [require-user-presence]
```

Notes:

- `surfaces` binds where the app can mount in the dashboard.
- `required_capabilities` entries are enforced by runtime policy.
- `optional_capabilities` are asked when optional paths are used.
- Keep `url` and `sha256` stable for reproducible deployment.

## 6) Load verification checklist

1. Start with the server config check:

```bash
cargo run -p edgerun-server -- --check-config --config /path/to/server.yaml
```

2. Open the app surface and check it appears in the dashboard.
3. Open browser console and verify module load logs.
4. Test a simple app path before adding network or persistence features.

The goal is to keep your first app deterministic: if it runs, then expand.
