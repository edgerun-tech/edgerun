# EdgeRun X86 WASM Runtime Port

This port vendors the ER-owned x86_64 WASM interpreter and JIT runtime out of
`~/edgerun-c` so app/runtime WAT tests can execute against the same runtime
shape used by the metal tree instead of relying on JavaScript.

The checked app ABI v0 layout and policy constants live in
`abi/app-abi-v0.js`. The smoke runner imports that module for header sizes,
field offsets, magic constants, receipt masks, manifest requirements, and
capability allowlists.

`policy/app-policy-v0.js` owns the reusable ABI v0 policy checks: manifest
validation, ABI export validation, transition/render header decoding,
transition/render authority checks, canonical wasm hashing, and event hash
chain construction. Both the smoke runner and memory simulator use this module.

`sim/local-memory-sim.js` is the memory-only host simulator for ABI v0. It
compiles a WAT app, runs `er_init`, transition handlers, and `er_render` through
the native x86 runtime, decodes returned buffers from linear memory, and returns
the app id, logical clock, event hash chain, decoded transitions, and decoded
render header.

`sim/local-kernel-commit-v0.js` layers a local device-kernel commit simulation
on top of the memory simulator. It consumes decoded transitions, validates
cumulative memory/storage grants, produces kernel-signed receipt records,
queues append-only event-log and durable-result writes, and verifies replay of
the committed event/state-root chain.

`sim/local-identity-relay-v0.js` layers a memory-only identity relay on top of
committed kernel events. It consumes committed `route_send_identity` and
`route_create_hidden_service` intent records, verifies sealed payloads and
identity targets, registers hidden-service identities, and emits local transit,
delivery, and hidden-service registration receipts without exposing IP, DNS,
raw ports, or raw sockets to the app. Hidden-service local artifacts are backed
by the real `tor-library.wat` exports: the relay builds HSDir fetch requests,
publish headers, armored descriptors, contact frames, and message frames through
trusted native WAT calls, then stores only byte lengths and hashes in the local
result. The message frame is also fed through `er_tor_hs_app_handle_frame` so
the accepted-delivery path is WAT-validated. Contact and message delivery are
then replayed in a single loaded WAT instance with `er_tor_hs_app_init`,
`er_tor_hs_app_handle_frame`, `er_tor_hs_app_contact_count`,
`er_tor_hs_app_message_count`, `er_tor_hs_app_contact_ptr`, and
`er_tor_hs_app_message_ptr`; the relay decodes and hashes the module-owned
contact/message records from WAT memory.

`sim/local-tor-circuit-v0.js` layers a memory-only Tor circuit simulation on
top of the identity relay. It verifies the route payload is sealed, relay
receipts exist, and the hidden service is registered, then runs the compiled
`tor-cell-codec.wat` module through trusted native x86 WASM calls. The simulator
stages minimal CREATE2/CREATED2 and relay bodies, calls `tor_cell_build_fixed`,
decodes the serialized cells for `CREATE2`, `CREATED2`, `RELAY_EXTEND2`,
`RELAY_EXTENDED2`, `RELAY_BEGIN`, `RELAY_DATA`, and `RELAY_END`. `RELAY_EXTEND2`
uses the real 119-byte EXTEND2 body emitted by `tor-library.wat`. Circuit
receipts bind to decoded cell hashes from 304-byte canonical records emitted
by `tests/local-tor-cell-record-v0.wat`, app id, source event hash, and the
relay body hash without
opening raw sockets, performing DNS lookup, exposing a listen port, or carrying
plaintext payload bytes.

The circuit simulator also emits one canonical local delivery proof,
`edgerun.local-tor-delivery.v0`. That proof binds the app id, committed event
hash, identity-route receipts, hidden-service registration receipt, WAT-built
HSDir/descriptor/frame hashes, WAT-owned contact/message state hashes, every
codec-built Tor cell hash, and the circuit receipt hash. Its `proofHash` is the
SHA-256 of a 728-byte canonical binary record emitted by
`tests/local-tor-delivery-proof-v0.wat`; JS harness code may stage field bytes,
but it must not assemble this proof with string concatenation. The smoke runner
recomputes the WAT record and rejects tampered WAT state hashes or Tor cell
hashes.

The supported local Tor scaffold is codec-backed: harness code may choose
identities, sealed payload sizes, and receipt metadata, but Tor cell bytes and
hidden-service protocol artifacts must come from the real WAT primitives. The
runner also loads the real Tor WAT module
`standards/build/wasm/app-primitives/tor-wat/tor-library.wat` through the same
native runtime and proves role/capability selection, basic exported cell
accessors, `RELAY_BEGIN`, and `EXTEND2` builders. The native interpreter now
has regression coverage for nested `br_table` block branches, because the Tor
role selector depends on that shape. The runner also loads the compiled
`tor-cell-codec.wat` module as a trusted internal primitive and checks exported
protocol constants, command validation/classification, fixed-cell length,
relay-data limits, fixed-cell building, and destroy-body building. The compiled
codec relies on shapes that untrusted app WAT still rejects, such as mutable
globals, so it uses `er_fn_load_trusted` while app fixtures continue through the
stricter `er_fn_run_args` path.

Transition intent records are fixed-size ABI records stored immediately after
the transition header. The header counts determine how many records the kernel
decodes. The commit simulator requires these records for committed transitions;
synthetic policy-only tests may still exercise header rejection without them.

## Source Layout

```text
source/kernel/x86_64/wasm/
  wasm_interpreter.asm
  wasm_run.asm
  wasm_exec*.asm
  wasm_decode*.asm
  wasm_jit*.asm

source/kernel/x86_64/
  macros.inc
  wasm_defines.inc

source/kernel/x86_64/rt/
  runtime_mem.asm
  runtime_mem_unit.asm

source/kernel/test/
  test_recursion_valid.asm
  test_recursion_invalid.asm
  test_wasm_jit_self.asm
  test_wasm_float.asm
  test_jit.ld
  test_macros.inc
```

`wasm_interpreter.asm` is the umbrella runtime include. Untrusted module loading
goes through `er_fn_load`/`er_fn_run`, which call the no-recursion validator in
`wasm_run.asm`. Trusted internal modules may use `er_fn_load_trusted`, which
skips that policy gate.

## Smoke Runner

```bash
node standards/runners/edgerun-x86-wasm-runtime-smoke.js
```

The simulator can also be run directly:

```bash
node standards/ports/edgerun-x86-wasm-runtime/sim/local-memory-sim.js \
  standards/ports/edgerun-x86-wasm-runtime/tests/app-abi-v0.wat
```

The local kernel commit simulator can be run directly:

```bash
node standards/ports/edgerun-x86-wasm-runtime/sim/local-kernel-commit-v0.js \
  standards/ports/edgerun-x86-wasm-runtime/tests/app-abi-v0.wat
```

The local identity relay simulator can be run directly:

```bash
node standards/ports/edgerun-x86-wasm-runtime/sim/local-identity-relay-v0.js \
  standards/ports/edgerun-x86-wasm-runtime/tests/app-abi-v0.wat
```

The local Tor circuit simulator can be run directly:

```bash
node standards/ports/edgerun-x86-wasm-runtime/sim/local-tor-circuit-v0.js \
  standards/ports/edgerun-x86-wasm-runtime/tests/app-abi-v0.wat
```

The runner assembles copied test fixtures with `yasm`, links them with the
ported memory primitives through `ld`, and executes the resulting native test
binaries.

It also compiles WAT fixtures from `tests/` with `wat2wasm`, generates a small
x86 harness per exported assertion, and runs those modules through
`er_fn_run`. Negative WAT fixtures can assert expected native runtime errors
such as recursion or `memory.grow`, while policy fixtures assert host/app
requirements such as a required `er.manifest` custom section.

That keeps new app/runtime invariant tests in WAT while exercising the ER-owned
native WASM runtime rather than Node's WebAssembly engine.

Current untrusted app-module gates covered by WAT fixtures:

- exactly one `er.manifest` custom section with `abi=edgerun.app.v0`, `mode`,
  `memory.min`, `storage.min`, `recursion=false`, `memory.static=true`, and
  `storage.direct_access=false`
- ABI apps require `transition.emit`, `storage.intent`, and `render.emit`
  capabilities in the embedded manifest
- manifest capabilities are allowlisted; unknown or ambient authority names are
  rejected
- manifest dependencies, when present, must be pinned `name:sha256hex`
  executable identities; dynamic URLs, unpinned names, and postinstall hooks are
  rejected; pinned dependencies require `dependency.use`
- one wasm memory section whose minimum matches `manifest.memory.min`
- `memory.min` must be wasm-page aligned and `storage.min` must be storage-page
  aligned
- no wasm memory maximum; the user/kernel grant controls available memory
- function exports only; no exported memory, table, or globals
- no static data segments; transition buffers are written explicitly at runtime
- no direct recursion
- no `memory.grow`
- no start section
- no `call_indirect`
- no mutable globals
- no unresolved ambient imports such as direct storage, raw network, DNS, TLS,
  or hostcall authority
- no wall-clock or ambient-random imports; time and entropy must enter as
  explicit receipts or input material

The first ABI-shaped fixture is `tests/app-abi-v0.wat`. It exercises
`er_init`, `er_handle_message`, `er_handle_action`, and `er_render` through
`er_fn_run_args` with pointer/length style arguments and a preallocated runtime
memory grant staged into the native `RuntimeConfig`. The fixture writes
deterministic markers into app memory and the native harness verifies them after
each ABI call. The runner also validates the ABI export signatures:

```text
er_init(ctx: i32) -> i32
er_handle_message(ctx: i32, msg_ptr: i32, msg_len: i32) -> i32
er_handle_action(ctx: i32, action_ptr: i32, action_len: i32) -> i32
er_render(ctx: i32) -> i32
```

No extra `er_*` exports are allowed.

The identity fixtures `tests/app-identity-release.wat` and
`tests/app-identity-developer.wat` compile the same ABI body with different
embedded manifest modes. The runner hashes the resulting wasm bytes, asserts
that recompiling the same source is stable, and asserts that release/developer
mode changes app identity.

The ABI fixture now returns transition-buffer pointers for message/action
handlers. The checked v0 transition header is:

```text
u32 magic        ; 'TRAN'
u32 kind         ; 1=message, 2=action
u32 input_len
u32 clock_delta  ; must be 1 for accepted transitions
u32 storage_intent_count
u32 direct_storage_write_count ; must be 0
u32 storage_bytes_requested
u32 route_intent_count
u32 direct_network_write_count ; must be 0
u32 route_bytes_requested
u32 sealed_object_intent_count
u32 sealed_principal_mask ; device=1, app=2, user=4
u32 plaintext_private_bytes ; must be 0
u32 child_spawn_intent_count
u32 child_memory_bytes
u32 child_storage_bytes
u32 child_inspect_handle_count ; must be 0
u32 route_identity_intent_count
u32 raw_port_open_count ; must be 0
u32 dns_lookup_count ; must be 0
u32 event_append_intent_count
u32 immediate_storage_io_count ; must be 0
u32 cache_write_intent_count ; must be 0
u32 durable_result_intent_count
u32 receipt_intent_count
u32 receipt_required_mask ; storage=1, sealed=2, route=4, child=8
u32 unsigned_receipt_count ; must be 0
u32 fuel_used
u32 fuel_limit
u32 unmetered_loop_count ; must be 0
u32 previous_state_root_count ; must be 1
u32 next_state_root_count ; must be 1
u32 state_root_override_count ; must be 0
u32 emitted_message_count
u32 emitted_message_identity_target_count
u32 emitted_sealed_message_count
u32 plaintext_message_bytes ; must be 0
u32 contribution_receipt_count
u32 unpaid_resource_use_count ; must be 0
u32 resource_budget_overrun_count ; must be 0
u32 migration_intent_count
u32 user_signed_migration_count
u32 developer_to_release_migration_count
u32 hidden_service_intent_count
u32 raw_listen_port_count ; must be 0
u32 clearnet_ingress_count ; must be 0
u32 tls_intent_count
u32 tls_identity_route_count
u32 raw_tls_socket_count ; must be 0
u32 tls_plaintext_key_export_count ; must be 0
u32 dependency_call_intent_count
u32 dependency_pinned_identity_count
u32 dynamic_dependency_call_count ; must be 0
u32 dependency_receipt_count
u32 object_requirement_count
u32 public_object_intent_count
u32 integrity_object_intent_count
u32 object_without_requirement_count ; must be 0
```

Intent records are 32 bytes:

```text
u32 kind       ; 1=storage_append, 2=route_send_identity,
               ; 3=route_create_hidden_service, 4=sealed_store,
               ; 5=child_spawn, 6=dependency_call
u32 flags
u32 amount
u32 principal_mask
u32 aux0
u32 aux1
u32 target_lo
u32 target_hi
```

The positive ABI fixture writes storage and sealed records for
`er_handle_message`, and storage, identity-route, hidden-service registration,
and child-spawn records for `er_handle_action`. Kernel commit tests reject
missing records, route records that are not explicitly identity-targeted, and
hidden-service records that carry raw port-like material.

The runner hashes those transition headers with `app_id`, app clock, and the
previous event hash to exercise the append-only event-log invariant. Render
output is checked separately and must not advance the app clock. The checked
render header is:

For the positive ABI flow, the runner executes the WAT app through the native
x86 runtime, dumps the runtime linear memory after each ABI call, decodes these
headers from the returned pointers, and then applies policy checks to the
decoded buffers.

```text
u32 magic ; 'UI\0\0'
u32 sealed_ref_count
u32 plaintext_private_bytes ; must be 0
u32 sealed_input_request_count
u32 raw_input_capture_count ; must be 0
u32 reveal_without_user_action_count ; must be 0
u32 ui_action_intent_count
u32 render_direct_state_mutation_count ; must be 0
u32 render_direct_storage_mutation_count ; must be 0
u32 render_direct_network_mutation_count ; must be 0
```

The hash chain is order-sensitive: reversing the same transitions must produce
a different final event hash. It is also identity-bound: the same transition
sequence under developer-mode app bytes must not match the release-mode event
hash. Transition app identity claims are checked against the host-computed
canonical wasm hash; spoofed app ids and release/developer identity swaps are
rejected.

Transition policy fixtures also reject direct storage writes: app transitions
may queue storage intents, but the kernel owns actual storage IO scheduling.
Accepted transitions must tick the app clock exactly once. Transition buffers
must stay inside the memory grant, and queued storage bytes must stay inside the
manifest storage grant. Accepted transitions must bind exactly one previous
state root and emit exactly one next state root; app-supplied continuity
overrides are rejected. Accepted transitions must declare positive fuel use
within a positive fuel limit; unmetered loops are rejected. Storage intents must
append hashed events and produce durable results; immediate IO claims and
hidden cache writes are rejected.
Storage, sealed-object, route, and child-spawn intents must also produce
kernel-bound receipt intents before commit, and unsigned receipts are rejected.
Compute, storage, route, and child work must carry contribution receipts;
unpaid resource use and resource-budget overruns are rejected.
App identity migration requires explicit user-signed migration, and
developer-to-release migration is not implicit.
Networking follows the same shape: app transitions may queue route intents, but
direct network writes are rejected and route intents require the `route.intent`
capability. Route intents must target identities or hidden services; raw ports
and DNS lookup requests are rejected. Ingress is hidden-service only: raw listen
ports and clearnet ingress are rejected, and hidden-service intents require
`route.intent`. Emitted app messages use the same identity-routed substrate:
each message must target an identity or hidden service, each message must be
sealed, and plaintext message bytes are rejected. TLS is available only as an
explicit `tls.intent` bound to identity-routed transport; raw TLS sockets and
plaintext key export are rejected.
Dependency calls follow pinned executable identities only: dynamic dependency
resolution is rejected, dependency calls require `dependency.use`, and each
dependency call must carry a dependency receipt.
Object data follows the same explicit requirements model: public,
integrity-only, and sealed object intents must carry data requirement records,
and omitted object requirements are rejected. Private data remains explicit:
transitions may queue sealed-object intents, but private plaintext bytes are
rejected, sealed intents require `sealed.intent`, and the principal mask must
name a non-empty subset of device, app, and user. Render output has the same
privacy guard: it may
reference sealed data for the host UI pipeline and request sealed user input,
but it must not expose private plaintext bytes, capture raw input, or reveal
private data without user action. Render output is pure UI structure and
host-mediated action intent; direct state, storage, and network mutation from
render output is rejected.

Child apps follow the resource-transfer invariant: a transition may queue a
child-spawn intent only with `child.spawn`, and child memory/storage must be
moved out of the parent grant. Child handles are opaque; transition policy
rejects handles that expose child memory or storage inspection.
