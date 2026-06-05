#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath = process.argv[2] || "standards/build/wasm/codec-primitives/http2-frame.wat";

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
    value: Number((packed >> 32n) & 0xffffffffn),
  };
}

function readHeader(view, ptr) {
  return {
    length: view.getUint32(ptr, true),
    typeClass: view.getUint32(ptr + 4, true),
    flags: view.getUint32(ptr + 8, true),
    reserved: view.getUint32(ptr + 12, true),
    streamId: view.getUint32(ptr + 16, true),
    headerLen: view.getUint32(ptr + 20, true),
    totalLen: view.getUint32(ptr + 24, true),
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
  assert.strictEqual(exports.proto_standard_id(), 300010);

  assert.strictEqual(exports.http2_frame_type_classify(0), 0);
  assert.strictEqual(exports.http2_frame_type_classify(1), 1);
  assert.strictEqual(exports.http2_frame_type_classify(4), 4);
  assert.strictEqual(exports.http2_frame_type_classify(6), 6);
  assert.strictEqual(exports.http2_frame_type_classify(99), 255);

  let packed = unpack(exports.http2_frame_header_encode(5, 0, 0x01, 1, inPtr, 9));
  assert.deepStrictEqual(packed, { status: 0, value: 9 });
  memory.set([1, 2, 3, 4, 5], inPtr + 9);
  assert.strictEqual(exports.http2_frame_header_decode(inPtr, 14, 16384, outPtr), 0);
  assert.deepStrictEqual(readHeader(view, outPtr), {
    length: 5,
    typeClass: 0,
    flags: 1,
    reserved: 0,
    streamId: 1,
    headerLen: 9,
    totalLen: 14,
  });

  packed = unpack(exports.http2_frame_header_encode(0, 1, 0x24, 0x80000003, inPtr, 9));
  assert.deepStrictEqual(packed, { status: 0, value: 9 });
  assert.deepStrictEqual(Array.from(memory.slice(inPtr, inPtr + 9)), [
    0x00, 0x00, 0x00, 0x01, 0x24, 0x00, 0x00, 0x00, 0x03,
  ]);

  memory.set([0x00, 0x00, 0x00, 0x01, 0x04, 0x80, 0x00, 0x00, 0x03], inPtr);
  assert.strictEqual(exports.http2_frame_header_decode(inPtr, 9, 16384, outPtr), 0);
  assert.deepStrictEqual(readHeader(view, outPtr), {
    length: 0,
    typeClass: 1,
    flags: 4,
    reserved: 1,
    streamId: 3,
    headerLen: 9,
    totalLen: 9,
  });

  for (const [type, flags, streamId] of [
    [4, 0x01, 0],
    [6, 0x01, 0],
    [99, 0x00, 7],
  ]) {
    packed = unpack(exports.http2_frame_header_encode(0, type, flags, streamId, inPtr, 9));
    assert.deepStrictEqual(packed, { status: 0, value: 9 });
    assert.strictEqual(exports.http2_frame_header_decode(inPtr, 9, 16384, outPtr), 0);
    const decoded = readHeader(view, outPtr);
    assert.strictEqual(decoded.length, 0);
    assert.strictEqual(decoded.typeClass, type <= 9 ? type : 255);
    assert.strictEqual(decoded.flags, flags);
    assert.strictEqual(decoded.streamId, streamId);
  }

  assert.deepStrictEqual(unpack(exports.http2_frame_header_encode(1, 0, 0, 1, inPtr, 8)), {
    status: 2,
    value: 0,
  });
  assert.deepStrictEqual(unpack(exports.http2_frame_header_encode(0x01000000, 0, 0, 1, inPtr, 9)), {
    status: 4,
    value: 0,
  });
  assert.deepStrictEqual(unpack(exports.http2_frame_header_encode(0, 256, 0, 1, inPtr, 9)), {
    status: 4,
    value: 0,
  });
  assert.deepStrictEqual(unpack(exports.http2_frame_header_encode(0, 0, 256, 1, inPtr, 9)), {
    status: 4,
    value: 0,
  });

  assert.strictEqual(exports.http2_frame_header_decode(inPtr, 8, 16384, outPtr), 1);
  memory.set([0x00, 0x40, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01], inPtr);
  assert.strictEqual(exports.http2_frame_header_decode(inPtr, 9, 16384, outPtr), 3);
  memory.set([0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01], inPtr);
  assert.strictEqual(exports.http2_frame_header_decode(inPtr, 12, 16384, outPtr), 1);

  console.log(
    JSON.stringify(
      {
        unit: "http2-frame",
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
