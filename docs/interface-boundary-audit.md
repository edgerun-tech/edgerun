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

### 6.1 New `CommandExecutionContext` struct (edgerun-core/src/command.rs)
- Added `CommandExecutionContext` struct with owned data for passing execution context
- Includes: `has_local_session`, `has_user_presence`, `accepted_assurance_claims`, `transport_class`, `location_classes`, `target_stream_id`, `target_view_type`, `target_domain`, `execution_class`, `storage_class`
- Added `test_default()` constructor for testing

### 6.2 Canonical `build_command_result_payload()` helper (command_dispatch.rs)
- Created canonical helper that always includes `CommandRef` (command_id + command_hash)
- Replaced manual `CommandResultPayload` construction in 6 handlers:
  - `dispatch_create_identity`
  - `dispatch_import_identity`
  - `dispatch_add_bootstrap_node`
  - `dispatch_add_reachability_hint`
  - `dispatch_query_node_state`
  - `dispatch_request_user_presence`
  - `dispatch_request_signature`

### 6.3 Fixed `InstallApp` object path (command_dispatch.rs)
- Changed from `get_blob()` on `object_id` to `get_object()` on `ObjectRef`
- Added proper `AppPackage` decoding from object content
- Resolves `wasm_object` and `assets` as `ObjectRef`s

### 6.4 Updated `dispatch_command` signature (command_dispatch.rs)
- Added `exec_ctx: &CommandExecutionContext` parameter
- `CommandValidationContext` now built from `CommandExecutionContext` + node-local state
- Updated all call sites in `command_dispatch.rs` (tests) and `store_task.rs`

### 6.5 Renamed bootstrap dispatch (edgerun-app)
- Renamed `dispatch_commands_local` → `simulate_bootstrap_commands`
- Updated reference in `edgerun-app/src/main.rs`
- Returns `SimulationResult` (not protocol `CommandResultPayload`)

### 6.6 Fixed compilation errors (unrelated but blocking)
- Fixed `edgerun-secret-service`: `&[get_node_id()]` → `&get_node_id().to_vec()`
- Fixed `provisioning_listener.rs`: Added type annotation for `parse()`
- Fixed `command_dispatch.rs`: `as_str()` → `s as &str` (stable alternative)
- Fixed `command_dispatch.rs`: `package_bytes.as_slice()` → `package_bytes.content.as_slice()`
- Removed non-existent module imports (`metering`, `running_workloads`, `workload_policy`)
- Fixed brace mismatch errors in multiple locations
- Fixed duplicate code block in `dispatch_install_app`

---

## 7. Verification Results

### 7.1 Compile Checks
```bash
cargo check -p edgerun-core     # ✅ Success (1 warning: unused macro)
cargo check -p edgerun-storage   # ✅ Success
cargo check -p edgerun-node --features std  # ✅ Success
```

### 7.2 Test Compilation & Execution
```bash
cargo test -p edgerun-node --features std --no-run  # ✅ Success
cargo test -p edgerun-node --features std          # ✅ 105 tests passed, 0 failed
```

### 7.3 Fixed Issues During Implementation
1. Fixed `edgerun-secret-service/src/backend.rs:209`: `&[get_node_id()]` → `&get_node_id().to_vec()`
2. Fixed `store_task.rs`: Added `CommandExecutionContext` import and passing
3. Fixed `dispatch_command` calls: Updated all call sites to pass `exec_ctx`
4. Removed non-existent module imports (`metering`, `running_workloads`, `workload_policy`)
5. Fixed `as_str()` unstable feature usage → changed to `s as &str`
6. Fixed `package_bytes.as_slice()` → `package_bytes.content.as_slice()`
7. Fixed duplicate code block in `dispatch_install_app`
8. Fixed brace mismatch errors in multiple locations
9. Fixed `provisioning_listener.rs`: Added type annotation for `parse()`
10. Fixed test helpers: Updated to use `CommandExecutionContext::test_default()`

### 7.4 Remaining Work
- Extension command semantics and ControllerSet refactoring require larger changes (see section 3.4, 3.5)
- These are design-level changes that need careful consideration before implementation
