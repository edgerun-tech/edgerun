# DNS/TLS Hardening Audit

> Updated: 2026-05-03
> Scope: `edgerun-dns`, `edgerun-tls`, `edgerun-acme`, `edgerun-rt`

## Status

The first runtime/ACME hardening pass has landed:

- `edgerun-rt::LazyStatic` and `OnceCell` unsafe auto-trait bounds were tightened.
- `OnceCell::try_insert()` no longer uses normal-path `unwrap()` / `expect()`.
- ACME order creation validates DNS identifiers before sending to a CA.
- ACME custom directory URLs require HTTPS, host, and no URL fragment.
- ACME JWK thumbprint canonical JSON was restored after the hardening regression.

This document tracks the remaining network-facing parser hardening work.

## DNS Parser Risks

### 1. Message size limits

`DnsMessage::from_wire()` currently parses arbitrary-sized input slices. UDP DNS should normally fit in 512 bytes unless EDNS0 is used; TCP/DoH may carry larger messages, but the parser still needs a hard cap.

Recommended constants:

```rust
const MAX_DNS_MESSAGE_LEN: usize = 65_535;
const MAX_DNS_SECTION_RECORDS: u16 = 256;
const MAX_DNS_QUESTIONS: u16 = 16;
```

Rules:

- Reject `data.len() > MAX_DNS_MESSAGE_LEN`.
- Reject `question_count > MAX_DNS_QUESTIONS`.
- Reject any RR section count above `MAX_DNS_SECTION_RECORDS`.
- Reject total section count overflow.

### 2. Compression pointer traversal

`decode_domain_name()` follows compression pointers but needs explicit loop detection and hop bounds.

Required behavior:

- Track pointer hops with `MAX_DNS_POINTER_HOPS`, for example `32`.
- Reject pointer target `>= data.len()`.
- Reject forward-pointer loops and self-pointers.
- Reject unknown label modes where `(len_byte & 0xC0) != 0 && != 0xC0`.
- Enforce DNS name max length: 255 octets on wire / 253 presentation chars.
- Enforce label max length: 63.

### 3. `domain_name_wire_len()` safety

`domain_name_wire_len()` currently returns a length even for malformed/truncated names. That can cause the caller to advance `pos` incorrectly after a malformed name.

Recommended replacement:

```rust
fn domain_name_wire_len(data: &[u8], offset: usize) -> Result<usize, io::Error>
```

Then update callers to use `?` instead of trusting a synthetic length.

### 4. Unknown record types

`DnsQuestion::from_wire()` and `DnsRecord::from_wire()` currently default unknown types to `A`. That is incorrect and can misparse unknown records.

Recommended model:

- Add `DnsRecordType::Unknown(u16)` if enum shape allows it; or
- keep `rtype_val` separately and parse unknown records as raw without pretending they are A.

At minimum, unknown query types should return `InvalidData` instead of silently becoming A.

### 5. RDATA embedded names

Several RDATA parsers call `decode_domain_name()` with `data` instead of full DNS message bytes. For compressed RDATA names, pointer targets are relative to the full message, not the RDATA slice.

Fix direction:

- Pass full message bytes plus `rdata_start` into `DnsRecordData::from_wire()` for types with embedded names.
- Keep raw parsing for unknown/untrusted compressed RDATA until full-message context is available.

## TLS Hostname Risks

### 1. Hostname normalization

`Certificate::matches_hostname()` should normalize both inputs:

- lowercase ASCII
- trim trailing dot
- reject empty hostname
- reject hostnames containing whitespace or NUL
- reject IP literals for DNS SAN matching unless IP SAN support exists

### 2. Wildcard rules

Current wildcard logic is close, but needs stricter RFC-style behavior:

- wildcard only allowed as the entire left-most label: `*.example.com`
- wildcard must not match the bare suffix: `example.com`
- wildcard must match exactly one label: `a.example.com`, not `a.b.example.com`
- wildcard should be rejected for public suffix-like two-label patterns if no PSL exists; at minimum reject `*.com` and `*.local`

### 3. SAN/CN fallback

Current behavior correctly makes SAN take precedence over CN. Keep that.

Additional guard:

- Empty SAN entries should be ignored.
- Invalid SAN DNS names should not match anything.

### 4. Certificate validity behavior

`is_valid_now()` returns `true` when system time is unavailable. That is acceptable for bare-metal/no-RTC mode only if callers treat it as degraded assurance.

Recommended follow-up:

- Add `is_valid_at(now_secs: u64)`.
- Make TLS client validation require an explicit clock policy.

## Next Code PR Plan

Recommended split:

1. `fix(dns): bound DNS message and section counts`
   - Small change in `message.rs` only.
   - Add tests for oversized messages and excessive counts.

2. `fix(dns): make domain-name decoding loop-safe`
   - Change `decode_domain_name()` and `domain_name_wire_len()` together.
   - Add tests for pointer self-loop, pointer chain loop, pointer out-of-bounds, label >63.

3. `fix(tls): harden hostname matching`
   - Normalize DNS names.
   - Reject invalid wildcard patterns.
   - Add unit tests for wildcard one-label behavior and trailing-dot behavior.

4. `fix(dns): stop defaulting unknown record types to A`
   - Either add unknown type support or reject unknown query types.

## Verification Commands

```bash
cargo test -p edgerun-dns
cargo test -p edgerun-tls
cargo test -p edgerun-acme
cargo check -p edgerun-dns
cargo check -p edgerun-tls
cargo check -p edgerun-acme
```
