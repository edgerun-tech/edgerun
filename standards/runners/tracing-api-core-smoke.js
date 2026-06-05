const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/codec-primitives/tracing-api-core.wat");
const wasm = path.join(os.tmpdir(), `tracing-api-core-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm]);
const bytes = fs.readFileSync(wasm);

(async () => {
  const { instance } = await WebAssembly.instantiate(bytes);
  const e = instance.exports;
  const mem = new Uint8Array(e.memory.buffer);

  function put(s, off = 64) {
    mem.set(Buffer.from(s), off);
    return [off, Buffer.byteLength(s)];
  }
  function callString(fn, s) {
    return e[fn](...put(s));
  }

  assert.strictEqual(e.proto_abi_version(), 2);
  assert.strictEqual(e.proto_standard_id(), 300136);

  assert.strictEqual(callString("tracing_level_code", "OFF"), 0);
  assert.strictEqual(callString("tracing_level_code", "ERROR"), 1);
  assert.strictEqual(callString("tracing_level_code", "WARN"), 2);
  assert.strictEqual(callString("tracing_level_code", "INFO"), 3);
  assert.strictEqual(callString("tracing_level_code", "DEBUG"), 4);
  assert.strictEqual(callString("tracing_level_code", "TRACE"), 5);
  assert.strictEqual(callString("tracing_level_code", "bad"), -1);

  assert.strictEqual(e.tracing_level_enabled(1, 1), 1);
  assert.strictEqual(e.tracing_level_enabled(4, 3), 0);
  assert.strictEqual(e.tracing_level_enabled(3, 4), 1);
  assert.strictEqual(e.tracing_level_enabled(1, 0), 0);
  assert.strictEqual(e.tracing_static_max_level(1, 2, 4), 2);
  assert.strictEqual(e.tracing_static_max_level(0, 2, 4), 4);
  assert.strictEqual(e.tracing_static_max_level(0, -1, -1), 5);

  assert.strictEqual(e.tracing_interest_join(0, 0), 0);
  assert.strictEqual(e.tracing_interest_join(2, 0), 2);
  assert.strictEqual(e.tracing_interest_join(2, 1), 1);
  assert.strictEqual(e.tracing_cached_interest_enabled(2, 0), 1);
  assert.strictEqual(e.tracing_cached_interest_enabled(1, 0), 0);
  assert.strictEqual(e.tracing_cached_interest_enabled(1, 1), 1);
  assert.strictEqual(e.tracing_register_callsite_default(1), 2);
  assert.strictEqual(e.tracing_register_callsite_default(0), 0);

  assert.strictEqual(e.tracing_span_id_valid(0n), 0);
  assert.strictEqual(e.tracing_span_id_valid(42n), 1);
  assert.strictEqual(e.tracing_parent_code(1, 0), 2);
  assert.strictEqual(e.tracing_parent_code(0, 1), 1);
  assert.strictEqual(e.tracing_parent_code(0, 0), 0);
  assert.strictEqual(e.tracing_current_state(1, 0), 2);
  assert.strictEqual(e.tracing_current_state(0, 1), 1);
  assert.strictEqual(e.tracing_current_state(0, 0), 0);

  assert.strictEqual(e.tracing_field_index_valid(2, 3), 1);
  assert.strictEqual(e.tracing_field_index_valid(3, 3), 0);
  assert.strictEqual(e.tracing_value_present_count(0b10111, 5), 4);
  assert.strictEqual(e.tracing_value_kind_supported(1), 1);
  assert.strictEqual(e.tracing_value_kind_supported(9), 1);
  assert.strictEqual(e.tracing_value_kind_supported(10), 0);
  assert.strictEqual(callString("tracing_metadata_kind_code", "span"), 1);
  assert.strictEqual(callString("tracing_metadata_kind_code", "event"), 2);
  assert.strictEqual(callString("tracing_metadata_kind_code", "other"), 0);

  assert.strictEqual(e.tracing_macro_emit_allowed(3, 3, 2, 0), 1);
  assert.strictEqual(e.tracing_macro_emit_allowed(4, 3, 2, 1), 0);
  assert.strictEqual(e.tracing_macro_emit_allowed(3, 3, 1, 0), 0);
  assert.strictEqual(e.tracing_macro_emit_allowed(3, 3, 1, 1), 1);

  assert.strictEqual(e.tracing_log_enabled(3, 3, 0, 1), 1);
  assert.strictEqual(e.tracing_log_enabled(4, 3, 0, 1), 0);
  assert.strictEqual(e.tracing_log_enabled(3, 3, 1, 1), 0);
  assert.strictEqual(e.tracing_log_enabled(3, 3, 0, 0), 0);
  assert.strictEqual(e.tracing_log_dispatch_action(1), 1);
  assert.strictEqual(e.tracing_log_dispatch_action(0), 0);

  assert.strictEqual(e.tracing_instrument_shape(1, 1), 15);
  assert.strictEqual(e.tracing_instrument_shape(0, 0), 5);

  fs.unlinkSync(wasm);
  console.log("tracing-api-core smoke ok");
})().catch((err) => {
  try { fs.unlinkSync(wasm); } catch (_) {}
  throw err;
});
