const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/codec-primitives/proc-macro2-core.wat");
const wasm = path.join(os.tmpdir(), `proc-macro2-core-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm]);
const bytes = fs.readFileSync(wasm);

(async () => {
  const { instance } = await WebAssembly.instantiate(bytes);
  const e = instance.exports;

  assert.strictEqual(e.proto_abi_version(), 2);
  assert.strictEqual(e.proto_standard_id(), 300138);

  assert.strictEqual(e.pm2_delimiter_open_code("(".charCodeAt(0)), 1);
  assert.strictEqual(e.pm2_delimiter_open_code("{".charCodeAt(0)), 2);
  assert.strictEqual(e.pm2_delimiter_open_code("[".charCodeAt(0)), 3);
  assert.strictEqual(e.pm2_delimiter_close_code(")".charCodeAt(0)), 1);
  assert.strictEqual(e.pm2_delimiter_close_code("}".charCodeAt(0)), 2);
  assert.strictEqual(e.pm2_delimiter_close_code("]".charCodeAt(0)), 3);
  assert.strictEqual(e.pm2_delimiter_matches(1, 1), 1);
  assert.strictEqual(e.pm2_delimiter_matches(1, 2), 0);

  assert.strictEqual(e.pm2_leaf_token_choice(1, 1, 1, 0), 4);
  assert.strictEqual(e.pm2_leaf_token_choice(0, 1, 1, 0), 3);
  assert.strictEqual(e.pm2_leaf_token_choice(0, 0, 1, 0), 2);
  assert.strictEqual(e.pm2_leaf_token_choice(0, 0, 0, 1), 4);
  assert.strictEqual(e.pm2_leaf_token_choice(0, 0, 0, 0), 0);

  assert.strictEqual(e.pm2_ident_start_ascii("_".charCodeAt(0)), 1);
  assert.strictEqual(e.pm2_ident_start_ascii("a".charCodeAt(0)), 1);
  assert.strictEqual(e.pm2_ident_start_ascii("9".charCodeAt(0)), 0);
  assert.strictEqual(e.pm2_ident_continue_ascii("9".charCodeAt(0)), 1);
  assert.strictEqual(e.pm2_raw_ident_allowed(0), 1);
  assert.strictEqual(e.pm2_raw_ident_allowed(3), 0);

  assert.strictEqual(e.pm2_punct_char_valid("+".charCodeAt(0)), 1);
  assert.strictEqual(e.pm2_punct_char_valid("a".charCodeAt(0)), 0);
  assert.strictEqual(e.pm2_punct_spacing("+".charCodeAt(0), 1, 0, 0), 2);
  assert.strictEqual(e.pm2_punct_spacing("+".charCodeAt(0), 0, 0, 0), 1);
  assert.strictEqual(e.pm2_punct_spacing("/".charCodeAt(0), 1, 1, 0), 0);
  assert.strictEqual(e.pm2_punct_spacing("'".charCodeAt(0), 0, 0, 1), 2);
  assert.strictEqual(e.pm2_punct_spacing("'".charCodeAt(0), 0, 0, 0), 0);

  assert.strictEqual(e.pm2_skip_state(" ".charCodeAt(0), 0, 0, 0), 1);
  assert.strictEqual(e.pm2_skip_state("/".charCodeAt(0), "/".charCodeAt(0), "x".charCodeAt(0), 0), 2);
  assert.strictEqual(e.pm2_skip_state("/".charCodeAt(0), "/".charCodeAt(0), "/".charCodeAt(0), "x".charCodeAt(0)), 0);
  assert.strictEqual(e.pm2_skip_state("/".charCodeAt(0), "*".charCodeAt(0), "*".charCodeAt(0), "/".charCodeAt(0)), 4);
  assert.strictEqual(e.pm2_skip_state("/".charCodeAt(0), "*".charCodeAt(0), "x".charCodeAt(0), 0), 3);
  assert.strictEqual(e.pm2_block_comment_depth(1, "/".charCodeAt(0), "*".charCodeAt(0)), 2);
  assert.strictEqual(e.pm2_block_comment_depth(2, "*".charCodeAt(0), "/".charCodeAt(0)), 1);

  assert.strictEqual(e.pm2_negative_literal_accepts("7".charCodeAt(0)), 1);
  assert.strictEqual(e.pm2_negative_literal_accepts("x".charCodeAt(0)), 0);
  assert.strictEqual(e.pm2_negative_literal_token_count(1), 2);
  assert.strictEqual(e.pm2_negative_literal_token_count(0), 1);

  assert.strictEqual(e.pm2_raw_unit_valid(1, "x".charCodeAt(0)), 1);
  assert.strictEqual(e.pm2_raw_unit_valid(1, 13), 0);
  assert.strictEqual(e.pm2_raw_unit_valid(2, 200), 0);
  assert.strictEqual(e.pm2_raw_unit_valid(3, 0), 0);
  assert.strictEqual(e.pm2_escape_error_fatal(0), 0);
  assert.strictEqual(e.pm2_escape_error_fatal(5), 1);
  assert.strictEqual(e.pm2_escape_error_fatal(20), 0);

  assert.strictEqual(e.pm2_display_needs_space(0, 0), 0);
  assert.strictEqual(e.pm2_display_needs_space(1, 0), 1);
  assert.strictEqual(e.pm2_display_needs_space(1, 1), 0);
  assert.strictEqual(e.pm2_float_unsuffixed_appends_dot_zero(0), 1);
  assert.strictEqual(e.pm2_float_unsuffixed_appends_dot_zero(1), 0);
  assert.strictEqual(e.pm2_span_location_available(1, 0), 1);
  assert.strictEqual(e.pm2_span_location_available(1, 1), 0);
  assert.strictEqual(e.pm2_impl_kind(1, 0), 2);
  assert.strictEqual(e.pm2_impl_kind(1, 1), 1);
  assert.strictEqual(e.pm2_impl_kind(0, 0), 1);

  fs.unlinkSync(wasm);
  console.log("proc-macro2-core smoke ok");
})().catch((err) => {
  try { fs.unlinkSync(wasm); } catch (_) {}
  throw err;
});
