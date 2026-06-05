# Next WAT Codec Batch: DNS

This note scopes the next high-value DNS WAT codec batch. It is a planning
document only. Do not edit Rust DNS parsers or delete Rust code until the WAT
modules have Rust-backed parity, composition runners, and a host adapter path.

Current DNS WAT coverage:

- `dns-name.wat`: uncompressed RFC1035 wire-name scan and lowercase ASCII output.
- `dns-message-header.wat`: 12-byte header decode/encode, flags classify, and
  current section-count limits.

Current Rust DNS surfaces inspected:

- `crates/protocol/edgerun-protocols/src/dns/message.rs`
- `crates/protocol/edgerun-protocols/src/dns/record.rs`
- `crates/protocol/edgerun-protocols/src/dns/name.rs`
- `crates/protocol/edgerun-protocols/src/dns/limits.rs`
- Existing runners and audits under `standards/runners/` and `standards/wat-*.md`

## Summary

The highest-value next DNS batch is not a full DNS parser. It is a bounded wire
preflight layer that can walk common DNS query/response packets, resolve
compressed owner names safely, identify question and resource-record spans, and
classify EDNS0 OPT records before Rust allocates full `DnsMessage` and
`DnsRecordData` objects.

Recommended batch:

1. `dns-compressed-name.wat`
2. `dns-section-walk.wat`
3. `dns-rr-header.wat`
4. `dns-edns0-opt.wat`
5. Optional `dns-rdata-shape.wat` for strict fixed-size/common RDATA preflight

This batch directly closes the largest current DNS blocker: `dns-name.wat`
intentionally rejects compression pointers, while Rust `decode_domain_name`
follows pointers during message and RDATA parsing.

## Candidate Modules

### 1. `dns-compressed-name.wat`

Exports:

- `dns_name_wire_len(ptr, msg_len, start) -> u64`
- `dns_name_follow(ptr, msg_len, start, out_ptr, out_cap) -> i32`
- `dns_name_to_lower_ascii_compressed(ptr, msg_len, start, out_ptr, out_cap) -> u64`

Output record for `dns_name_follow`:

```text
0:u32 consumed_wire_bytes_at_start
4:u32 label_count
8:u32 normalized_name_len
12:u32 pointer_count
16:u32 terminal_offset
20.. repeated u32 label_offset, u32 label_len
```

Rules:

- Accept ordinary labels and RFC1035 compression pointers.
- Reject pointer loops.
- Reject pointers outside the message.
- Reject pointers into the middle of a label.
- Reject reserved top-bit label forms.
- Enforce label length <= 63 and normalized name length <= 253.
- Cap pointer traversal, for example at 16 jumps or message length, whichever is
  lower.

Parity targets:

- Rust `record.rs`: `decode_domain_name`.
- Rust `message.rs` and `record.rs`: local `domain_name_wire_len`.
- Existing mismatch case in `rust-parity-misc-codecs.js`:
  `dns_wire:compressed_suffix`.

Replacement/deletion value:

- High. This is the missing primitive before WAT can participate in full DNS
  message preflight.
- Do not delete Rust `decode_domain_name` yet. First use WAT as a stricter
  verifier in front of Rust parsing and as a parity oracle.

Blockers:

- Rust currently does not detect pointer loops explicitly. A self-pointer can
  loop forever. WAT should be strict and bounded, but parity must record this as
  an intentional Rust bug/strictness delta until Rust gains bounded traversal.
- Rust takes an `offset_map`, but normal message parsing maps offsets to
  themselves. WAT should operate on full message bytes and absolute offsets.

### 2. `dns-section-walk.wat`

Exports:

- `dns_message_walk(ptr, len, out_ptr, out_cap) -> i32`
- `dns_question_next(ptr, len, offset, out_ptr) -> i32`
- `dns_rr_next(ptr, len, offset, out_ptr) -> i32`

Question output record:

```text
0:u32 start
4:u32 next
8:u32 name_consumed
12:u32 qtype
16:u32 qclass
20:u32 name_label_count
```

RR output record:

```text
0:u32 start
4:u32 next
8:u32 owner_name_consumed
12:u32 rr_type
16:u32 rr_class
20:u32 ttl
24:u32 rdlength
28:u32 rdata_start
32:u32 section_index
36:u32 record_index
```

Message-walk output should summarize:

```text
0:u32 question_start
4:u32 answer_start
8:u32 authority_start
12:u32 additional_start
16:u32 end_offset
20:u32 opt_count
24:u32 tsig_count
28:u32 error_offset
```

Rules:

- Compose `dns-message-header` counts and `dns-compressed-name`.
- Walk question, answer, authority, and additional sections without allocating
  Rust strings or record objects.
- Check that each question has 4 bytes after the name.
- Check that each RR has 10 bytes after the owner name.
- Check `rdata_start + rdlength <= len`.
- Return exact offsets for all sections and final end.

Parity targets:

- Rust `DnsQuestion::from_wire`.
- Rust `DnsRecord::from_wire` header span and `rdlength` bounds.
- Rust `DnsMessage::from_wire` section order and count traversal.
- Rust `limits.rs`: `validate_dns_wire_bounds`.

Replacement/deletion value:

- Very high as a pre-parse guard for network-facing DNS.
- Good first integration target after the `edgerun-c` runtime bridge because it
  returns fixed records and does not own DNS semantics.
- Do not delete `DnsMessage`, `DnsQuestion`, or `DnsRecord`; they are canonical
  Rust object models.

Blockers:

- Rust maps unknown `qtype` and `rtype` to `A` in some parse paths. WAT should
  preserve raw type values and avoid defaulting.
- Rust does not require parsed packets to consume the entire input. Decide
  whether WAT strict mode rejects trailing bytes or reports `end_offset` and
  lets policy decide.

### 3. `dns-rr-header.wat`

Exports:

- `dns_question_header_decode(ptr, len, offset, out_ptr) -> i32`
- `dns_rr_header_decode(ptr, len, offset, out_ptr) -> i32`
- `dns_rr_type_classify(rr_type) -> i32`

Scope:

- Fixed question trailer: QTYPE/QCLASS.
- Fixed RR trailer: TYPE/CLASS/TTL/RDLENGTH.
- Record-type classification only; no semantic record parsing.

Parity targets:

- `DnsRecordType::from_u16` mapping.
- `DnsQuestion::from_wire` qtype/qclass extraction.
- `DnsRecord::from_wire` type/class/ttl/rdlength extraction.

Replacement/deletion value:

- Medium on its own, high when composed with `dns-section-walk`.
- Useful for deleting duplicate byteorder reads inside WAT-backed DNS preflight,
  not for deleting Rust record models.

Blockers:

- Rust fallback to `A` for unknown qtype/rtype is not suitable for canonical
  proof paths. WAT should return raw type plus a known/unknown classifier bit.

### 4. `dns-edns0-opt.wat`

Exports:

- `dns_opt_rr_decode(ptr, len, rr_start, out_ptr) -> i32`
- `dns_opt_option_next(ptr, len, option_offset, rdata_end, out_ptr) -> i32`

OPT output record:

```text
0:u32 udp_payload_size
4:u32 ext_rcode
8:u32 version
12:u32 flags
16:u32 dnssec_ok
20:u32 options_start
24:u32 options_len
28:u32 option_count
```

Option output record:

```text
0:u32 option_code
4:u32 option_len
8:u32 option_data_start
12:u32 next
```

Rules:

- Owner name must be root.
- Type must be 41.
- TTL splits into extended RCODE, version, and flags.
- Version 0 is strict-valid; nonzero versions should be reported distinctly.
- RDATA is option TLVs: code, length, value.

Parity targets:

- Rust `DnsRecord::opt`.
- Rust `DnsMessage::opt_record`, `has_edns0`, and `udp_payload_size`.
- Rust `DnsRecordData::OPT` parse behavior in `record.rs`.

Replacement/deletion value:

- High for resolver/admission preflight. EDNS0 determines UDP payload sizing and
  DNSSEC OK behavior without requiring full RDATA object parsing.
- Do not delete Rust OPT representation; WAT should feed strict preflight and
  proof records.

Blockers:

- Rust stores OPT options as raw bytes and accepts short OPT RDATA by returning
  zeroed fields plus raw options. WAT strict mode should reject malformed option
  TLVs and record this as a policy delta.

### 5. Optional `dns-rdata-shape.wat`

Exports:

- `dns_rdata_shape(ptr, len, rr_type, rdata_start, rdlength, out_ptr) -> i32`

Scope:

- Strict shape checks for records whose RDATA is byte-local:
  - `A`: length 4
  - `AAAA`: length 16
  - `TLSA`: length >= 3
  - `DS`: length >= 4
  - `DNSKEY`: length >= 4
  - `LOC`: length == 16 for parsed form
  - `URI`: length >= 4
  - `TXT`: one or more character-string spans that exactly consume RDATA
- Name-bearing RDATA should only be handled after `dns-compressed-name` exists:
  `CNAME`, `NS`, `PTR`, `MX`, `SOA`, `SRV`, `NAPTR`, `SVCB/HTTPS`, `RRSIG`,
  `NSEC`, `RP`, and `AFSDB`.

Parity targets:

- Rust `DnsRecordData::from_wire`.
- Rust fallback-to-`Raw` behavior for malformed fixed-size records.

Replacement/deletion value:

- Medium. It is valuable as a strict verifier, but Rust currently treats many
  malformed RDATA values as `Raw` for compatibility.
- Do not delete `DnsRecordData::from_wire`; WAT can classify strict canonical
  packets and leave compatibility parsing in Rust.

Blockers:

- Must decide whether strict DNS proof paths reject malformed RDATA that Rust can
  preserve as `Raw`.
- DNSSEC and TSIG records are policy/security-sensitive and should remain Rust.

## Composition Tests

Add runners only after the WAT modules exist:

- `codec-composition-dns-query.js`
  - `dns-message-header -> dns-section-walk -> dns-compressed-name`
  - Query with one uncompressed question.
- `codec-composition-dns-compressed-answer.js`
  - Query plus answer where answer owner is `0xc00c`.
  - Proves compressed-name traversal and RR span walking.
- `codec-composition-dns-edns0.js`
  - Query with an additional OPT RR.
  - Proves OPT root owner, type 41, UDP payload size, DO bit, and option TLVs.
- `codec-composition-dns-name-rdata.js`
  - CNAME/NS/MX/SOA/SRV records with compressed RDATA names.
  - Proves name traversal in both owner and RDATA positions.
- `codec-composition-dns-rejects.js`
  - Pointer loop, pointer out of bounds, pointer into middle of label, truncated
    RR header, truncated RDATA, malformed OPT option length.

Rust parity targets should include both success and rejection cases. For known
Rust/WAT strictness deltas, record recommendations instead of hiding them:

- pointer loop should be `wat-stricter` until Rust rejects it;
- unknown qtype/rtype should preserve raw values in WAT instead of defaulting to
  `A`;
- malformed fixed RDATA should be `wat-stricter` if Rust preserves it as `Raw`;
- EDNS0 nonzero version should be reported separately from malformed bytes.

## Replacement And Deletion Value

Best replacement surface after this batch:

- Add WAT-backed DNS preflight before `parse_dns_message_bounded`.
- Use `dns-message-header`, `dns-section-walk`, and `dns-compressed-name` to
  prove section offsets, compressed name safety, and RDLENGTH bounds.
- Keep Rust `DnsMessage::from_wire` as the object parser after WAT preflight.

Potential deletions after adapter parity and call-site migration:

- duplicate DNS fixed-header reads in `dns_section_counts`;
- duplicate `domain_name_wire_len` helpers in `message.rs` and `record.rs`;
- bounded compressed-name traversal internals if all callers move through one
  WAT-backed or shared Rust strict helper.

Not deletion candidates:

- `DnsMessage`, `DnsQuestion`, `DnsRecord`, and `DnsRecordData` object models;
- zone and zone-file parsing;
- DNSSEC and TSIG verification/signing;
- DoH, TCP framing, resolver config, AXFR, and runtime IO surfaces.

## Blockers To Resolve

- Decide strict trailing-byte policy for `dns_message_walk`.
- Decide whether unknown qtype/rtype should remain raw in Rust strict APIs
  rather than defaulting to `A`.
- Add bounded pointer-loop rejection to Rust or mark current Rust behavior as a
  known bug in parity.
- Define maximum pointer jumps and maximum output-label records in the ABI.
- Decide whether WAT accepts only ASCII label bytes or also preserves arbitrary
  DNS label octets for compatibility. Current `dns-name.wat` validates ASCII
  label bytes, while DNS wire labels can carry broader octets in practice.
- Keep EDNS0 option parsing as TLV preflight only. Do not interpret option
  semantics until there is a policy need.

## Recommended Agent Batch

Put agents on these tasks:

1. Write `dns-compressed-name.wat` and a smoke runner with valid compression,
   pointer loop, pointer-to-label-middle, pointer-out-of-bounds, reserved label
   form, and max-depth cases.
2. Write `dns-section-walk.wat` using the compressed-name module logic inline
   or duplicated locally, then prove query, response, and truncated RR cases.
3. Write `dns-edns0-opt.wat` and a smoke runner for root-owner OPT, DO flag,
   nonzero version classification, option TLV traversal, and truncated option
   rejection.
4. Build `rust-parity-dns.js` focused only on DNS wire preflight, compressed
   names, section spans, EDNS0, and strictness deltas.
5. Build composition runners after modules land. Do not integrate with Rust
   parser call sites until this batch is green and the `edgerun-c` runtime
   bridge has a fixed-record codec proof.

Highest-value first module: `dns-compressed-name.wat`. It unblocks realistic DNS
message traversal and closes the known current mismatch between Rust DNS message
parsing and the existing uncompressed-name WAT scanner.
