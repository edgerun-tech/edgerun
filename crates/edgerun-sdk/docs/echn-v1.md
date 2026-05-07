# ECHN v1

ECHN is the compact binary chain manifest for distributed Edgerun segment
reports.

It declares the ordered segment set for a distributed app and the hash links
between segment outputs and later segment inputs. It does not define transport.

All integer fields are little-endian.

```text
offset  size  field
0       4     magic = "ECHN"
4       2     manifest_version = 1
6       2     abi_version = 2
8       4     flags
12      2     segment_count
14      2     link_count
16      2     chain_id_len
18      N     chain_id utf-8 bytes
...           segment id records
...           link records
```

Each segment id record:

```text
offset  size  field
0       2     segment_id_len
2       N     segment_id utf-8 bytes
```

Each link record:

```text
offset  size  field
0       2     from_segment_index
2       2     from_output_index
4       2     to_segment_index
6       2     to_input_index
```

Flags:

```text
bit 0: deterministic chain
```

`verify-chain-reports` performs preflight-bound chain verification. It checks
every segment report against its discovered segment manifest, then verifies each
link by comparing:

```text
from_report.output_sha256 == to_report.input_sha256[to_input_index]
```

This proves report-to-report hash consistency, not that each reported output
hash was produced by executing the segment. Use `replay-segment-report` with
the original inputs for every segment report that must be authoritative without
a future node signature or attestation layer.

## EDSL Source

Checked-in ECHN binaries are generated from `chain.edsl` files next to each
`chain.echn`. Current source forms:

```text
chain <id>
segment <segment-id>
link <from-segment-index>.<from-output-index> -> <to-segment-index>.<to-input-index>
```

The builder validates the source against the pinned source entry before
writing the deterministic ECHN bytes.

Current chains:

```text
http-auth-preflight-chain-v1
  segments: http-auth-preflight-public-edge-v1

http-auth-decision-chain-v1
  segments: http-auth-preflight-public-edge-v1,
            auth-decision-private-node-v1
  links: segment 0 output 0 -> segment 1 input 0
```
