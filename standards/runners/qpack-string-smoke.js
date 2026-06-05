#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath = process.argv[2] || "standards/build/wasm/codec-primitives/qpack-string.wat";

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
  assert.strictEqual(exports.proto_standard_id(), 300020);

  function decode(size, input, expected, expectedMeta) {
    memory.fill(0, ptr, ptr + 512);
    memory.fill(0, outPtr, outPtr + 512);
    memory.fill(0, metaPtr, metaPtr + 32);
    memory.set(input, ptr);
    const got = unpack(exports.qpack_string_decode(size, ptr, input.length, outPtr, 512, metaPtr));
    assert.deepStrictEqual(got, { status: 0, value: expected.length });
    assert.deepStrictEqual(Array.from(memory.slice(outPtr, outPtr + expected.length)), expected);
    assert.deepStrictEqual(meta(view, metaPtr), expectedMeta);
    assert.strictEqual(exports.qpack_string_scan(size, ptr, input.length, metaPtr), 0);
  }

  decode(6, [0b0100_0011, 0x62, 0x61, 0x72], [0x62, 0x61, 0x72], {
    flags: 2,
    huffman: 0,
    consumed: 1,
    payloadOffset: 1,
    payloadLen: 3,
  });
  decode(
    6,
    [0b0110_1100, 168, 116, 149, 79, 6, 76, 231, 181, 42, 88, 89, 127],
    Array.from(Buffer.from("name without ref")),
    { flags: 3, huffman: 1, consumed: 1, payloadOffset: 1, payloadLen: 12 },
  );
  decode(
    8,
    [0b1000_1010, 168, 116, 149, 79, 6, 76, 234, 88, 89, 127],
    Array.from(Buffer.from("name with ref")),
    { flags: 1, huffman: 1, consumed: 1, payloadOffset: 1, payloadLen: 10 },
  );
  decode(8, [0x80], [], {
    flags: 1,
    huffman: 1,
    consumed: 1,
    payloadOffset: 1,
    payloadLen: 0,
  });

  memory.set([0b0100_0011, 0x62, 0x61], ptr);
  assert.strictEqual(exports.qpack_string_scan(6, ptr, 3, metaPtr), 1);
  assert.deepStrictEqual(unpack(exports.qpack_string_decode(6, ptr, 3, outPtr, 512, metaPtr)), {
    status: 1,
    value: 0,
  });

  memory.set([0b0100_0011, 0x62, 0x61, 0x72], ptr);
  assert.deepStrictEqual(unpack(exports.qpack_string_decode(6, ptr, 4, outPtr, 2, metaPtr)), {
    status: 2,
    value: 0,
  });

  memory.set([0x82, 0x3f, 0xff], ptr);
  assert.strictEqual(unpack(exports.qpack_string_decode(8, ptr, 3, outPtr, 512, metaPtr)).status, 3);

  memory.set([0xff, 0x80], ptr);
  assert.strictEqual(exports.qpack_string_scan(8, ptr, 2, metaPtr), 1);
  assert.strictEqual(exports.qpack_string_scan(1, ptr, 2, metaPtr), 3);

  console.log("qpack-string smoke ok");
})().catch((err) => {
  console.error(err);
  process.exit(1);
});
