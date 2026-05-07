# ESEG v1

ESEG is the compact binary segment manifest for distributed Edgerun execution.

A segment is a locally executable slice of a discovered ECMP composition. It
binds a node role, capability label, composition hash, input/output arity, and
component/step range. It does not define a transport.

All integer fields are little-endian.

```text
offset  size  field
0       4     magic = "ESEG"
4       2     manifest_version = 1
6       2     abi_version = 2
8       4     flags
12      2     input_count
14      2     output_count
16      2     component_start
18      2     component_count
20      2     step_start
22      2     step_count
24      2     segment_id_len
26      2     composition_id_len
28      2     node_role_len
30      2     capability_len
32      32    composition_sha256
64      N     segment_id utf-8 bytes
64+N    M     composition_id utf-8 bytes
...     R     node_role utf-8 bytes
...     C     capability utf-8 bytes
...     K     input kind bytes, one per input
```

Flags:

```text
bit 0: deterministic segment
```

Input kind bytes:

```text
0 unknown/reserved
1 external/public input
2 linked input from another segment report
3 private/capability-local input
```

The verifier checks that the segment binds to the discovered composition hash
and that its component and step ranges are inside the composition. It also
checks the input kind table against the discovered segment manifest. Segment
reports use the segment hash as the distributed execution boundary for
node-local reports.

## EDSL Source

Checked-in ESEG binaries are generated from `segment.edsl` files next to each
`segment.eseg`. Current source forms:

```text
segment <id>
composition <composition-id>
node-role <role>
capability <capability-label>
input <name> <public|linked|private>
outputs <count>
components <start> <count>
steps <start> <count>
```

The builder validates the source against the pinned source entry before
writing the deterministic ESEG bytes.

Current private verification segment:

```text
hmac-sha256-verify-private-node-v1
  composition: hmac-sha256-verify-rfc2104
  node_role: private-policy-node
  capability: hmac-sha256-verify
  input kinds: private key, linked message, linked expected tag
  output: one verification status byte
```
