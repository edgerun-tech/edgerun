#!/usr/bin/env node

const assert = require("assert");
const { execFileSync } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/authority-primitives/authority-storage-core.wat");
const wasm = path.join(os.tmpdir(), `authority-storage-core-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "pipe" });

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = instance.exports;
  const mem = new Uint8Array(e.memory.buffer);
  const put = (s, ptr = 1024) => {
    mem.fill(0, ptr, ptr + 256);
    mem.set(Buffer.from(s, "utf8"), ptr);
    return [ptr, Buffer.byteLength(s)];
  };
  const callPath = (s) => e.authority_vfs_path_result(...put(s));
  const fmt = (s) => e.authority_disk_format_from_path(...put(s));

  assert.strictEqual(e.authority_vfs_wire_abi_version(), 1);
  assert.strictEqual(e.authority_vfs_default_object_packet_bytes(), 65536);
  assert.strictEqual(e.authority_vfs_compression_none(), 0);
  assert.strictEqual(e.authority_vfs_compression_deflate_raw(), 1);
  assert.strictEqual(e.authority_vfs_seal_aes256_gcm(), 1);
  assert.strictEqual(e.authority_vfs_wire_record_tag(5), 5);
  assert.strictEqual(e.authority_vfs_wire_record_tag(6), -1);

  assert.strictEqual(e.authority_vfs_packet_count(0, 16), 1);
  assert.strictEqual(e.authority_vfs_packet_count(33, 16), 3);
  assert.strictEqual(e.authority_vfs_packet_count(33, 0), -1);
  assert.strictEqual(e.authority_vfs_packet_offset(3, 16), 48);
  assert.strictEqual(e.authority_vfs_packet_shape_result(1, 33, 0, 3, 0, 16), 0);
  assert.strictEqual(e.authority_vfs_packet_shape_result(1, 33, 1, 3, 16, 0), 1);
  assert.strictEqual(e.authority_vfs_packet_shape_result(1, 33, 3, 3, 48, 1), 2);
  assert.strictEqual(e.authority_vfs_packet_shape_result(1, 33, 2, 3, 32, 1), 0);

  assert.strictEqual(e.authority_vfs_transform_result(1, 0, 1, 1), 0);
  assert.strictEqual(e.authority_vfs_transform_result(1, 1, 1, 1), 0);
  assert.strictEqual(e.authority_vfs_transform_result(1, 2, 1, 1), 1);
  assert.strictEqual(e.authority_vfs_file_ref_result(1, 1, 1, 1, 1), 0);
  assert.strictEqual(e.authority_vfs_file_ref_result(1, 0, 1, 1, 1), 2);
  assert.strictEqual(e.authority_vfs_manifest_order(-1), 0);
  assert.strictEqual(e.authority_vfs_manifest_order(1), 1);
  assert.strictEqual(e.authority_vfs_seal_request_payload_result(0, 0), 0);
  assert.strictEqual(e.authority_vfs_seal_request_payload_result(1, 0), 1);
  assert.strictEqual(e.authority_vfs_seal_request_payload_result(1, 1), 0);
  assert.strictEqual(e.authority_vfs_memory_write_usage(100, 10, 30), 120);
  assert.strictEqual(e.authority_vfs_text_edit_result(1, 0, 1), 0);
  assert.strictEqual(e.authority_vfs_text_edit_result(1, 1, 1), 1);
  assert.strictEqual(e.authority_vfs_text_edit_result(0, 0, 1), 2);
  assert.strictEqual(e.authority_vfs_text_edit_result(1, 0, 0), 3);

  assert.strictEqual(callPath("src/lib.rs"), 0);
  assert.strictEqual(callPath("/src/lib.rs/"), 0);
  assert.strictEqual(callPath("../x"), 1);
  assert.strictEqual(callPath("a//b"), 1);
  assert.strictEqual(callPath("a\\b"), 1);
  assert.strictEqual(callPath("."), 1);

  assert.strictEqual(fmt("disk.raw"), 1);
  assert.strictEqual(fmt("disk.qcow2"), 2);
  assert.strictEqual(fmt("disk.QCOW2"), 2);
  assert.strictEqual(fmt("disk.vhd"), 3);
  assert.strictEqual(fmt("disk.vhdx"), 4);
  assert.strictEqual(fmt("disk.img"), 1);
  assert.strictEqual(e.authority_disk_format_requires_qemu(1), 0);
  assert.strictEqual(e.authority_disk_format_requires_qemu(2), 1);
  assert.strictEqual(e.authority_disk_validate_spec_result(8, 4096, 1, 0), 0);
  assert.strictEqual(e.authority_disk_validate_spec_result(8, 4096, 2, 0), 2);
  assert.strictEqual(e.authority_disk_validate_spec_result(0, 4096, 1, 0), 1);
  assert.strictEqual(e.authority_disk_qemu_size_unit(1073741824), 3);
  assert.strictEqual(e.authority_disk_qemu_size_unit(1048576), 2);
  assert.strictEqual(e.authority_disk_qemu_size_unit(1024), 1);
  assert.strictEqual(e.authority_disk_qemu_size_unit(1000), 0);

  assert.strictEqual(e.authority_block_total_size(512, 8), 4096n);
  assert.strictEqual(e.authority_block_transfer_result(512, 8, 0, 1, 0, 1, 512), 0);
  assert.strictEqual(e.authority_block_transfer_result(512, 8, 1, 2, 0, 1, 512), 2);
  assert.strictEqual(e.authority_block_transfer_result(512, 8, 0, 1, 7, 2, 1024), 3);
  assert.strictEqual(e.authority_block_transfer_result(512, 8, 0, 1, 0, 2, 512), 4);
  assert.strictEqual(e.authority_block_next_request_id(7), 8);
  assert.strictEqual(e.authority_block_next_request_id(-1), -1);
  assert.strictEqual(e.authority_file_backend_open_result(1, 1, 512, 4096), 0);
  assert.strictEqual(e.authority_file_backend_open_result(1, 1, 512, 4097), 2);

  assert.strictEqual(e.authority_nbd_ioctl_code(1), 0xab00);
  assert.strictEqual(e.authority_nbd_ioctl_code(8), 0xab0a);
  assert.strictEqual(e.authority_nbd_attach_result(4096, 512, 0, 0), 0);
  assert.strictEqual(e.authority_nbd_attach_result(4097, 512, 0, 0), 1);
  assert.strictEqual(e.authority_nbd_attach_result(4096, 512, 1, 0), 2);
  assert.strictEqual(e.authority_nbd_len(1), 18);
  assert.strictEqual(e.authority_nbd_len(2), 4);
  assert.strictEqual(e.authority_nbd_len(3), 16);
  assert.strictEqual(e.authority_nbd_len(4), 134);
  assert.strictEqual(e.authority_nbd_len(5), 28);
  assert.strictEqual(e.authority_nbd_len(6), 16);
  assert.strictEqual(e.authority_nbd_len(7), 20);

  console.log("authority storage core smoke passed");
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
