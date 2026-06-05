# Next Batch: DER/TLS Certificate Scanners

This batch should extend the existing DER, PEM, and TLS WAT stack into the
minimum X.509 certificate surface needed by TLS and QUIC verification paths. It
should not replace certificate verification, trust policy, TLS AEAD, chain
validation, or certificate generation.

Current WAT coverage already includes:

- `der-tlv.wat`: DER length, tag, and header scans.
- `der-asn1-basic.wat`: INTEGER, BIT STRING, OCTET STRING, NULL, SEQUENCE, and
  child iteration.
- `der-oid.wat`: OID root/arc decode and value validation.
- `pem-rfc7468.wat`: PEM label, boundary, base64 compaction, and encoded length.
- `tls-frame.wat`: record and handshake headers.
- `tls-vector.wat`: TLS vectors, extension iteration, and ALPN iteration.
- `tls-name.wat`: DNS name normalization and wildcard matching.
- `tls-clienthello.wat`: ClientHello body scan, SNI, ALPN, and extension lookup.
- `tls-certificate-list.wat`: TLS 1.3 Certificate message list and entry spans.

Existing composition already proves:

```text
tls-frame -> tls-certificate-list -> der-tlv -> der-asn1-basic -> der-oid
tls-frame -> tls-clienthello -> tls-vector -> tls-name
```

## Rust Surfaces Inspected

- `crates/protocol/edgerun-protocols/src/tls/certificate.rs`
  - `Certificate::from_der`
  - `Certificate::from_pem`
  - `Certificate::parse_list`
  - `Certificate::matches_hostname`
  - local `DerReader`
  - `parse_certificate_der`
  - `parse_validity`
  - `parse_time`
  - `extract_cn`
  - `parse_extensions`
  - `parse_san_dns_names`
  - `parse_spki_public_key`
  - signature verification helpers
- `crates/protocol/edgerun-protocols/src/tls/server/client_hello.rs`
  - `ClientHello::parse`
  - extension iteration patterns for SNI, ALPN, supported versions, groups,
    signature algorithms, and key shares.
- `crates/protocol/edgerun-protocols/src/tls/handshake.rs`
  - `ClientHelloBuilder`
  - `ServerHello::parse`
- `crates/protocol/edgerun-protocols/src/tls/name_match.rs`
  - canonical DNS name and wildcard policy.
- `crates/utility/edgerun-crypto/src/certs.rs`
  - PEM extraction and local X.509 construction helpers.
- `crates/utility/edgerun-crypto/src/pem_rfc7468.rs`
  - strict-ish PEM decoder/encoder surface and current lowercase-label
    divergence.

## Candidate Modules

### 1. `x509-certificate-core.wat`

Highest value candidate. It should scan the top-level certificate and
TBSCertificate layout and return spans into the input:

```text
x509_certificate_scan(ptr, len, out_ptr) -> i32
x509_tbs_scan(ptr, len, out_ptr) -> i32
x509_next_tbs_extra(ptr, len, offset, out_ptr) -> i32
```

Suggested certificate record:

```text
certificate_offset: u32
certificate_len: u32
tbs_offset: u32
tbs_len: u32
signature_algorithm_offset: u32
signature_algorithm_len: u32
signature_value_offset: u32
signature_value_len: u32
```

Suggested TBS record:

```text
version_offset: u32
version_len: u32
serial_offset: u32
serial_len: u32
signature_offset: u32
signature_len: u32
issuer_offset: u32
issuer_len: u32
validity_offset: u32
validity_len: u32
subject_offset: u32
subject_len: u32
spki_offset: u32
spki_len: u32
extras_offset: u32
extras_len: u32
```

Parity target:

- Rust `Certificate::from_der` top-level structure acceptance/rejection.
- Rust local `DerReader` behavior for optional v3 version and fixed TBS field
  order.
- Reject trailing certificate data, missing TBSCertificate, missing signature
  algorithm, missing signature value, truncated DER nodes, invalid DER length,
  and unsupported BIT STRING padding.

Replacement value:

- Strong. This can replace the local `DerReader` traversal inside
  `certificate.rs` while keeping the Rust `Certificate` struct and crypto
  verification canonical.
- It composes directly after `tls-certificate-list.wat`.

Blockers:

- Needs full-record parity against generated self-signed P-256 certificates and
  hand-built malformed DER cases.
- Must preserve `tbs_certificate_der` exactly, because signature verification
  hashes that byte span.

### 2. `der-time.wat`

Parse X.509 UTCTime and GeneralizedTime into Unix seconds:

```text
der_utc_time_decode(ptr, len) -> i64
der_generalized_time_decode(ptr, len) -> i64
der_time_decode(ptr, len, tag) -> i64
```

Packed return:

```text
low32  = status
high32 = unix_seconds if representable in u32
```

If dates beyond `u32::MAX` matter, use an output record with `u64` seconds.

Parity target:

- Rust `parse_utc_time`, `parse_generalized_time`, `two_digits`, and
  `unix_from_ymdhms`.
- Valid UTCTime around the 1950/2049 split.
- Valid GeneralizedTime beyond 2049.
- Reject missing `Z`, bad length, non-digit fields, month/day/hour/minute
  bounds, and pre-Unix dates.

Replacement value:

- Medium. It removes duplicate date arithmetic from certificate parsing and
  gives audit-friendly validity bounds.

Blockers:

- Rust currently accepts `second == 60`. Decide whether WAT should match that
  exactly or enforce stricter leap-second policy before replacement.
- The current Rust day validation allows any day 1..31 before civil conversion.
  WAT should either match Rust or make the stricter policy explicit.

### 3. `x509-name.wat`

Scan X.509 `Name` values and expose Common Name spans:

```text
x509_name_next_rdn(ptr, len, offset, out_ptr) -> i32
x509_rdn_next_attribute(ptr, len, offset, out_ptr) -> i32
x509_attribute_scan(ptr, len, out_ptr) -> i32
x509_name_find_common_name(ptr, len, out_ptr) -> i32
```

Parity target:

- Rust `extract_cn` and `string_value`.
- Recognize OID `2.5.4.3` from encoded body `[0x55, 0x04, 0x03]`.
- Accept string tags currently accepted by Rust: UTF8String `0x0c`,
  PrintableString `0x13`, and IA5String `0x16`.
- Ignore non-CN attributes and unsupported string tags.

Replacement value:

- Medium-high. This is a direct leaf replacement for subject/issuer CN
  extraction. It also helps chain signature prechecks that compare issuer CN to
  issuer subject CN.

Blockers:

- CN is not the hostname authority when SAN exists. Keep hostname policy in
  Rust and continue using `tls-name.wat` only as a normalized matcher.
- Name encoding has many string types. The first WAT module should only claim
  parity for the Rust-supported tags.

### 4. `x509-san.wat`

Scan Subject Alternative Name extensions and return DNSName spans:

```text
x509_extensions_next(ptr, len, offset, out_ptr) -> i32
x509_extension_scan(ptr, len, out_ptr) -> i32
x509_san_dns_next(ptr, len, offset, out_ptr) -> i32
```

Parity target:

- Rust `parse_extensions` and `parse_san_dns_names`.
- Recognize SAN OID `2.5.29.17` from encoded body `[0x55, 0x1d, 0x11]`.
- Handle optional critical BOOLEAN before OCTET STRING, matching Rust.
- Decode OCTET STRING containing GeneralNames SEQUENCE.
- Return only dNSName context-specific tag `0x82`; ignore other GeneralName
  forms.

Replacement value:

- High. SAN parsing feeds certificate hostname matching, which is a core TLS
  decision path in HTTP client and QUIC handshake code.

Blockers:

- WAT should return spans, not allocate names. Rust remains responsible for
  UTF-8 conversion and `tls-name` policy.
- Need malformed extension tests: critical BOOLEAN without value, non-OCTET
  extension value, bad nested SEQUENCE, truncated dNSName, and mixed DNS/IP/URI
  names.

### 5. `x509-spki-alg.wat`

Scan SubjectPublicKeyInfo and classify known algorithm OIDs:

```text
x509_spki_scan(ptr, len, out_ptr) -> i32
x509_algorithm_identifier_scan(ptr, len, out_ptr) -> i32
x509_signature_algorithm_classify(ptr, len) -> i64
```

Suggested algorithm ids:

```text
1 ec_public_key
2 rsa_encryption
3 ed25519
4 ecdsa_sha256
5 ecdsa_sha384
6 ecdsa_sha512
7 sha256_with_rsa
8 sha384_with_rsa
9 sha512_with_rsa
10 rsassa_pss
```

Parity target:

- Rust `parse_spki_public_key`, `first_oid`, and OID constants in
  `certificate.rs`.
- Return algorithm OID span and BIT STRING public-key span.
- Reject empty BIT STRING or nonzero unused-bit count.

Replacement value:

- High as a scanner. It helps separate byte extraction from crypto verification.
  The verifier still chooses Rust ECDSA/RSA/Ed25519 paths.

Blockers:

- Do not classify curve parameters as trust policy yet. P-256 curve OID is
  present in locally generated certs, but signature verification currently
  relies on Rust key parsing.
- RSA public key extraction in `edgerun_crypto::certs` has a separate scanner
  path; replacement should wait until X.509 SPKI parity and RSA PKCS#1 parity
  are both proven.

### 6. `x509-pem-cert.wat`

Optional wrapper composition module:

```text
x509_pem_certificate_scan(ptr, len, out_ptr) -> i32
```

This would compose PEM boundary scan plus base64 compacting length checks with a
DER certificate scan. It should not base64-decode unless the WAT module also
owns a strict base64 decode path.

Parity target:

- `Certificate::from_pem`
- `edgerun_crypto::certs::cert_from_pem`
- `pem_rfc7468::decode_label`

Replacement value:

- Conditional. Useful for proof-path strict PEM, but current Rust has two PEM
  surfaces: `pem_rfc7468` and the simpler `certs::pem_block`.

Blockers:

- Existing strictness mismatch: WAT rejects lowercase labels while
  `pem_rfc7468` accepts them. Keep strict proof-path PEM and compatibility PEM
  as named policies.
- `certs::pem_block` is simpler and label-specific; it does not enforce all
  RFC7468 rules.

## Parity Targets

Add a dedicated runner:

```text
standards/runners/rust-parity-x509-cert.js
```

It should generate a temporary Rust program that:

- builds a local P-256 self-signed cert with
  `edgerun_crypto::certs::self_signed_p256_der_for_names`;
- parses it through `Certificate::from_der`;
- prints top-level spans and semantic fields:
  - subject CN
  - issuer CN
  - notBefore/notAfter
  - SAN DNS list
  - subject public key algorithm OID
  - subject public key length
  - signature algorithm OID
  - signature value length
  - exact TBSCertificate DER hex
- emits malformed cases for each scanner.

Expected WAT comparison:

- `x509-certificate-core.wat` must locate the same TBS, issuer, validity,
  subject, SPKI, signature algorithm, and signature value spans.
- `der-time.wat` must produce the same validity seconds.
- `x509-name.wat` must find the same CN byte spans for issuer and subject.
- `x509-san.wat` must find the same DNSName byte spans.
- `x509-spki-alg.wat` must return the same algorithm OID and public-key bytes.

Keep known policy mismatches named rather than hidden:

- PEM lowercase label compatibility.
- TLS raw record parsing versus strict record validation.
- Possible stricter date validation if WAT rejects impossible calendar dates
  that Rust currently normalizes through `days_from_civil`.

## Composition Tests

Add composition runners in this order:

```text
codec-composition-tls-cert-core.js
codec-composition-pem-x509-san-name.js
codec-composition-tls-cert-spki-alg.js
codec-composition-tls-cert-validity.js
```

Target pipelines:

```text
tls-frame
-> tls-certificate-list
-> x509-certificate-core
-> x509-san
-> tls-name

pem-rfc7468
-> encoding-text/base64 strict decode if available
-> x509-certificate-core
-> x509-name
-> x509-san

tls-frame
-> tls-certificate-list
-> x509-certificate-core
-> x509-spki-alg
-> der-oid

tls-frame
-> tls-certificate-list
-> x509-certificate-core
-> der-time
```

The first two composition tests are the most valuable because they connect
certificate bytes to hostname-relevant output. They still must not claim trust
or chain validity.

## Replacement And Deletion Value

Good replacement candidates after parity:

- Local `DerReader` traversal inside
  `crates/protocol/edgerun-protocols/src/tls/certificate.rs`.
- `parse_validity`, `parse_time`, and digit/time arithmetic, if policy matches.
- `extract_cn` and `string_value` for the Rust-supported string tags.
- `parse_extensions` and `parse_san_dns_names` for SAN DNS spans.
- `parse_spki_public_key` extraction of algorithm OID and BIT STRING bytes.
- `Certificate::parse_list` list span scanning, already covered by
  `tls-certificate-list.wat`, once the certificate-entry-to-X.509 composition
  runner proves end-to-end parity.

Do not delete yet:

- `Certificate` as the Rust object model.
- `Certificate::matches_hostname`; it owns SAN-over-CN policy and Rust string
  ownership, though it can call `tls-name` helpers later.
- `verify_certificate_signature_with_issuer` and all ECDSA/RSA/Ed25519
  verification helpers.
- `Certificate::from_pem` until strict and compatibility PEM policies are split.
- `edgerun_crypto::certs` generation helpers.
- QUIC and HTTP client handshake certificate policy.

## Blockers

- The bridge to execute WAT through `/home/ken/edgerun-c` is not yet integrated
  into this repo. These modules can be authored and proven with Node runners
  first, then integrated through the bridge later.
- X.509 parsing must preserve exact byte spans for signed data. Any copy,
  normalization, or re-encoding before signature verification is unacceptable.
- Rust certificate code currently mixes byte scanning, allocation, semantic
  extraction, hostname policy, and signature verification in one file. WAT
  should replace only byte scanners; Rust keeps ownership and policy.
- Date strictness needs a documented decision before replacement.
- PEM has an existing lowercase-label divergence. Keep proof-path PEM strict.
- SubjectAltName parsing must return spans and ignore non-DNS names unless a
  later module explicitly supports IP address and URI GeneralName policies.

## Recommendation

The highest-value next batch is:

```text
300028 x509-certificate-core
300029 der-time
300030 x509-name
300031 x509-san
300032 x509-spki-alg
```

Build in this order:

1. `x509-certificate-core.wat`
2. `x509-san.wat`
3. `x509-name.wat`
4. `der-time.wat`
5. `x509-spki-alg.wat`

The first integration proof should be:

```text
tls-frame -> tls-certificate-list -> x509-certificate-core -> x509-san -> tls-name
```

That path is more valuable than another generic DER helper because it proves the
hostname-relevant certificate path without touching cryptographic verification.
It also creates a clean bridge target for replacing scanner internals inside
`certificate.rs` while keeping Rust certificate policy and trust decisions in
place.
