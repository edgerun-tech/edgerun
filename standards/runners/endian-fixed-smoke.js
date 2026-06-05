#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/endian-fixed.wat";

const STATUS_OK = 0;
const STATUS_INPUT_SHORT = 1;
const STATUS_OUTPUT_SHORT = 2;
const STATUS_INVALID = 3;
const STATUS_OVERFLOW = 4;
const LE = 0;
const BE = 1;

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

function unpack(packed) {
  return {
    status: Number(packed & 0xffffffffn),
    value: Number(packed >> 32n),
  };
}

function readU64(view, ptr) {
  return view.getBigUint64(ptr, true);
}

function readI64(view, ptr) {
  return view.getBigInt64(ptr, true);
}

(async () => {
  const module = await WebAssembly.instantiate(compileWat(watPath), {});
  const e = module.instance.exports;
  const memory = new Uint8Array(e.memory.buffer);
  const view = new DataView(e.memory.buffer);
  const inPtr = 1024;
  const outPtr = 4096;

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300063);

  write(memory, inPtr, Buffer.from([0x78, 0x56, 0x34, 0x12, 0xef, 0xcd, 0xab, 0x90]));
  assert.equal(e.endian_read(inPtr, 8, 0, 2, LE, 0, outPtr), STATUS_OK);
  assert.equal(readU64(view, outPtr), 0x5678n);
  assert.equal(e.endian_read(inPtr, 8, 0, 4, LE, 0, outPtr), STATUS_OK);
  assert.equal(readU64(view, outPtr), 0x12345678n);
  assert.equal(e.endian_read(inPtr, 8, 0, 8, LE, 0, outPtr), STATUS_OK);
  assert.equal(readU64(view, outPtr), 0x90abcdef12345678n);

  write(memory, inPtr, Buffer.from([0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]));
  assert.equal(e.endian_read(inPtr, 8, 0, 2, LE, 1, outPtr), STATUS_OK);
  assert.equal(readI64(view, outPtr), -1n);
  assert.equal(e.endian_read(inPtr, 8, 0, 4, LE, 1, outPtr), STATUS_OK);
  assert.equal(readI64(view, outPtr), -1n);
  assert.equal(e.endian_read(inPtr, 8, 0, 8, LE, 1, outPtr), STATUS_OK);
  assert.equal(readI64(view, outPtr), -1n);

  write(memory, inPtr, Buffer.from([0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef]));
  assert.equal(e.endian_read(inPtr, 8, 0, 2, BE, 0, outPtr), STATUS_OK);
  assert.equal(readU64(view, outPtr), 0x1234n);
  assert.equal(e.endian_read(inPtr, 8, 0, 3, BE, 0, outPtr), STATUS_OK);
  assert.equal(readU64(view, outPtr), 0x123456n);
  assert.equal(e.endian_read(inPtr, 8, 0, 4, BE, 0, outPtr), STATUS_OK);
  assert.equal(readU64(view, outPtr), 0x12345678n);
  assert.equal(e.endian_read(inPtr, 8, 0, 8, BE, 0, outPtr), STATUS_OK);
  assert.equal(readU64(view, outPtr), 0x1234567890abcdefn);

  write(memory, inPtr, Buffer.from([0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]));
  assert.equal(e.endian_read(inPtr, 8, 0, 4, BE, 1, outPtr), STATUS_OK);
  assert.equal(readI64(view, outPtr), -1n);
  assert.equal(e.endian_read(inPtr, 8, 0, 8, BE, 1, outPtr), STATUS_OK);
  assert.equal(readI64(view, outPtr), -1n);

  write(memory, inPtr, Buffer.from([0x12, 0x34, 0x56, 0x78]));
  assert.equal(e.endian_read(inPtr, 4, 3, 2, LE, 0, outPtr), STATUS_INPUT_SHORT);
  assert.equal(e.endian_read(inPtr, 4, 1, 4, BE, 0, outPtr), STATUS_INPUT_SHORT);
  assert.equal(e.endian_read(inPtr, 4, 0, 8, BE, 1, outPtr), STATUS_INPUT_SHORT);
  assert.equal(e.endian_read(inPtr, 4, 0, 3, BE, 1, outPtr), STATUS_INVALID);
  assert.equal(e.endian_read(inPtr, 4, 0, 5, BE, 0, outPtr), STATUS_INVALID);
  assert.equal(e.endian_read(inPtr, 4, 0, 2, 2, 0, outPtr), STATUS_INVALID);
  assert.equal(e.endian_read(inPtr, 4, 0, 2, LE, 0, 0), STATUS_OUTPUT_SHORT);

  assert.deepEqual(unpack(e.endian_write(0xabcd, 0, 2, LE, outPtr, 2)), {
    status: STATUS_OK,
    value: 2,
  });
  assert.deepEqual(Array.from(memory.slice(outPtr, outPtr + 2)), [0xcd, 0xab]);

  assert.deepEqual(unpack(e.endian_write(0x123456, 0, 3, BE, outPtr, 3)), {
    status: STATUS_OK,
    value: 3,
  });
  assert.deepEqual(Array.from(memory.slice(outPtr, outPtr + 3)), [0x12, 0x34, 0x56]);

  assert.deepEqual(unpack(e.endian_write(0x12345678, 0, 4, BE, outPtr, 4)), {
    status: STATUS_OK,
    value: 4,
  });
  assert.deepEqual(Array.from(memory.slice(outPtr, outPtr + 4)), [0x12, 0x34, 0x56, 0x78]);

  assert.deepEqual(unpack(e.endian_write(0x12345678, 0, 4, LE, outPtr, 4)), {
    status: STATUS_OK,
    value: 4,
  });
  assert.deepEqual(Array.from(memory.slice(outPtr, outPtr + 4)), [0x78, 0x56, 0x34, 0x12]);

  assert.deepEqual(unpack(e.endian_write(0xfffffffe, 0xffffffff, 4, LE, outPtr, 4)), {
    status: STATUS_OK,
    value: 4,
  });
  assert.deepEqual(Array.from(memory.slice(outPtr, outPtr + 4)), [0xfe, 0xff, 0xff, 0xff]);

  assert.deepEqual(unpack(e.endian_write(0x89abcdef, 0x01234567, 8, LE, outPtr, 8)), {
    status: STATUS_OK,
    value: 8,
  });
  assert.deepEqual(
    Array.from(memory.slice(outPtr, outPtr + 8)),
    [0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x01],
  );

  assert.deepEqual(unpack(e.endian_write(0x89abcdef, 0x01234567, 8, BE, outPtr, 8)), {
    status: STATUS_OK,
    value: 8,
  });
  assert.deepEqual(
    Array.from(memory.slice(outPtr, outPtr + 8)),
    [0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef],
  );

  assert.deepEqual(unpack(e.endian_write(0x10000, 0, 2, BE, outPtr, 2)), {
    status: STATUS_OVERFLOW,
    value: 0,
  });
  assert.deepEqual(unpack(e.endian_write(0x01000000, 0, 3, BE, outPtr, 3)), {
    status: STATUS_OVERFLOW,
    value: 0,
  });
  assert.deepEqual(unpack(e.endian_write(0, 1, 4, LE, outPtr, 4)), {
    status: STATUS_OVERFLOW,
    value: 0,
  });
  assert.deepEqual(unpack(e.endian_write(1, 0, 2, LE, outPtr, 1)), {
    status: STATUS_OUTPUT_SHORT,
    value: 0,
  });
  assert.deepEqual(unpack(e.endian_write(1, 0, 5, LE, outPtr, 5)), {
    status: STATUS_INVALID,
    value: 0,
  });

  console.log(
    JSON.stringify({
      unit: "endian-fixed",
      abi_version: e.proto_abi_version(),
      standard_id: e.proto_standard_id(),
      ok: true,
    }),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
