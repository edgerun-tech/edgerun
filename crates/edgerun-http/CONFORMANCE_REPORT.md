# HTTP Conformance Report

Generated: 2026-04-10
Crate: `edgerun-http`

---

## Summary

| Layer | Test Suite | Cases | Passed | Failed |
|-------|-----------|-------|--------|--------|
| **HPACK Decoder** | nghttp2/hpack-test-case (32 stories) | 292 | 292 | 0 |
| **HPACK Encoder** | raw-data round-trip (32 stories) | 292+ | 292+ | 0 |
| **HTTP/2 Frames** | http2jp/http2-frame-test-case (10 types) | 34 | 34 | 0 |
| **HTTP/2 Connection** | RFC 9113 §3.4, §5.1, §6.5 inline | 32 | 32 | 0 |
| **HTTP/2 Frame Sequences** | RFC 9113 frame ordering rules | 9 | 9 | 0 |
| **Typed Frame Structs** | PriorityFrame, ContinuationFrame, PushPromiseFrame round-trip | 4 | 4 | 0 |
| **HTTP Semantics** | RFC 9110, 9112, 3986 inline | 66 | 66 | 0 |
| **Total** | | **729+** | **729+** | **0** |

**196 unit tests + 3 doctests, all passing.**

**Run command:** `cargo test -p edgerun-http conformance`

---

## HPACK (RFC 7541)

**Status: ✅ PASS** — Encoder and decoder round-trip across 32 stories.

| What's Tested | Coverage |
|---|---|
| Indexed header field representation | ✅ |
| Literal header field with incremental indexing | ✅ |
| Literal header field without indexing | ✅ |
| Literal header field never indexed | ✅ |
| Dynamic table size update | ✅ |
| Huffman encoding/decoding | ✅ (via `hpack-patched` crate) |
| Dynamic table management | ✅ |
| Integer encoding (variable-length) | ✅ |
| **Encoder round-trip** | ✅ (encode → decode → compare) |

---

## HTTP/2 Frames (RFC 9113 / RFC 7540 Section 6)

**Status: ✅ PASS** — 34 test cases across all 10 frame types + 4 typed struct round-trips.

### Raw Frame Decoding

| Frame Type | Normal Cases | Error Cases | Semantic Validation |
|---|---|---|---|
| DATA | ✅ | ✅ | ✅ (stream 0 rejection) |
| HEADERS | ✅ | ✅ | ✅ (stream 0 rejection) |
| PRIORITY | ✅ | ✅ | ✅ (stream 0, size = 5) |
| RST_STREAM | ✅ | ✅ | ✅ (stream ≠ 0, size = 4) |
| SETTINGS | ✅ | ✅ | ✅ (stream 0, ACK size = 0, non-ACK % 6 = 0) |
| PUSH_PROMISE | ✅ | ✅ | ✅ (stream ≠ 0, promised ID ≠ 0 and odd) |
| PING | ✅ | ✅ | ✅ (stream 0, size = 8) |
| GOAWAY | ✅ | ✅ | ✅ (stream 0, size ≥ 8) |
| WINDOW_UPDATE | ✅ | ✅ | ✅ (size = 4, increment ≠ 0) |
| CONTINUATION | ✅ | ✅ | ✅ (stream ≠ 0) |

### Typed Frame Structs (Round-Trip)

| Type | Encode → Decode | Features |
|---|---|---|
| `PriorityFrame` | ✅ | Exclusive flag, stream dependency, weight |
| `ContinuationFrame` | ✅ | END_HEADERS flag, header block fragment |
| `PushPromiseFrame` | ✅ | Promised stream ID, padding support |

---

## HTTP/2 Connection (RFC 9113)

**Status: ✅ PASS** — 32 tests.

### Connection Preface (RFC 9113 §3.4)

| Test | Status |
|---|---|
| Preface is correct 24 bytes | ✅ |
| Preface is not a valid frame | ✅ |
| Truncated preface detection | ✅ |

### SETTINGS Negotiation (RFC 9113 §6.5, RFC 7540 §6.5.2)

| Test | Status |
|---|---|
| SETTINGS frame parsing | ✅ |
| ACK SETTINGS parsing | ✅ |
| ACK with payload rejected | ✅ |
| Non-zero stream ID rejected | ✅ |
| Partial payload (not % 6) rejected | ✅ |
| Default settings values | ✅ |
| Setting identifiers match RFC | ✅ |
| **Round-trip encode → decode → apply** | ✅ |
| **Custom values round-trip** | ✅ |
| **Invalid MAX_FRAME_SIZE rejection** | ✅ |
| **Invalid WINDOW_SIZE rejection** | ✅ |
| ACK frame has zero payload | ✅ |

### Stream State Machine (RFC 9113 §5.1 / RFC 7540 §5.1)

| Transition | Status |
|---|---|
| Idle → Open | ✅ |
| Open → HalfClosedLocal → Closed | ✅ |
| Open → HalfClosedRemote → Closed | ✅ |
| Reject Open from Closed | ✅ |
| Reject half-close from Idle | ✅ |
| Client streams are odd IDs | ✅ |
| Server streams are even IDs | ✅ |
| Manager creates streams in order | ✅ |
| Manager respects max concurrent | ✅ |
| Manager cleans up closed streams | ✅ |

### Frame Sequence Validation (RFC 9113 §5.1)

| Sequence | Status |
|---|---|
| SETTINGS → HEADERS → DATA (valid) | ✅ |
| DATA before HEADERS (invalid) | ✅ detected |
| CONTINUATION without HEADERS (invalid) | ✅ detected |
| HEADERS without END_HEADERS expects CONTINUATION | ✅ |
| HEADERS → CONTINUATION (valid) | ✅ |
| DATA after GOAWAY (invalid) | ✅ detected |
| DATA after RST_STREAM (invalid) | ✅ detected |
| RST_STREAM on stream 0 (invalid) | ✅ detected |
| GOAWAY on non-zero stream (invalid) | ✅ detected |

---

## HTTP Semantics (RFC 9110, RFC 9112, RFC 3986)

**Status: ✅ PASS** — 63 tests.

### Methods (RFC 9110 Section 9)

| Test | Status |
|---|---|
| Standard methods exist | ✅ |
| Case-insensitive parsing | ✅ |
| Invalid method rejection | ✅ |
| Extension methods (WebDAV, custom) | ✅ |
| `has_body()` semantics | ✅ |
| `expects_response_body()` | ✅ |

### Status Codes (RFC 9110 Section 15)

| Test | Status |
|---|---|
| Valid range (100-599) | ✅ |
| Out-of-range rejection | ✅ |
| Category checks (1xx-5xx) | ✅ |
| Registered reason phrases | ✅ |
| Unknown status reason phrase | ✅ |

### Headers (RFC 9110 Section 5.5, 5.6.2)

| Test | Status |
|---|---|
| Valid header names (strict tchar) | ✅ |
| Invalid header name rejection | ✅ |
| Valid header values | ✅ |
| Control + non-ASCII rejection | ✅ |
| HeaderMap operations | ✅ |

### URIs (RFC 3986, RFC 9112)

| Test | Status |
|---|---|
| Absolute URI parsing (HTTP/HTTPS) | ✅ |
| Origin-form parsing | ✅ |
| Request target (excludes fragment) | ✅ |
| Default ports, explicit ports | ✅ |
| Fragment handling | ✅ |
| Empty path, userinfo stripping | ✅ |
| Empty URI rejection | ✅ |

### HTTP/1.1 Request Parsing (RFC 9112)

| Test | Status |
|---|---|
| Basic GET | ✅ |
| POST with body (Content-Length) | ✅ |
| All standard methods | ✅ |
| Extension methods (PROPFIND) | ✅ |
| All request target forms | ✅ |
| Chunked request body | ✅ |
| Content-Length body boundary | ✅ |
| Invalid request rejection | ✅ |

### HTTP/1.1 Response Parsing (RFC 9112)

| Test | Status |
|---|---|
| Basic response parsing | ✅ |
| Multiple headers | ✅ |
| Body with Content-Length | ✅ |
| Chunked transfer encoding | ✅ |
| Chunk extensions | ✅ |
| **Trailer headers** | ✅ |
| No body for 1xx/204/304 | ✅ |
| Invalid status line rejection | ✅ |

---

## Known Gaps

| Area | Gap | Priority |
|---|---|---|
| HTTP/2 | No frame sequence validation for server push (PUSH_PROMISE → CONTINUATION) | Low |
| HTTP/2 | No SETTINGS negotiation round-trip with connection state application | Medium |
| HTTP/3 | Zero conformance tests | TODO |
| Interop | No h2spec tool integration | TODO |

---

## Test Data Sources

- **HPACK:** `specs/hpack-test-case/` — [http2jp/hpack-test-case](https://github.com/http2jp/hpack-test-case)
- **HTTP/2 Frames:** `specs/http2-frame-test-case/` — [http2jp/http2-frame-test-case](https://github.com/http2jp/http2-frame-test-case)
- **Semantics:** Inline tests based on RFC 9110, RFC 9112, RFC 3986
