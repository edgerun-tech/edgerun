# edgerun-json Value Core Extraction

This note captures the remaining useful behavior in
`crates/utility/edgerun-json/src/{value,number,map,model}.rs` that should
survive after the Rust crate is retired as a source implementation.

The canonical executable extraction is
`standards/build/wasm/codec-primitives/json-value-core.wat`. It works over the
existing `json-tape.wat` token layout instead of allocating a Rust-like heap
tree.

## Value tags

Rust `JsonValue` has six variants:

| Variant | WAT value id |
| --- | ---: |
| `Null` | 0 |
| `Bool` | 1 |
| `Number` | 2 |
| `String` | 3 |
| `Array` | 4 |
| `Object` | 5 |

`json-tape.wat` token kinds map into those ids:

| Tape kind | Meaning | Value id |
| ---: | --- | ---: |
| 1 | object | 5 |
| 2 | array | 4 |
| 3 | string | 3 |
| 5 | number | 2 |
| 6 | boolean | 1 |
| 7 | null | 0 |

Unknown token kinds are rejected with `-1`.

## Number semantics

Rust stores numbers as `JsonNumber::{I64, U64, F64}`.

- Positive integer inputs are represented as `U64`.
- Negative integer inputs are represented as `I64`.
- Decimal or exponent inputs are represented as `F64`.
- Non-finite floating point values are not representable.
- `as_i64` accepts `I64` and `U64` values that fit in `i64`.
- `as_u64` accepts `U64` and non-negative `I64` values.
- `as_f64` accepts all three numeric representations.
- Narrow integer conversions require the corresponding range check.

`json_number_caps` returns a capability bitset:

| Bit | Meaning |
| ---: | --- |
| 0 | can convert to `i64` / `is_i64` |
| 1 | can convert to `u64` / `is_u64` |
| 2 | can convert to `f64` |
| 3 | can convert to `i32` |
| 4 | can convert to `u32` |
| 5 | can convert to `usize` on the 64-bit runtime path |
| 6 | can convert to `i128` |
| 7 | can convert to `u128` |
| 8 | is represented as `F64` |

## Object lookup semantics

`Map` is insertion-order storage over `(String, JsonValue)` pairs. Lookup,
mutation, removal, replacement, and equality all use the first matching key
found by linear scan.

`insert` and `shift_insert` replace the first existing key. `push_field` appends
without deduplicating, so duplicate keys can exist. `get`, `required`, and typed
field accessors therefore observe the first matching duplicate.

`json_object_find_field` preserves this first-match behavior over a JSON tape.
The current primitive compares raw unescaped ASCII key bytes between the quotes;
escaped-key canonicalization remains a string-decoding responsibility.

## Typed conversions

The model layer expresses these expectations:

- `Option<T>` accepts `null` as `None`; otherwise it attempts `T`.
- `()` accepts only `null`.
- Arrays, tuples, and fixed arrays require JSON arrays; tuple length is exactly
  2 and fixed arrays require exactly `N`.
- `Vec<T>` accepts only arrays and converts each element.
- Maps accept only objects and convert every field value.
- Scalar conversions are exact by variant and range. Wrong variants and
  out-of-range numbers are errors, not coercions.

The value-core primitive encodes the scalar part with
`json_value_can_convert`. Collection arity and recursive element conversion are
record-level checks to be performed by higher-level WAT modules over the tape.
