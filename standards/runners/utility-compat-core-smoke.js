#!/usr/bin/env node

const assert = require("assert");
const { execFileSync } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/codec-primitives/utility-compat-core.wat");
const wasm = path.join(os.tmpdir(), `utility-compat-core-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "pipe" });

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = instance.exports;

  assert.strictEqual(e.utility_compat_abi_version(), 1);
  assert.strictEqual(e.arrayvec_push_result(1, 2), 0);
  assert.strictEqual(e.arrayvec_push_result(2, 2), 1);
  assert.strictEqual(e.arrayvec_len_after_push(1, 2), 2);
  assert.strictEqual(e.arrayvec_len_after_pop(0), 0);
  assert.strictEqual(e.arrayvec_len_after_pop(2), 1);
  assert.strictEqual(e.arrayvec_len_after_truncate(4, 2), 2);
  assert.strictEqual(e.arrayvec_len_after_truncate(4, 8), 4);
  assert.strictEqual(e.arrayvec_remaining_capacity(2, 4), 2);

  assert.strictEqual(e.bytes_slice_result(10, 2, 5), 0);
  assert.strictEqual(e.bytes_slice_result(10, 6, 5), 1);
  assert.strictEqual(e.bytes_split_to_result(10, 11), 1);
  assert.strictEqual(e.bytes_len_after_advance(10, 4), 6);

  assert.strictEqual(e.log_level_name_code(0), 1);
  assert.strictEqual(e.log_level_name_code(9), 5);
  assert.strictEqual(e.log_enabled(3, 2), 1);
  assert.strictEqual(e.log_enabled(1, 2), 0);
  assert.strictEqual(e.log_dispatch_path(1, 1, 1), 1);
  assert.strictEqual(e.log_dispatch_path(1, 0, 1), 2);
  assert.strictEqual(e.log_dispatch_path(0, 1, 1), 0);

  assert.strictEqual(e.try_lock_result(0), 0);
  assert.strictEqual(e.try_lock_result(1), 1);
  assert.strictEqual(e.try_lock_ordering_result(2, 3), 0);
  assert.strictEqual(e.try_lock_ordering_result(0, 3), 1);
  assert.strictEqual(e.try_lock_ordering_result(2, 0), 2);

  assert.strictEqual(e.async_channel_try_recv_result(1, 0), 0);
  assert.strictEqual(e.async_channel_try_recv_result(0, 0), 1);
  assert.strictEqual(e.async_channel_try_recv_result(0, 1), 2);
  assert.strictEqual(e.async_channel_bounded_capacity(8), 8);
  assert.strictEqual(e.async_channel_unbounded_capacity_code(), 0);

  assert.strictEqual(e.error_repr_display_code(3), 3);
  assert.strictEqual(e.error_repr_display_code(9), 0);
  assert.strictEqual(e.error_chain_next_result(0, 0), 0);
  assert.strictEqual(e.error_chain_next_result(1, 1), 1);
  assert.strictEqual(e.error_chain_next_result(1, 0), 2);
  assert.strictEqual(e.ensure_result(1), 0);
  assert.strictEqual(e.ensure_result(0), 1);

  assert.strictEqual(e.lazy_static_state_result(1, 0), 0);
  assert.strictEqual(e.lazy_static_state_result(0, 0), 1);
  assert.strictEqual(e.lazy_static_state_result(0, 1), 2);
  assert.strictEqual(e.time_target_code(1), 1);
  assert.strictEqual(e.time_target_code(0), 2);
  assert.strictEqual(e.derive_macro_kind_code(10), 10);
  assert.strictEqual(e.derive_macro_kind_code(11), 0);

  console.log("utility compat core smoke passed");
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
