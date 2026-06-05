# edgerun-c HTTP/2 WAT Bridge Proof Plan

This plan scopes the first owned-runtime proof for the WAT codec batch:
execute `http2-frame.wat` through `/home/ken/edgerun-c` instead of through the
Node `WebAssembly.instantiate` runner.

The proof is not a Rust protocol replacement. It is the smallest executable
bridge needed before Rust deletion becomes defensible.

## Current Feasibility

`/home/ken/edgerun-c` has the required runtime entry points:

```text
/home/ken/edgerun-c/kernel/x86_64/wasm/wasm_run.asm
  er_fn_load(runtime_ptr, wasm_bytes_ptr, wasm_bytes_len) -> rdx error
  er_fn_call(runtime_ptr, export_name_ptr, export_name_len) -> rax result, rdx error
  er_fn_call_args(runtime_ptr, export_name_ptr, export_name_len, args_ptr, args_count)
    -> rax result, rdx error
```

The same file stages host memory from `RuntimeConfig`, primes the module, and
leaves the module ready for repeated calls. The runtime uses the memory pointer
from the host config for WASM loads and stores, so the bridge can write input
bytes into the owned memory arena before calling a codec export and read output
records afterward.

The checked build artifacts expose the symbols:

```text
/home/ken/edgerun-c/.build/kernel/kernel_wasm_interpreter.o
  er_fn_load
  er_fn_call
  er_fn_call_args
  er_fn_run_args
  er_fn_init
```

However, those checked kernel objects are currently ELF 32-bit relocatables.
The ready host tools in `/home/ken/edgerun-c/.build/host` are 64-bit static
executables, but I did not find a small existing userspace host harness that
links the WASM runtime object and exposes a command like:

```text
edgerun-c-wasm-call http2-frame.wasm http2_frame_header_decode ...
```

So an executable proof runner under `/home/ken/edgerun/standards/runners` is not
straightforward yet. The next code step should happen in `edgerun-c`: add or
materialize one tiny host-linkable proof harness, then call it from this
workspace.

## Inputs

Compile the existing WAT module:

```bash
wat2wasm \
  /home/ken/edgerun/standards/build/wasm/codec-primitives/http2-frame.wat \
  -o /tmp/edgerun-http2-frame.wasm

wasm-validate /tmp/edgerun-http2-frame.wasm
```

Required module shape:

```text
imports: none
exports:
  memory
  proto_abi_version() -> i32
  proto_standard_id() -> i32
  http2_frame_type_classify(frame_type: i32) -> i32
  http2_frame_header_decode(in_ptr: i32, in_len: i32, max_frame_size: i32, out_ptr: i32) -> i32
  http2_frame_header_encode(payload_len: i32, frame_type: i32, flags: i32, stream_id: i32, out_ptr: i32, out_cap: i32) -> i64
```

Identity expectations:

```text
proto_abi_version() == 2
proto_standard_id() == 300010
```

## Runtime Config

Use the `RuntimeConfig` layout from
`/home/ken/edgerun-c/kernel/x86_64/wasm_defines.inc`:

```text
offset 0   memory_ptr: u64
offset 8   memory_len: u64
offset 16  ticks_ptr: u64
offset 24  memory_grow_fn: u64
offset 32  memory_grow_ctx: u64
offset 40  table_grow_fn: u64
offset 48  table_grow_ctx: u64
offset 56  initial_pages: u64
offset 64  has_pages: u8
offset 72  imports_ptr: u64
offset 80  imports_len: u64
size 88
```

First proof values:

```text
memory_len = 65536
initial_pages = 1
has_pages = 1
imports_ptr = 0
imports_len = 0
grow hooks = 0
ticks_ptr = optional zeroed u64 buffer
```

Memory offsets for the codec proof:

```text
in_ptr = 1024
out_ptr = 2048
out_record_len = 28
encode_out_ptr = 3072
encode_out_cap = 9
```

The harness owns the backing memory buffer. It writes input bytes directly into
`memory_ptr + in_ptr` before `er_fn_call_args`, and reads fixed output records
from `memory_ptr + out_ptr` after the call returns.

## Harness Contract

The first executable harness can be C, ER, assembly, or Rust FFI inside
`edgerun-c`, but it should expose a simple command shape that this workspace can
call:

```text
edgerun-c-http2-bridge-proof /tmp/edgerun-http2-frame.wasm
```

Minimum host steps:

```text
1. allocate one 64 KiB aligned-or-stable memory buffer;
2. fill RuntimeConfig with that memory, one initial page, and zero imports;
3. call er_fn_load(runtime, wasm_ptr, wasm_len);
4. call proto_abi_version through er_fn_call;
5. call proto_standard_id through er_fn_call;
6. for each decode case:
   a. clear memory;
   b. copy input frame bytes into memory at in_ptr;
   c. fill args64 = [in_ptr, in_len, max_frame_size, out_ptr];
   d. call er_fn_call_args(runtime, "http2_frame_header_decode", args64, 4);
   e. distinguish runtime error rdx from codec status rax;
   f. read 28 bytes from memory at out_ptr;
7. for each encode case:
   a. clear output memory;
   b. fill args64 = [payload_len, frame_type, flags, stream_id, encode_out_ptr, encode_out_cap];
   c. call er_fn_call_args(runtime, "http2_frame_header_encode", args64, 6);
   d. split returned i64 into status low32 and written high32;
   e. read written bytes from encode_out_ptr when status is 0.
```

The harness should print machine-readable JSON so
`standards/runners/edgerun-c-http2-bridge-proof.js` can compare it to the
existing Node/Rust oracle later.

## Required Cases

Decode cases:

```text
DATA len=0 flags=0 stream=1
  input: 00 00 00 00 00 00 00 00 01
  status: 0
  output:
    payload_length=0
    frame_type_class=0
    flags=0
    stream_id_reserved_bit=0
    stream_id_cleared=1
    header_len=9
    total_len=9

SETTINGS len=6 flags=0 stream=0 with complete payload
  input: 00 00 06 04 00 00 00 00 00 00 03 00 00 00 64
  status: 0
  output:
    payload_length=6
    frame_type_class=4
    flags=0
    stream_id_reserved_bit=0
    stream_id_cleared=0
    header_len=9
    total_len=15

PING len=8 flags=0xaa stream=0 with complete payload
  input: 00 00 08 06 aa 00 00 00 00 01 02 03 04 05 06 07 08
  status: 0
  output flags=170, total_len=17

reserved stream bit set
  input: 00 00 00 00 00 80 00 00 01
  status: 0
  output stream_id_reserved_bit=1, stream_id_cleared=1

short header
  input: 00 00 00 00 00 00 00 00
  status: 1

payload larger than max_frame_size
  input: 00 40 01 00 00 00 00 00 01
  max_frame_size=16384
  status: 3

declared payload not present
  input: 00 00 01 00 00 00 00 00 01
  status: 1
```

Encode cases:

```text
DATA len=0 type=0 flags=0 stream=1
  status=0, written=9
  bytes: 00 00 00 00 00 00 00 00 01

SETTINGS len=6 type=4 flags=0 stream=0
  status=0, written=9
  bytes: 00 00 06 04 00 00 00 00 00

payload_len=0x01000000
  status=4, written=0

out_cap=8
  status=2, written=0

stream_id high bit set
  args stream_id=0x80000001
  status=0, written=9
  bytes mask high bit and encode stream id 1
```

## Expected Runner After Harness Exists

Once the `edgerun-c` executable exists, add this workspace runner:

```text
standards/runners/edgerun-c-http2-bridge-proof.js
```

Runner steps:

```text
1. compile http2-frame.wat to a temp .wasm with wat2wasm;
2. validate it with wasm-validate;
3. call the edgerun-c harness executable;
4. parse its JSON;
5. compare identity, decode records, encode bytes, codec statuses, and runtime errors;
6. fail if any runtime error appears on a codec-status case.
```

The runner should also compare the same cases against:

```text
standards/runners/http-frame-wat-adapter-proof.js
standards/runners/rust-parity-http-ws.js
```

## Blockers To Resolve In edgerun-c

1. Provide one host-linkable runtime object or executable that can call
   `er_fn_load`, `er_fn_call`, and `er_fn_call_args` from userspace.
2. Confirm whether the checked 32-bit kernel object is the intended artifact for
   this proof or whether a 64-bit host build of the WASM runtime should be
   materialized.
3. Expose a tiny stable bridge command or fixture runner. The standards runner
   should not link raw kernel objects itself.
4. Keep runtime errors separate from codec statuses in the output contract.
5. Require zero imports for codec modules during this first proof.

## Success Criteria

The bridge proof is green only when the owned runtime proves all of this:

```text
er_fn_load accepts compiled http2-frame.wasm with zero imports.
er_fn_call returns ABI version 2 and standard id 300010.
er_fn_call_args can call the 4-arg decode export repeatedly.
er_fn_call_args can call the 6-arg encode export repeatedly.
Host memory writes are visible to WASM i32.load8_u.
WASM i32.store and i32.store8 writes are visible to the host memory reader.
The 28-byte decode records match the Node WAT oracle byte-for-byte.
The fixed header encode bytes match Rust and Node parity cases.
Runtime error rdx remains zero for all expected codec-status cases.
```

Only after this proof should `/home/ken/edgerun` add a feature-gated Rust
adapter facade, and only after that adapter has corpus parity should any Rust
scanner code be removed.
