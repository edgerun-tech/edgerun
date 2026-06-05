#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/c-string.wat";

function compileWat(file) {
  const wasmPath = path.join(os.tmpdir(), `edgerun-${path.basename(file)}-${process.pid}.wasm`);
  execFileSync("wat2wasm", [file, "-o", wasmPath], { stdio: "pipe" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "pipe" });
  const bytes = fs.readFileSync(wasmPath);
  fs.unlinkSync(wasmPath);
  return bytes;
}

function write(memory, ptr, bytes) {
  memory.fill(0, ptr, ptr + bytes.length + 64);
  memory.set(bytes, ptr);
}

function record(view, outPtr) {
  return {
    strOff: view.getUint32(outPtr, true),
    strLen: view.getUint32(outPtr + 4, true),
    consumed: view.getUint32(outPtr + 8, true),
  };
}

function unpack(packed) {
  return {
    status: Number(packed & 0xffffffffn),
    nextOffset: Number(packed >> 32n),
  };
}

(async () => {
  const module = await WebAssembly.instantiate(compileWat(watPath), {});
  const exports = module.instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);
  const view = new DataView(exports.memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300049);

  const inPtr = 1024;
  const outPtr = 4096;

  write(memory, inPtr, Buffer.from("hello\0tail", "ascii"));
  assert.strictEqual(exports.c_string_scan(inPtr, 10, outPtr), 0);
  assert.deepStrictEqual(record(view, outPtr), {
    strOff: 0,
    strLen: 5,
    consumed: 6,
  });

  write(memory, inPtr, Buffer.from("hello", "ascii"));
  assert.strictEqual(exports.c_string_scan(inPtr, 5, outPtr), 4);
  assert.deepStrictEqual(record(view, outPtr), {
    strOff: 0,
    strLen: 5,
    consumed: 5,
  });

  write(memory, inPtr, Buffer.from("\0tail", "ascii"));
  assert.strictEqual(exports.c_string_scan(inPtr, 5, outPtr), 0);
  assert.deepStrictEqual(record(view, outPtr), {
    strOff: 0,
    strLen: 0,
    consumed: 1,
  });

  write(memory, inPtr, Buffer.from("foo\0bar\0\0tail", "ascii"));
  let next = unpack(exports.c_multi_string_next(inPtr, 14, 0, outPtr));
  assert.deepStrictEqual(next, { status: 0, nextOffset: 4 });
  assert.deepStrictEqual(record(view, outPtr), {
    strOff: 0,
    strLen: 3,
    consumed: 4,
  });

  next = unpack(exports.c_multi_string_next(inPtr, 14, next.nextOffset, outPtr));
  assert.deepStrictEqual(next, { status: 0, nextOffset: 8 });
  assert.deepStrictEqual(record(view, outPtr), {
    strOff: 4,
    strLen: 3,
    consumed: 4,
  });

  next = unpack(exports.c_multi_string_next(inPtr, 14, next.nextOffset, outPtr));
  assert.deepStrictEqual(next, { status: 5, nextOffset: 9 });
  assert.deepStrictEqual(record(view, outPtr), {
    strOff: 8,
    strLen: 0,
    consumed: 1,
  });

  write(memory, inPtr, Buffer.from("foo\0bar", "ascii"));
  next = unpack(exports.c_multi_string_next(inPtr, 7, 4, outPtr));
  assert.deepStrictEqual(next, { status: 4, nextOffset: 7 });
  assert.deepStrictEqual(record(view, outPtr), {
    strOff: 4,
    strLen: 3,
    consumed: 3,
  });

  next = unpack(exports.c_multi_string_next(inPtr, 7, 7, outPtr));
  assert.deepStrictEqual(next, { status: 5, nextOffset: 7 });
  assert.deepStrictEqual(record(view, outPtr), {
    strOff: 7,
    strLen: 0,
    consumed: 0,
  });

  console.log(
    JSON.stringify(
      {
        unit: "c-string",
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
