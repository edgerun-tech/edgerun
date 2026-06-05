#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/length-field.wat";

const STATUS_OK = 0;
const STATUS_INPUT_SHORT = 1;
const STATUS_OUTPUT_SHORT = 2;
const STATUS_OVERFLOW = 4;

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

function span(view, outPtr) {
  return {
    valueOff: view.getUint32(outPtr, true),
    valueLen: view.getUint32(outPtr + 4, true),
  };
}

(async () => {
  const module = await WebAssembly.instantiate(compileWat(watPath), {});
  const e = module.instance.exports;
  const memory = new Uint8Array(e.memory.buffer);
  const view = new DataView(e.memory.buffer);
  const inPtr = 1024;
  const outPtr = 4096;

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300058);

  write(memory, inPtr, Buffer.from([3, 0xaa, 0xbb, 0xcc, 0xdd]));
  assert.deepEqual(unpack(e.length_field_scan_u8(inPtr, 5, 0, outPtr)), {
    status: STATUS_OK,
    value: 4,
  });
  assert.deepEqual(span(view, outPtr), { valueOff: 1, valueLen: 3 });

  write(memory, inPtr, Buffer.from([0x02, 0x00, 0x10, 0x11, 0x12]));
  assert.deepEqual(unpack(e.length_field_scan_u16_le(inPtr, 5, 0, outPtr)), {
    status: STATUS_OK,
    value: 4,
  });
  assert.deepEqual(span(view, outPtr), { valueOff: 2, valueLen: 2 });

  write(memory, inPtr, Buffer.from([0x99, 0x00, 0x00, 0x00, 0x02, 0x31, 0x32]));
  assert.deepEqual(unpack(e.length_field_scan_u32_be(inPtr, 7, 1, outPtr)), {
    status: STATUS_OK,
    value: 7,
  });
  assert.deepEqual(span(view, outPtr), { valueOff: 5, valueLen: 2 });

  write(memory, inPtr, Buffer.from([0x02, 0x00, 0x00, 0x00, 0x41, 0x42]));
  assert.deepEqual(unpack(e.length_field_scan_u32_le(inPtr, 6, 0, outPtr)), {
    status: STATUS_OK,
    value: 6,
  });
  assert.deepEqual(span(view, outPtr), { valueOff: 4, valueLen: 2 });

  write(memory, inPtr, Buffer.from([0x02, 0, 0, 0, 0, 0, 0, 0, 0x51, 0x52]));
  assert.deepEqual(unpack(e.length_field_scan_u64_le(inPtr, 10, 0, outPtr)), {
    status: STATUS_OK,
    value: 10,
  });
  assert.deepEqual(span(view, outPtr), { valueOff: 8, valueLen: 2 });

  write(memory, inPtr, Buffer.from([0x02]));
  assert.deepEqual(unpack(e.length_field_scan_u16_le(inPtr, 1, 0, outPtr)), {
    status: STATUS_INPUT_SHORT,
    value: 0,
  });

  write(memory, inPtr, Buffer.from([4, 0xaa, 0xbb]));
  assert.deepEqual(unpack(e.length_field_scan_u8(inPtr, 3, 0, outPtr)), {
    status: STATUS_INPUT_SHORT,
    value: 0,
  });

  write(memory, inPtr, Buffer.from([0, 0, 0, 0, 1, 0, 0, 0]));
  assert.deepEqual(unpack(e.length_field_scan_u64_le(inPtr, 8, 0, outPtr)), {
    status: STATUS_OVERFLOW,
    value: 0,
  });

  write(memory, inPtr, Buffer.from([0]));
  assert.deepEqual(unpack(e.length_field_scan_u8(inPtr, 1, 0, 0)), {
    status: STATUS_OUTPUT_SHORT,
    value: 0,
  });

  assert.deepEqual(unpack(e.length_field_write_u8(255n, outPtr, 1)), {
    status: STATUS_OK,
    value: 1,
  });
  assert.equal(memory[outPtr], 255);

  assert.deepEqual(unpack(e.length_field_write_u16_le(0x1234n, outPtr, 2)), {
    status: STATUS_OK,
    value: 2,
  });
  assert.deepEqual(Array.from(memory.slice(outPtr, outPtr + 2)), [0x34, 0x12]);

  assert.deepEqual(unpack(e.length_field_write_u32_be(0x01020304n, outPtr, 4)), {
    status: STATUS_OK,
    value: 4,
  });
  assert.deepEqual(Array.from(memory.slice(outPtr, outPtr + 4)), [1, 2, 3, 4]);

  assert.deepEqual(unpack(e.length_field_write_u32_le(0x01020304n, outPtr, 4)), {
    status: STATUS_OK,
    value: 4,
  });
  assert.deepEqual(Array.from(memory.slice(outPtr, outPtr + 4)), [4, 3, 2, 1]);

  assert.deepEqual(unpack(e.length_field_write_u64_le(0x0102030405060708n, outPtr, 8)), {
    status: STATUS_OK,
    value: 8,
  });
  assert.deepEqual(Array.from(memory.slice(outPtr, outPtr + 8)), [8, 7, 6, 5, 4, 3, 2, 1]);

  assert.deepEqual(unpack(e.length_field_write_u16_le(3n, outPtr, 1)), {
    status: STATUS_OUTPUT_SHORT,
    value: 0,
  });
  assert.deepEqual(unpack(e.length_field_write_u8(256n, outPtr, 1)), {
    status: STATUS_OVERFLOW,
    value: 0,
  });
  assert.deepEqual(unpack(e.length_field_write_u32_le(0x1_0000_0000n, outPtr, 4)), {
    status: STATUS_OVERFLOW,
    value: 0,
  });

  console.log(
    JSON.stringify({
      unit: "length-field",
      abi_version: e.proto_abi_version(),
      standard_id: e.proto_standard_id(),
      ok: true,
    }),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
