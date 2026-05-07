# ECMP v1

ECMP is the compact binary composition manifest for deterministic unit graphs.

All integer fields are little-endian.

```text
offset  size  field
0       4     magic = "ECMP"
4       2     manifest_version = 1
6       2     abi_version = 2
8       4     flags
12      2     component_count
14      2     step_count
16      2     composition_id_len
18      N     composition_id utf-8 bytes
18+N    2     output_unit_id_len
20+N    M     output_unit_id utf-8 bytes
...           component records
...           step records
```

Flags:

```text
bit 0: deterministic composition
```

Each component record:

```text
offset  size  field
0       2     unit_id_len
2       32    wasm_sha256
34      N     unit_id utf-8 bytes
```

Each step record is fixed-width:

```text
offset  size  field
0       1     opcode
1       1     component_index
2       1     function_index
3       1     reserved = 0
4       4     arg0
8       4     arg1
12      4     arg2
16      4     arg3
20      4     arg4
```

Opcodes:

```text
1 copy external input into component memory
  arg0 = external input index
  arg1 = destination pointer
  arg2 = length reference

2 call component export
  function_index = index into EAPI function records
  arg0..arg4 = scalar argument references

3 copy bytes between component memories
  arg0 = source component index
  arg1 = source pointer
  arg2 = destination pointer in component_index memory
  arg3 = length reference

4 publish composition output
  arg0 = output index
  arg1 = source pointer in component_index memory
  arg2 = length reference

5 conditional branch
  arg0 = left comparison reference with comparison bits
  arg1 = right comparison reference
  arg2 = target step index when true
  arg3 = target step index when false

6 unconditional jump
  arg0 = target step index

7 write constant byte into component memory
  arg0 = destination pointer reference
  arg1 = byte value

8 call component export and capture i32 result
  function_index = index into EAPI function records
  arg0..arg3 = scalar argument references
  arg4 = scalar slot index
  nonzero return values are data, not host execution failure

9 load little-endian u32 from component memory into scalar slot
  arg0 = source pointer reference
  arg1 = scalar slot index
```

Argument and length references are 32-bit words:

```text
bits 31..24  kind
bits 23..0   value

kind 0: constant value
kind 1: external input length by index
kind 2: captured scalar slot
kind 3: constant plus external input length
        value bits 23..8 = constant
        value bits 7..0  = external input index
```

Conditional branch references use the same lower 28 bits plus comparison bits:

```text
bits 31..28 comparison
bits 27..24 reference kind
bits 23..0  value

comparison 1: left > right
comparison 2: left == right
comparison 3: left != right
```

The current verifier checks that each component is present in the unit index,
that the pinned component hash equals the discovered hash, and that every step
references valid components, functions, branch targets, and argument-reference
kinds. Execution uses byte copies between owned memories; ECMP never introduces
shared memory.

The HMAC-SHA256 composition records both RFC 2104 key paths: `key_len <= 64`
uses the supplied key directly, while `key_len > 64` hashes the key first and
then pads the 32-byte SHA-256 digest as the effective key.

The SDK verifier executes this composition against HMAC-SHA256 test vectors by
instantiating only the pinned wasm components and applying the ECMP
copy/call/branch/output steps. The executor does not implement HMAC itself.

Execution cost is deterministic:

```text
copy external input: copied byte count
copy between units: copied byte count
publish output: copied byte count
branch/jump: 1
call: EAPI cost_base + EAPI cost_per_byte * function metered_len
```

The current runner prints cost from input lengths and the executed branch path.
The quote path uses the same ECMP branch rules and EAPI costs, but receives
only input lengths and never sees input bytes.

## EDSL Source

Checked-in ECMP binaries are generated from `compose.edsl` files next to each
`compose.edm`. The EDSL is the editable source; ECMP remains the execution and
verification artifact.

Current source declarations:

```text
composition <id>
output <output-unit-id>
input <name>
component <alias> <unit-id>
```

Current operation forms:

```text
copy-input <input> <component> <dst-ref> <len-ref>
call <component>.<function> <arg0> <arg1> <arg2> <arg3> <arg4>
copy <source-component> <src-ref> <target-component> <dst-ref> <len-ref>
publish <component> <src-ref> <len-ref>
branch <gt|eq|ne> <left-ref> <right-ref> <then-step> <else-step>
jump <target-step>
write-byte <component> <ptr-ref> <byte>
capture <component>.<function> <arg0> <arg1> <arg2> <arg3> -> slot:<n>
load-u32 <component> <ptr-ref> -> slot:<n>
```

Reference forms:

```text
const:<n>
len:<input-name>
slot:<n>
add:<n>+len:<input-name>
```

The builder resolves component aliases, function names, input names, and slot
references into fixed ECMP indexes and 32-bit references. If the generated ECMP
hash differs from the discovered hash, `build-artifacts` fails.

Current executable compositions:

```text
hmac-sha256-rfc2104-composed
  components: rfc2104-pad, sha256-fips180

hkdf-extract-sha256-rfc5869
  components: hmac-sha256-rfc2104
  output: RFC 5869 PRK = HMAC-SHA256(salt, IKM)

hkdf-expand-sha256-rfc5869-l42
  components: hmac-sha256-rfc2104
  output: RFC 5869 OKM for L = 42

http-auth-preflight-rfc9110
  components: http-field-rfc9110, byte-tools-v1
  output: one status byte, where 0 = accepted header name,
          1 = field-line parse rejected, 2 = wrong field name

auth-decision-private-v1
  components: decision-byte-v1
  output: one status byte, where 0 = linked status matched private expected
          byte, 1 = linked status differed

hmac-sha256-verify-rfc2104
  components: hmac-sha256-rfc2104, constant-time-eq-v1
  output: one status byte, where 0 = expected tag matched computed HMAC,
          1 = tag differed, 2 = tag length differed from 32 bytes
```
