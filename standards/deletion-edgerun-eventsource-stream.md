# Deletion: edgerun-eventsource-stream

`crates/utility/edgerun-eventsource-stream` is deleted.

Useful portable behavior is now owned by `sse-event-stream.wat` (`300072`):

- Server-Sent Events frame parsing.
- Blank-line dispatch for `\n\n` and `\r\n\r\n` line forms.
- `data:` accumulation with newline joining.
- Default `message` event type and `event:` override.
- `id:` update, carry, empty-id clear, and NUL-bearing id rejection.
- Numeric `retry:` capture.
- Comment lines beginning with `:` and unknown fields ignored.
- Optional one leading space after `:`.
- Strict UTF-8 rejection before field parsing.
- Bounded output records with assembled event/data/id spans.

The deleted Rust value was the async `Stream` wrapper, Rust `String` object
shape, generic transport error plumbing, and display/error API scaffolding.
Remaining Codex/API callers that import `edgerun_eventsource_stream` are
intentional caller-demolition fallout. They should route SSE byte buffers
through host-side WAT invocation and project the returned records locally, not
restore this compatibility crate.
