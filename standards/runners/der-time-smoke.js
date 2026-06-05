#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] || path.join(__dirname, "../build/wasm/codec-primitives/der-time.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  return execFileSync("wat2wasm", [file, "-o", "-"]);
}

function tlv(tag, ascii) {
  const bytes = [...Buffer.from(ascii, "ascii")];
  return [tag, bytes.length, ...bytes];
}

function u32(memory, offset) {
  return (
    memory[offset] |
    (memory[offset + 1] << 8) |
    (memory[offset + 2] << 16) |
    (memory[offset + 3] << 24)
  ) >>> 0;
}

function i64FromRecord(memory, offset) {
  const lo = BigInt(u32(memory, offset));
  const hi = BigInt(u32(memory, offset + 4));
  const value = (hi << 32n) | lo;
  return value & (1n << 63n) ? value - (1n << 64n) : value;
}

function write(memory, data, ptr = 1024) {
  memory.fill(0, ptr, ptr + 512);
  memory.set(data, ptr);
  return ptr;
}

function record(memory, out = 4096) {
  return {
    kind: u32(memory, out),
    year: u32(memory, out + 4),
    month: u32(memory, out + 8),
    day: u32(memory, out + 12),
    hour: u32(memory, out + 16),
    minute: u32(memory, out + 20),
    second: u32(memory, out + 24),
    unix: i64FromRecord(memory, out + 28),
    headerLen: u32(memory, out + 36),
    totalLen: u32(memory, out + 40),
  };
}

function unixSeconds(year, month, day, hour, minute, second) {
  return BigInt(Date.UTC(year, month - 1, day, hour, minute, second) / 1000);
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const exports = instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);
  const out = 4096;

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300029);

  let data = tlv(0x17, "700101000000Z");
  assert.strictEqual(exports.der_time_decode(write(memory, data), data.length, out), 0);
  assert.deepStrictEqual(record(memory, out), {
    kind: 1,
    year: 1970,
    month: 1,
    day: 1,
    hour: 0,
    minute: 0,
    second: 0,
    unix: 0n,
    headerLen: 2,
    totalLen: data.length,
  });

  data = tlv(0x17, "491231235959Z");
  assert.strictEqual(exports.der_time_decode(write(memory, data), data.length, out), 0);
  let got = record(memory, out);
  assert.strictEqual(got.kind, 1);
  assert.strictEqual(got.year, 2049);
  assert.strictEqual(got.unix, unixSeconds(2049, 12, 31, 23, 59, 59));

  data = tlv(0x17, "500101000000Z");
  assert.strictEqual(exports.der_time_decode(write(memory, data), data.length, out), 0);
  got = record(memory, out);
  assert.strictEqual(got.year, 1950);
  assert.strictEqual(got.unix, unixSeconds(1950, 1, 1, 0, 0, 0));

  data = tlv(0x18, "20240229010203Z");
  assert.strictEqual(exports.der_time_decode(write(memory, data), data.length, out), 0);
  got = record(memory, out);
  assert.strictEqual(got.kind, 2);
  assert.strictEqual(got.year, 2024);
  assert.strictEqual(got.month, 2);
  assert.strictEqual(got.day, 29);
  assert.strictEqual(got.unix, unixSeconds(2024, 2, 29, 1, 2, 3));

  for (const [name, bad] of [
    ["bad tag", tlv(0x16, "20240229010203Z")],
    ["utc bad len", tlv(0x17, "7001010000Z")],
    ["generalized bad len", tlv(0x18, "202402290102Z")],
    ["no z", tlv(0x17, "700101000000+0000")],
    ["bad digit", tlv(0x18, "2024AA29010203Z")],
    ["bad month", tlv(0x18, "20241301000000Z")],
    ["bad day", tlv(0x18, "20230229000000Z")],
    ["bad hour", tlv(0x17, "700101240000Z")],
    ["bad minute", tlv(0x17, "700101006000Z")],
    ["bad second", tlv(0x17, "700101000060Z")],
    ["trailing byte", [...tlv(0x17, "700101000000Z"), 0x00]],
    ["indefinite", [0x17, 0x80, ...Buffer.from("700101000000Z", "ascii")]],
    ["non-minimal long len", [0x17, 0x81, 13, ...Buffer.from("700101000000Z", "ascii")]],
  ]) {
    assert.strictEqual(exports.der_time_decode(write(memory, bad), bad.length, out), 3, name);
  }

  assert.strictEqual(exports.der_time_decode(write(memory, tlv(0x17, "700101000000Z")), 1, out), 1);

  console.log(JSON.stringify({
    unit: "der-time",
    abi_version: exports.proto_abi_version(),
    standard_id: exports.proto_standard_id(),
    ok: true,
  }, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
