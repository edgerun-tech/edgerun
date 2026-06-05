#!/usr/bin/env node

const assert = require("assert");
const { execFileSync } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/agent-primitives/node-surfaces-core.wat");
const wasm = path.join(os.tmpdir(), `node-surfaces-core-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "pipe" });

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = instance.exports;

  assert.strictEqual(e.node_surfaces_abi_version(), 1);
  assert.strictEqual(e.frontend_work_projection_schema_version(), 1);
  assert.strictEqual(e.frontend_input_buffer_capacity(), 4096);
  assert.strictEqual(e.frontend_frame_active(0n), 1);
  assert.strictEqual(e.frontend_frame_active(800n), 0);
  assert.strictEqual(e.frontend_action_dirty(3), 1);
  assert.strictEqual(e.frontend_action_dirty(9), 0);
  assert.strictEqual(e.frontend_pointer_click_dirty(0, 1), 1);
  assert.strictEqual(e.frontend_input_len(5000), 4096);
  assert.strictEqual(e.frontend_hit_code(0, 1, 7) >>> 0, 0xffffffff);
  assert.strictEqual(e.frontend_hit_code(1, 2, 0x123456), 0x02123456);
  assert.strictEqual(e.web_key_code(8), 1);
  assert.strictEqual(e.web_key_code(40), 12);
  assert.strictEqual(e.web_key_code(65), 1065);
  assert.strictEqual(e.native_arg_result(1, 0, 1), 1);
  assert.strictEqual(e.native_arg_result(3, 1, 0), 2);
  assert.strictEqual(e.native_arg_result(4, 0, 0), 3);
  assert.strictEqual(e.scheme_code(2), 2);
  assert.strictEqual(e.scheme_code(4), 0);

  assert.strictEqual(e.ws_masked_client_frame_header_len(10n), 6);
  assert.strictEqual(e.ws_masked_client_frame_header_len(126n), 8);
  assert.strictEqual(e.ws_masked_client_frame_header_len(70000n), 14);
  assert.strictEqual(e.ws_server_binary_result(2, 0), 0);
  assert.strictEqual(e.ws_server_binary_result(1, 0), 1);
  assert.strictEqual(e.ws_server_binary_result(2, 1), 2);
  assert.strictEqual(e.ws_handshake_result(0, 1024, 1, 1), 0);
  assert.strictEqual(e.ws_handshake_result(0, 4097, 1, 1), 2);
  assert.strictEqual(e.ws_handshake_result(0, 1024, 0, 1), 3);
  assert.strictEqual(e.ws_handshake_result(0, 1024, 1, 0), 4);

  assert.strictEqual(e.authority_command_decision(1, 0), 1);
  assert.strictEqual(e.authority_command_decision(0, 0), 2);
  assert.strictEqual(e.authority_reason_code(1, 1), 2);
  assert.strictEqual(e.authority_stream_len_after_event(2), 3);
  assert.strictEqual(e.sdk_runtime_projection_result(1, 1, 1, 1), 0);
  assert.strictEqual(e.sdk_runtime_projection_result(1, 1, 0, 1), 3);

  console.log("node surfaces core smoke passed");
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
