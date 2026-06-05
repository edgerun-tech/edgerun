#!/usr/bin/env node

const assert = require("assert");
const { execFileSync } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/network-primitives/network-driver-core.wat");
const wasm = path.join(os.tmpdir(), `network-driver-core-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "pipe" });

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = instance.exports;

  assert.strictEqual(e.network_driver_abi_version(), 1);
  assert.strictEqual(e.ethernet_header_len(), 14);
  assert.strictEqual(e.ethernet_mtu(), 1500);
  assert.strictEqual(e.ethernet_frame_len(), 1514);
  assert.strictEqual(e.rtl8125_vendor_id(), 0x10ec);
  assert.strictEqual(e.rtl8125_device_id(), 0x8125);
  assert.strictEqual(e.frame_driver_kind_valid(5), 1);
  assert.strictEqual(e.frame_driver_kind_valid(6), 0);

  assert.strictEqual(e.in_memory_push_rx_result(64, 1514, 0, 4), 0);
  assert.strictEqual(e.in_memory_push_rx_result(1515, 1514, 0, 4), 1);
  assert.strictEqual(e.in_memory_push_rx_result(64, 1514, 4, 4), 2);
  assert.strictEqual(e.in_memory_send_result(64, 1514, 0, 4), 0);
  assert.strictEqual(e.in_memory_recv_result(0, 64, 60), 0);
  assert.strictEqual(e.in_memory_recv_result(1, 32, 60), 2);
  assert.strictEqual(e.in_memory_recv_result(1, 64, 60), 1);
  assert.strictEqual(e.ring_next_index(31, 32), 0);

  assert.strictEqual(e.virtio_error_map(1), 1);
  assert.strictEqual(e.virtio_error_map(9), 7);
  assert.strictEqual(e.std_udp_send_result(0, 1), 4);
  assert.strictEqual(e.std_udp_send_result(1, 0), 7);
  assert.strictEqual(e.std_udp_recv_result(1), 1);
  assert.strictEqual(e.tap_open_result(1, 1), 0);
  assert.strictEqual(e.tap_open_result(1, 0), 7);

  assert.strictEqual(e.rtl_init_result(1, 1), 0);
  assert.strictEqual(e.rtl_init_result(0, 1), 1);
  assert.strictEqual(e.rtl_link_up(2), 1);
  assert.strictEqual(e.rtl_link_up(0), 0);
  assert.strictEqual(e.rtl_tx_available(31, 0), 1);
  assert.strictEqual(e.rtl_tx_available(32, 0), 0);
  assert.strictEqual(e.rtl_send_result(60, 1, 0), 0);
  assert.strictEqual(e.rtl_send_result(0, 1, 0), 1);
  assert.strictEqual(e.rtl_send_result(60, 0, 0), 2);
  assert.strictEqual(e.rtl_tx_opts1(0, 60), (0x80000000 | 0x20000000 | 0x10000000 | 60) | 0);
  assert.strictEqual(e.rtl_tx_opts1(31, 1514), (0x80000000 | 0x40000000 | 0x20000000 | 0x10000000 | 1514) | 0);
  assert.strictEqual(e.rtl_rx_owned_opts1(0), (0x80000000 | 2048) | 0);
  assert.strictEqual(e.rtl_rx_payload_len(64), 60);
  assert.strictEqual(e.rtl_rx_payload_len(2), 0);
  assert.strictEqual(e.rtl_recv_result(0x30000000 | 64, 1514), 1);
  assert.strictEqual(e.rtl_recv_result(0x80000000, 1514), 0);
  assert.strictEqual(e.rtl_recv_result(0x30000000 | 0x00100000 | 64, 1514), 2);
  assert.strictEqual(e.rtl_reap_tx_step(1, 0), 1);
  assert.strictEqual(e.rtl_reap_tx_step(1, 1), 0);

  assert.strictEqual(e.pci_config_io_available(1), 1);
  assert.strictEqual(e.pci_config_io_available(0), 0);
  assert.strictEqual(e.pci_address(1, 2, 3, 0x10) >>> 0, 0x80011310);
  assert.strictEqual(e.pci_memory_bar_result(0), 1);
  assert.strictEqual(e.pci_memory_bar_result(0xfffffff0), 0);
  assert.strictEqual(e.pci_memory_bar_result(0x0000c001), 2);
  assert.strictEqual(e.pci_bar_offset_step(4), 8);
  assert.strictEqual(e.pci_bar_offset_step(0), 4);

  console.log("network driver core smoke passed");
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
