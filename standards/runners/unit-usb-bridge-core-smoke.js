#!/usr/bin/env node
const assert = require("node:assert/strict");
const { readFileSync } = require("node:fs");
const { spawnSync } = require("node:child_process");

const wat = "standards/build/wasm/device-primitives/unit-usb-bridge-core.wat";
const wasm = "/tmp/unit-usb-bridge-core.wasm";

const compiled = spawnSync("wat2wasm", [wat, "-o", wasm], { encoding: "utf8" });
assert.equal(compiled.status, 0, compiled.stderr);

const instance = new WebAssembly.Instance(new WebAssembly.Module(readFileSync(wasm)), {});
const e = instance.exports;

assert.equal(e.edgerun_unit_abi_version(), 2);
assert.equal(e.edgerun_unit_scalar_abi_type_valid(1), 1);
assert.equal(e.edgerun_unit_scalar_abi_type_valid(4), 1);
assert.equal(e.edgerun_unit_scalar_abi_type_valid(5), 0);
assert.equal(e.edgerun_unit_pointer_is_memory_offset(), 1);
assert.equal(e.edgerun_unit_imports_host_memory_allowed(), 0);
assert.equal(e.edgerun_unit_no_alloc_result(), 0);
assert.equal(e.edgerun_unit_panic_strategy(), 1);

assert.equal(e.usb_ppp_ap_default_channel(), 6);
assert.equal(e.usb_ppp_ap_max_connections(), 4);
assert.equal(e.usb_ppp_ap_status_bit(0, 0), 0);
assert.equal(e.usb_ppp_ap_status_bit(1, 0), 1);
assert.equal(e.usb_ppp_ap_status_bit(0, 1), 2);
assert.equal(e.usb_ppp_ap_status_bit(1, 1), 3);

assert.equal(e.usb_ppp_ap_start_result(0, 1), 1);
assert.equal(e.usb_ppp_ap_start_result(7, 0), 2);
assert.equal(e.usb_ppp_ap_start_result(7, 1), 0);
assert.equal(e.usb_ppp_ap_password_auth_mode(0), 0);
assert.equal(e.usb_ppp_ap_password_auth_mode(12), 2);
assert.equal(e.usb_ppp_ap_effective_channel(0), 6);
assert.equal(e.usb_ppp_ap_effective_channel(11), 11);
assert.equal(e.usb_ppp_ap_napt_result_ok(0), 1);
assert.equal(e.usb_ppp_ap_napt_result_ok(259), 1);
assert.equal(e.usb_ppp_ap_napt_result_ok(1), 0);

assert.equal(e.usb_ppp_ap_ip_event_bits_after(0, 0), 2);
assert.equal(e.usb_ppp_ap_ip_event_bits_after(1, 3), 1);
assert.equal(e.usb_ppp_ap_ip_event_bits_after(9, 3), 3);
assert.equal(e.usb_ppp_ap_wifi_event_bits_after(0, 0), 1);
assert.equal(e.usb_ppp_ap_wifi_event_bits_after(5, 0), 0);
assert.equal(e.usb_ppp_ap_cdc_rx_should_receive(1, 1, 8, 1), 1);
assert.equal(e.usb_ppp_ap_cdc_rx_should_receive(1, 1, 0, 1), 0);
assert.equal(e.usb_ppp_ap_cdc_rx_should_receive(0, 1, 8, 1), 0);
assert.equal(e.usb_ppp_ap_nvs_init_action(4354), 1);
assert.equal(e.usb_ppp_ap_nvs_init_action(4355), 1);
assert.equal(e.usb_ppp_ap_nvs_init_action(0), 0);

assert.equal(e.usb_ppp_ap_main_stack_size(), 8192);
assert.equal(e.usb_ppp_ap_event_stack_size(), 4096);
assert.equal(e.usb_ppp_ap_cdc_rx_bufsize(), 1024);
assert.equal(e.usb_ppp_ap_cdc_tx_bufsize(), 1024);
assert.equal(e.usb_ppp_ap_ppp_ipv4_enabled(), 1);
assert.equal(e.usb_ppp_ap_ipv4_napt_enabled(), 1);
assert.equal(e.usb_ppp_ap_component_version_code(1), 20101);
assert.equal(e.usb_ppp_ap_component_version_code(2), 19003);
assert.equal(e.usb_ppp_ap_component_version_code(3), 50503);
assert.equal(e.usb_ppp_ap_component_version_code(9), 0);
assert.equal(e.usb_ppp_ap_component_target_supported(2), 1);
assert.equal(e.usb_ppp_ap_component_target_supported(3), 1);
assert.equal(e.usb_ppp_ap_component_target_supported(5), 1);
assert.equal(e.usb_ppp_ap_component_target_supported(9), 0);
assert.equal(e.usb_ppp_ap_main_loop_sleep_secs(), 5);

console.log("unit-usb-bridge-core smoke passed");
