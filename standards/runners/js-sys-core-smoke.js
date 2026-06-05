const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/codec-primitives/js-sys-core.wat");
const wasm = path.join(os.tmpdir(), `js-sys-core-${process.pid}.wasm`);

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
  assert.strictEqual(e.proto_standard_id(), 300134);

  assert.strictEqual(callString("js_sys_global_kind", "Promise"), 4);
  assert.strictEqual(callString("js_sys_global_kind", "WeakMap"), 7);
  assert.strictEqual(callString("js_sys_global_kind", "JSON"), 10);
  assert.strictEqual(callString("js_sys_global_kind", "BigInt64Array"), 19);
  assert.strictEqual(callString("js_sys_global_kind", "Temporal"), 20);
  assert.strictEqual(callString("js_sys_global_kind", "Atomics"), 21);
  assert.strictEqual(callString("js_sys_global_kind", "document"), 0);

  assert.strictEqual(e.js_sys_binding_family(4), 1);
  assert.strictEqual(e.js_sys_binding_family(7), 2);
  assert.strictEqual(e.js_sys_binding_family(10), 3);
  assert.strictEqual(e.js_sys_binding_family(13), 4);
  assert.strictEqual(e.js_sys_binding_family(16), 5);
  assert.strictEqual(e.js_sys_binding_family(19), 7);
  assert.strictEqual(e.js_sys_binding_family(20), 8);
  assert.strictEqual(e.js_sys_binding_family(21), 9);

  assert.strictEqual(callString("js_sys_temporal_option_code", "constrain"), 1);
  assert.strictEqual(callString("js_sys_temporal_option_code", "reject"), 2);
  assert.strictEqual(callString("js_sys_temporal_option_code", "compatible"), 4);
  assert.strictEqual(callString("js_sys_temporal_option_code", "prefer"), 8);
  assert.strictEqual(callString("js_sys_temporal_option_code", "critical"), 13);
  assert.strictEqual(callString("js_sys_temporal_option_code", "previous"), 15);
  assert.strictEqual(callString("js_sys_temporal_option_code", "bad"), 0);

  assert.strictEqual(callString("js_sys_temporal_unit_code", "year"), 1);
  assert.strictEqual(callString("js_sys_temporal_unit_code", "months"), 2);
  assert.strictEqual(callString("js_sys_temporal_unit_code", "weeks"), 3);
  assert.strictEqual(callString("js_sys_temporal_unit_code", "days"), 4);
  assert.strictEqual(callString("js_sys_temporal_unit_code", "hours"), 5);
  assert.strictEqual(callString("js_sys_temporal_unit_code", "minutes"), 6);
  assert.strictEqual(callString("js_sys_temporal_unit_code", "seconds"), 7);
  assert.strictEqual(callString("js_sys_temporal_unit_code", "milliseconds"), 8);
  assert.strictEqual(callString("js_sys_temporal_unit_code", "microseconds"), 9);
  assert.strictEqual(callString("js_sys_temporal_unit_code", "nanoseconds"), 10);

  assert.strictEqual(callString("js_sys_fractional_second_digits", "auto"), 10);
  assert.strictEqual(callString("js_sys_fractional_second_digits", "7"), 7);
  assert.strictEqual(callString("js_sys_fractional_second_digits", "11"), -1);

  assert.strictEqual(e.js_sys_queue_schedule_state(1, 1), 0);
  assert.strictEqual(e.js_sys_queue_schedule_state(0, 1), 1);
  assert.strictEqual(e.js_sys_queue_schedule_state(0, 0), 2);
  assert.strictEqual(e.js_sys_queue_tick_run_count(5), 5);

  assert.strictEqual(e.js_sys_stream_poll(1, 0, 1, 1, 1, 0), 1);
  assert.strictEqual(e.js_sys_stream_poll(0, 0, 0, 1, 1, 0), 3);
  assert.strictEqual(e.js_sys_stream_poll(0, 1, 1, 0, 1, 0), 0);
  assert.strictEqual(e.js_sys_stream_poll(0, 1, 1, 1, 0, 0), 3);
  assert.strictEqual(e.js_sys_stream_poll(0, 1, 1, 1, 1, 1), 1);
  assert.strictEqual(e.js_sys_stream_poll(0, 1, 1, 1, 1, 0), 2);

  assert.strictEqual(e.js_sys_wait_async_strategy(1), 1);
  assert.strictEqual(e.js_sys_wait_async_strategy(0), 2);

  fs.unlinkSync(wasm);
  console.log("js-sys-core smoke ok");
})().catch((err) => {
  try { fs.unlinkSync(wasm); } catch (_) {}
  throw err;
});
