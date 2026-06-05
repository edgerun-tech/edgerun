#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath = process.argv[2] || "standards/build/wasm/codec-primitives/hpack-huffman.wat";

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

(async () => {
  const module = await WebAssembly.instantiate(compileWat(watPath), {});
  const exports = module.instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);
  const ptr = 1024;
  const outPtr = 2048;

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300018);

  function decode(input, expected) {
    memory.fill(0, ptr, ptr + 128);
    memory.fill(0, outPtr, outPtr + 128);
    memory.set(input, ptr);
    const got = unpack(exports.hpack_huffman_decode(ptr, input.length, outPtr, 128));
    assert.deepStrictEqual(got, { status: 0, value: expected.length });
    assert.deepStrictEqual(Array.from(memory.slice(outPtr, outPtr + expected.length)), expected);
    assert.strictEqual(exports.hpack_huffman_validate(ptr, input.length), 0);
  }

  decode([0x3f], [0x6f]);
  decode([0x07], [0x30]);
  decode([0x87], [0x41]);
  decode([0xff, 0xaf], [0x23]);
  decode([0xff, 0xcf], [0x24]);
  decode([0xff, 0xff, 0xff, 0xf3], [0x0a]);
  decode([0xfe, 0x01], [0x21, 0x30]);
  decode([0x53, 0xf8], [0x20, 0x21]);

  memory.set([0x3f], ptr);
  assert.deepStrictEqual(unpack(exports.hpack_huffman_decode(ptr, 1, outPtr, 0)), {
    status: 2,
    value: 0,
  });

  for (const input of [
    [0xff, 0xff, 0xff, 0xff],
    [0x3f, 0xff],
    [0x3e],
    [0xfe, 0x00],
  ]) {
    memory.set(input, ptr);
    assert.strictEqual(unpack(exports.hpack_huffman_decode(ptr, input.length, outPtr, 128)).status, 3);
    assert.strictEqual(exports.hpack_huffman_validate(ptr, input.length), 3);
  }

  console.log("hpack-huffman smoke ok");
})().catch((err) => {
  console.error(err);
  process.exit(1);
});
