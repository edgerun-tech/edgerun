#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath = process.argv[2] || "standards/build/wasm/codec-primitives/hpack-string.wat";

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

function meta(view, ptr) {
  return {
    flags: view.getUint32(ptr, true),
    huffman: view.getUint32(ptr + 4, true),
    consumed: view.getUint32(ptr + 8, true),
    payloadOffset: view.getUint32(ptr + 12, true),
    payloadLen: view.getUint32(ptr + 16, true),
  };
}

(async () => {
  const module = await WebAssembly.instantiate(compileWat(watPath), {});
  const exports = module.instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);
  const view = new DataView(exports.memory.buffer);
  const ptr = 1024;
  const outPtr = 2048;
  const metaPtr = 3072;

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300019);

  function decode(input, expected, expectedMeta) {
    memory.fill(0, ptr, ptr + 512);
    memory.fill(0, outPtr, outPtr + 512);
    memory.fill(0, metaPtr, metaPtr + 32);
    memory.set(input, ptr);
    const got = unpack(exports.hpack_string_decode(ptr, input.length, outPtr, 512, metaPtr));
    assert.deepStrictEqual(got, { status: 0, value: expected.length });
    assert.deepStrictEqual(Array.from(memory.slice(outPtr, outPtr + expected.length)), expected);
    assert.deepStrictEqual(meta(view, metaPtr), expectedMeta);
    assert.strictEqual(exports.hpack_string_scan(ptr, input.length, metaPtr), 0);
  }

  decode([3, 0x61, 0x62, 0x63], [0x61, 0x62, 0x63], {
    flags: 0,
    huffman: 0,
    consumed: 1,
    payloadOffset: 1,
    payloadLen: 3,
  });
  decode([0x81, 0x3f], [0x6f], {
    flags: 1,
    huffman: 1,
    consumed: 1,
    payloadOffset: 1,
    payloadLen: 1,
  });

  const longRaw = [127, 1, ...new Array(128).fill(0x78)];
  decode(longRaw, new Array(128).fill(0x78), {
    flags: 0,
    huffman: 0,
    consumed: 2,
    payloadOffset: 2,
    payloadLen: 128,
  });

  memory.set([3, 0x61, 0x62], ptr);
  assert.strictEqual(exports.hpack_string_scan(ptr, 3, metaPtr), 1);
  assert.deepStrictEqual(unpack(exports.hpack_string_decode(ptr, 3, outPtr, 512, metaPtr)), {
    status: 1,
    value: 0,
  });

  memory.set([3, 0x61, 0x62, 0x63], ptr);
  assert.deepStrictEqual(unpack(exports.hpack_string_decode(ptr, 4, outPtr, 2, metaPtr)), {
    status: 2,
    value: 0,
  });

  memory.set([0x82, 0x3f, 0xff], ptr);
  assert.strictEqual(unpack(exports.hpack_string_decode(ptr, 3, outPtr, 512, metaPtr)).status, 3);

  memory.set([127, 0x80], ptr);
  assert.strictEqual(exports.hpack_string_scan(ptr, 2, metaPtr), 1);

  console.log("hpack-string smoke ok");
})().catch((err) => {
  console.error(err);
  process.exit(1);
});
