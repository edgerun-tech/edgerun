# Standard Module ABI v1

Standard modules are WebAssembly modules that implement one externally-defined
standard surface, such as an RFC section, algorithm, parser, serializer, or
state transition rule.

The ABI does not force every standard into one message protocol. Each module
defines the exports that fit its standard. The shared rules are identity,
determinism, byte-oriented invocation, and machine-readable discovery.

## Required Exports

Every standard module exports:

```text
proto_abi_version() -> i32
proto_standard_id() -> i32
```

`proto_abi_version` is `2` for this model. `proto_standard_id` is the stable
numeric identifier chosen by the standards index for the implemented
standard or profile.

## Source Creation Path

The SDK creates units from Rust source crates under `units/<unit-id>/rust`.
There is no WAT source path and no handwritten catalog. A unit crate declares:

```rust
#![no_std]

edgerun_unit::no_alloc!();
edgerun_unit::metadata!(9110);

#[edgerun_unit::export]
unsafe fn example_unit_function(input_ptr: i32, input_len: i32, out_ptr: i32) -> i32 {
    0
}
```

The crate's `Cargo.toml` must also contain:

```toml
[package.metadata.edgerun.unit]
id = "example-unit"
standard = "RFC9110"
```

`build-artifacts` compiles each Rust unit to `unit.wasm`, derives `manifest.edm`
and `api.edm` from the wasm exports, and rejects units that bypass
`edgerun_unit::metadata!` / `#[edgerun_unit::export]`, import host state, omit
owned memory, omit protocol metadata exports, or expose functions without a
deterministic cost profile.

## Memory And Invocation

Standard modules own their memory. A module may export memory:

```wat
(memory (export "memory") 1)
```

Standard modules must not import another module's memory, and composers must
not rely on shared memory as a protocol boundary. A caller may copy bytes into a
unit's exported memory, call the unit's API, and copy bytes or scalar results
out. Composition is therefore value transfer between unit APIs, not pointer
sharing.

This keeps standards independent of transports. A TFTP unit receives TFTP bytes;
those bytes may have arrived over UDP, email, storage, mesh, or any other
capability that can deliver bytes.

## Standard-Specific APIs

Modules export the API their standard needs. Examples:

```text
memory
sha256_digest(input_ptr, input_len, out_ptr) -> status
sha384_digest(input_ptr, input_len, out_ptr) -> status
sha512_digest(input_ptr, input_len, out_ptr) -> status
rfc2104_key_pad(key_ptr, key_len, block_size, ipad_out, opad_out) -> status
hmac_sha256(key_ptr, key_len, data_ptr, data_len, out_ptr) -> status
udp_parse(packet_ptr, packet_len, out_ptr) -> status
```

The API name is part of the module identity. Callers compose modules by copying
values between wasm unit APIs, not by sharing memory or by a host runner
implementing protocol behavior.

## Determinism

A deterministic standard module must not use clocks, randomness, syscalls,
thread races, floating-point nondeterminism, host callbacks, or ambient
authority unless that dependency is declared as an import and included in the
composed artifact identity.

For the same module bytes, same input bytes copied into the module, and same
exported function call, the output bytes and status code must be identical.

## Status Codes

Status code `0` means success. Non-zero status codes are module-defined, but
must be deterministic and documented by the module manifest.
