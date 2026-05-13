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
- `crates/edgerun-codex/core/src/stub_lib.rs` already exposes the portable
  Codex turn path through `ModelClient`, `Prompt`, `TurnRequest`, and
  `ResponseEvent` without pulling in terminal state, rollout storage, plugin
  scanning, sandboxing, or native process state.
- `crates/edgerun-codex/tui/src/main.rs` is a useful native reference for
  converting user/assistant chat messages into Codex `ResponseItem`s and
  streaming `OutputTextDelta` events back into a transcript.
- `crates/protocol/edgerun-work/src/program_io.rs` and
  `src/std_runtime/program_process.rs` already define the host execution path:
  `ProgramIoService<NativeProcessAdapter>` accepts signed compute packets for
  open/stdin/poll/close and returns signed program events.

## Recommended split

Use three node identities instead of one overloaded process:

```text
browser chat app node
  -> signs user chat intent and stores chat projections

codex agent node
  -> runs the model turn and emits assistant/chat/tool-status messages

host executor node
  -> runs ProgramIoService<NativeProcessAdapter> under admission policy
```

The browser chat app should be the UI and local projection owner. It should
render the conversation, store `MessageObject`/payload rows, show proof hashes,
and submit user intent through admission.

The Codex agent should own model streaming and Codex protocol conversion. It can
use `codex_core::ModelClient` with either native transport on the host or an
injected fetch transport in a browser/WASM build. It should translate:

```text
user chat message -> codex_protocol::models::ResponseItem::Message(role=user)
assistant delta -> EdgeRun chat message draft/projection
completed assistant response -> finalized MessageObject + payload hash
tool request/status -> system/tool chat message projection
```

The host executor should remain a compute/capability leaf. It should not be
called directly by the browser UI. It should accept only admitted compute work
for the existing program work types:

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

For the first implementation, keep the message payload as text or a small JSON
projection stored in the existing `payloads` store. Keep `MessageObject` as the
authoritative index row and keep the full Codex `ResponseItem` list as derived
conversation state that can be rebuilt from message payloads.

## Execution flow

The clean flow is:

```text
browser chat app
  -> signs user message as chat work
  -> admission admits message to Codex agent
  -> Codex agent streams model output
  -> model asks for host command/tool
  -> Codex agent asks admission for host executor work
  -> host executor runs ProgramIoService<NativeProcessAdapter>
  -> stdout/stderr/exit events return as signed program event packets
  -> Codex agent summarizes or attaches events into chat
  -> browser stores/render chat projection and proof refs
```

This keeps the browser from becoming a privileged shell client and keeps the
executor from becoming a chat authority.

## What not to do

- Do not embed `std::process::Command`, native filesystem, sockets, or
  sandboxing into `frontend/chat.html`, `chat_wasm.rs`, or no-default
  `edgerun-work`.
- Do not make the Codex UI call a localhost shell endpoint directly. Route
  execution through admission and `ProgramIoService`.
- Do not duplicate Codex chat storage beside `chat_index.rs`. Use Codex
  protocol objects as payload/projection data, not as a new authority model.
- Do not make the chat app the decryption authority for private message
  history. The Trust Container capability remains the private key boundary.
- Do not add marketplace/app-specific protocol objects for this. Chat,
  capability, compute, admission, channel, proof, and receipt primitives are
  already enough.

## Minimal implementation sequence

1. Add a small `edgerun-codex` adapter crate or module that maps between
   `MessageObject` payloads and Codex `ResponseItem`s.
2. Add a host runner binary that owns a Codex agent node identity, creates a
   `ModelClient`, connects to admission/relay, and consumes admitted chat
   messages.
3. In that same host runner, expose a host executor node using
   `ProgramIoService<NativeProcessAdapter>` with a restrictive policy allowlist.
4. Update `frontend/chat.html` to add a Codex contact/thread type that sends
   messages to the Codex agent node through the existing envelope path.
5. Render program events as expandable proof-backed tool messages in the chat
   thread, with stdout/stderr/exit linked to packet/message hashes.
6. Once the loop works locally, bind it to real `WorkRequest`/`WorkAdmission`
   instead of saved route fields or demo route hashes.

The current local runner is:

```bash
cd crates/edgerun-codex
cargo run -p codex-host -- --listen 127.0.0.1:8787
```

For a no-network smoke test:

```bash
cd crates/edgerun-codex
cargo run -p codex-host -- --listen 127.0.0.1:8787 --mock-response 'mock reply: {input}'
```

Then run the static chat app:

```bash
cd frontend
bun run build:chat-wasm
bun run chat
```

Open `frontend/chat.html`, set the relay URL to `ws://127.0.0.1:8787`,
connect, import the contact card printed by `codex-host`, and send a message to
that contact.

## Open implementation decisions

- Whether the Codex agent runs as part of `edgerun-node` or as a separate
  `edgerun-codex-host` binary. A separate binary is lower risk now because
  `edgerun-codex` is already its own workspace.
- Whether assistant streaming should produce many append-only partial messages
  or one local draft plus one finalized signed message. Prefer local draft plus
  finalized message first; it avoids polluting the signed chat history with
  unstable deltas.
- Whether command stdout/stderr should be mirrored into text chat immediately
  or only attached after exit. Prefer streaming UI state plus finalized event
  refs so the signed history stays compact.
