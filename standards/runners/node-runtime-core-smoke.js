#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/node-runtime-core.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  const wasm = execFileSync("wat2wasm", [file, "-o", "-"]);
  execFileSync("wasm-validate", ["-"], { input: wasm });
  return wasm;
}

function writeAscii(memory, value, ptr) {
  memory.fill(0, ptr, ptr + Math.max(128, value.length + 1));
  memory.set(Buffer.from(value, "ascii"), ptr);
  return ptr;
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const e = instance.exports;
  const memory = new Uint8Array(e.memory.buffer);
  const str = (fn, value) => {
    const ptr = writeAscii(memory, value, 4096);
    return e[fn](ptr, value.length);
  };
  const route = (fn, method, pathValue) => {
    const m = writeAscii(memory, method, 4096);
    const p = writeAscii(memory, pathValue, 5120);
    return e[fn](m, method.length, p, pathValue.length);
  };

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300115);
  assert.equal(str("node_transport_protocol_code", "stream"), 1);
  assert.equal(str("node_transport_protocol_code", "datagram"), 2);
  assert.equal(str("node_transport_carrier_code", "hostSocket"), 1);
  assert.equal(str("node_transport_carrier_code", "browserIpc"), 5);
  assert.equal(str("node_cli_command_code", "init-provisioned"), 3);
  assert.equal(str("node_cli_command_code", "xray-server"), 8);
  assert.equal(e.node_binding_decision(1, 1, 80), 1);
  assert.equal(e.node_binding_decision(1, 1, 0), 3);
  assert.equal(e.node_binding_decision(2, 99, 0), 2);
  assert.equal(e.node_binding_decision(5, 1, 80), 3);
  assert.equal(e.node_binding_transport_protocol(3), 2);
  assert.equal(e.node_binding_transport_protocol(10), 2);
  assert.equal(e.node_bind_check_port(0, 1), 18080);
  assert.equal(e.node_bind_check_port(1, 2), 443);
  assert.equal(e.node_bind_check_port(0, 4), 11067);
  assert.equal(e.node_quic_send_result(1, 0, 1), 1);
  assert.equal(e.node_quic_send_result(2, 0, 1), 2);
  assert.equal(e.node_quic_send_result(1, 1, 1), 3);
  assert.equal(Number(e.node_quic_next_stream_id(3)), 12);
  assert.equal(e.node_quic_accept_result(1, 1, 0, 1), 1);
  assert.equal(e.node_quic_accept_result(0, 1, 0, 1), 2);
  assert.equal(e.node_quic_accept_result(1, 0, 0, 1), 4);
  assert.equal(e.node_quic_accept_result(1, 1, 1, 1), 3);
  assert.equal(e.node_mesh_tick_action(1, 1, 1, 0), 1);
  assert.equal(e.node_mesh_tick_action(1, 1, 0, 0), 2);
  assert.equal(e.node_mesh_tick_action(0, 0, 0, 1), 3);
  assert.equal(route("node_wasm_route_code", "GET", "/health"), 1);
  assert.equal(route("node_wasm_route_code", "GET", "/protocol/node/status"), 2);
  assert.equal(route("node_wasm_route_code", "POST", "/protocol/tools/invoke"), 7);
  assert.equal(route("node_wasm_route_code", "POST", "/protocol/approvals/a1/approve"), 9);
  assert.equal(route("node_wasm_route_code", "GET", "/protocol/approvals/a1/reject"), 405);
  assert.equal(route("node_xray_route_code", "OPTIONS", "/anything"), 1);
  assert.equal(route("node_xray_route_code", "GET", "/connections"), 3);
  assert.equal(route("node_xray_route_code", "GET", "/graph"), 4);
  assert.equal(str("node_xray_tcp_state_code", "01"), 1);
  assert.equal(str("node_xray_tcp_state_code", "0A"), 10);
  assert.equal(str("node_service_lifecycle_code", "requested"), 1);
  assert.equal(str("node_service_lifecycle_code", "shutdown_requested"), 6);
  assert.equal(str("node_binding_decision_code", "native_socket"), 1);
  assert.equal(str("node_binding_decision_code", "unavailable"), 7);
  assert.equal(str("node_command_verdict_code", "duplicate"), 4);
  assert.equal(str("node_command_verdict_code", "rejected"), 6);
  assert.equal(str("node_command_reason_code", "duplicate_command"), 1);
  assert.equal(str("node_command_reason_code", "PLAINTEXT_PAYLOAD_REJECTED"), 4);
  assert.equal(str("node_command_reason_code", "cannot_remove_last_controller"), 7);
  assert.equal(str("node_command_reason_code", "unsupported_command_type"), 12);
  assert.equal(str("node_init_mode_code", "software"), 1);
  assert.equal(str("node_init_mode_code", "provisioned"), 5);

  console.log(JSON.stringify({ unit: "node-runtime-core", standard_id: e.proto_standard_id(), ok: true }));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
