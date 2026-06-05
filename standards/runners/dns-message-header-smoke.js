#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/dns-message-header.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  return execFileSync("wat2wasm", [file, "-o", "-"]);
}

function low32High32(value) {
  return {
    status: Number(value & 0xffffffffn),
    written: Number((value >> 32n) & 0xffffffffn),
  };
}

function writeBytes(memory, offset, bytes) {
  memory.set(Uint8Array.from(bytes), offset);
  return bytes.length;
}

function readU32(memory, offset) {
  return new DataView(memory.buffer).getUint32(offset, true);
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const exports = instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300026);

  let len = writeBytes(memory, 1024, [
    0x12, 0x34,
    0x01, 0x00,
    0x00, 0x01,
    0x00, 0x00,
    0x00, 0x00,
    0x00, 0x00,
  ]);
  assert.strictEqual(exports.dns_header_decode(1024, len, 2048), 0);
  assert.strictEqual(readU32(memory, 2048), 0x1234);
  assert.strictEqual(readU32(memory, 2052), 0x0100);
  assert.strictEqual(readU32(memory, 2056), 0);
  assert.strictEqual(readU32(memory, 2060), 0);
  assert.strictEqual(readU32(memory, 2072), 1);
  assert.strictEqual(readU32(memory, 2084), 1);

  len = writeBytes(memory, 1024, [
    0xab, 0xcd,
    0x85, 0x83,
    0x00, 0x01,
    0x00, 0x02,
    0x00, 0x03,
    0x00, 0x04,
  ]);
  assert.strictEqual(exports.dns_header_decode(1024, len, 2048), 0);
  assert.strictEqual(readU32(memory, 2048), 0xabcd);
  assert.strictEqual(readU32(memory, 2056), 1);
  assert.strictEqual(readU32(memory, 2060), 0);
  assert.strictEqual(readU32(memory, 2064), 1);
  assert.strictEqual(readU32(memory, 2072), 1);
  assert.strictEqual(readU32(memory, 2076), 1);
  assert.strictEqual(readU32(memory, 2080), 3);
  assert.strictEqual(readU32(memory, 2088), 2);
  assert.strictEqual(readU32(memory, 2092), 3);
  assert.strictEqual(readU32(memory, 2096), 4);

  assert.strictEqual(exports.dns_header_decode(1024, 11, 2048), 1);

  len = writeBytes(memory, 1024, [
    0, 1, 0, 0,
    0, 17,
    0, 0,
    0, 0,
    0, 0,
  ]);
  assert.strictEqual(exports.dns_header_decode(1024, len, 2048), 3);

  let packed = exports.dns_header_encode(0xbeef, 0x0100, 1, 0, 0, 0, 3072, 12);
  assert.deepStrictEqual(low32High32(packed), { status: 0, written: 12 });
  assert.deepStrictEqual(Array.from(memory.slice(3072, 3084)), [
    0xbe, 0xef,
    0x01, 0x00,
    0x00, 0x01,
    0x00, 0x00,
    0x00, 0x00,
    0x00, 0x00,
  ]);

  packed = exports.dns_header_encode(0xbeef, 0x0100, 1, 0, 0, 0, 3072, 11);
  assert.deepStrictEqual(low32High32(packed), { status: 2, written: 0 });
  packed = exports.dns_header_encode(0xbeef, 0x0100, 17, 0, 0, 0, 3072, 12);
  assert.deepStrictEqual(low32High32(packed), { status: 3, written: 0 });

  const classified = exports.dns_header_flags_classify(0x8583);
  assert.strictEqual(classified & 1, 1);
  assert.strictEqual((classified >> 1) & 0x0f, 0);
  assert.strictEqual((classified >> 5) & 1, 1);
  assert.strictEqual((classified >> 7) & 1, 1);
  assert.strictEqual((classified >> 8) & 1, 1);
  assert.strictEqual((classified >> 9) & 0x0f, 3);

  console.log(JSON.stringify({
    unit: "dns-message-header",
    abi_version: exports.proto_abi_version(),
    standard_id: exports.proto_standard_id(),
    ok: true,
  }, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
