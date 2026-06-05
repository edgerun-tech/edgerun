# HTTP/1 WAT Next Batch

This note scopes the next high-value HTTP/1 WAT batch after the current
`http1-scan`, `http1-lines`, `http1-body`, and `percent-url-form` primitives.
The goal is integration later, so this is a target map rather than an
implementation patch.

## Current Coverage

Existing WAT modules cover leaf scanners:

- `http1-scan.wat`
  - `http_find_crlf`
  - `http_find_double_crlf`
  - `http_validate_header_name`
  - `http_validate_header_value`
  - `http_value_has_token`
  - `http_parse_chunk_step`
- `http1-lines.wat`
  - `http_parse_request_line`
  - `http_parse_status_line`
  - `http_next_header`
- `http1-body.wat`
  - `http_parse_content_length`
  - `http_has_transfer_token`
  - `http_classify_body_framing`
- `percent-url-form.wat`
  - `uri_scan_path_query`
  - `form_urlencoded_next_pair`
  - strict percent encode/decode helpers

The current composition proof for HTTP/1 is:

```text
http1-lines -> percent-url-form
```

That proves request-target span flow into path/query/form scanning. It does not
prove full request/response acceptance, duplicate-header policy, body
consumption, chunk decoding, trailer parsing, or compatibility with
`HttpRequest::from_http` and `HttpResponse::from_http`.

## Rust Surfaces

The relevant Rust implementation lives in:

- `crates/protocol/edgerun-protocols/src/http/message.rs`
  - `HttpRequest::from_http`
  - `HttpResponse::from_http`
  - `to_http_bytes`
- `crates/protocol/edgerun-protocols/src/http/header.rs`
  - `HeaderName`
  - `HeaderValue`
  - `HeaderMap`
  - `header_value_has_token`
- `crates/protocol/edgerun-protocols/src/http/chunked.rs`
  - `has_chunked_transfer_coding`
  - `parse_body`
  - `parse_body_with_trailers`
- `crates/utility/edgerun-encoding/src/chunked.rs`
  - `decode_chunked`
  - chunked encoders
- `crates/protocol/edgerun-protocols/src/http/uri.rs`
  - `Uri::parse`
  - `Uri::request_target`
- `crates/protocol/edgerun-protocols/src/http/method.rs`
  - `Method::from_str`
- `crates/protocol/edgerun-protocols/src/http/status.rs`
  - `StatusCode::new`

Important current behavior:

- `HttpRequest::from_http` accepts any version starting with `HTTP/`.
- It ignores malformed header lines that do not contain `:`.
- It defaults a missing request `Host` to `localhost`.
- It rewrites relative targets into absolute `http://host/...` URIs.
- It truncates `Content-Length` bodies to available bytes.
- Invalid `Content-Length` on requests falls back to the full body.
- `HttpResponse::from_http` also accepts any version starting with `HTTP/`.
- Response `Content-Length` is parsed strictly, but bodies are still truncated
  to available bytes.
- Chunked decoding allocates materialized body bytes and later parses trailer
  bytes into `HeaderMap`.

Those behaviors are useful compatibility notes, but they are not a strict
canonical grammar for proof, admission, cache, or policy paths.

## Candidate Modules

### 1. `http1-message-strict`

Purpose: parse a complete HTTP/1 request or response into offset records without
allocating, copying body bytes, or constructing Rust objects.

Proposed exports:

```text
http1_parse_request_message(ptr, len, out_ptr) -> i32
http1_parse_response_message(ptr, len, out_ptr) -> i32
```

Request output record:

```text
kind
method_off
method_len
target_off
target_len
major
minor
headers_off
headers_len
body_off
body_len
body_kind
content_length_low
content_length_high
header_count
flags
```

Response output record:

```text
kind
major
minor
status
reason_off
reason_len
headers_off
headers_len
body_off
body_len
body_kind
content_length_low
content_length_high
header_count
flags
```

Value:

- Highest proof value because it aligns scan, parse, route, and body-boundary
  decisions.
- Gives the future Rust adapter one canonical preflight record before
  constructing `HttpRequest` or `HttpResponse`.
- Enables deterministic request/response hashing over exact wire spans.

Parity targets:

- Valid request with absolute URI.
- Valid request with origin-form target and `Host`.
- Valid request with empty body.
- Valid request with exact `Content-Length`.
- Valid response with exact `Content-Length`.
- Valid response with no-body status: 1xx, 204, 304.
- Reject missing `\r\n\r\n`.
- Reject malformed header without `:`.
- Reject invalid header name and invalid header value.
- Reject non-`HTTP/1.0` / `HTTP/1.1` versions unless a named compatibility mode
  is deliberately added.
- Reject missing `Host` for origin-form HTTP/1.1 requests.
- Reject `Content-Length` longer than available body.
- Reject conflicting duplicate `Content-Length`.
- Prefer `Transfer-Encoding: chunked` over `Content-Length` only under an
  explicit policy.

Replacement/deletion value:

- Good adapter target.
- Not a direct deletion target for `HttpRequest`, `HttpResponse`, or `HeaderMap`.
  Those remain the Rust semantic object model.
- Can eventually replace permissive parsing internals for strict/proof paths.

Blockers:

- Need an explicit compatibility-vs-strict policy for existing permissive
  `from_http` behavior.
- Need body-kind and status mappings shared with `http1-body.wat`.
- Need a clear host/target policy for proxy-form, authority-form, origin-form,
  and asterisk-form targets.

### 2. `http1-chunk-stream`

Purpose: turn the existing chunk step primitive into a full deterministic chunk
body scanner that returns chunk spans and trailer spans without materializing the
decoded body.

Proposed exports:

```text
http1_chunk_next(ptr, len, start, out_ptr) -> i32
http1_chunk_scan_body(ptr, len, out_ptr) -> i32
```

Chunk step output:

```text
size
size_line_off
size_line_len
data_off
data_len
next_off
is_final
flags
```

Full scan output:

```text
decoded_len_low
decoded_len_high
chunk_count
trailers_off
trailers_len
end_off
flags
```

Value:

- High replacement value for `edgerun_encoding::chunked::decode_chunked`
  preflight.
- Avoids allocating decoded body bytes just to prove framing correctness.
- Lets Rust copy body chunks only after a strict WAT scan proves boundaries.

Parity targets:

- Single chunk.
- Multiple chunks.
- Empty chunked body.
- Chunk extension.
- Trailer headers.
- Missing CRLF after chunk data.
- Truncated chunk data.
- Invalid hex size.
- Overflowing size.
- Missing final empty line after trailers.

Replacement/deletion value:

- Can replace scanner and preflight logic in `decode_chunked`.
- Should not delete Rust chunked encoders or body materialization yet.
- Should not delete trailer `HeaderMap` parsing until header-map span scanning
  and trailer policy are proven.

Blockers:

- Current Rust accepts last-chunk trailers by finding `\r\n\r\n`, and if that
  terminator is absent it uses the remaining bytes as trailer bytes. The strict
  WAT policy should reject missing trailer terminators.
- Need max decoded length policy so large chunked bodies do not overflow or
  become accidental allocation requests.

### 3. `http1-header-block`

Purpose: scan the full header block into validated spans and aggregate the
small header policies that matter for body, routing, cache, and proof paths.

Proposed exports:

```text
http1_header_block_scan(ptr, len, start, out_ptr) -> i32
http1_header_block_next(ptr, len, start, out_ptr) -> i32
```

Output summary:

```text
headers_off
headers_len
header_count
content_length_count
content_length_low
content_length_high
transfer_encoding_count
has_chunked
host_count
host_off
host_len
end_off
flags
```

Value:

- Strong support module for full-message parsing.
- Captures duplicate `Content-Length`, `Transfer-Encoding`, and `Host` policy in
  one deterministic record.
- Useful even before full parser integration as an adapter preflight around
  `HeaderMap`.

Parity targets:

- Empty header block.
- Case-insensitive `Host`, `Content-Length`, and `Transfer-Encoding`.
- Duplicate matching `Content-Length`.
- Duplicate conflicting `Content-Length`.
- `Transfer-Encoding: gzip, chunked`.
- `Transfer-Encoding: xchunked` rejection for token match.
- Invalid name.
- Invalid value byte.
- Header without colon.
- Empty header name.

Replacement/deletion value:

- Can replace repeated Rust header scanning/token helper calls.
- Does not replace `HeaderMap`, because Rust still owns allocation, lookup, and
  semantic object APIs.

Blockers:

- Need max header count and max header block length policy.
- Need to decide whether obs-fold is always rejected. The recommended strict
  policy is reject.

### 4. `http1-target`

Purpose: scan HTTP/1 request-target forms and compose them with
`percent-url-form.wat`.

Proposed exports:

```text
http1_scan_request_target(ptr, len, out_ptr) -> i32
http1_origin_target_scan(ptr, len, out_ptr) -> i32
http1_authority_target_scan(ptr, len, out_ptr) -> i32
```

Output:

```text
target_kind
scheme_off
scheme_len
authority_off
authority_len
host_off
host_len
port
path_off
path_len
query_off
query_len
fragment_off
fragment_len
flags
```

Value:

- High composition value with `percent-url-form.wat`.
- Good fit for canonical request proof paths because request-target bytes affect
  routing, cache keys, and capability policy.
- Separates strict canonical target parsing from current Rust URI normalization.

Parity targets:

- Origin-form `/path`.
- Origin-form `/path?x=1`.
- Asterisk-form `*`.
- Authority-form `example.com:443` for CONNECT.
- Absolute URI `http://example.com/path?x=1`.
- Absolute URI with IPv6 authority.
- Reject fragment in request target for canonical HTTP request paths.
- Reject invalid port.
- Reject missing host where required.
- Reject malformed percent escapes when strict percent decode is requested.

Replacement/deletion value:

- Can replace strict request-target preflight and proof hashing.
- Does not replace `Uri::parse` wholesale. Rust still owns display,
  construction, default ports, and compatibility parsing.

Blockers:

- Need explicit policy for default port insertion. Proof paths should hash the
  wire target, not an auto-normalized URI.
- Need method-aware target policy: CONNECT uses authority-form, OPTIONS may use
  `*`, normal methods use origin-form or absolute URI depending on role.

## Composition Tests

The next batch should include composition runners even before Rust integration:

```text
http1-message-strict
  -> http1-header-block
  -> http1-body
```

Cases:

- request with exact content length
- response with exact content length
- response with no-body status
- duplicate matching content length
- duplicate conflicting content length rejection
- transfer-encoding chunked classification

```text
http1-message-strict
  -> http1-chunk-stream
  -> http1-header-block for trailers
```

Cases:

- multiple chunks plus trailers
- chunk extension
- missing final trailer terminator rejection
- chunk size overflow rejection

```text
http1-message-strict
  -> http1-target
  -> percent-url-form
```

Cases:

- origin-form path/query
- strict percent decode
- form pair spans
- malformed percent escape rejection
- `+` policy separation between generic URI and form decode

Rust parity runners should compare WAT strict results with two Rust oracles:

- current compatibility parser behavior, to document intentional divergences
- new strict Rust test helpers or expected tables, to define the future adapter
  contract

## Replacement Order

1. Build `http1-header-block`.
   - It is the smallest high-value support module.
   - It creates clear duplicate `Content-Length`, `Host`, and transfer-token
     evidence.
2. Build `http1-chunk-stream`.
   - It gives the strict body proof path without copying decoded body bytes.
   - It prepares deletion of scanner/preflight portions of chunked decode.
3. Build `http1-message-strict`.
   - It composes line, header, and body classification into one record.
   - It is the first real parser adapter candidate.
4. Build `http1-target`.
   - It closes the request-target normalization gap and composes with
     `percent-url-form`.

This order gives useful integration checkpoints after each module and avoids
starting with the broadest parser.

## Recommended Next Batch

Put the next agents on this batch:

1. `http1-header-block.wat`
   - manifest
   - corpus
   - smoke runner
   - Rust parity runner additions around `HeaderMap`,
     `has_chunked_transfer_coding`, and duplicate `Content-Length`
2. `http1-chunk-stream.wat`
   - manifest
   - corpus
   - smoke runner
   - parity against `edgerun_encoding::chunked::decode_chunked`
3. HTTP/1 strict policy tests
   - no WAT changes required
   - enumerate expected strict accept/reject rows for request/response messages
4. Composition runner
   - `http1-header-block -> http1-body`
   - `http1-chunk-stream -> http1-header-block`

Defer `http1-message-strict.wat` until the header block and chunk stream modules
are proven. Defer `http1-target.wat` unless the immediate integration objective
is request routing/cache-key proof rather than parser replacement.

## Deletion Boundaries

Safe future deletion candidates after this batch proves parity:

- duplicated Rust scans for CRLF, header line validation, transfer-token checks,
  and `Content-Length` conflict checks in strict paths
- scanner/preflight portions of chunked decode

Not deletion candidates:

- `HttpRequest`
- `HttpResponse`
- `HeaderMap`
- `Uri`
- chunked encoders
- response helpers and builders
- compatibility `from_http` behavior unless a strict replacement policy is
  adopted deliberately

The WAT batch should make Rust construction safer and smaller, not remove the
semantic HTTP object model.
