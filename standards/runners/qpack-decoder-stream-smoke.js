#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath = process.argv[2] || "standards/build/wasm/codec-primitives/qpack-decoder-stream.wat";

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
  const base = ptr + index * 24;
  return {
    kind: view.getUint32(base, true),
    offset: view.getUint32(base + 4, true),
    consumed: view.getUint32(base + 8, true),
    flags: view.getUint32(base + 12, true),
    value: view.getBigUint64(base + 16, true),
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
  assert.strictEqual(exports.proto_standard_id(), 300036);

  function decode(input, expected) {
    memory.fill(0, ptr, ptr + 128);
    memory.fill(0, outPtr, outPtr + 128);
    memory.set(input, ptr);
    assert.strictEqual(exports.qpack_decoder_instruction_decode(ptr, input.length, outPtr), 0);
    assert.deepStrictEqual(record(view, outPtr), expected);
  }

  decode([0x80 | 42], {
    kind: 1,
    offset: 0,
    consumed: 1,
    flags: 1,
    value: 42n,
  });
  decode([0x40 | 42], {
    kind: 2,
    offset: 0,
    consumed: 1,
    flags: 1,
    value: 42n,
  });
  decode([42], {
    kind: 3,
    offset: 0,
    consumed: 1,
    flags: 0,
    value: 42n,
  });
  decode([0xff, 0x00], {
    kind: 1,
    offset: 0,
    consumed: 2,
    flags: 1,
    value: 127n,
  });
  decode([0x7f, 0x00], {
    kind: 2,
    offset: 0,
    consumed: 2,
    flags: 1,
    value: 63n,
  });

  memory.fill(0, ptr, ptr + 128);
  memory.fill(0, outPtr, outPtr + 128);
  memory.set([0x80 | 1, 0x40 | 2, 3, 0xff, 0x00], ptr);
  assert.deepStrictEqual(unpack(exports.qpack_decoder_stream_scan(ptr, 5, outPtr, 96)), {
    status: 0,
    value: 4,
  });
  assert.deepStrictEqual(record(view, outPtr, 0), {
    kind: 1,
    offset: 0,
    consumed: 1,
    flags: 1,
    value: 1n,
  });
  assert.deepStrictEqual(record(view, outPtr, 1), {
    kind: 2,
    offset: 1,
    consumed: 1,
    flags: 1,
    value: 2n,
  });
  assert.deepStrictEqual(record(view, outPtr, 2), {
    kind: 3,
    offset: 2,
    consumed: 1,
    flags: 0,
    value: 3n,
  });
  assert.deepStrictEqual(record(view, outPtr, 3), {
    kind: 1,
    offset: 3,
    consumed: 2,
    flags: 1,
    value: 127n,
  });

  memory.set([0xff], ptr);
  assert.strictEqual(exports.qpack_decoder_instruction_decode(ptr, 1, outPtr), 1);
  assert.deepStrictEqual(unpack(exports.qpack_decoder_stream_scan(ptr, 1, outPtr, 96)), {
    status: 1,
    value: 0,
  });

  memory.set([0x40 | 1, 0x40 | 2], ptr);
  assert.deepStrictEqual(unpack(exports.qpack_decoder_stream_scan(ptr, 2, outPtr, 24)), {
    status: 2,
    value: 1,
  });

  memory.set([0x3f, 0x02], ptr);
  assert.strictEqual(exports.qpack_decoder_instruction_decode(ptr, 2, outPtr), 4);

  memory.set([0xff, 128, 254, 255, 255, 255, 255, 255, 255, 255, 255, 1], ptr);
  assert.strictEqual(exports.qpack_decoder_instruction_decode(ptr, 12, outPtr), 4);

  console.log("qpack-decoder-stream smoke ok");
})().catch((err) => {
  console.error(err.stack || String(err));
  process.exit(1);
});
