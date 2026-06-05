#!/usr/bin/env node

const assert = require("assert");
const { execFileSync } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/protocol-primitives/protocol-suite-core.wat");
const wasm = path.join(os.tmpdir(), `protocol-suite-core-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "pipe" });

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = instance.exports;

  assert.strictEqual(e.protocol_suite_abi_version(), 1);
  assert.strictEqual(e.proxy_http_request_kind(1, 1, 1, 1, 1), 1);
  assert.strictEqual(e.proxy_http_request_kind(1, 1, 1, 1, 0), 2);
  assert.strictEqual(e.proxy_http_request_kind(0, 1, 1, 1, 0), -2);
  assert.strictEqual(e.proxy_socks5_greeting_result(3, 5, 1, 1), 0);
  assert.strictEqual(e.proxy_socks5_greeting_result(2, 5, 1, 1), 1);
  assert.strictEqual(e.proxy_socks5_greeting_result(3, 5, 2, 1), 2);
  assert.strictEqual(e.proxy_socks5_greeting_result(3, 5, 1, 0), 3);
  assert.strictEqual(e.proxy_socks5_request_result(10, 5, 1, 0, 1, 0), 0);
  assert.strictEqual(e.proxy_socks5_request_result(8, 5, 1, 0, 3, 2), 3);
  assert.strictEqual(e.proxy_socks5_request_result(22, 5, 1, 0, 4, 0), 4);
  assert.strictEqual(e.pkce_verifier_result(43), 0);
  assert.strictEqual(e.pkce_verifier_result(129), 1);

  assert.strictEqual(e.bootstrap_genesis_result(1, 1, 1, 0n, 1, 1), 0);
  assert.strictEqual(e.bootstrap_genesis_result(1, 0, 1, 0n, 1, 1), 2);
  assert.strictEqual(e.bootstrap_genesis_result(1, 1, 1, 1n, 1, 1), 3);
  assert.strictEqual(e.ethernet_ipv4_packet_result(34, 0x0800), 0);
  assert.strictEqual(e.ethernet_ipv4_packet_result(33, 0x0800), 1);
  assert.strictEqual(e.ethernet_ipv4_packet_result(34, 0x0806), 2);
  assert.strictEqual(e.ipv4_is_private(10, 0), 1);
  assert.strictEqual(e.ipv4_is_private(172, 31), 1);
  assert.strictEqual(e.ipv4_is_private(172, 32), 0);
  assert.strictEqual(e.ipv4_route_uses_gateway(192, 192), 0);
  assert.strictEqual(e.ipv4_route_uses_gateway(8, 192), 1);
  assert.strictEqual(e.udp_packet_len_result(1472), 0);
  assert.strictEqual(e.udp_packet_len_result(1473), 1);

  assert.strictEqual(e.dbus_consume_type_result("s".charCodeAt(0), 1, 1), 0);
  assert.strictEqual(e.dbus_consume_type_result("(".charCodeAt(0), 2, 0), 2);
  assert.strictEqual(e.dbus_decode_message_result(16, "l".charCodeAt(0), 1, 0, 0), 0);
  assert.strictEqual(e.dbus_decode_message_result(15, "l".charCodeAt(0), 1, 0, 0), 1);
  assert.strictEqual(e.dbus_decode_message_result(16, "B".charCodeAt(0), 1, 0, 0), 2);
  assert.strictEqual(e.dbus_decode_message_result(16, "l".charCodeAt(0), 5, 0, 0), 3);

  assert.strictEqual(e.goodix_packet_result(12, 1, 4, 1), 0);
  assert.strictEqual(e.goodix_packet_result(11, 1, 4, 1), 1);
  assert.strictEqual(e.goodix_packet_result(12, 0, 4, 1), 2);
  assert.strictEqual(e.goodix_packet_result(12, 1, 3, 1), 4);
  assert.strictEqual(e.goodix_ack_result(170, 2), 0);
  assert.strictEqual(e.goodix_ack_result(1, 2), 1);
  assert.strictEqual(e.goodix_template_result(71, 67, 2), 0);
  assert.strictEqual(e.goodix_template_result(71, 66, 2), 2);
  assert.strictEqual(e.goodix_result_success(127), 1);
  assert.strictEqual(e.goodix_result_success(128), 0);
  assert.strictEqual(e.goodix_finger_mode_code(0), 1);
  assert.strictEqual(e.goodix_finger_mode_code(199), 2);
  assert.strictEqual(e.goodix_finger_list_result(2, 0, 20), 0);
  assert.strictEqual(e.goodix_finger_list_result(2, 128, 1), 2);

  assert.strictEqual(e.wifi_ap_config_result(1), 0);
  assert.strictEqual(e.wifi_ap_config_result(0), 1);
  assert.strictEqual(e.wifi_ap_config_result(33), 2);
  assert.strictEqual(e.wifi_open_ap_action(24, 0, 4, 1, 1, 1, 1, 1, 0, 1), 1);
  assert.strictEqual(e.wifi_open_ap_action(24, 0, 11, 1, 1, 1, 1, 1, 0, 1), 2);
  assert.strictEqual(e.wifi_open_ap_action(24, 0, 0, 1, 1, 1, 1, 1, 0, 1), 3);
  assert.strictEqual(e.wifi_open_ap_action(32, 2, 0, 1, 1, 1, 1, 1, 0, 1), 4);
  assert.strictEqual(e.wifi_open_ap_action(32, 2, 0, 1, 1, 1, 0, 1, 0, 1), -2);
  assert.strictEqual(e.wifi_next_seq(4095), 0);

  console.log("protocol suite core smoke passed");
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
