# Deletion: edgerun-tungstenite

Date: 2026-06-05.

`crates/utility/edgerun-tungstenite` was a Rust compatibility facade for
tungstenite-style WebSocket APIs. It is deleted after extracting the portable
protocol value into WAT primitives.

WAT coverage now owns:

- `ws-accept.wat`: RFC WebSocket accept-key derivation using SHA-1 over
  `Sec-WebSocket-Key || 258EAFA5-E914-47DA-95CA-C5AB0DC85B11` and standard
  base64 output.
- `ws-frame.wat`: frame prefix decode, extended payload length decode,
  full header parse including FIN/RSV/opcode/mask/length/header length, server
  and general frame header formatting, mask/unmask XOR, and close payload
  status-code plus reason-byte shape.

Verified:

```bash
node standards/runners/ws-frame-smoke.js
node standards/runners/ws-accept-smoke.js
```

The deleted crate also contained sync/async stream wrappers, config builders,
HTTP request/response facades, and compatibility object conversions. Those are
host-language glue and were intentionally not preserved in WAT.

Remaining references to `edgerun-tungstenite` are broken caller fallout. Route
future WebSocket runtime work through `ws-frame.wat`, `ws-accept.wat`, and
owner-local host adapters rather than recreating a shared Rust compatibility
crate.
