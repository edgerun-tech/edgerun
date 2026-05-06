# From Working Code To Provable Implementations

The current protocol crates already have the important raw material: explicit
wire parsers, serializers, no_std-friendly boundaries, and tests that cite RFC
sections. The missing step is turning those into stable evidence:

```text
standard requirement -> profile -> implementation hook -> trace -> checker -> finding
```

## What "Provable" Means Here

This does not mean proving the RFC itself correct. It means producing a
reproducible conformance claim:

```text
implementation artifact hash
+ selected profile
+ exact requirement IDs
+ exact checker WASM hashes
+ exact input corpus
+ exact implementation traces
= report hash
```

If any checker, requirement, registry snapshot, input case, or implementation
artifact changes, the report hash changes.

## Existing Edgerun Shape

Good proof targets already exist:

- `edgerun-tftp`: small RFC 1350/RFC 2347 surface, passing no-default-feature
  tests, clear `TftpMessage::from_wire` and `to_wire` boundary.
- `edgerun-dhcp`: RFC 2131 message codec and option parser, passing
  no-default-feature tests.
- `edgerun-http`: broader and more valuable, with existing conformance modules,
  but it needs requirement IDs, profile splits, and no_std test cleanup.
- `edgerun-quic`, `edgerun-tls`, `edgerun-dns`, `edgerun-hpack`,
  `edgerun-qpack`: useful later once the proof harness works on smaller
  protocols.

## Required Refactor Pattern

Each protocol should expose three deterministic surfaces:

1. `parse(bytes) -> parsed | error`
2. `serialize(parsed) -> bytes`
3. `validate(parsed or trace) -> findings`

The first two can be normal Rust library APIs. The third should return
requirement-addressed findings instead of only `Ok`/`Err`.

Example:

```rust
pub struct Finding {
    pub requirement: &'static str,
    pub severity: Severity,
    pub message: &'static str,
}
```

Tests can still assert `Ok`, but conformance reports need the requirement IDs.

## WASM Component Path

Do not move implementation logic into WASM first. Start by making checkers and
oracles WASM-compatible:

```text
Rust protocol crate
  -> adapter emits trace
  -> WASM checker consumes bytes/trace
  -> findings mention requirement IDs
```

Once the ABI stabilizes, selected parsers can also compile to WASM components.

## Recommended Order

1. TFTP packet codec proof.
2. DHCPv4 message/options proof.
3. HTTP token/header/status proof.
4. HTTP/2 frame proof.
5. HPACK integer/string proof.
6. DNS message/name compression proof.
7. TCP/UDP/IP lower-layer definitions.
8. TLS/QUIC trace-level conformance.

This order avoids starting with protocols whose correctness depends on large
state machines, cryptography, timers, congestion control, or interop behavior.

## HTTP Findings From Current Code

`edgerun-http` already has useful RFC-linked behavior in code:

- `src/header.rs`: HTTP token and field value validation.
- `src/method.rs`: method parsing and extension methods.
- `src/http2/frame/mod.rs`: HTTP/2 frame parsing, serialization, and frame
  semantic validation.
- `src/http2/connection_conformance.rs`: tests for preface, SETTINGS, stream
  state, and flow control.
- `src/http2/frame_sequence_conformance.rs`: a stateful frame sequence
  validator.

Immediate issues to resolve before claiming HTTP conformance:

- `src/http2/mod.rs` has conformance modules commented out, so those tests are
  not part of normal verification.
- `cargo test -p edgerun-http --features http2 --lib --no-default-features`
  currently fails in test code because some tests assume `std` or miss
  `alloc::vec`.
- `Method::from_str` accepts standardized method names case-insensitively, but
  RFC 9110 says the method token is case-sensitive. This might be acceptable for
  a permissive profile, but not for a strict HTTP semantics profile.
- HTTP/2 PUSH_PROMISE validation needs a requirement-level review. The code
  comment and even/odd check do not currently line up cleanly with the RFC
  rule that the promised stream identifier must be a valid next stream choice
  for the sender.

## First Concrete Milestone

The first milestone should be:

```text
edgerun-tftp RFC1350 packet-codec profile
```

Deliverables:

- requirement records for opcode, RRQ/WRQ, DATA, ACK, ERROR, and OACK wire
  forms,
- a corpus of valid and invalid byte strings,
- an adapter that calls `TftpMessage::from_wire` and `to_wire`,
- a trace file containing bytes, parse result, reserialization, and findings,
- a WASM checker that validates those traces using the shared WIT ABI.

Once that works, the same pattern can be applied to DHCP and HTTP.

