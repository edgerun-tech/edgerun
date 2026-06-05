#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath = process.argv[2] || "standards/build/wasm/codec-primitives/length-frame.wat";

function compileWat(file) {
  const wasmPath = path.join(os.tmpdir(), `edgerun-${path.basename(file)}-${process.pid}.wasm`);
  execFileSync("wat2wasm", [file, "-o", wasmPath], { stdio: "pipe" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "pipe" });
  const bytes = fs.readFileSync(wasmPath);
  fs.unlinkSync(wasmPath);
  return bytes;
}

function unpack(packed) {
  return {
    status: Number(packed & 0xffffffffn),
    written: Number((packed >> 32n) & 0xffffffffn),
  };
}

function readRecord(view, ptr) {
  return {
    lenLow: view.getUint32(ptr, true),
    lenHigh: view.getUint32(ptr + 4, true),
    headerLen: view.getUint32(ptr + 8, true),
  };
}

(async () => {
  const module = await WebAssembly.instantiate(compileWat(watPath), {});
  const exports = module.instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);
  const view = new DataView(exports.memory.buffer);
  const inPtr = 1024;
  const outPtr = 2048;

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300056);

  let packed = unpack(exports.frame_header_encode_u16_be(0, inPtr, 2));
  assert.deepStrictEqual(packed, { status: 0, written: 2 });
  assert.deepStrictEqual(Array.from(memory.slice(inPtr, inPtr + 2)), [0x00, 0x00]);
  assert.strictEqual(exports.frame_header_decode_u16_be(inPtr, 2, outPtr), 0);
  assert.deepStrictEqual(readRecord(view, outPtr), { lenLow: 0, lenHigh: 0, headerLen: 2 });

  packed = unpack(exports.frame_header_encode_u16_be(9, inPtr, 2));
  assert.deepStrictEqual(packed, { status: 0, written: 2 });
  assert.deepStrictEqual(Array.from(memory.slice(inPtr, inPtr + 2)), [0x00, 0x09]);
  assert.strictEqual(exports.frame_header_decode_u16_be(inPtr, 2, outPtr), 0);
  assert.deepStrictEqual(readRecord(view, outPtr), { lenLow: 9, lenHigh: 0, headerLen: 2 });

  packed = unpack(exports.frame_header_encode_u16_be(65535, inPtr, 2));
  assert.deepStrictEqual(packed, { status: 0, written: 2 });
  assert.deepStrictEqual(Array.from(memory.slice(inPtr, inPtr + 2)), [0xff, 0xff]);
  assert.strictEqual(exports.frame_header_decode_u16_be(inPtr, 2, outPtr), 0);
  assert.deepStrictEqual(readRecord(view, outPtr), { lenLow: 65535, lenHigh: 0, headerLen: 2 });

  assert.deepStrictEqual(unpack(exports.frame_header_encode_u16_be(65536, inPtr, 2)), {
    status: 4,
    written: 0,
  });
  assert.deepStrictEqual(unpack(exports.frame_header_encode_u16_be(1, inPtr, 1)), {
    status: 2,
    written: 0,
  });
  assert.strictEqual(exports.frame_header_decode_u16_be(inPtr, 1, outPtr), 1);

  packed = unpack(exports.frame_header_encode_u64_be(0x05060708, 0x01020304, inPtr, 8));
  assert.deepStrictEqual(packed, { status: 0, written: 8 });
  assert.deepStrictEqual(Array.from(memory.slice(inPtr, inPtr + 8)), [
    0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
  ]);
  assert.strictEqual(exports.frame_header_decode_u64_be(inPtr, 8, outPtr), 0);
  assert.deepStrictEqual(readRecord(view, outPtr), {
    lenLow: 0x05060708,
    lenHigh: 0x01020304,
    headerLen: 8,
  });

  packed = unpack(exports.frame_header_encode_u64_le(0x05060708, 0x01020304, inPtr, 8));
  assert.deepStrictEqual(packed, { status: 0, written: 8 });
  assert.deepStrictEqual(Array.from(memory.slice(inPtr, inPtr + 8)), [
    0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01,
  ]);
  assert.strictEqual(exports.frame_header_decode_u64_le(inPtr, 8, outPtr), 0);
  assert.deepStrictEqual(readRecord(view, outPtr), {
    lenLow: 0x05060708,
    lenHigh: 0x01020304,
    headerLen: 8,
  });

  assert.deepStrictEqual(unpack(exports.frame_header_encode_u64_be(1, 0, inPtr, 7)), {
    status: 2,
    written: 0,
  });
  assert.deepStrictEqual(unpack(exports.frame_header_encode_u64_le(1, 0, inPtr, 7)), {
    status: 2,
    written: 0,
  });
  assert.strictEqual(exports.frame_header_decode_u64_be(inPtr, 7, outPtr), 1);
  assert.strictEqual(exports.frame_header_decode_u64_le(inPtr, 7, outPtr), 1);

  console.log(
    JSON.stringify(
      {
        unit: "length-frame",
        abi_version: exports.proto_abi_version(),
        standard_id: exports.proto_standard_id(),
        ok: true,
      },
      null,
      2,
    ),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
