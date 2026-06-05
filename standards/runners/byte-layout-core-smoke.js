#!/usr/bin/env node

const assert = require("assert");
const { execFileSync } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/codec-primitives/byte-layout-core.wat");
const wasm = path.join(os.tmpdir(), `byte-layout-core-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "pipe" });

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = instance.exports;

  assert.strictEqual(e.bytecheck_type_needs_source(0), 0);
  assert.strictEqual(e.bytecheck_type_needs_source(1), 1);
  assert.strictEqual(e.bytecheck_type_needs_source(7), 0);
  assert.strictEqual(e.bytecheck_type_needs_source(12), 1);

  assert.strictEqual(e.bytecheck_bool_valid(0), 1);
  assert.strictEqual(e.bytecheck_bool_valid(1), 1);
  assert.strictEqual(e.bytecheck_bool_valid(2), 0);
  assert.strictEqual(e.bytecheck_char_valid(0x41), 1);
  assert.strictEqual(e.bytecheck_char_valid(0xd800), 0);
  assert.strictEqual(e.bytecheck_char_valid(0x110000), 0);
  assert.strictEqual(e.bytecheck_nonzero_valid_i64(1n), 1);
  assert.strictEqual(e.bytecheck_nonzero_valid_i64(0n), 0);

  assert.strictEqual(e.bytecheck_utf8_lead_len(0x41), 1);
  assert.strictEqual(e.bytecheck_utf8_lead_len(0xc2), 2);
  assert.strictEqual(e.bytecheck_utf8_lead_len(0xe0), 3);
  assert.strictEqual(e.bytecheck_utf8_lead_len(0xf4), 4);
  assert.strictEqual(e.bytecheck_utf8_lead_len(0x80), 0);
  assert.strictEqual(e.bytecheck_utf8_cont_valid(0x80), 1);
  assert.strictEqual(e.bytecheck_utf8_cont_valid(0xc0), 0);
  assert.strictEqual(e.bytecheck_cstr_shape_valid(1, 1, 0), 1);
  assert.strictEqual(e.bytecheck_cstr_shape_valid(0, 1, 0), 0);
  assert.strictEqual(e.bytecheck_cstr_shape_valid(3, 1, 1), 0);

  assert.strictEqual(e.bytecheck_container_checks(6, 0, 4), 4);
  assert.strictEqual(e.bytecheck_container_checks(8, 3, 0), 3);
  assert.strictEqual(e.bytecheck_range_field_count(0), 2);
  assert.strictEqual(e.bytecheck_range_field_count(2), 0);
  assert.strictEqual(e.bytecheck_range_field_count(4), 1);

  assert.strictEqual(e.rancor_result_transition(1, 1, 1, 0), 0);
  assert.strictEqual(e.rancor_result_transition(0, 1, 0, 0), 1);
  assert.strictEqual(e.rancor_result_transition(0, 1, 1, 0), 2);
  assert.strictEqual(e.rancor_result_transition(0, 1, 0, 1), 3);
  assert.strictEqual(e.rancor_option_transition(0, 1), 2);
  assert.strictEqual(e.rancor_always_ok_state(0, 1), 4);

  assert.strictEqual(e.rend_swap_needed(1, 1), 0);
  assert.strictEqual(e.rend_swap_needed(1, 0), 1);
  assert.strictEqual(e.rend_byte_at_u32(0x01020304, 0, 0), 0x04);
  assert.strictEqual(e.rend_byte_at_u32(0x01020304, 0, 1), 0x01);
  assert.strictEqual(e.rend_fetch_ordering(1), 0);
  assert.strictEqual(e.rend_fetch_ordering(3), 2);
  assert.strictEqual(e.rend_layout_align(8, 0), 8);
  assert.strictEqual(e.rend_layout_align(8, 1), 1);

  assert.strictEqual(e.ptr_meta_class(1, 0, 0, 0), 0);
  assert.strictEqual(e.ptr_meta_class(0, 1, 0, 0), 1);
  assert.strictEqual(e.ptr_meta_class(0, 0, 1, 0), 2);
  assert.strictEqual(e.ptr_meta_class(0, 0, 0, 1), 3);
  assert.strictEqual(e.ptr_meta_roundtrip_valid(10n, 4n, 4n), 1);
  assert.strictEqual(e.ptr_meta_roundtrip_valid(0n, 4n, 4n), 0);
  assert.strictEqual(e.dyn_metadata_equal(77n, 77n), 1);
  assert.strictEqual(e.dyn_metadata_equal(77n, 78n), 0);

  assert.strictEqual(e.munge_destructure_valid(0, 1, 1, 1, 0), 1);
  assert.strictEqual(e.munge_destructure_valid(1, 1, 1, 1, 1), 0);
  assert.strictEqual(e.munge_destructure_valid(1, 1, 1, 0, 1), 1);
  assert.strictEqual(e.munge_restructured_action(0, 1), 1);
  assert.strictEqual(e.munge_restructured_action(1, 0), 2);
  assert.strictEqual(e.munge_restructured_action(1, 1), 3);

  console.log("byte layout core smoke passed");
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
