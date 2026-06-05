#!/usr/bin/env node

const assert = require("assert");
const { execFileSync } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/codec-primitives/async-runtime-core.wat");
const wasm = path.join(os.tmpdir(), `async-runtime-core-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "pipe" });

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = instance.exports;

  assert.strictEqual(e.mpsc_capacity(4, 3), 7);
  assert.strictEqual(e.mpsc_send_decision(1, 1, 0, 4, 0), 1);
  assert.strictEqual(e.mpsc_send_decision(1, 1, 5, 4, 1), 3);
  assert.strictEqual(e.mpsc_send_decision(1, 0, 0, 4, 0), 4);
  assert.strictEqual(e.mpsc_sender_parks_after_send(1, 5, 4), 1);
  assert.strictEqual(e.mpsc_sender_parks_after_send(0, 99, 0), 0);
  assert.strictEqual(e.mpsc_recv_decision(1, 1, 1), 1);
  assert.strictEqual(e.mpsc_recv_decision(0, 1, 1), 0);
  assert.strictEqual(e.mpsc_recv_decision(0, 0, 1), 2);
  assert.strictEqual(e.mpsc_try_recv_error(0, 1), 0);
  assert.strictEqual(e.mpsc_try_recv_error(0, 0), 4);
  assert.strictEqual(e.mpsc_close_action(3, 2), (2 << 16) | 3);
  assert.strictEqual(e.mpsc_drop_sender_closes(1), 1);

  assert.strictEqual(e.oneshot_send_decision(0, 1, 0, 0), 1);
  assert.strictEqual(e.oneshot_send_decision(1, 1, 0, 0), 4);
  assert.strictEqual(e.oneshot_send_decision(0, 1, 1, 0), 4);
  assert.strictEqual(e.oneshot_recv_decision(0, 0, 0), 0);
  assert.strictEqual(e.oneshot_recv_decision(1, 1, 0), 1);
  assert.strictEqual(e.oneshot_recv_decision(1, 0, 0), 5);
  assert.strictEqual(e.oneshot_is_terminated(1, 0), 1);
  assert.strictEqual(e.oneshot_is_terminated(1, 1), 0);

  assert.strictEqual(e.atomic_waker_register_result(0, 0), 0);
  assert.strictEqual(e.atomic_waker_register_result(0, 1), 1);
  assert.strictEqual(e.atomic_waker_register_result(2, 0), 2);
  assert.strictEqual(e.atomic_waker_register_result(1, 0), 3);
  assert.strictEqual(e.atomic_waker_take_result(0, 1), 1);
  assert.strictEqual(e.atomic_waker_take_result(1, 1), 0);

  assert.strictEqual(e.once_set_result(0), 1);
  assert.strictEqual(e.once_set_result(2), 4);
  assert.strictEqual(e.once_get_result(2), 1);
  assert.strictEqual(e.once_get_result(0), 0);
  assert.strictEqual(e.once_get_or_init_transition(0, 1), 2);
  assert.strictEqual(e.once_get_or_init_transition(0, 0), 0);
  assert.strictEqual(e.once_get_or_init_transition(3, 1), 3);
  assert.strictEqual(e.lazy_force_result(2, 0), 1);
  assert.strictEqual(e.lazy_force_result(0, 0), 3);

  assert.strictEqual(e.executor_enter_result(0), 7);
  assert.strictEqual(e.executor_enter_result(1), 8);
  assert.strictEqual(e.local_pool_step_result(1, 0, 0, 0), 1);
  assert.strictEqual(e.local_pool_step_result(0, 0, 2, 0), 0);
  assert.strictEqual(e.local_pool_step_result(0, 0, 0, 0), 2);
  assert.strictEqual(e.spawn_result(1), 1);
  assert.strictEqual(e.spawn_result(0), 6);

  assert.strictEqual(e.parker_park_result(2, 0, 0), 1);
  assert.strictEqual(e.parker_park_result(0, 1, 0), 0);
  assert.strictEqual(e.parker_park_result(0, 0, 1), 1);
  assert.strictEqual(e.parker_unpark_result(0), 1);
  assert.strictEqual(e.parker_unpark_result(2), 0);

  assert.strictEqual(e.thread_local_bucket_index(0), 0);
  assert.strictEqual(e.thread_local_bucket_index(1), 1);
  assert.strictEqual(e.thread_local_bucket_index(7), 3);
  assert.strictEqual(e.thread_local_insert_result(1, 0), 1);
  assert.strictEqual(e.thread_local_insert_result(0, 0), 4);

  assert.strictEqual(e.tokio_join_poll_count(3), 3);
  assert.strictEqual(e.tokio_select_winner(1, 1, 1, 1, 1, 1), 1);
  assert.strictEqual(e.tokio_select_winner(0, 1, 1, 1, 1, 1), 2);
  assert.strictEqual(e.tokio_select_winner(1, 1, 1, 0, 1, 1), 2);
  assert.strictEqual(e.tokio_select_winner(0, 0, 0, 1, 1, 1), 0);

  assert.strictEqual(e.pin_project_valid(0, 1, 1, 0), 1);
  assert.strictEqual(e.pin_project_valid(1, 1, 0, 0), 0);
  assert.strictEqual(e.pin_project_valid(0, 0, 1, 0), 0);
  assert.strictEqual(e.pin_project_field_kind(1, 0, 0), 1);
  assert.strictEqual(e.pin_project_field_kind(0, 1, 0), 4);
  assert.strictEqual(e.pin_project_field_kind(1, 1, 1), 5);

  console.log("async runtime core smoke passed");
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
