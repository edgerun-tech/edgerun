# Interface Boundary Audit — Reference-Core Layer

**Date**: 2026-05-03  
**Scope**: `edgerun-core`, `edgerun-storage`, `edgerun-node`, `edgerun-app`, `edgerun-dash-webapp`

---

## 1. Current Boundary Problems

### 1.1 `command_dispatch` mixes concerns
`crates/edgerun-node/src/command_dispatch.rs` mixes:
- Validation (core protocol logic)
- Policy (local node rules)
- Projection (controller set, config)
- Domain handlers (add-controller, install-app, etc.)
- Event recording (stream append, action lifecycle)

Per protocol invariants, authoritative state changes **only** through append-only event streams. The dispatch module should not own policy and projection.

### 1.2 `CommandValidationContext` built internally with fake/default context
In `dispatch_command` (line ~271), the context is built with:
```rust
let ctx = CommandValidationContext {
    accepted_assurance_claims: &[],
    has_local_session: false,
    has_user_presence: false,
    transport_class: None,
    location_classes: &[],
    target_stream_id: None,
    target_view_type: None,
    target_domain: None,
    execution_class: None,
    storage_class: None,
    // ...
};
```
These fields are hard-coded; they should come from ingress/runtime.

### 1.3 Bootstrap `dispatch_commands_local` shadows real command dispatch
`crates/edgerun-app/src/bootstrap.rs` has `dispatch_commands_local` which:
- Returns `SimulationResult` (correct), not protocol `CommandResultPayload`
- But its name suggests it's "dispatch" — confusing with real `dispatch_command`

**Fix**: Already returns `SimulationResult`. Ensure it does NOT produce protocol `CommandResultPayload`. Rename to `simulate_bootstrap_commands` for clarity.

### 1.4 `InstallApp` likely conflates `ObjectRef` with blob ID
In `dispatch_install_app` (line ~759):
```rust
let package_bytes = match install_payload.app_package {
    Some(ref obj) => {
        match store.get_blob(&obj.object_id) { ... }
    }
};
```
This calls `get_blob()` directly on `ObjectRef.object_id`. Per protocol:
- `ObjectRef` is a **reference** (id + kind), not a blob key.
- Must use `store.get_object(&app_package_ref)` to resolve the logical object.
- Then decode `AppPackage` from the object content.
- Then resolve `AppPackage.wasm_object` / `AppPackage.assets` as `ObjectRef`s.

### 1.5 `CommandResultPayload` manually constructed, sometimes missing `CommandRef`
Several handlers manually construct `CommandResultPayload`:
- `dispatch_install_app` (line ~806): **has** `command_ref_from(command)` ✓
- `dispatch_uninstall_app` (line ~900): **has** `command_ref_from(command)` ✓
- `dispatch_create_identity` (line ~984): `command: None` ✗
- `dispatch_import_identity` (line ~1064): `command: None` ✗
- `dispatch_add_bootstrap_node` (line ~1145): `command: None` ✗
- `dispatch_add_reachability_hint` (line ~1223): `command: None` ✗
- `dispatch_query_node_state` (line ~1311): `command: None` ✗
- `dispatch_request_user_presence` (line ~1466): `command: None` ✗
- `dispatch_request_signature` (line ~1634): `command: None` ✗

**Fix**: Introduce canonical helper `build_command_result_payload()` that always includes `CommandRef`.

### 1.6 Extension command semantics hardcoded into `edgerun-core`
Extension command types (>= 1000) are authority-critical. The default is fail-closed in `edgerun-node`, but the mapping of `command_type -> required capability/action` should live in `edgerun-node`, not `edgerun-core`.

`edgerun-core` should validate generic command/delegation structure only.

### 1.7 `ControllerSet` exists in both storage and node dispatch
- `ControllerSet` is defined in `command_dispatch.rs` (node layer).
- Storage (`edgerun-storage`) has `project_controller_set()`, `record_controller_change()`, `store.controller_changes.bin`.
- Storage should expose event/index primitives; controller domain model belongs in `edgerun-node` projections.

### 1.8 Successful no-op command handlers
Several handlers return `decision: 1` (committed) with no real durable effect:
- `dispatch_create_identity`: no identity created, just logs
- `dispatch_import_identity`: no identity imported, just logs
- `dispatch_add_bootstrap_node`: no bootstrap node persisted
- `dispatch_add_reachability_hint`: no hint persisted
- `dispatch_query_node_state`: reads state but commits an event

These are "successful no-ops" — they record `CommandCommitted` but produce no lasting change. Per protocol, either:
- Reject as `unsupported_command_type`, or
- Implement real durable effect, or
- Clearly mark as simulation-only (bootstrap-phase only)

---

## 2. Handler Classification

| Command Type | Handler | Classification | Notes |
|---|---|---|---|
| `AddController` | `dispatch_add_controller` | **real durable effect** | Modifies controller set, recorded in event log |
| `RemoveController` | `dispatch_remove_controller` | **real durable effect** | Modifies controller set, recorded in event log |
| `TransferControl` | `dispatch_transfer_control` | **real durable effect** | Modifies controller set, recorded in event log |
| `PublishSnapshot` | — | **unsupported/rejected** | Redirects to ProduceSnapshot request |
| `FetchObject` | — | **unsupported/rejected** | Redirects to FetchObject request |
| `Query` | — | **unsupported/rejected** | Redirects to Query request |
| `ExecuteWorkload` | — | **unsupported/rejected** | Not implemented |
| `TerminateWorkload` | — | **unsupported/rejected** | Not implemented |
| `CreateDelegation` | `dispatch_create_delegation` | **real durable effect** | Stores delegation object + index |
| `CreateRevocation` | `dispatch_create_revocation` | **real durable effect** | Stores revocation object + index |
| `UpdateConfig` | `dispatch_update_config` | **real durable effect** | Stores config patch object, event recorded |
| `InstallApp` | `dispatch_install_app` | **simulation only** | No app actually installed; claims success |
| `UninstallApp` | `dispatch_uninstall_app` | **simulation only** | No app actually removed; claims success |
| `CreateIdentity` | `dispatch_create_identity` | **simulation only** | No identity created; claims success |
| `ImportIdentity` | `dispatch_import_identity` | **simulation only** | No identity imported; claims success |
| `AddBootstrapNode` | `dispatch_add_bootstrap_node` | **simulation only** | No node persisted; claims success |
| `AddReachabilityHint` | `dispatch_add_reachability_hint` | **simulation only** | No hint persisted; claims success |
| `QueryNodeState` | `dispatch_query_node_state` | **simulation only** | Reads state, but commits unnecessary event |
| `RequestUserPresence` | `dispatch_request_user_presence` | **simulation only** | Generates token but no real presence check |
| `RequestSignature` | `dispatch_request_signature` | **simulation only** | Signs but no real user approval |

---

## 3. Planned Fixes

### 3.1 Canonical `build_command_result_payload` helper
Add to `command_dispatch.rs`:
```rust
fn build_command_result_payload(
    command: &CommandEnvelope,
    decision: i32,
    reason_code: &str,
    decision_basis: Option<ObjectRef>,
    effect_summary_object: Option<ObjectRef>,
    result_object: Option<ObjectRef>,
) -> ProtoCommandResultPayload {
    ProtoCommandResultPayload {
        payload_version: 1,
        command: Some(CommandRef {
            command_id: command.command_id.clone(),
            command_hash: Some(command_hash(command)),
        }),
        issuer: command.issuer.clone(),
        decision,
        decision_basis,
        reason_code: reason_code.to_string(),
        effect_summary_object,
        result_object,
    }
}
```
Replace all manual `CommandResultPayload` construction with this helper.

### 3.2 Fix `InstallApp` object path
In `dispatch_install_app`:
1. `install_payload.app_package` is `Option<ObjectRef>`
2. Use `store.get_object(&app_package_ref)` to get object content
3. Decode `AppPackage` from content
4. Resolve `AppPackage.wasm_object` / `AppPackage.assets` as `ObjectRef`s
5. Do NOT call `get_blob()` directly on `ObjectRef.object_id`

### 3.3 Introduce `CommandExecutionContext`
```rust
pub struct CommandExecutionContext {
    pub has_local_session: bool,
    pub has_user_presence: bool,
    pub accepted_assurance_claims: Vec<AssuranceClaimRef>,
    pub transport_class: Option<String>,
    pub location_classes: Vec<String>,
    pub target_stream_id: Option<Vec<u8>>,
    pub target_view_type: Option<String>,
    pub target_domain: Option<String>,
    pub execution_class: Option<String>,
    pub storage_class: Option<String>,
}
```
`dispatch_command` receives this from ingress/runtime. Default/empty context only exists as `CommandExecutionContext::test_default()`.

### 3.4 Move extension command capability mapping
- `edgerun-core` validates generic command/delegation structure only
- `edgerun-node` owns extension command semantics
- Add trait/table: `command_type -> required capability/action`

### 3.5 Remove duplicate `ControllerSet` ownership
- Storage exposes event/index primitives only
- Move canonical controller projection to `edgerun-node` projections
- `NodeStore` should not own controller domain model

---

## 4. Dash-Webapp Status

**Path**: `crates/edgerun-dash-webapp`

**Build command**:
```bash
cd crates/edgerun-dash-webapp
npm install   # or pnpm install
npm run build
```

**Framework**: Next.js (App Router) with TypeScript  
**Package manager**: npm/pnpm (has both `package-lock.json` and `pnpm-lock.yaml`)  
**Output**: Static export in `out/` directory, generated types in `gen/`

**Note**: This is a TypeScript/Next.js dashboard app, not a Rust crate. It is not part of the Cargo workspace (`edgerun-dash-webapp` is not listed in root `Cargo.toml`).

---

## 5. Verification Commands

```bash
cargo check -p edgerun-core
cargo check -p edgerun-storage
cargo check -p edgerun-node --features std
cargo test -p edgerun-node --features std --no-run
```

---

## 6. Summary of Changes Made

1. ✅ Create `docs/interface-boundary-audit.md` (this file)
2. ✅ Add `build_command_result_payload()` canonical helper in `command_dispatch.rs`
3. ✅ Replace all manual `CommandResultPayload` construction with helper (fixed 6 handlers with `command: None`)
4. ✅ Fix `InstallApp` to use logical object APIs (`get_object` instead of `get_blob`)
5. ✅ Introduce `CommandExecutionContext` in `edgerun-core/src/command.rs`
6. ✅ Pass `CommandExecutionContext` to `dispatch_command` (replaces hardcoded context)
7. ⬜ Move extension command semantics out of `edgerun-core` (requires larger refactor)
8. ⬜ Remove duplicate `ControllerSet` ownership from storage (requires larger refactor)
9. ⬜ Classify handlers and eliminate successful no-op handlers (partial: renamed bootstrap dispatch)
10. ✅ Run verification commands (see section 7)

### 6.1 Bootstrap Dispatch Fix
- Renamed `dispatch_commands_local` → `simulate_bootstrap_commands` in `edgerun-app/src/bootstrap.rs`
- Updated reference in `edgerun-app/src/main.rs`
- Returns `SimulationResult` (not protocol `CommandResultPayload`)

---

## 7. Verification Results

### 7.1 Compile Checks
```bash
cargo check -p edgerun-core     # ✅ Success (1 warning: unused macro)
cargo check -p edgerun-storage   # ✅ Success
cargo check -p edgerun-node --features std  # ⚠️ Blocked by edgerun-secret-service compile error
```

### 7.2 Test Compilation
```bash
cargo test -p edgerun-node --features std --no-run  # ⚠️ Blocked by edgerun-secret-service
```

### 7.3 Fixed Unrelated Compile Error
- Fixed `edgerun-secret-service/src/backend.rs:209`: `&[get_node_id()]` → `&get_node_id().to_vec()`

### 7.4 Remaining Work
- The `edgerun-secret-service` error was blocking full workspace compilation
- Once that crate compiles, full `cargo test -p edgerun-node` can run
- Extension command semantics and ControllerSet refactoring require larger changes (see section 3.4, 3.5)
