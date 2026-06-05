# edgerun-simdutf8 deletion

`crates/utility/edgerun-simdutf8` was inspected from source before deletion.

Useful behavior found:

- strict UTF-8 validation returning only valid/invalid status in `basic`;
- strict UTF-8 validation with `valid_up_to` and optional `error_len` in
  `compat`;
- mutable borrowed `&mut str` variants after validation;
- low-level streaming validators behind `public_imp`.

Behavior not kept as WAT:

- AVX2, SSE4.2, aarch64 Neon, and wasm32 SIMD implementations;
- runtime CPU feature dispatch;
- public unsafe SIMD implementation modules;
- Rust borrowed `&str` and `&mut str` API shape;
- `std::error::Error` and display text.

Existing WAT owner:

- `utf8-scan.wat`, `standard_id = 300068`
- `utf8_scan(ptr,len,out)->i32` covers strict scan with detailed failure
  location and incomplete suffix metadata.
- `utf8_lossy_repair(in_ptr,in_len,out_ptr,out_cap)->i64` covers replacement
  repair with U+FFFD.

Deletion action:

- delete `crates/utility/edgerun-simdutf8`;
- remove the root workspace member and workspace dependency;
- remove the stale lockfile package;
- the former `simd_cesu8` caller has since been deleted too; CESU-8/MUTF-8
  behavior is now owned by `cesu8-mutf8.wat`.

Remaining `simdutf8` text in `edgerun-bytecheck` is a feature label and comment,
not a dependency edge.
