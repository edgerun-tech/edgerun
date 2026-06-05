#!/usr/bin/env node

const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const modulePath = path.join(
  __dirname,
  "..",
  "build",
  "wasm",
  "codec-primitives",
  "derive-macro-core.wat",
);

function assertEq(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`${label}: expected ${expected}, got ${actual}`);
  }
}

function compileWat(watPath) {
  const wasmPath = path.join(os.tmpdir(), `derive-macro-core-${process.pid}.wasm`);
  execFileSync("wat2wasm", [watPath, "-o", wasmPath], { stdio: "pipe" });
  return fs.readFileSync(wasmPath);
}

WebAssembly.instantiate(compileWat(modulePath), {})
  .then(({ instance }) => {
    const e = instance.exports;

    assertEq(e.quote_repetition_tokens(3, 1), (3 << 16) | 2, "quote separates repeated items");
    assertEq(e.quote_repetition_tokens(0, 1), 0, "quote emits no separator without items");
    assertEq(e.quote_span_source(0, 0), 1, "quote body uses call_site span");
    assertEq(e.quote_span_source(1, 0), 2, "interpolation preserves span");
    assertEq(e.quote_span_source(1, 1), 3, "quote_spanned overrides span");

    assertEq(e.synstructure_add_bounds_scope(3), 3, "synstructure Both bounds");
    assertEq(e.synstructure_bind_tokens(0), 0, "move bind emits no prefix");
    assertEq(e.synstructure_bind_tokens(3), 3, "ref mut bind prefix");
    assertEq(e.synstructure_fuse_generics(0b0101, 0b1010), 0b1111, "generic use masks fuse");
    assertEq(e.synstructure_bound_fields(5, 2), 3, "omitted fields suppress bounds");

    assertEq(e.thiserror_non_field_attrs_valid(0, 0, 0, 1, 0, 0), 1, "transparent attr alone is valid");
    assertEq(e.thiserror_non_field_attrs_valid(1, 0, 0, 0, 0, 0), 0, "from is field-only");
    assertEq(e.thiserror_non_field_attrs_valid(0, 0, 0, 1, 1, 0), 0, "transparent conflicts with display");
    assertEq(e.thiserror_non_field_attrs_valid(0, 0, 0, 0, 1, 1), 0, "display conflicts with fmt path");
    assertEq(e.thiserror_field_attrs_valid(2, 1, 0, 1, 0, 0), 1, "from plus distinct backtrace is valid");
    assertEq(e.thiserror_field_attrs_valid(3, 1, 0, 0, 1, 0), 0, "from rejects extra fields");
    assertEq(e.thiserror_impl_bits(1, 0, 1, 1), 16 | 32 | 64 | 128, "transparent source from impls");

    assertEq(e.rkyv_type_attrs_valid(1, 0, 0, 0), 1, "rkyv as type can stand alone");
    assertEq(e.rkyv_type_attrs_valid(1, 1, 0, 0), 0, "rkyv as type rejects archived name");
    assertEq(e.rkyv_field_bound_kind(0, 0, 0), 1, "native archive bound");
    assertEq(e.rkyv_field_bound_kind(0, 1, 2), 6, "with deserialize bound");
    assertEq(e.rkyv_field_bound_kind(1, 1, 2), 0, "omit_bounds suppresses with bound");
    assertEq(e.rkyv_archive_impl_bits(0, 1), 1 | 4, "local archive serialize impls");
    assertEq(e.rkyv_archive_impl_bits(1, 1), 2 | 8, "remote archive serialize impls");
    assertEq(e.rkyv_enum_valid(256, 0), 1, "rkyv enum u8 tag limit");
    assertEq(e.rkyv_enum_valid(257, 0), 0, "rkyv enum rejects too many variants");
    assertEq(e.rkyv_other_variant_valid(1, 1, 1), 1, "rkyv other variant remote unit last");
    assertEq(e.rkyv_other_variant_valid(1, 0, 1), 0, "rkyv other variant must be unit");

    assertEq(e.bytecheck_data_valid(0, 0), 1, "bytecheck struct accepts rust repr");
    assertEq(e.bytecheck_data_valid(1, 2), 1, "bytecheck enum requires primitive repr");
    assertEq(e.bytecheck_data_valid(1, 0), 0, "bytecheck enum rejects implicit rust repr");
    assertEq(e.bytecheck_data_valid(2, 2), 0, "bytecheck rejects unions");
    assertEq(e.bytecheck_error_bound_kind(1, 1), 2 | 4, "bytecheck enum source plus verify bounds");

    assertEq(e.small_error_derive_bits(1, 0, 2, 1), 16 | 32 | 64 | 128, "small enum error impls");
    assertEq(e.small_error_derive_bits(0, 1, 2, 1), 0, "small transparent struct requires one field");

    console.log("derive macro core smoke passed");
  })
  .catch((error) => {
    console.error(error.message);
    process.exit(1);
  });
