#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/wasm-bindgen-core.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  const wasm = execFileSync("wat2wasm", [file, "-o", "-"]);
  execFileSync("wasm-validate", ["-"], { input: wasm });
  return wasm;
}

(async () => {
  const wasm = compileWat(watPath);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const e = instance.exports;
  const mem = new Uint8Array(e.memory.buffer);

  function put(s, off = 64) {
    mem.set(Buffer.from(s), off);
    return [off, Buffer.byteLength(s)];
  }
  function callString(name, s) {
    return e[name](...put(s));
  }

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300135);

  assert.equal(e.wasm_bindgen_descriptor_code(5, 0, 0), 5);
  assert.equal(e.wasm_bindgen_descriptor_code(16, 0, 0), 5);
  assert.equal(e.wasm_bindgen_descriptor_code(16, 1, 0), 18);
  assert.equal(e.wasm_bindgen_descriptor_code(17, 1, 0), 19);
  assert.equal(e.wasm_bindgen_descriptor_code(18, 0, 0), 20);
  assert.equal(e.wasm_bindgen_descriptor_code(18, 0, 1), 21);
  assert.equal(e.wasm_bindgen_descriptor_code(19, 0, 0), 16);
  assert.equal(e.wasm_bindgen_descriptor_wrapper(1, 2, 1, 1, 1), 1);
  assert.equal(e.wasm_bindgen_descriptor_wrapper(0, 2, 1, 0, 0), 3);
  assert.equal(e.wasm_bindgen_descriptor_wrapper(0, 0, 1, 0, 0), 4);
  assert.equal(e.wasm_bindgen_descriptor_wrapper(0, 0, 0, 1, 0), 5);

  assert.equal(e.wasm_bindgen_abi_primitive_count(0), 0);
  assert.equal(e.wasm_bindgen_abi_primitive_count(9), 2);
  assert.equal(e.wasm_bindgen_abi_primitive_count(20), 2);
  assert.equal(e.wasm_bindgen_abi_primitive_count(21), 3);
  assert.equal(e.wasm_bindgen_option_abi_count(3), 4);
  assert.equal(e.wasm_bindgen_option_abi_count(4), -1);
  assert.equal(e.wasm_bindgen_result_abi_count(2), 4);
  assert.equal(e.wasm_bindgen_result_abi_count(3), -1);
  assert.equal(e.wasm_bindgen_option_tag(0), 0);
  assert.equal(e.wasm_bindgen_option_tag(1), 1);
  assert.equal(e.wasm_bindgen_result_is_err(0), 0);
  assert.equal(e.wasm_bindgen_result_is_err(1), 1);
  assert.equal(e.wasm_bindgen_f64_option_sentinel(), Number.MAX_SAFE_INTEGER);
  assert.equal(e.wasm_bindgen_null_slice_ptr(), 0);
  assert.equal(e.wasm_bindgen_slice_abi_count(0), 2);
  assert.equal(e.wasm_bindgen_slice_abi_count(1), 3);

  assert.equal(e.wasm_bindgen_closure_descriptor(1, 1, 1), 1111);
  assert.equal(e.wasm_bindgen_closure_descriptor(0, 0, 0), 1000);
  assert.equal(e.wasm_bindgen_closure_lifetime_action(1, 0, 0, 0), 1);
  assert.equal(e.wasm_bindgen_closure_lifetime_action(0, 0, 0, 0), 2);
  assert.equal(e.wasm_bindgen_closure_lifetime_action(1, 1, 0, 0), 3);
  assert.equal(e.wasm_bindgen_closure_lifetime_action(1, 0, 1, 1), 4);
  assert.equal(e.wasm_bindgen_closure_lifetime_action(1, 0, 1, 0), 5);
  assert.equal(e.wasm_bindgen_once_call_state(0), 1);
  assert.equal(e.wasm_bindgen_once_call_state(1), 2);

  assert.equal(callString("wasm_bindgen_attr_code", "catch"), 1);
  assert.equal(callString("wasm_bindgen_attr_code", "constructor"), 2);
  assert.equal(callString("wasm_bindgen_attr_code", "js_namespace"), 6);
  assert.equal(callString("wasm_bindgen_attr_code", "module"), 7);
  assert.equal(callString("wasm_bindgen_attr_code", "getter_with_clone"), 43);
  assert.equal(callString("wasm_bindgen_attr_code", "assert_no_shim"), 52);
  assert.equal(callString("wasm_bindgen_attr_code", "unknown"), 0);
  assert.equal(e.wasm_bindgen_attr_value_shape(1), 1);
  assert.equal(e.wasm_bindgen_attr_value_shape(5), 2);
  assert.equal(e.wasm_bindgen_attr_value_shape(6), 4);
  assert.equal(e.wasm_bindgen_attr_value_shape(7), 3);

  assert.equal(callString("wasm_bindgen_computed_key_status", "plainName"), 0);
  assert.equal(callString("wasm_bindgen_computed_key_status", "[Symbol.iterator]"), 1);
  assert.equal(callString("wasm_bindgen_computed_key_status", "[Symbol.]"), 2);
  assert.equal(callString("wasm_bindgen_computed_key_status", "[Foo.bar]"), 2);
  assert.equal(e.wasm_bindgen_namespace_status(0, 0, 0), 2);
  assert.equal(e.wasm_bindgen_namespace_status(1, 1, 0), 3);
  assert.equal(e.wasm_bindgen_namespace_status(1, 1, 1), 1);

  assert.equal(e.wasm_bindgen_import_module_kind(1, 0, 0), 3);
  assert.equal(e.wasm_bindgen_import_module_kind(0, 1, 0), 1);
  assert.equal(e.wasm_bindgen_import_module_kind(0, 0, 1), 4);
  assert.equal(e.wasm_bindgen_import_module_kind(0, 0, 0), 2);
  assert.equal(e.wasm_bindgen_import_kind_code(6), 6);
  assert.equal(e.wasm_bindgen_import_kind_code(9), 0);
  assert.equal(e.wasm_bindgen_method_operation_code(1, 0), 1);
  assert.equal(e.wasm_bindgen_method_operation_code(0, 4), 5);
  assert.equal(e.wasm_bindgen_struct_export_status(1, 0, 0), 2);
  assert.equal(e.wasm_bindgen_struct_export_status(0, 2, 0), 3);
  assert.equal(e.wasm_bindgen_struct_export_status(0, 1, 1), 4);
  assert.equal(e.wasm_bindgen_struct_export_status(0, 1, 0), 1);
  assert.equal(e.wasm_bindgen_dynamic_union_variant_kind(0, 0, 0), 1);
  assert.equal(e.wasm_bindgen_dynamic_union_variant_kind(1, 0, 0), 2);
  assert.equal(e.wasm_bindgen_dynamic_union_variant_kind(1, 1, 1), 3);

  assert.equal(e.wasm_bindgen_leb128_u32_len(0), 1);
  assert.equal(e.wasm_bindgen_leb128_u32_len(127), 1);
  assert.equal(e.wasm_bindgen_leb128_u32_len(128), 2);
  assert.equal(e.wasm_bindgen_leb128_u32_len(16384), 3);
  assert.equal(e.wasm_bindgen_option_encode_len(0, 99), 1);
  assert.equal(e.wasm_bindgen_option_encode_len(1, 9), 10);
  assert.equal(e.wasm_bindgen_slice_encode_len(128), 130);
  assert.equal(e.wasm_bindgen_vec_encode_len(3, 12), 13);
  assert.equal(e.wasm_bindgen_enum_encode_len(8), 9);

  console.log(JSON.stringify({ unit: "wasm-bindgen-core", standard_id: e.proto_standard_id(), ok: true }));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
