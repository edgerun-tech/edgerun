#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const root = path.resolve(__dirname, "..", "..");
const watPath =
  process.argv[2] || path.join(root, "standards/build/wasm/codec-primitives/http3-frame.wat");
const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "http3-frame-"));
const wasmPath = path.join(tmpDir, "http3-frame.wasm");

process.on("exit", () => {
  fs.rmSync(tmpDir, { recursive: true, force: true });
});

execFileSync("wat2wasm", [watPath, "-o", wasmPath], { stdio: "inherit" });
execFileSync("wasm-validate", [wasmPath], { stdio: "inherit" });

function unpack(value) {
  const bits = BigInt.asUintN(64, value);
  return {
    status: Number(bits & 0xffff_ffffn),
    high: Number((bits >> 32n) & 0xffff_ffffn),
  };
}

function readU32Le(mem, offset) {
  return (
    mem[offset] |
    (mem[offset + 1] << 8) |
    (mem[offset + 2] << 16) |
    (mem[offset + 3] << 24)
  ) >>> 0;
}

function readRecord(mem, offset) {
  const typeLow = readU32Le(mem, offset);
  const typeHigh = readU32Le(mem, offset + 4);
  const payloadLow = readU32Le(mem, offset + 8);
  const payloadHigh = readU32Le(mem, offset + 12);
  return {
    frameType: (BigInt(typeHigh) << 32n) | BigInt(typeLow),
    payloadLen: (BigInt(payloadHigh) << 32n) | BigInt(payloadLow),
    headerLen: readU32Le(mem, offset + 16),
    classification: readU32Le(mem, offset + 20),
  };
}

function writeBytes(mem, offset, bytes) {
  mem.fill(0, offset, offset + Math.max(bytes.length + 32, 64));
  mem.set(bytes, offset);
}

function bytesOf(mem, offset, len) {
  return Array.from(mem.slice(offset, offset + len));
}

(async () => {
  const wasm = fs.readFileSync(wasmPath);
  const { instance } = await WebAssembly.instantiate(wasm);
  const e = instance.exports;
  const mem = new Uint8Array(e.memory.buffer);

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300012);

  assert.equal(e.http3_frame_type_classify(0n), 0);
  assert.equal(e.http3_frame_type_classify(1n), 1);
  assert.equal(e.http3_frame_type_classify(4n), 2);
  assert.equal(e.http3_frame_type_classify(3n), 3);
  assert.equal(e.http3_frame_type_classify(5n), 4);
  assert.equal(e.http3_frame_type_classify(7n), 5);
  assert.equal(e.http3_frame_type_classify(8n), 6);
  assert.equal(e.http3_frame_type_classify(9n), 7);
  assert.equal(e.http3_frame_type_classify(0x21n), 8);
  assert.equal(e.http3_frame_type_classify(0x25n), 8);
  assert.equal(e.http3_frame_type_classify(0x41n), 9);

  const frameCases = [
    { name: "DATA", bytes: [0x00, 0x05, 1, 2, 3, 4, 5], type: 0n, len: 5n, headerLen: 2, cls: 0 },
    { name: "HEADERS", bytes: [0x01, 0x00], type: 1n, len: 0n, headerLen: 2, cls: 1 },
    { name: "SETTINGS", bytes: [0x04, 0x00], type: 4n, len: 0n, headerLen: 2, cls: 2 },
    { name: "reserved", bytes: [0x21, 0x00], type: 0x21n, len: 0n, headerLen: 2, cls: 8 },
    { name: "unknown", bytes: [0x40, 0x41, 0x00], type: 65n, len: 0n, headerLen: 3, cls: 9 },
  ];

  for (const frame of frameCases) {
    writeBytes(mem, 1024, frame.bytes);
    assert.equal(e.http3_frame_header_decode(1024, frame.bytes.length, 2048), 0, frame.name);
    assert.deepEqual(readRecord(mem, 2048), {
      frameType: frame.type,
      payloadLen: frame.len,
      headerLen: frame.headerLen,
      classification: frame.cls,
    });
  }

  const thresholds = [
    { value: 0n, bytes: [0x00] },
    { value: 63n, bytes: [0x3f] },
    { value: 64n, bytes: [0x40, 0x40] },
    { value: 16_383n, bytes: [0x7f, 0xff] },
    { value: 16_384n, bytes: [0x80, 0x00, 0x40, 0x00] },
    { value: 1_073_741_823n, bytes: [0xbf, 0xff, 0xff, 0xff] },
    { value: 1_073_741_824n, bytes: [0xc0, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00] },
    {
      value: 4_611_686_018_427_387_903n,
      bytes: [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
    },
  ];

  for (const { value, bytes } of thresholds) {
    mem.fill(0, 4096, 4112);
    const packed = e.http3_frame_header_encode(0n, value, 4096, 16);
    assert.deepEqual(unpack(packed), { status: 0, high: bytes.length + 1 });
    assert.deepEqual(bytesOf(mem, 4096, bytes.length + 1), [0x00, ...bytes]);

    writeBytes(mem, 1024, [0x00, ...bytes, ...new Array(Number(value <= 16n ? value : 0n)).fill(0)]);
    const decodeLen = value <= 16n ? bytes.length + 1 + Number(value) : bytes.length + 1;
    const expectedStatus = value <= 16n ? 0 : 5;
    assert.equal(e.http3_frame_header_decode(1024, decodeLen, 2048), expectedStatus);
    if (expectedStatus === 0) {
      const record = readRecord(mem, 2048);
      assert.equal(record.frameType, 0n);
      assert.equal(record.payloadLen, value);
      assert.equal(record.headerLen, bytes.length + 1);
    }
  }

  writeBytes(mem, 1024, [0x40]);
  assert.equal(e.http3_frame_header_decode(1024, 1, 2048), 5, "truncated type varint");
  writeBytes(mem, 1024, [0x00, 0x40]);
  assert.equal(e.http3_frame_header_decode(1024, 2, 2048), 5, "truncated length varint");
  writeBytes(mem, 1024, [0x00, 0x05, 1, 2]);
  assert.equal(e.http3_frame_header_decode(1024, 4, 2048), 5, "payload incomplete");
  assert.equal(e.http3_frame_header_decode(1024, 0, 2048), 1, "empty input");

  assert.deepEqual(unpack(e.http3_frame_header_encode(0n, 64n, 4096, 2)), {
    status: 2,
    high: 0,
  });
  assert.deepEqual(unpack(e.http3_frame_header_encode(0n, 4_611_686_018_427_387_904n, 4096, 16)), {
    status: 4,
    high: 0,
  });

  console.log("http3-frame smoke ok");
})();
