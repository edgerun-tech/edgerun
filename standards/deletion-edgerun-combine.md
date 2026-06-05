# edgerun-combine deletion

Date: 2026-06-05.

`crates/utility/edgerun-combine` is deleted. The useful remaining behavior was
not the parser-combinator framework itself; the only meaningful live caller was
JNI signature parsing in `crates/utility/edgerun-jni/src/signature.rs`.

## Audit

Exact live dependency refs before deletion:

- `Cargo.toml`: workspace member `crates/utility/edgerun-combine`
- `Cargo.toml`: root `[patch.crates-io]` entry `combine = { path = "crates/utility/edgerun-combine" }`
- `Cargo.lock`: package `combine`
- `Cargo.lock`: `jni` dependency entry `"combine"`
- `crates/utility/edgerun-jni/Cargo.toml`: `combine = { path = "../edgerun-combine" }`
- `crates/utility/edgerun-jni/src/signature.rs`: `use combine::{...}`

Other broad text hits were inside the deleted crate itself or ordinary prose
using the English word "combine".

## Rationale

`combine` is a Rust parser-combinator compatibility crate. Its value is trait,
stream, error, and parser API scaffolding, not a compact portable protocol
primitive. The JNI lane only needs deterministic JVM/JNI descriptor validation
and projection: field type, method argument list, return type, primitive/object
classification, array depth, and invalid/trailing-input rejection. That belongs
in the JNI WAT primitive owned by the JNI extraction lane, not in a retained Rust
combinator crate.

No standalone WAT primitive was added for `combine`.

## Deleted Rust Value

- parser trait stack
- stream adapters and position/error wrappers
- parser combinator macros and helper functions
- examples, benches, docs, and release scripts
- `bytes` and `memchr` compatibility dependency edges through `combine`

## Intentional Fallout

`crates/utility/edgerun-jni/src/signature.rs` still imports `combine` and is an
intentional broken caller until the JNI signature parser is routed to the JNI
WAT extraction output or deleted with the broader JNI crate lane.

Do not restore `crates/utility/edgerun-combine`.
