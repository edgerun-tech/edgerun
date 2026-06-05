#!/usr/bin/env node

const assert = require("assert");
const { execFileSync } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/device-primitives/virtio-core.wat");
const wasm = path.join(os.tmpdir(), `virtio-core-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "pipe" });

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = instance.exports;

  assert.strictEqual(e.virtio_modern_device_type(0x1041), 1);
  assert.strictEqual(e.virtio_modern_device_type(0x1042), 2);
  assert.strictEqual(e.virtio_modern_device_type(0x1043), 3);
  assert.strictEqual(e.virtio_modern_device_type(0x1044), 4);
  assert.strictEqual(e.virtio_modern_device_type(0x1045), 0);
  assert.strictEqual(e.virtio_interrupt_flags(3), 3);
  assert.strictEqual(e.virtio_mmio_valid(1, 0x74726976, 2), 1);
  assert.strictEqual(e.virtio_mmio_valid(1, 0, 2), 0);
  assert.strictEqual(e.virtio_mmio_device_info_valid(1, 4), 1);
  assert.strictEqual(e.virtio_mmio_device_info_valid(1, 0), 0);
  assert.strictEqual(e.virtio_open_device_result(1, 2, 1, 0), 1);
  assert.strictEqual(e.virtio_open_device_result(1, 1, 0, 0), 2);
  assert.strictEqual(e.virtio_open_device_result(1, 1, 1, 1), 3);
  assert.strictEqual(e.virtio_open_device_result(1, 1, 1, 0), 0);
  assert.strictEqual(e.virtio_modern_pci_transport_valid(1, 1), 1);
  assert.strictEqual(e.virtio_modern_pci_transport_valid(1, 0), 0);

  assert.strictEqual(e.virtio_negotiate_features(0, 1, 0, 1, 1), 11);
  assert.strictEqual(e.virtio_negotiate_features(0, 0, 0, 1, 1), 128);
  assert.strictEqual(e.virtio_negotiate_features(0, 1, 0, 1, 0), 139);
  assert.strictEqual(e.virtio_driver_feature_low(0b1010, 0b1100), 0b1000);
  assert.strictEqual(e.virtio_driver_feature_high(0b1010, 0b0011), 0b0010);

  assert.strictEqual(e.virtio_queue_configured_size(64, 16, 16), 16);
  assert.strictEqual(e.virtio_queue_configured_size(8, 16, 3), 8);
  assert.strictEqual(e.virtio_queue_configured_size(2, 16, 3), 0);
  assert.strictEqual(e.virtio_post_avail_next_idx(16, 0xffff), 0);
  assert.strictEqual(e.virtio_post_avail_next_idx(0, 7), 7);
  assert.strictEqual(e.virtio_post_avail_ring_slot(16, 18), 2);
  assert.strictEqual(e.virtio_post_avail_ring_slot(0, 18), -1);
  assert.strictEqual(e.virtio_next_used_result(3, 2), 0x10003);
  assert.strictEqual(e.virtio_next_used_result(3, 3), 0);
  assert.strictEqual(e.virtio_single_used_result(1, 0), 1);
  assert.strictEqual(e.virtio_single_used_result(3, 0), 2);
  assert.strictEqual(e.virtio_wait_completion_result(1, 0, 0), 1);
  assert.strictEqual(e.virtio_wait_completion_result(0, 0, 5001), 2);

  assert.strictEqual(e.virtio_net_tx_frame_len(0), 0);
  assert.strictEqual(e.virtio_net_tx_frame_len(1), 13);
  assert.strictEqual(e.virtio_net_tx_frame_len(2036), 2048);
  assert.strictEqual(e.virtio_net_tx_frame_len(2037), 0);
  assert.strictEqual(e.virtio_net_rx_payload_len(11), -1);
  assert.strictEqual(e.virtio_net_rx_payload_len(12), 0);
  assert.strictEqual(e.virtio_net_rx_payload_len(2048), 2036);
  assert.strictEqual(e.virtio_tx_take_descriptor(0), -1);
  assert.strictEqual(e.virtio_tx_take_descriptor(0b1000), 3);
  assert.strictEqual(e.virtio_tx_free_after_reap(0, 4), 16);
  assert.strictEqual(e.virtio_tx_free_after_reap(0, 20), 0);

  assert.strictEqual(e.virtio_blk_chunk_sector_count(0), 0);
  assert.strictEqual(e.virtio_blk_chunk_sector_count(512), 1);
  assert.strictEqual(e.virtio_blk_chunk_sector_count(1024), 0);
  assert.strictEqual(e.virtio_blk_range_in_bounds(3n, 1n, 4n), 1);
  assert.strictEqual(e.virtio_blk_range_in_bounds(3n, 2n, 4n), 0);
  assert.strictEqual(e.virtio_blk_request_valid(0, 0, 0, 512, 1), 1);
  assert.strictEqual(e.virtio_blk_request_valid(1, 0, 0, 511, 1), 2);
  assert.strictEqual(e.virtio_blk_request_valid(1, 0, 0, 512, 0), 3);
  assert.strictEqual(e.virtio_blk_request_valid(1, 1, 1, 512, 1), 4);
  assert.strictEqual(e.virtio_blk_request_valid(1, 0, 1, 512, 1), 0);
  assert.strictEqual(e.virtio_blk_descriptor_chain(0, 0), (2 << 16) | 2);
  assert.strictEqual(e.virtio_blk_descriptor_chain(512, 0), 1 | (1 << 8) | (2 << 16));
  assert.strictEqual(e.virtio_blk_descriptor_chain(512, 1), 1 | (3 << 8) | (2 << 16));
  assert.strictEqual(e.virtio_blk_init_result(0, 1, 512, 3), 1);
  assert.strictEqual(e.virtio_blk_init_result(1, 0, 512, 3), 2);
  assert.strictEqual(e.virtio_blk_init_result(1, 1, 1024, 3), 3);
  assert.strictEqual(e.virtio_blk_init_result(1, 1, 512, 2), 4);
  assert.strictEqual(e.virtio_blk_init_result(1, 1, 512, 3), 0);

  assert.strictEqual(e.virtio_rng_request_len(300), 256);
  assert.strictEqual(e.virtio_rng_completion_valid(0, 128, 256), 128);
  assert.strictEqual(e.virtio_rng_completion_valid(1, 128, 256), -1);
  assert.strictEqual(e.virtio_rng_completion_valid(0, 0, 256), -1);
  assert.strictEqual(e.virtio_console_rx_len(0), 0);
  assert.strictEqual(e.virtio_console_rx_len(256), 256);
  assert.strictEqual(e.virtio_console_rx_len(257), -1);
  assert.strictEqual(e.virtio_console_write_chunk_len(300), 256);
  assert.strictEqual(e.virtio_console_tx_completion_result(0, 12), 12);
  assert.strictEqual(e.virtio_console_tx_completion_result(2, 12), -1);

  assert.strictEqual(e.virtio_pci_address(1, 2, 3, 0x10), 0x80011310 | 0);
  assert.strictEqual(e.virtio_pci_address(1, 2, 3, 0x13), 0x80011310 | 0);
  assert.strictEqual(e.virtio_pci_bar_base_valid(0), 0);
  assert.strictEqual(e.virtio_pci_bar_base_valid(-1), 0);
  assert.strictEqual(e.virtio_pci_bar_base_valid(1), 0);
  assert.strictEqual(e.virtio_pci_bar_base_valid(0x1000), 1);
  assert.strictEqual(e.virtio_scan_count_next(2, 0x1af4, 0x1044), 3);
  assert.strictEqual(e.virtio_scan_count_next(2, 0x1234, 0x1044), 2);

  console.log("virtio core smoke passed");
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
