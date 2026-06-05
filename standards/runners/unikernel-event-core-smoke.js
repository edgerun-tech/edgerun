#!/usr/bin/env node
const assert = require("node:assert/strict");
const { readFileSync } = require("node:fs");
const { spawnSync } = require("node:child_process");

const wat = "standards/build/wasm/device-primitives/unikernel-event-core.wat";
const wasm = "/tmp/unikernel-event-core.wasm";

const compiled = spawnSync("wat2wasm", [wat, "-o", wasm], { encoding: "utf8" });
assert.equal(compiled.status, 0, compiled.stderr);

const moduleBytes = readFileSync(wasm);
const instance = new WebAssembly.Instance(new WebAssembly.Module(moduleBytes), {});
const e = instance.exports;

assert.equal(e.unikernel_abi_version(), 1);
assert.equal(e.unikernel_event_max_data_len(), 4096);

assert.equal(e.unikernel_event_type_valid(0), 0);
assert.equal(e.unikernel_event_type_valid(1), 1);
assert.equal(e.unikernel_event_type_valid(3), 1);
assert.equal(e.unikernel_event_type_valid(4), 0);
assert.equal(e.unikernel_network_subtype_valid(4), 1);
assert.equal(e.unikernel_network_subtype_valid(5), 0);
assert.equal(e.unikernel_disk_subtype_valid(3), 1);
assert.equal(e.unikernel_disk_subtype_valid(4), 0);
assert.equal(e.unikernel_timer_subtype_code(255), 1);

assert.equal(e.unikernel_event_new_result(1, 4096), 0);
assert.equal(e.unikernel_event_new_result(9, 1), 1);
assert.equal(e.unikernel_event_new_result(1, 4097), 2);
assert.equal(e.unikernel_network_event_len(1, 99), 5);
assert.equal(e.unikernel_network_event_len(3, 0), 9);
assert.equal(e.unikernel_network_event_len(3, 4087), 4096);
assert.equal(e.unikernel_network_event_len(3, 4088), 0);
assert.equal(e.unikernel_disk_event_len(1, 4091), 4096);
assert.equal(e.unikernel_disk_event_len(1, 4092), 0);
assert.equal(e.unikernel_disk_event_len(2, 99), 5);
assert.equal(e.unikernel_timer_event_len(), 9);
assert.equal(e.unikernel_total_len(5), 6);

assert.equal(e.unikernel_from_bytes_result(0, 1), 1);
assert.equal(e.unikernel_from_bytes_result(1, 9), 2);
assert.equal(e.unikernel_from_bytes_result(4098, 1), 3);
assert.equal(e.unikernel_from_bytes_result(4097, 1), 0);

assert.equal(e.unikernel_sock_id_available(1, 4), 1);
assert.equal(e.unikernel_sock_id_available(2, 4), 0);
assert.equal(e.unikernel_network_payload_result(1, 12, 3, 3), 0);
assert.equal(e.unikernel_network_payload_result(2, 12, 3, 3), 1);
assert.equal(e.unikernel_network_payload_result(1, 8, 3, 0), 2);
assert.equal(e.unikernel_network_payload_result(1, 11, 3, 3), 3);
assert.equal(e.unikernel_disk_data_result(2, 5, 1), 0);
assert.equal(e.unikernel_disk_data_result(1, 5, 1), 1);
assert.equal(e.unikernel_disk_data_result(2, 4, 1), 2);
assert.equal(e.unikernel_timer_id_available(3, 9), 1);
assert.equal(e.unikernel_timer_id_available(3, 8), 0);

assert.equal(e.unikernel_queue_push_result(0, 2, 1, 0), 0);
assert.equal(e.unikernel_queue_push_result(2, 2, 1, 0), 1);
assert.equal(e.unikernel_queue_push_result(0, 2, 7, 0), 2);
assert.equal(e.unikernel_queue_push_result(0, 2, 1, 4097), 3);
assert.equal(e.unikernel_queue_len_after_push(1, 2, 1), 2);
assert.equal(e.unikernel_queue_len_after_push(2, 2, 1), 2);
assert.equal(e.unikernel_queue_len_after_pop(2), 1);
assert.equal(e.unikernel_queue_len_after_pop(0), 0);
assert.equal(e.unikernel_queue_next_index(31, 32), 0);
assert.equal(e.unikernel_queue_clear_len(), 0);

assert.equal(e.unikernel_virtio_rx_result(0, 0, 1), 0);
assert.equal(e.unikernel_virtio_rx_result(0, 1, 1), 1);
assert.equal(e.unikernel_virtio_rx_result(3, 0, 1), 2);
assert.equal(e.unikernel_virtio_rx_result(3, 1, 1), 3);
assert.equal(e.unikernel_virtio_rx_result(3, 1, 0), 4);
assert.equal(e.unikernel_virtio_pending_after_rx(0, 1), 0);
assert.equal(e.unikernel_virtio_pending_after_rx(3, 0), 1);
assert.equal(e.unikernel_virtio_sock_after_rx(3, 0, 41, 7), 42);
assert.equal(e.unikernel_virtio_sock_after_rx(3, 1, 41, 7), 7);
assert.equal(e.unikernel_virtio_record_error_result(1), 1);
assert.equal(e.unikernel_virtio_record_error_result(0), 0);
assert.equal(e.unikernel_poll_many_count(3, 5), 3);
assert.equal(e.unikernel_poll_many_count(7, 5), 5);

assert.equal(e.unikernel_linker_script_code(0), 1);
assert.equal(e.unikernel_linker_script_code(1), 2);
assert.equal(e.unikernel_xtensa_wifi_blob_link_items(1), 9);
assert.equal(e.unikernel_xtensa_ble_blob_link_items(1), 12);
assert.equal(e.unikernel_build_emit_count(0, 1, 1, 1), 0);
assert.equal(e.unikernel_build_emit_count(1, 0, 1, 1), 2);
assert.equal(e.unikernel_build_emit_count(1, 1, 0, 0), 7);
assert.equal(e.unikernel_build_emit_count(1, 1, 1, 1), 28);

console.log("unikernel-event-core smoke passed");
