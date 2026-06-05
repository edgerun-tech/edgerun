#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  "standards/build/wasm/codec-primitives/protocols-block-transfer.wat";

function compileWat(file) {
  const wasmPath = path.join(os.tmpdir(), `edgerun-${path.basename(file)}-${process.pid}.wasm`);
  execFileSync("wat2wasm", [file, "-o", wasmPath], { stdio: "pipe" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "pipe" });
  const bytes = fs.readFileSync(wasmPath);
  fs.unlinkSync(wasmPath);
  return bytes;
}

function put(memory, bytes, ptr = 1024) {
  const data = Buffer.isBuffer(bytes) ? bytes : Buffer.from(bytes, "ascii");
  memory.fill(0, ptr, ptr + data.length + 64);
  memory.set(data, ptr);
  return [ptr, data.length];
}

function u16be(n) {
  return Buffer.from([(n >>> 8) & 0xff, n & 0xff]);
}

function u32be(n) {
  return Buffer.from([(n >>> 24) & 0xff, (n >>> 16) & 0xff, (n >>> 8) & 0xff, n & 0xff]);
}

(async () => {
  const { instance } = await WebAssembly.instantiate(compileWat(watPath), {});
  const e = instance.exports;
  const memory = new Uint8Array(e.memory.buffer);

  assert.strictEqual(e.proto_abi_version(), 2);
  assert.strictEqual(e.proto_standard_id(), 300116);

  let [ptr, len] = put(memory, Buffer.concat([u16be(1), Buffer.from("boot.ipxe\0octet\0")]));
  assert.strictEqual(e.tftp_message_kind(ptr, len), 1);
  [ptr, len] = put(memory, Buffer.concat([u16be(3), u16be(7), Buffer.from("payload")]));
  assert.strictEqual(e.tftp_message_kind(ptr, len), 3);
  [ptr, len] = put(memory, Buffer.concat([u16be(4), u16be(7)]));
  assert.strictEqual(e.tftp_message_kind(ptr, len), 4);
  [ptr, len] = put(memory, Buffer.concat([u16be(99), u16be(0)]));
  assert.strictEqual(e.tftp_message_kind(ptr, len), 0);
  assert.strictEqual(e.tftp_error_class(5), 5);
  assert.strictEqual(e.tftp_error_class(99), -1);

  [ptr, len] = put(memory, "blksize");
  assert.strictEqual(e.tftp_option_code(ptr, len), 1);
  [ptr, len] = put(memory, "TSIZE");
  assert.strictEqual(e.tftp_option_code(ptr, len), 2);
  [ptr, len] = put(memory, "Timeout");
  assert.strictEqual(e.tftp_option_code(ptr, len), 3);
  [ptr, len] = put(memory, "window");
  assert.strictEqual(e.tftp_option_code(ptr, len), 0);
  assert.strictEqual(e.tftp_blksize_clamp(1), 8);
  assert.strictEqual(e.tftp_blksize_clamp(1456), 1456);
  assert.strictEqual(e.tftp_blksize_clamp(70000), 65464);

  assert.strictEqual(e.tftp_read_action(1, 1, 1, 0, 0, 0), 1);
  assert.strictEqual(e.tftp_read_action(1, 1, 1, 1, 0, 0), 2);
  assert.strictEqual(e.tftp_read_action(1, 1, 0, 0, 0, 0), 4);
  assert.strictEqual(e.tftp_read_action(1, 0, 1, 0, 0, 0), 6);
  assert.strictEqual(e.tftp_read_action(2, 1, 1, 0, 0, 0), 5);
  assert.strictEqual(e.tftp_read_action(4, 1, 1, 0, 1, 0), 1);
  assert.strictEqual(e.tftp_read_action(4, 1, 1, 0, 1, 1), 3);

  const serverHandshake = Buffer.concat([
    Buffer.from("NBDMAGIC", "ascii"),
    Buffer.from("IHAVEOPT", "ascii"),
    u16be(1),
  ]);
  [ptr, len] = put(memory, serverHandshake);
  assert.strictEqual(e.nbd_frame_kind(ptr, len), 1);
  [ptr, len] = put(memory, Buffer.concat([Buffer.from("IHAVEOPT", "ascii"), u32be(1), u32be(4)]));
  assert.strictEqual(e.nbd_frame_kind(ptr, len), 2);
  [ptr, len] = put(memory, Buffer.concat([u32be(0x25609513), Buffer.alloc(24)]));
  assert.strictEqual(e.nbd_frame_kind(ptr, len), 3);
  [ptr, len] = put(memory, Buffer.concat([u32be(0x67446698), Buffer.alloc(12)]));
  assert.strictEqual(e.nbd_frame_kind(ptr, len), 4);
  [ptr, len] = put(memory, Buffer.concat([u32be(0x0003e889), u32be(0x045565a9), Buffer.alloc(12)]));
  assert.strictEqual(e.nbd_frame_kind(ptr, len), 5);

  assert.strictEqual(e.nbd_command_class(0), 1);
  assert.strictEqual(e.nbd_command_class(1), 2);
  assert.strictEqual(e.nbd_command_class(2), 3);
  assert.strictEqual(e.nbd_command_class(3), 4);
  assert.strictEqual(e.nbd_command_class(4), 5);
  assert.strictEqual(e.nbd_command_class(6), 6);
  assert.strictEqual(e.nbd_command_class(9), 0);
  assert.strictEqual(e.nbd_errno_from_block_error(1), 30);
  assert.strictEqual(e.nbd_errno_from_block_error(2), 22);
  assert.strictEqual(e.nbd_errno_from_block_error(3), 22);
  assert.strictEqual(e.nbd_errno_from_block_error(4), 95);
  assert.strictEqual(e.nbd_errno_from_block_error(5), 110);
  assert.strictEqual(e.nbd_errno_from_block_error(6), 11);
  assert.strictEqual(e.nbd_errno_from_block_error(7), 5);
  assert.strictEqual(e.nbd_transmission_flags(0, 1, 1, 0), 37);
  assert.strictEqual(e.nbd_transmission_flags(1, 1, 1, 1), 103);

  [ptr, len] = put(memory, Buffer.from([5, 0, 0, 0, 1, 2, 3, 4, 5]));
  assert.strictEqual(e.block_frame_payload_len(ptr, len), 5);
  [ptr, len] = put(memory, Buffer.from([0, 0, 0, 2]));
  assert.strictEqual(e.block_frame_payload_len(ptr, len), -2);
  [ptr, len] = put(memory, Buffer.from([3, 0, 0, 0, 1]));
  assert.strictEqual(e.block_frame_payload_len(ptr, len), -3);

  assert.strictEqual(e.block_transfer_validate(512, 8, 2, 2, 1024), 0);
  assert.strictEqual(e.block_transfer_validate(0, 8, 2, 2, 1024), 1);
  assert.strictEqual(e.block_transfer_validate(512, 8, 7, 2, 1024), 2);
  assert.strictEqual(e.block_transfer_validate(512, 8, 2, 2, 512), 3);

  console.log(JSON.stringify({ unit: "protocols-block-transfer", standard_id: 300116, ok: true }));
})();
