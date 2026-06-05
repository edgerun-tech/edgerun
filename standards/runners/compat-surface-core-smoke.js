#!/usr/bin/env node

const assert = require("assert");
const { execFileSync } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/codec-primitives/compat-surface-core.wat");
const wasm = path.join(os.tmpdir(), `compat-surface-core-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "pipe" });

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = instance.exports;

  assert.strictEqual(e.http_header_name_byte_valid("x".charCodeAt(0)), 1);
  assert.strictEqual(e.http_header_name_byte_valid("!".charCodeAt(0)), 1);
  assert.strictEqual(e.http_header_name_byte_valid(":".charCodeAt(0)), 0);
  assert.strictEqual(e.http_header_name_byte_valid(" ".charCodeAt(0)), 0);
  assert.strictEqual(e.http_header_value_byte_valid(9), 1);
  assert.strictEqual(e.http_header_value_byte_valid(31), 0);
  assert.strictEqual(e.http_method_byte_valid("G".charCodeAt(0)), 1);
  assert.strictEqual(e.http_method_byte_valid(32), 0);
  assert.strictEqual(e.http_status_class(99), 0);
  assert.strictEqual(e.http_status_class(204), 2);
  assert.strictEqual(e.http_status_class(404), 4);
  assert.strictEqual(e.http_status_class(503), 5);
  assert.strictEqual(e.http_status_class(302), 1);
  assert.strictEqual(e.http_known_reason(101), 1);
  assert.strictEqual(e.http_known_reason(418), 0);
  assert.strictEqual(e.http_request_builder_valid(1, 1, 1, 1), 1);
  assert.strictEqual(e.http_request_builder_valid(1, 0, 1, 1), 0);
  assert.strictEqual(e.http_response_builder_valid(1, 0), 0);
  assert.strictEqual(e.http_uri_path_mode(0, 0), 0);
  assert.strictEqual(e.http_uri_path_mode(1, 0), 1);
  assert.strictEqual(e.http_uri_path_mode(1, 1), 2);

  assert.strictEqual(e.reqwest_cookie_store_action(1, 1, 0, 0), 1);
  assert.strictEqual(e.reqwest_cookie_store_action(1, 1, 1, 0), 0);
  assert.strictEqual(e.reqwest_cookie_store_action(0, 1, 0, 0), 0);
  assert.strictEqual(e.reqwest_cookie_send(1, 1, 0), 0);
  assert.strictEqual(e.reqwest_cookie_send(1, 1, 1), 1);
  assert.strictEqual(e.reqwest_proxy_scheme_valid(1), 1);
  assert.strictEqual(e.reqwest_proxy_scheme_valid(3), 0);
  assert.strictEqual(e.reqwest_https_connect_proxy_valid(1, 1, 1, 200), 1);
  assert.strictEqual(e.reqwest_https_connect_proxy_valid(2, 1, 1, 200), 0);
  assert.strictEqual(e.reqwest_error_for_status(399), 0);
  assert.strictEqual(e.reqwest_error_for_status(500), 1);
  assert.strictEqual(e.reqwest_http_message_complete(1, 5, 4), 1);
  assert.strictEqual(e.reqwest_http_message_complete(1, 3, 4), 0);
  assert.strictEqual(e.reqwest_builder_version(0, 0, 0), 0);
  assert.strictEqual(e.reqwest_builder_version(1, 0, 0), 1);
  assert.strictEqual(e.reqwest_builder_version(1, 1, 0), 2);
  assert.strictEqual(e.reqwest_builder_version(1, 1, 1), 3);

  assert.strictEqual(e.json_escape_action("a".charCodeAt(0)), 0);
  assert.strictEqual(e.json_escape_action('"'.charCodeAt(0)), 1);
  assert.strictEqual(e.json_escape_action(10), 2);
  assert.strictEqual(e.json_escape_action(13), 3);
  assert.strictEqual(e.json_escape_action(9), 4);
  assert.strictEqual(e.json_escape_action(8), 5);
  assert.strictEqual(e.json_escape_action(12), 6);
  assert.strictEqual(e.json_escape_action(1), 7);
  assert.strictEqual(e.schemars_instance_json_code(6), 6);
  assert.strictEqual(e.schemars_instance_json_code(9), 0);
  assert.strictEqual(e.schemars_schema_field_count(1, 1, 0, 1, 0, 2, 0, 3, 1, 1, 0), 7);
  assert.strictEqual(e.schemars_container_kind(7), 7);

  assert.strictEqual(e.wbg_schema_version_code(), 2121);
  assert.strictEqual(e.wbg_export_char_action("A".charCodeAt(0)), 0);
  assert.strictEqual(e.wbg_export_char_action(".".charCodeAt(0)), 1);
  assert.strictEqual(e.wbg_export_char_action("[".charCodeAt(0)), 2);
  assert.strictEqual(e.wbg_named_function_kind(4), 4);
  assert.strictEqual(e.wbg_named_function_kind(9), 0);
  assert.strictEqual(e.js_ident_start_ascii("$".charCodeAt(0)), 1);
  assert.strictEqual(e.js_ident_start_ascii("1".charCodeAt(0)), 0);
  assert.strictEqual(e.js_ident_continue_ascii("1".charCodeAt(0)), 1);
  assert.strictEqual(e.js_keyword_kind(0, 0), 0);
  assert.strictEqual(e.js_keyword_kind(1, 1), 1);
  assert.strictEqual(e.js_keyword_kind(1, 0), 2);
  assert.strictEqual(e.js_ident_repair_action(0, 0), 1);
  assert.strictEqual(e.js_ident_repair_action(1, 1), 2);
  assert.strictEqual(e.js_ident_repair_action(1, 0), 0);

  assert.strictEqual(e.unicode_terminal_width(0x0301), 0);
  assert.strictEqual(e.unicode_terminal_width(0x1100), 2);
  assert.strictEqual(e.unicode_terminal_width("A".charCodeAt(0)), 1);
  assert.strictEqual(e.unicode_ident_table_partition(0x007f), 1);
  assert.strictEqual(e.unicode_ident_table_partition(0x3042), 2);
  assert.strictEqual(e.unicode_ident_table_partition(0x10000), 3);
  assert.strictEqual(e.unicode_ident_table_partition(0x110000), 0);

  assert.strictEqual(e.jni_primitive_size(1, 8), 1);
  assert.strictEqual(e.jni_primitive_size(3, 8), 2);
  assert.strictEqual(e.jni_primitive_size(5, 8), 4);
  assert.strictEqual(e.jni_primitive_size(6, 8), 8);
  assert.strictEqual(e.jni_primitive_size(9, 8), 8);
  assert.strictEqual(e.jni_status_class(0), 0);
  assert.strictEqual(e.jni_status_class(-3), 3);
  assert.strictEqual(e.jni_status_class(-9), 99);
  assert.strictEqual(e.jni_version_value(18), 0x00010008);
  assert.strictEqual(e.jni_version_value(21), 0x00150000);
  assert.strictEqual(e.jni_ref_type_valid(3), 1);
  assert.strictEqual(e.jni_ref_type_valid(4), 0);
  assert.strictEqual(e.jni_array_release_action(0), 0);
  assert.strictEqual(e.jni_array_release_action(1), 1);
  assert.strictEqual(e.jni_array_release_action(2), 2);
  assert.strictEqual(e.jni_version_gate(0x00150000, 0x00090000), 1);
  assert.strictEqual(e.jni_version_gate(0x00010008, 0x00090000), 0);

  console.log("compat surface core smoke passed");
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
