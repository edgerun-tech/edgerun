#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const root = path.resolve(__dirname, "..", "..");
const watPath = path.join(root, "standards/build/wasm/codec-primitives/encoding-core.wat");
const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "encoding-core-"));
const wasmPath = path.join(tmpDir, "encoding-core.wasm");

process.on("exit", () => {
  fs.rmSync(tmpDir, { recursive: true, force: true });
});

execFileSync("wat2wasm", [watPath, "-o", wasmPath], { stdio: "inherit" });
execFileSync("wasm-validate", [wasmPath], { stdio: "inherit" });

function unpack(result) {
  const value = BigInt.asUintN(64, result);
  return {
    status: Number(value & 0xffff_ffffn),
    high: Number((value >> 32n) & 0xffff_ffffn),
  };
}

function readU64Le(bytes, offset) {
  const view = new DataView(bytes.buffer, bytes.byteOffset + offset, 8);
  return view.getBigUint64(0, true);
}

function writeBytes(mem, offset, bytes) {
  mem.fill(0, offset, offset + bytes.length + 8);
  mem.set(bytes, offset);
}

function assertPacked(actual, status, high, label) {
  const packed = unpack(actual);
  assert.equal(packed.status, status, `${label} status`);
  assert.equal(packed.high, high, `${label} high`);
}

function adler32Reference(bytes, initial = 1) {
  let a = initial & 0xffff;
  let b = (initial >>> 16) & 0xffff;
  for (const byte of bytes) {
    a = (a + byte) % 65521;
    b = (b + a) % 65521;
  }
  return ((b << 16) | a) >>> 0;
}

function assertAdler32(e, mem, offset, bytes, expected, label) {
  writeBytes(mem, offset, bytes);
  assert.equal(adler32Reference(bytes), expected, `${label} reference`);
  assert.equal(e.adler32(offset, bytes.length) >>> 0, expected, `${label} wasm`);
}

function assertAdler32Update(e, mem, offset, first, second, label) {
  const bytes = first.concat(second);
  const expected = adler32Reference(bytes);
  writeBytes(mem, offset, first);
  const partial = e.adler32(offset, first.length) >>> 0;
  assert.equal(partial, adler32Reference(first), `${label} partial`);
  writeBytes(mem, offset + first.length, second);
  assert.equal(
    e.adler32_update(partial, offset + first.length, second.length) >>> 0,
    expected,
    `${label} resumed wasm`,
  );
  assert.equal(adler32Reference(second, partial), expected, `${label} resumed reference`);
}

(async () => {
  const wasm = fs.readFileSync(wasmPath);
  const { instance } = await WebAssembly.instantiate(wasm);
  const e = instance.exports;
  const mem = new Uint8Array(e.memory.buffer);

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300001);

  mem.set([0x12, 0x34, 0x56], 0);
  assertPacked(e.read_u16_be(0, 3, 0), 0, 0x1234, "read_u16_be");
  assertPacked(e.read_u24_be(0, 3, 0), 0, 0x12_34_56, "read_u24_be");
  assertPacked(e.read_u32_be(0, 3, 0), 1, 0, "read_u32_be short");
  writeBytes(mem, 8, [0x78, 0x56, 0x34, 0x12]);
  assertPacked(e.read_u16_le(8, 4, 0), 0, 0x5678, "read_u16_le");
  assertPacked(e.read_u32_le(8, 4, 0), 0, 0x1234_5678, "read_u32_le");
  assertPacked(e.read_u32_le(8, 4, 1), 1, 0, "read_u32_le offset short");

  mem.fill(0, 16, 32);
  assertPacked(e.varint_encode_u64(300, 0, 16, 16), 0, 2, "varint 300 encode");
  assert.deepEqual(Array.from(mem.slice(16, 18)), [0xac, 0x02]);

  mem.fill(0, 32, 48);
  assertPacked(e.varint_decode_u64(16, 2, 32), 0, 2, "varint 300 decode");
  assert.equal(readU64Le(mem, 32), 300n);

  mem.fill(0, 16, 32);
  assertPacked(e.varint_encode_u64(300, 0, 16, 1), 2, 1, "varint encode out_cap partial");
  assert.deepEqual(Array.from(mem.slice(16, 17)), [0xac]);
  assertPacked(e.varint_encode_u64(300, 0, 16, 0), 2, 0, "varint encode out_cap zero");

  mem.fill(0, 16, 32);
  assertPacked(e.varint_encode_u64(0xffff_ffff, 0xffff_ffff, 16, 10), 0, 10, "varint u64 max encode");
  assert.deepEqual(Array.from(mem.slice(16, 26)), [
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01,
  ]);
  mem.fill(0, 32, 48);
  assertPacked(e.varint_decode_u64(16, 10, 32), 0, 10, "varint u64 max decode");
  assert.equal(readU64Le(mem, 32), 0xffff_ffff_ffff_ffffn);

  assertPacked(e.varint_decode_u64(16, 0, 32), 1, 0, "varint decode empty");
  writeBytes(mem, 16, [0xac]);
  assertPacked(e.varint_decode_u64(16, 1, 32), 5, 1, "varint decode truncated");
  writeBytes(mem, 16, [0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80]);
  assertPacked(e.varint_decode_u64(16, 10, 32), 6, 10, "varint decode too long");
  writeBytes(mem, 16, [0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x02]);
  assertPacked(e.varint_decode_u64(16, 10, 32), 4, 9, "varint decode u64 overflow");

  const quicCases = [
    { value: 0n, bytes: [0x00], label: "quic 0" },
    { value: 63n, bytes: [0x3f], label: "quic 63" },
    { value: 64n, bytes: [0x40, 0x40], label: "quic 64" },
    { value: 16_383n, bytes: [0x7f, 0xff], label: "quic 16383" },
    { value: 16_384n, bytes: [0x80, 0x00, 0x40, 0x00], label: "quic 16384" },
    { value: 1_073_741_823n, bytes: [0xbf, 0xff, 0xff, 0xff], label: "quic 1073741823" },
    { value: 1_073_741_824n, bytes: [0xc0, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00], label: "quic 1073741824" },
    { value: 4_611_686_018_427_387_903n, bytes: [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff], label: "quic max" },
  ];

  for (const { value, bytes, label } of quicCases) {
    const low = Number(value & 0xffff_ffffn);
    const high = Number((value >> 32n) & 0xffff_ffffn);
    mem.fill(0, 48, 64);
    assertPacked(e.quic_varint_encode_u64(low, high, 48, 8), 0, bytes.length, `${label} encode`);
    assert.deepEqual(Array.from(mem.slice(48, 48 + bytes.length)), bytes, `${label} bytes`);
    mem.fill(0, 64, 80);
    assertPacked(e.quic_varint_decode_u64(48, bytes.length, 64), 0, bytes.length, `${label} decode`);
    assert.equal(readU64Le(mem, 64), value, `${label} value`);
  }

  mem.fill(0, 64, 80);
  assertPacked(e.quic_varint_encode_u64(64, 0, 64, 1), 2, 0, "quic encode out_cap two-byte");
  assertPacked(e.quic_varint_encode_u64(16_384, 0, 64, 3), 2, 0, "quic encode out_cap four-byte");
  assertPacked(e.quic_varint_encode_u64(0, 0x4000_0000, 64, 8), 4, 0, "quic encode overflow");
  assertPacked(e.quic_varint_decode_u64(48, 0, 64), 1, 0, "quic decode empty");
  writeBytes(mem, 48, [0x40]);
  assertPacked(e.quic_varint_decode_u64(48, 1, 64), 5, 0, "quic decode truncated two-byte");
  writeBytes(mem, 48, [0x80, 0x00, 0x40]);
  assertPacked(e.quic_varint_decode_u64(48, 3, 64), 5, 0, "quic decode truncated four-byte");

  assert.equal(e.crc32(80, 0) >>> 0, 0x0000_0000);

  mem.set(Buffer.from("hello"), 80);
  assert.equal(e.crc32(80, 5) >>> 0, 0x3610_a686);

  mem.set(Buffer.from("123456789"), 96);
  assert.equal(e.crc32(96, 9) >>> 0, 0xcbf4_3926);

  mem.set(Buffer.from("Wikipedia"), 112);
  assert.equal(e.crc32(112, 9) >>> 0, 0xadaac02e);

  const adlerCases = [
    { bytes: [], expected: 0x0000_0001, label: "adler empty" },
    { bytes: [0, 0, 0, 0], expected: 0x0004_0001, label: "adler zeros" },
    { bytes: new Array(16).fill(0xff), expected: 0x8788_0ff1, label: "adler ff repeated" },
    { bytes: Array.from(Buffer.from("hello")), expected: 0x062c_0215, label: "adler hello" },
    { bytes: Array.from(Buffer.from("123456789")), expected: 0x091e_01de, label: "adler 123456789" },
    { bytes: Array.from(Buffer.from("Wikipedia")), expected: 0x11e6_0398, label: "adler wikipedia" },
    {
      bytes: Array.from(Buffer.from("EdgeRun codec primitive Adler-32 vector")),
      expected: 0x1ae3_0e0f,
      label: "adler mixed ascii",
    },
  ];

  for (const { bytes, expected, label } of adlerCases) {
    assertAdler32(e, mem, 128, bytes, expected, label);
  }

  assertAdler32Update(
    e,
    mem,
    192,
    Array.from(Buffer.from("rust")),
    Array.from(Buffer.from("acean")),
    "adler rustacean resume",
  );
  assertAdler32Update(
    e,
    mem,
    256,
    new Array(257).fill(0xff),
    new Array(1024 - 257).fill(0xff),
    "adler repeated ff resume",
  );

  console.log("encoding-core smoke ok");
})();
