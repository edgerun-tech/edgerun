#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/dns-compressed-name.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  return execFileSync("wat2wasm", [file, "-o", "-"]);
}

function low32High32(value) {
  return {
    status: Number(value & 0xffffffffn),
    value: Number((value >> 32n) & 0xffffffffn),
  };
}

function writeBytes(memory, offset, bytes) {
  memory.set(Uint8Array.from(bytes), offset);
  return bytes.length;
}

function wireName(labels) {
  const out = [];
  for (const label of labels) {
    const bytes = Buffer.from(label, "ascii");
    out.push(bytes.length, ...bytes);
  }
  out.push(0);
  return out;
}

function readU32(memory, offset) {
  return new DataView(memory.buffer).getUint32(offset, true);
}

function readName(memory, offset, len) {
  return Buffer.from(memory.slice(offset, offset + len)).toString("ascii");
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const exports = instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300033);

  let len = writeBytes(memory, 1024, [0]);
  assert.strictEqual(exports.dns_name_follow(1024, len, 0, 2048, 64), 0);
  assert.strictEqual(readU32(memory, 2048), 1);
  assert.strictEqual(readU32(memory, 2052), 0);
  assert.strictEqual(readU32(memory, 2056), 0);
  assert.strictEqual(readU32(memory, 2060), 0);
  assert.strictEqual(readU32(memory, 2064), 0);
  assert.deepStrictEqual(low32High32(exports.dns_name_wire_len(1024, len, 0)), {
    status: 0,
    value: 1,
  });

  len = writeBytes(memory, 1024, wireName(["Example", "COM"]));
  assert.strictEqual(exports.dns_name_follow(1024, len, 0, 2048, 64), 0);
  assert.strictEqual(readU32(memory, 2048), len);
  assert.strictEqual(readU32(memory, 2052), 2);
  assert.strictEqual(readU32(memory, 2056), 11);
  assert.strictEqual(readU32(memory, 2060), 0);
  assert.strictEqual(readU32(memory, 2064), len - 1);
  let packed = exports.dns_name_to_lower_ascii_compressed(1024, len, 0, 3072, 64);
  assert.deepStrictEqual(low32High32(packed), { status: 0, value: 11 });
  assert.strictEqual(readName(memory, 3072, 11), "example.com");

  const message = [
    0x12, 0x34, 0x81, 0x80, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
    ...wireName(["Example", "COM"]),
    0x00, 0x01, 0x00, 0x01,
  ];
  const answerNameOffset = message.length;
  message.push(0xc0, 0x0c);
  message.push(0x00, 0x01, 0x00, 0x01);
  message.push(0x00, 0x00, 0x00, 0x3c);
  message.push(0x00, 0x04, 127, 0, 0, 1);
  len = writeBytes(memory, 1024, message);
  assert.strictEqual(exports.dns_name_follow(1024, len, answerNameOffset, 2048, 64), 0);
  assert.strictEqual(readU32(memory, 2048), 2);
  assert.strictEqual(readU32(memory, 2052), 2);
  assert.strictEqual(readU32(memory, 2056), 11);
  assert.strictEqual(readU32(memory, 2060), 1);
  assert.strictEqual(readU32(memory, 2064), 24);
  assert.strictEqual(readU32(memory, 2068), 13);
  assert.strictEqual(readU32(memory, 2072), 7);
  assert.strictEqual(readU32(memory, 2076), 21);
  assert.strictEqual(readU32(memory, 2080), 3);
  packed = exports.dns_name_to_lower_ascii_compressed(1024, len, answerNameOffset, 3072, 64);
  assert.deepStrictEqual(low32High32(packed), { status: 0, value: 11 });
  assert.strictEqual(readName(memory, 3072, 11), "example.com");
  packed = exports.dns_name_wire_len(1024, len, answerNameOffset);
  assert.deepStrictEqual(low32High32(packed), { status: 0, value: 2 });

  len = writeBytes(memory, 1024, [3, 0x43, 0x4f, 0x4d, 0, 3, 0x57, 0x57, 0x57, 0xc0, 0x00]);
  assert.strictEqual(exports.dns_name_follow(1024, len, 5, 2048, 64), 0);
  packed = exports.dns_name_to_lower_ascii_compressed(1024, len, 5, 3072, 64);
  assert.deepStrictEqual(low32High32(packed), { status: 0, value: 7 });
  assert.strictEqual(readName(memory, 3072, 7), "www.com");

  len = writeBytes(memory, 1024, [3, 0x57, 0x57, 0x57, 0xc0, 0x00]);
  assert.strictEqual(exports.dns_name_follow(1024, len, 0, 2048, 64), 3);

  len = writeBytes(memory, 1024, [0xc0, 0x00]);
  assert.strictEqual(exports.dns_name_follow(1024, len, 0, 2048, 64), 3);

  len = writeBytes(memory, 1024, [0xc0, 0x02, 0xc0, 0x00]);
  assert.strictEqual(exports.dns_name_follow(1024, len, 0, 2048, 64), 3);

  len = writeBytes(memory, 1024, [0xc0, 0x40]);
  assert.strictEqual(exports.dns_name_follow(1024, len, 0, 2048, 64), 3);

  len = writeBytes(memory, 1024, [0xc0]);
  assert.strictEqual(exports.dns_name_follow(1024, len, 0, 2048, 64), 1);

  len = writeBytes(memory, 1024, [0x40, 0]);
  assert.strictEqual(exports.dns_name_follow(1024, len, 0, 2048, 64), 3);

  len = writeBytes(memory, 1024, [3, 0x61, 0x62]);
  assert.strictEqual(exports.dns_name_follow(1024, len, 0, 2048, 64), 1);

  len = writeBytes(memory, 1024, wireName(["bad-label-"]));
  assert.strictEqual(exports.dns_name_follow(1024, len, 0, 2048, 64), 3);

  len = writeBytes(memory, 1024, wireName(["Example", "COM"]));
  assert.strictEqual(exports.dns_name_follow(1024, len, 0, 2048, 24), 2);
  packed = exports.dns_name_to_lower_ascii_compressed(1024, len, 0, 3072, 4);
  assert.deepStrictEqual(low32High32(packed), { status: 2, value: 0 });

  len = writeBytes(memory, 1024, [3, 0x77, 0x77, 0x77, 0xc0, 0x01]);
  assert.strictEqual(exports.dns_name_follow(1024, len, 0, 2048, 64), 3);

  const manyLabels = Array.from({ length: 128 }, () => "a");
  len = writeBytes(memory, 1024, wireName(manyLabels));
  assert.strictEqual(exports.dns_name_follow(1024, len, 0, 2048, 2048), 6);

  console.log(JSON.stringify({
    unit: "dns-compressed-name",
    abi_version: exports.proto_abi_version(),
    standard_id: exports.proto_standard_id(),
    ok: true,
  }, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
