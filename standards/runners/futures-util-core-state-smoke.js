#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/futures-util-core-state.wat");

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

  assert.equal(e.futures_select_poll(1, 1), 1);
  assert.equal(e.futures_select_poll(0, 1), 2);
  assert.equal(e.futures_select_poll(0, 0), 0);
  assert.equal(e.futures_try_select_poll(3, 1), 4);
  assert.equal(e.futures_try_select_poll(0, 3), 5);
  assert.equal(e.futures_try_select_poll(0, 1), 2);

  assert.equal(e.futures_maybe_done_poll(0, 0), 0);
  assert.equal(e.futures_maybe_done_poll(0, 1), 11);
  assert.equal(e.futures_maybe_done_poll(1, 0), 11);
  assert.equal(e.futures_maybe_done_poll(2, 0), 29);
  assert.equal(e.futures_maybe_done_take_output(1), 21);
  assert.equal(e.futures_maybe_done_take_output(0), 0);

  assert.equal(e.futures_future_fuse_poll(1, 0), 10);
  assert.equal(e.futures_future_fuse_poll(1, 1), 11);
  assert.equal(e.futures_future_fuse_poll(0, 1), 0);
  assert.equal(e.futures_stream_fuse_poll(0, 1), 1);
  assert.equal(e.futures_stream_fuse_poll(0, 2), 12);
  assert.equal(e.futures_stream_fuse_poll(1, 1), 12);

  assert.equal(e.futures_join_poll(0b001, 0b110, 3), 1);
  assert.equal(e.futures_join_poll(0b001, 0b010, 3), 0);
  assert.equal(e.futures_try_join_poll(0, 0b111, 3, 0), 1);
  assert.equal(e.futures_try_join_poll(0, 0b011, 3, 2), 12);

  assert.equal(e.futures_stream_select_poll(0, 0, 1, 1), 1);
  assert.equal(e.futures_stream_select_poll(0, 1, 1, 1), 2);
  assert.equal(e.futures_stream_select_poll(0, 0, 2, 0), 10);
  assert.equal(e.futures_stream_select_poll(1, 0, 0, 1), 21);
  assert.equal(e.futures_stream_select_poll(1, 0, 0, 2), 33);
  assert.equal(e.futures_stream_select_poll(3, 0, 1, 1), 33);

  assert.equal(e.futures_ready_chunks_poll(0, 3, 0), 0);
  assert.equal(e.futures_ready_chunks_poll(2, 3, 0), 1);
  assert.equal(e.futures_ready_chunks_poll(2, 3, 1), 1);
  assert.equal(e.futures_ready_chunks_poll(1, 3, 1), 3);
  assert.equal(e.futures_ready_chunks_poll(0, 3, 2), 2);
  assert.equal(e.futures_ready_chunks_poll(2, 3, 2), 1);

  assert.equal(e.futures_sink_buffer_ready(0, 9, 1), 1);
  assert.equal(e.futures_sink_buffer_ready(0, 9, 0), 0);
  assert.equal(e.futures_sink_buffer_ready(4, 2, 0), 0);
  assert.equal(e.futures_sink_buffer_ready(4, 4, 1), 0);
  assert.equal(e.futures_sink_buffer_ready(4, 2, 1), 1);
  assert.equal(e.futures_sink_fanout_ready(1, 1), 1);
  assert.equal(e.futures_sink_fanout_ready(1, 0), 0);

  console.log(JSON.stringify({ unit: "futures-util-core-state", standard_id: e.proto_standard_id(), ok: true }));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
