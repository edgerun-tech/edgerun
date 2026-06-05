#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/rkyv-core-archive.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  const wasm = execFileSync("wat2wasm", [file, "-o", "-"]);
  execFileSync("wasm-validate", ["-"], { input: wasm });
  return wasm;
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const e = instance.exports;

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300134);
  assert.equal(e.rkyv_endian_layout_code(0, 0), 1);
  assert.equal(e.rkyv_endian_layout_code(0, 1), 2);
  assert.equal(e.rkyv_endian_layout_code(1, 0), 3);
  assert.equal(e.rkyv_fixed_pointer_bytes(16), 2);
  assert.equal(e.rkyv_fixed_pointer_bytes(32), 4);
  assert.equal(e.rkyv_fixed_pointer_bytes(64), 8);
  assert.equal(e.rkyv_copy_optimization_allowed(1, 1, 1), 1);
  assert.equal(e.rkyv_copy_optimization_allowed(1, 0, 1), 0);

  assert.equal(e.rkyv_signed_offset_status(10n, 25n, 127n), 1);
  assert.equal(e.rkyv_signed_offset_value(10n, 25n), 15n);
  assert.equal(e.rkyv_signed_offset_value(25n, 10n), -15n);
  assert.equal(e.rkyv_signed_offset_status(0n, 128n, 127n), 2);
  assert.equal(e.rkyv_offset_storage_fits(127n, 8, 1), 1);
  assert.equal(e.rkyv_offset_storage_fits(128n, 8, 1), 0);
  assert.equal(e.rkyv_offset_storage_fits(255n, 8, 0), 1);
  assert.equal(e.rkyv_rel_ptr_target(1000n, -24n), 976n);
  assert.equal(e.rkyv_rel_ptr_invalid(1n), 1);
  assert.equal(e.rkyv_rel_ptr_invalid(0n), 0);

  assert.equal(e.rkyv_archive_ptr_check(1088n, 16n, 16n, 1024n, 2048n), 1);
  assert.equal(e.rkyv_archive_ptr_check(1089n, 16n, 16n, 1024n, 2048n), 3);
  assert.equal(e.rkyv_archive_ptr_check(2032n, 32n, 16n, 1024n, 2048n), 2);
  assert.equal(e.rkyv_subtree_push_status(1n), 1);
  assert.equal(e.rkyv_subtree_push_status(0n), 4);
  assert.equal(e.rkyv_subtree_pop_status(4096n, 2048n, 3n), 1);
  assert.equal(e.rkyv_subtree_pop_status(1024n, 2048n, 3n), 5);

  assert.equal(e.rkyv_aligned_vec_max_capacity(16n, 9223372036854775807n), 9223372036854775792n);
  assert.equal(e.rkyv_aligned_vec_max_capacity(24n, 9223372036854775807n), -1n);
  assert.equal(e.rkyv_buffer_write_status(64n, 60n, 4n), 1);
  assert.equal(e.rkyv_buffer_write_status(64n, 60n, 5n), 7);
  assert.equal(e.rkyv_suballocator_alloc_offset(3n, 64n, 8n, 8n), 8n);
  assert.equal(e.rkyv_suballocator_alloc_offset(60n, 64n, 8n, 8n), -1n);

  assert.equal(e.rkyv_string_inline_capacity(32), 8);
  assert.equal(e.rkyv_string_inline_capacity(64), 16);
  assert.equal(e.rkyv_string_repr_kind(8n, 32), 1);
  assert.equal(e.rkyv_string_repr_kind(9n, 32), 2);
  assert.equal(e.rkyv_string_out_of_line_capacity(32), 1073741823n);
  assert.equal(e.rkyv_option_tag_code(0), 0);
  assert.equal(e.rkyv_option_tag_code(1), 1);
  assert.equal(e.rkyv_niche_zero(0n), 1);
  assert.equal(e.rkyv_niche_zero(9n), 0);
  assert.equal(e.rkyv_niche_null(0n), 1);
  assert.equal(e.rkyv_niche_nan_f64(Number.NaN), 1);
  assert.equal(e.rkyv_niche_nan_f64(1), 0);
  assert.equal(e.rkyv_archive_order_code(1), 1);
  assert.equal(e.rkyv_archive_order_code(2), 2);
  assert.equal(e.rkyv_archive_order_code(3), 3);

  console.log(JSON.stringify({ unit: "rkyv-core-archive", standard_id: e.proto_standard_id(), ok: true }));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
