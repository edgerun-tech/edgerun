#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/dns-name.wat");

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

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const exports = instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300025);

  let len = writeBytes(memory, 1024, [0]);
  assert.strictEqual(exports.dns_name_scan(1024, len, 2048, 64), 0);
  assert.strictEqual(readU32(memory, 2048), 1);
  assert.strictEqual(readU32(memory, 2052), 0);
  assert.strictEqual(readU32(memory, 2056), 0);
  let packed = exports.dns_name_to_lower_ascii(1024, len, 3072, 16);
  assert.deepStrictEqual(low32High32(packed), { status: 0, written: 0 });

  len = writeBytes(memory, 1024, wireName(["Example", "COM"]));
  assert.strictEqual(exports.dns_name_scan(1024, len, 2048, 64), 0);
  assert.strictEqual(readU32(memory, 2048), len);
  assert.strictEqual(readU32(memory, 2052), 2);
  assert.strictEqual(readU32(memory, 2056), 11);
  assert.strictEqual(readU32(memory, 2060), 1);
  assert.strictEqual(readU32(memory, 2064), 7);
  assert.strictEqual(readU32(memory, 2068), 9);
  assert.strictEqual(readU32(memory, 2072), 3);

  packed = exports.dns_name_to_lower_ascii(1024, len, 3072, 64);
  assert.deepStrictEqual(low32High32(packed), { status: 0, written: 11 });
  assert.strictEqual(Buffer.from(memory.slice(3072, 3083)).toString("ascii"), "example.com");
  packed = exports.dns_name_to_lower_ascii(1024, len, 3072, 4);
  assert.deepStrictEqual(low32High32(packed), { status: 2, written: 0 });

  len = writeBytes(memory, 1024, wireName(["a".repeat(63), "com"]));
  assert.strictEqual(exports.dns_name_scan(1024, len, 2048, 64), 0);

  len = writeBytes(memory, 1024, [64, ...Buffer.from("a".repeat(64), "ascii"), 0]);
  assert.strictEqual(exports.dns_name_scan(1024, len, 2048, 128), 3);

  len = writeBytes(memory, 1024, [0xc0, 0x0c]);
  assert.strictEqual(exports.dns_name_scan(1024, len, 2048, 64), 3);

  len = writeBytes(memory, 1024, [0x40, 1, 0]);
  assert.strictEqual(exports.dns_name_scan(1024, len, 2048, 64), 3);

  len = writeBytes(memory, 1024, [3, 0x62, 0x61]);
  assert.strictEqual(exports.dns_name_scan(1024, len, 2048, 64), 1);

  len = writeBytes(memory, 1024, wireName(["bad-label-"]));
  assert.strictEqual(exports.dns_name_scan(1024, len, 2048, 64), 3);

  const manyLabels = Array.from({ length: 128 }, () => "a");
  len = writeBytes(memory, 1024, wireName(manyLabels));
  assert.strictEqual(len, 257);
  assert.strictEqual(exports.dns_name_scan(1024, len, 2048, 2048), 6);

  console.log(JSON.stringify({
    unit: "dns-name",
    abi_version: exports.proto_abi_version(),
    standard_id: exports.proto_standard_id(),
    ok: true,
  }, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
