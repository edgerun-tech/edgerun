#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath = process.argv[2] || "standards/build/wasm/codec-primitives/qpack-encoder-stream.wat";

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

function record(view, ptr, index = 0) {
  const base = ptr + index * 48;
  return {
    kind: view.getUint32(base, true),
    offset: view.getUint32(base + 4, true),
    consumed: view.getUint32(base + 8, true),
    flags: view.getUint32(base + 12, true),
    value: view.getBigUint64(base + 16, true),
    namePayloadOffset: view.getUint32(base + 24, true),
    namePayloadLen: view.getUint32(base + 28, true),
    nameHuffman: view.getUint32(base + 32, true),
    valuePayloadOffset: view.getUint32(base + 36, true),
    valuePayloadLen: view.getUint32(base + 40, true),
    valueHuffman: view.getUint32(base + 44, true),
  };
}

(async () => {
  const module = await WebAssembly.instantiate(compileWat(watPath), {});
  const exports = module.instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);
  const view = new DataView(exports.memory.buffer);
  const ptr = 1024;
  const outPtr = 2048;

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300037);

  function decode(input, expected) {
    memory.fill(0, ptr, ptr + 256);
    memory.fill(0, outPtr, outPtr + 256);
    memory.set(input, ptr);
    assert.strictEqual(exports.qpack_encoder_instruction_decode(ptr, input.length, outPtr), 0);
    assert.deepStrictEqual(record(view, outPtr), expected);
  }

  decode([0x20 | 25], {
    kind: 1,
    offset: 0,
    consumed: 1,
    flags: 1,
    value: 25n,
    namePayloadOffset: 0,
    namePayloadLen: 0,
    nameHuffman: 0,
    valuePayloadOffset: 0,
    valuePayloadLen: 0,
    valueHuffman: 0,
  });

  decode([0xc0 | 12, 0x04, 0x2f, 0x62, 0x61, 0x72], {
    kind: 2,
    offset: 0,
    consumed: 6,
    flags: 3,
    value: 12n,
    namePayloadOffset: 0,
    namePayloadLen: 0,
    nameHuffman: 0,
    valuePayloadOffset: 2,
    valuePayloadLen: 4,
    valueHuffman: 0,
  });

  decode([0x80 | 1, 0x80 | 1, 0x3f], {
    kind: 2,
    offset: 0,
    consumed: 3,
    flags: 2,
    value: 1n,
    namePayloadOffset: 0,
    namePayloadLen: 0,
    nameHuffman: 0,
    valuePayloadOffset: 2,
    valuePayloadLen: 1,
    valueHuffman: 1,
  });

  decode([0x40 | 3, 0x66, 0x6f, 0x6f, 0x03, 0x62, 0x61, 0x72], {
    kind: 3,
    offset: 0,
    consumed: 8,
    flags: 2,
    value: 0n,
    namePayloadOffset: 1,
    namePayloadLen: 3,
    nameHuffman: 0,
    valuePayloadOffset: 5,
    valuePayloadLen: 3,
    valueHuffman: 0,
  });

  decode([0x60 | 1, 0x3f, 0x00], {
    kind: 3,
    offset: 0,
    consumed: 3,
    flags: 3,
    value: 0n,
    namePayloadOffset: 1,
    namePayloadLen: 1,
    nameHuffman: 1,
    valuePayloadOffset: 3,
    valuePayloadLen: 0,
    valueHuffman: 0,
  });

  decode([0x00 | 1], {
    kind: 4,
    offset: 0,
    consumed: 1,
    flags: 0,
    value: 1n,
    namePayloadOffset: 0,
    namePayloadLen: 0,
    nameHuffman: 0,
    valuePayloadOffset: 0,
    valuePayloadLen: 0,
    valueHuffman: 0,
  });

  decode([0x3f, 0x00], {
    kind: 1,
    offset: 0,
    consumed: 2,
    flags: 1,
    value: 31n,
    namePayloadOffset: 0,
    namePayloadLen: 0,
    nameHuffman: 0,
    valuePayloadOffset: 0,
    valuePayloadLen: 0,
    valueHuffman: 0,
  });

  memory.fill(0, ptr, ptr + 256);
  memory.fill(0, outPtr, outPtr + 256);
  memory.set([0x20 | 25, 0x00 | 1, 0x40 | 1, 0x61, 0x01, 0x62], ptr);
  assert.deepStrictEqual(unpack(exports.qpack_encoder_stream_scan(ptr, 6, outPtr, 144)), {
    status: 0,
    value: 3,
  });
  assert.deepStrictEqual(record(view, outPtr, 0).offset, 0);
  assert.deepStrictEqual(record(view, outPtr, 1).offset, 1);
  assert.deepStrictEqual(record(view, outPtr, 2), {
    kind: 3,
    offset: 2,
    consumed: 4,
    flags: 2,
    value: 0n,
    namePayloadOffset: 1,
    namePayloadLen: 1,
    nameHuffman: 0,
    valuePayloadOffset: 3,
    valuePayloadLen: 1,
    valueHuffman: 0,
  });

  memory.set([0xc0 | 12, 0x04, 0x2f], ptr);
  assert.strictEqual(exports.qpack_encoder_instruction_decode(ptr, 3, outPtr), 1);
  assert.deepStrictEqual(unpack(exports.qpack_encoder_stream_scan(ptr, 3, outPtr, 144)), {
    status: 1,
    value: 0,
  });

  memory.set([0x40 | 3, 0x66, 0x6f], ptr);
  assert.strictEqual(exports.qpack_encoder_instruction_decode(ptr, 3, outPtr), 1);

  memory.set([0x20 | 1, 0x20 | 2], ptr);
  assert.deepStrictEqual(unpack(exports.qpack_encoder_stream_scan(ptr, 2, outPtr, 48)), {
    status: 2,
    value: 1,
  });

  memory.set([0xff, 128, 254, 255, 255, 255, 255, 255, 255, 255, 255, 1], ptr);
  assert.strictEqual(exports.qpack_encoder_instruction_decode(ptr, 12, outPtr), 4);

  console.log("qpack-encoder-stream smoke ok");
})().catch((err) => {
  console.error(err.stack || String(err));
  process.exit(1);
});
