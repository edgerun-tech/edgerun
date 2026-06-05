const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/codec-primitives/runtime-core-state.wat");
const wasm = path.join(os.tmpdir(), `runtime-core-state-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm]);
const bytes = fs.readFileSync(wasm);

(async () => {
  const { instance } = await WebAssembly.instantiate(bytes);
  const e = instance.exports;

  const pack = (low, high) => (low & 0xffff) | ((high & 0xffff) << 16);

  assert.strictEqual(e.proto_abi_version(), 2);
  assert.strictEqual(e.proto_standard_id(), 300137);

  assert.strictEqual(e.rt_spawn_pending_after(4), 5);
  assert.strictEqual(e.rt_run_queue_result(5, 4, 3), pack(2, 1));
  assert.strictEqual(e.rt_builder_threads(0), 1);
  assert.strictEqual(e.rt_builder_threads(8), 8);

  assert.strictEqual(e.rt_elapsed_since(15n, 10n), 5n);
  assert.strictEqual(e.rt_elapsed_since(5n, 10n), 0n);
  assert.strictEqual(e.rt_us_to_ticks(7n), 70n);
  assert.strictEqual(e.rt_ms_to_ticks(3n), 30000n);
  assert.strictEqual(e.rt_timer_poll_result(5, 3, 1), pack(3, 3));
  assert.strictEqual(e.rt_sleep_poll(9n, 10n), 0);
  assert.strictEqual(e.rt_sleep_poll(10n, 10n), 1);
  assert.strictEqual(e.rt_timeout_poll(5n, 10n, 1), 1);
  assert.strictEqual(e.rt_timeout_poll(10n, 10n, 1), 2);

  assert.strictEqual(e.rt_oneshot_recv_state(0, 0), 0);
  assert.strictEqual(e.rt_oneshot_recv_state(1, 1), 1);
  assert.strictEqual(e.rt_oneshot_recv_state(1, 0), 2);
  assert.strictEqual(e.rt_oneshot_send_state(0), 1);
  assert.strictEqual(e.rt_oneshot_send_state(1), 3);

  assert.strictEqual(e.rt_mpsc_try_send(2, 1, 0), 0);
  assert.strictEqual(e.rt_mpsc_try_send(2, 2, 0), 1);
  assert.strictEqual(e.rt_mpsc_try_send(0, 200, 0), 0);
  assert.strictEqual(e.rt_mpsc_try_send(2, 1, 1), 2);
  assert.strictEqual(e.rt_mpsc_try_recv(2, 0), 0);
  assert.strictEqual(e.rt_mpsc_try_recv(0, 0), 1);
  assert.strictEqual(e.rt_mpsc_try_recv(0, 1), 2);

  assert.strictEqual(e.rt_semaphore_try_acquire(3), 2);
  assert.strictEqual(e.rt_semaphore_try_acquire(0), -1);
  assert.strictEqual(e.rt_semaphore_release(2, 0), pack(3, 0));
  assert.strictEqual(e.rt_semaphore_release(2, 4), pack(3, 1));

  assert.strictEqual(e.rt_select_ready_index(0b0100, 4), 2);
  assert.strictEqual(e.rt_select_ready_index(0b1010, 4), 1);
  assert.strictEqual(e.rt_select_ready_index(0, 4), -1);
  assert.strictEqual(e.rt_interval_next_after_poll(100n, 90n, 10n), 110n);
  assert.strictEqual(e.rt_interval_next_after_poll(80n, 90n, 10n), 90n);

  assert.strictEqual(e.rt_cancel_poll(1, 0, 0), 1);
  assert.strictEqual(e.rt_cancel_poll(0, 0, 0), 0);
  assert.strictEqual(e.rt_cancel_poll(0, 1, 1), 1);

  assert.strictEqual(e.rt_ring_capacity(1), 2);
  assert.strictEqual(e.rt_ring_capacity(9), 16);
  assert.strictEqual(e.rt_ring_len(6, 2, 8), 4);
  assert.strictEqual(e.rt_ring_push_count(3, 8, 10), 4);
  assert.strictEqual(e.rt_ring_push_count(3, 8, 2), 2);

  assert.strictEqual(e.rt_host_io_poll_state(1, 0), 0);
  assert.strictEqual(e.rt_host_io_poll_state(0, 1), 1);
  assert.strictEqual(e.rt_host_io_poll_state(0, 0), 2);
  assert.strictEqual(e.rt_socket_bind_allowed(1, 1), 1);
  assert.strictEqual(e.rt_socket_bind_allowed(1, 0), 0);
  assert.strictEqual(e.rt_socket_bind_allowed(0, 1), 0);

  fs.unlinkSync(wasm);
  console.log("runtime-core-state smoke ok");
})().catch((err) => {
  try { fs.unlinkSync(wasm); } catch (_) {}
  throw err;
});
