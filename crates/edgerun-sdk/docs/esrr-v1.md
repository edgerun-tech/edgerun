# ESRR v1

ESRR is the compact binary segment execution report format.

ESRR is an SDK artifact format. It is not the Edgerun internal wire, storage,
cache, or local bridge protocol. Internal Edgerun payloads remain rkyv-only;
external standards keep their standards-defined encodings.

It binds one node-local segment execution to the segment hash, parent
composition hash, public input lengths, output hash, deterministic cost, and
status. It is transport-neutral and does not expose private input bytes.

All integer fields are little-endian.

```text
offset  size  field
0       4     magic = "ESRR"
4       2     report_version = 1
6       2     abi_version = 2
8       4     flags
12      2     status
14      2     input_count
16      2     output_count
18      2     segment_id_len
20      2     composition_id_len
22      2     node_role_len
24      8     cost
32      4     output_len
36      2     failed_step, or 0xffff for success
38      2     reserved = 0
40      32    segment_sha256
72      32    composition_sha256
104     32    output_sha256
136     N     segment_id utf-8 bytes
136+N   M     composition_id utf-8 bytes
...     R     node_role utf-8 bytes
...     4*K   input_lengths
...     32*K  input_sha256 commitments
```

Status:

```text
0 success
nonzero segment-local failure code
```

`verify-segment-report` is a preflight-bound verifier. For success reports, it
checks the segment hash, composition hash, segment identity, role, input arity,
output arity, input commitment count, and that the reported cost and output
length fit the segment's length-only preflight bounds. It cannot prove
`output_sha256` without the original inputs, because ESRR stores only input
commitments.

`replay-segment-report` is the authoritative local verifier for a successful
report. It takes the original inputs, checks their SHA-256 commitments against
the report, re-executes the pinned composition, and verifies the reported cost,
output length, and `output_sha256`.

Future revisions can append node identity and signatures once the identity
format is fixed.
