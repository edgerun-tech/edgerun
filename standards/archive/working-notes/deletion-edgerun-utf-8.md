# edgerun-utf-8 deletion

`crates/utility/edgerun-utf-8` was inspected from source before deletion.

Useful behavior found:

- strict UTF-8 byte validation with the first valid prefix span;
- invalid sequence split into valid prefix, one invalid byte, and remaining input;
- incomplete trailing sequence capture for up to four bytes;
- lossy repair that copies valid spans and writes U+FFFD for invalid or trailing
  incomplete byte sequences.

Behavior not kept as WAT:

- `BufReadDecoder` and `BufReadDecoderError` are Rust `std::io::BufRead` glue;
- callback-owned `LossyDecoder<F>` and borrowed `&str` return shapes are API
  abstractions over the byte behavior;
- `DecodeError` display text and `Error` impls are Rust diagnostics.

Existing WAT coverage check:

- `encoding-text.wat` covers hex and base64url text encodings only;
- `json-scalar.wat` scans JSON string escape syntax and emits UTF-8 for Unicode
  escapes, but it is not a raw UTF-8 validator or lossy repair primitive.

New owner:

- `utf8-scan.wat`, `standard_id = 300068`
- exports `utf8_scan(ptr,len,out)->i32`
- exports `utf8_lossy_repair(in_ptr,in_len,out_ptr,out_cap)->i64`

`utf8_scan` writes a 16-byte record:

```text
valid_up_to: u32
error_len: u32      # 0 for incomplete trailing sequences
suffix_len: u32     # trailing incomplete byte count
expected_len: u32   # expected sequence width for incomplete suffixes
```

Deletion action:

- delete `src/lib.rs`, `src/lossy.rs`, `src/read.rs`;
- delete crate metadata;
- remove root workspace member/dependency entries if present.

The crate had no live code callers in the source scan. Remaining `utf-8` text
matches elsewhere are ordinary content-type strings, diagnostics, or separate
UTF-8 validation crates and are not blockers.
