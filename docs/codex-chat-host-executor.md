# Codex Chat And Host Executor Boundary

This note captures the current best integration path for putting Codex chat
messages in the EdgeRun chat app while keeping execution on a host node.

## Current building blocks

- `crates/protocol/edgerun-work/src/chat_index.rs` already defines the portable
  chat projection: contacts, thread ids, message objects, thread objects, and
  content-addressed message hashes.
- `crates/protocol/edgerun-work/examples/chat_wasm.rs` already exposes browser
  functions for message-node identity, signed contact cards, text message
  sealing, and indexing incoming channel envelopes.
- `frontend/chat.html` already stores contacts, threads, messages, payloads,
  inbox, and outbox in IndexedDB and treats WebSocket frames as EdgeRun work
  envelopes.
- `edgerun-zig/src/ui_stream.zig` already defines the UI streaming contract.
  Do not add a second UI protocol for Codex. Any native host bridge that wants
  to drive the Zig UI must emit the same `ui_stream` tree/patch bytes.
- `crates/edgerun-codex/core/src/stub_lib.rs` already exposes the portable
  Codex turn path through `ModelClient`, `Prompt`, `TurnRequest`, and
  `ResponseEvent` without pulling in terminal state, rollout storage, plugin
  scanning, sandboxing, or native process state.
- `crates/edgerun-codex/tui/src/main.rs` is a useful native reference for
  converting user/assistant chat messages into Codex `ResponseItem`s and
  streaming `OutputTextDelta` events back into a transcript. It is not the UI
  contract.
- `crates/protocol/edgerun-work/src/program_io.rs` and
  `src/std_runtime/program_process.rs` already define the host execution path:
  `ProgramIoService<NativeProcessAdapter>` accepts signed compute packets for
  open/stdin/poll/close and returns signed program events.

## UI stream contract

The Zig UI stream contract is the authority for host-to-UI updates:

```text
MessageType.tree  = 0
MessageType.patch = 1

patch bytes:
  [0] patch_kind    // ui_stream.PatchKind
  [1] component_id  // target component 0-255
  [2..] payload     // kind-specific binary payload
```

Payloads are the exact encodings in `ui_stream.zig`:

```text
bool                       [1] 0/1
u16                        [2] little-endian
f32                        [4] little-endian IEEE bits
string                     [1 len] [N utf-8 bytes], max 255 bytes
two strings                string string
two strings plus bool      string string [1]
Color                      [4] RGBA
```

A Codex/agent host must therefore adapt model and tool events into existing UI
patches such as `card_text`, `row_item`, `toast`, `progress_value`,
`button_label`, `textarea_placeholder`, `rect_color`, or `style_color`. It must
not invent JSONL, SSE, custom websocket message types, or another patch schema.

## Recommended split

Use three node identities instead of one overloaded process:

```text
Zig UI/app node
  -> owns pixels, input, component tree, ui_stream patch application, and local projection

codex agent node
  -> runs the model turn and emits assistant/tool/status updates as existing chat/work payloads
     and, when attached to the native UI, as ui_stream patch bytes

host executor node
  -> runs ProgramIoService<NativeProcessAdapter> under admission policy
```

The UI should remain the UI and local projection owner. It should render the
conversation, store `MessageObject`/payload rows, show proof hashes, apply
`ui_stream` patches, and submit user intent through admission.

The Codex agent should own model streaming and Codex protocol conversion. It can
use `codex_core::ModelClient` with native transport on the host. It should
translate:

```text
user chat message -> codex_protocol::models::ResponseItem::Message(role=user)
assistant delta -> ui_stream patch to the active assistant component + local draft state
completed assistant response -> finalized MessageObject + payload hash
model status -> ui_stream patch to status/toast/progress component
tool request/status -> system/tool-status payload + ui_stream row/card patch
```

The host executor should remain a compute/capability leaf. It should not be
called directly by the UI. It should accept only admitted compute work for the
existing program work types:

```text
WORK_TYPE_PROGRAM_OPEN
WORK_TYPE_PROGRAM_STDIN
WORK_TYPE_PROGRAM_POLL
WORK_TYPE_PROGRAM_CLOSE
WORK_TYPE_PROGRAM_EVENT
```

## Message model

Do not add a second chat schema just for Codex. Project Codex transcript items
into the existing chat model:

| Codex item | EdgeRun projection |
|---|---|
| User prompt | `MessageObject` with `CHAT_MESSAGE_KIND_TEXT` from the active UI/app node instance to the Codex agent node |
| Assistant output | `MessageObject` with `CHAT_MESSAGE_KIND_TEXT` from the Codex agent node to the active UI/app node instance |
| Model status | `CHAT_MESSAGE_KIND_SYSTEM` when it is user-visible state |
| Tool call begin/end | system/tool-status payload stored as content-addressed bytes |
| Command stdout/stderr | program event payloads referenced from a system/tool message, not raw ambient UI state |

For live UI updates, use `ui_stream` patches against the current component tree.
For durable history, keep `MessageObject` as the canonical projection row and
store the full Codex `ResponseItem` list only as derived state that can be rebuilt
from message payloads.

## Execution flow

The clean flow is:

```text
Zig UI/app
  -> signs user message as chat/work intent
  -> admission admits message to Codex agent
  -> Codex agent streams model output
  -> host bridge emits ui_stream patches for live draft/status updates
  -> model asks for host command/tool
  -> Codex agent asks admission for host executor work
  -> host executor runs ProgramIoService<NativeProcessAdapter>
  -> stdout/stderr/exit events return as signed program event packets
  -> host bridge mirrors live status with ui_stream patches
  -> Codex agent finalizes durable chat/tool payloads
  -> UI stores/render chat projection and proof refs
```

This keeps the UI from becoming a privileged shell client and keeps the executor
from owning chat state or admission decisions.

## What not to do

- Do not embed `std::process::Command`, native filesystem, sockets, or
  sandboxing into browser/WASM UI code or no-default `edgerun-work`.
- Do not make the Codex UI call a localhost shell endpoint directly. Route
  execution through admission and `ProgramIoService`.
- Do not add JSONL, SSE, or a custom Codex UI protocol beside
  `edgerun-zig/src/ui_stream.zig`.
- Do not duplicate Codex chat storage beside `chat_index.rs`. Use Codex protocol
  objects as payload/projection data, not as a new authority model.
- Do not make the chat app the decryption authority for private message history.
  The Trust Container capability remains the private key boundary.
- Do not add marketplace/app-specific protocol objects for this. Chat,
  capability, compute, admission, channel, proof, receipt, and UI-stream patch
  primitives are enough.

## Minimal implementation sequence

1. Add a small `edgerun-codex` adapter module that maps between `MessageObject`
   payloads and Codex `ResponseItem`s.
2. Add a host-side `ui_stream` encoder mirror for the exact Zig patch bytes:
   `patch_kind`, `component_id`, payload.
3. Add a host runner binary that owns a Codex agent node identity, creates a
   `ModelClient`, connects to admission/relay, and consumes admitted chat
   messages.
4. In that same host runner, expose a host executor node using
   `ProgramIoService<NativeProcessAdapter>` with an owner/autonomous local policy.
5. Wire model deltas and tool events into existing UI components by emitting
   `ui_stream` patches; do not create a separate UI transport schema.
6. Render program events as expandable proof-backed tool messages in the chat
   thread, with stdout/stderr/exit linked to packet/message hashes.
7. Once the loop works locally, bind it to real `WorkRequest`/`WorkAdmission`
   instead of saved route fields or demo route hashes.

The current local runner is:

```bash
cargo run -p codex-host -- --listen 127.0.0.1:8787 --base-url http://127.0.0.1:5000/v1 --model local
```

For TabbyAPI on another PC:

```bash
cargo run -p codex-host -- \
  --listen 127.0.0.1:8787 \
  --base-url http://TABBY_PC:5000/v1 \
  --model MODEL_NAME
```

If TabbyAPI requires an API key:

```bash
cargo run -p codex-host -- \
  --listen 127.0.0.1:8787 \
  --base-url http://TABBY_PC:5000/v1 \
  --api-key local \
  --model MODEL_NAME
```

For a no-network smoke test:

```bash
cargo run -p codex-host -- --listen 127.0.0.1:8787 --mock-response 'mock reply: {input}'
```

## Open implementation decisions

- Whether the Codex agent runs as part of `edgerun-node` or as a separate
  `codex-host` binary. A separate binary is lower risk now because
  `edgerun-codex` is already its own workspace.
- Which stable component ids the agent UI reserves for assistant draft, tool
  list, status/progress, stdout, stderr, and diff preview. The byte stream should
  still remain `ui_stream.zig`; only the component id registry needs to be fixed.
- Whether command stdout/stderr should be mirrored into text chat immediately or
  only attached after exit. Prefer streaming UI state plus finalized event refs
  so the signed history stays compact.
