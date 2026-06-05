#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/generic-tlv.wat";

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

function readBytes(memory, ptr, len) {
  return Array.from(memory.slice(ptr, ptr + len));
}

function record(view, outPtr) {
  return {
    tag: view.getUint32(outPtr, true),
    valueOff: view.getUint32(outPtr + 4, true),
    valueLen: view.getUint32(outPtr + 8, true),
    headerLen: view.getUint32(outPtr + 12, true),
    totalLen: view.getUint32(outPtr + 16, true),
  };
}

function unpack(packed) {
  return {
    status: Number(packed & 0xffffffffn),
    value: Number(packed >> 32n),
  };
}

(async () => {
  const module = await WebAssembly.instantiate(compileWat(watPath), {});
  const exports = module.instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);
  const view = new DataView(exports.memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300057);

  const inPtr = 1024;
  const outPtr = 4096;

  write(memory, inPtr, [0x42, 0x03, 0x61, 0x62, 0x63]);
  assert.deepStrictEqual(unpack(exports.generic_tlv_next(inPtr, 5, 0, outPtr)), {
    status: 0,
    value: 5,
  });
  assert.deepStrictEqual(record(view, outPtr), {
    tag: 0x42,
    valueOff: 2,
    valueLen: 3,
    headerLen: 2,
    totalLen: 5,
  });

  write(memory, inPtr, [0x7f, 0x81, 0x80, ...new Array(128).fill(0x55)]);
  assert.deepStrictEqual(unpack(exports.generic_tlv_next(inPtr, 131, 0, outPtr)), {
    status: 0,
    value: 131,
  });
  assert.deepStrictEqual(record(view, outPtr), {
    tag: 0x7f,
    valueOff: 3,
    valueLen: 128,
    headerLen: 3,
    totalLen: 131,
  });

  write(memory, inPtr, [0x80, 0x82, 0x01, 0x00, ...new Array(256).fill(0x22)]);
  assert.deepStrictEqual(unpack(exports.generic_tlv_next(inPtr, 260, 0, outPtr)), {
    status: 0,
    value: 260,
  });
  assert.deepStrictEqual(record(view, outPtr), {
    tag: 0x80,
    valueOff: 4,
    valueLen: 256,
    headerLen: 4,
    totalLen: 260,
  });

  write(memory, inPtr, [0x01, 0x01, 0xaa, 0x02, 0x02, 0xbb, 0xcc]);
  let next = unpack(exports.generic_tlv_next(inPtr, 7, 0, outPtr));
  assert.deepStrictEqual(next, { status: 0, value: 3 });
  assert.deepStrictEqual(record(view, outPtr), {
    tag: 1,
    valueOff: 2,
    valueLen: 1,
    headerLen: 2,
    totalLen: 3,
  });

  next = unpack(exports.generic_tlv_next(inPtr, 7, next.value, outPtr));
  assert.deepStrictEqual(next, { status: 0, value: 7 });
  assert.deepStrictEqual(record(view, outPtr), {
    tag: 2,
    valueOff: 5,
    valueLen: 2,
    headerLen: 2,
    totalLen: 4,
  });

  write(memory, inPtr, [0x42]);
  assert.deepStrictEqual(unpack(exports.generic_tlv_next(inPtr, 1, 0, outPtr)), {
    status: 1,
    value: 1,
  });

  write(memory, inPtr, [0x42, 0x03, 0x61]);
  assert.deepStrictEqual(unpack(exports.generic_tlv_next(inPtr, 3, 0, outPtr)), {
    status: 1,
    value: 3,
  });

  write(memory, inPtr, [0x42, 0x80]);
  assert.deepStrictEqual(unpack(exports.generic_tlv_next(inPtr, 2, 0, outPtr)), {
    status: 3,
    value: 0,
  });

  write(memory, inPtr, [0x42, 0x83, 0x00, 0x00, 0x01, 0xaa]);
  assert.deepStrictEqual(unpack(exports.generic_tlv_next(inPtr, 6, 0, outPtr)), {
    status: 3,
    value: 0,
  });

  let packed = unpack(exports.generic_tlv_encode_header(0x42, 3, outPtr, 2));
  assert.deepStrictEqual(packed, { status: 0, value: 2 });
  assert.deepStrictEqual(readBytes(memory, outPtr, 2), [0x42, 0x03]);

  packed = unpack(exports.generic_tlv_encode_header(0x7f, 128, outPtr, 3));
  assert.deepStrictEqual(packed, { status: 0, value: 3 });
  assert.deepStrictEqual(readBytes(memory, outPtr, 3), [0x7f, 0x81, 0x80]);

  packed = unpack(exports.generic_tlv_encode_header(0x80, 256, outPtr, 4));
  assert.deepStrictEqual(packed, { status: 0, value: 4 });
  assert.deepStrictEqual(readBytes(memory, outPtr, 4), [0x80, 0x82, 0x01, 0x00]);

  assert.deepStrictEqual(unpack(exports.generic_tlv_encode_header(0x01, 127, outPtr, 1)), {
    status: 2,
    value: 0,
  });
  assert.deepStrictEqual(unpack(exports.generic_tlv_encode_header(0x01, 128, outPtr, 2)), {
    status: 2,
    value: 0,
  });
  assert.deepStrictEqual(unpack(exports.generic_tlv_encode_header(0x01, 65536, outPtr, 4)), {
    status: 3,
    value: 0,
  });
  assert.deepStrictEqual(unpack(exports.generic_tlv_encode_header(0x100, 1, outPtr, 4)), {
    status: 3,
    value: 0,
  });

  console.log(
    JSON.stringify(
      {
        unit: "generic-tlv",
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
