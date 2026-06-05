#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/deflate-stored.wat";

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
    written: Number((packed >> 32n) & 0xffffffffn),
  };
}

function write(memory, ptr, bytes) {
  memory.fill(0, ptr, ptr + bytes.length + 64);
  memory.set(bytes, ptr);
  return bytes.length;
}

function record(view, ptr) {
  return {
    blockCount: view.getUint32(ptr, true),
    payloadTotal: view.getUint32(ptr + 4, true),
    consumed: view.getUint32(ptr + 8, true),
    finalSeen: view.getUint32(ptr + 12, true),
  };
}

function storedBlock(payload, final = true) {
  const len = payload.length;
  const nlen = len ^ 0xffff;
  return Buffer.concat([
    Buffer.from([final ? 1 : 0, len & 0xff, (len >> 8) & 0xff, nlen & 0xff, (nlen >> 8) & 0xff]),
    Buffer.from(payload),
  ]);
}

(async () => {
  const module = await WebAssembly.instantiate(compileWat(watPath), {});
  const exports = module.instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);
  const view = new DataView(exports.memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300066);

  const inPtr = 1024;
  const outPtr = 140000;
  const recPtr = 220000;

  let encoded = Buffer.from([0x01, 0x00, 0x00, 0xff, 0xff]);
  write(memory, inPtr, encoded);
  assert.strictEqual(exports.deflate_stored_scan(inPtr, encoded.length, recPtr), 0);
  assert.deepStrictEqual(record(view, recPtr), {
    blockCount: 1,
    payloadTotal: 0,
    consumed: 5,
    finalSeen: 1,
  });

  const hello = Buffer.from("Hello, zlib!", "ascii");
  encoded = storedBlock(hello);
  write(memory, inPtr, encoded);
  assert.strictEqual(exports.deflate_stored_scan(inPtr, encoded.length, recPtr), 0);
  assert.deepStrictEqual(record(view, recPtr), {
    blockCount: 1,
    payloadTotal: hello.length,
    consumed: encoded.length,
    finalSeen: 1,
  });

  write(memory, inPtr, hello);
  let packed = unpack(exports.deflate_stored_encode(inPtr, hello.length, outPtr, 64));
  assert.deepStrictEqual(packed, { status: 0, written: encoded.length });
  assert.deepStrictEqual(Array.from(memory.slice(outPtr, outPtr + packed.written)), Array.from(encoded));

  const large = Buffer.alloc(70000);
  for (let i = 0; i < large.length; i += 1) large[i] = i & 0xff;
  write(memory, inPtr, large);
  packed = unpack(exports.deflate_stored_encode(inPtr, large.length, outPtr, large.length + 16));
  assert.deepStrictEqual(packed, { status: 0, written: large.length + 10 });
  assert.strictEqual(memory[outPtr], 0);
  assert.strictEqual(view.getUint16(outPtr + 1, true), 65535);
  assert.strictEqual(view.getUint16(outPtr + 3, true), 0);
  assert.strictEqual(memory[outPtr + 5 + 65535], 1);
  assert.strictEqual(view.getUint16(outPtr + 5 + 65535 + 1, true), 4465);
  assert.strictEqual(view.getUint16(outPtr + 5 + 65535 + 3, true), 65535 ^ 4465);
  const decodedLarge = Buffer.concat([
    Buffer.from(memory.slice(outPtr + 5, outPtr + 5 + 65535)),
    Buffer.from(memory.slice(outPtr + 5 + 65535 + 5, outPtr + packed.written)),
  ]);
  assert.deepStrictEqual(Array.from(decodedLarge), Array.from(large));
  assert.strictEqual(exports.deflate_stored_scan(outPtr, packed.written, recPtr), 0);
  assert.deepStrictEqual(record(view, recPtr), {
    blockCount: 2,
    payloadTotal: large.length,
    consumed: large.length + 10,
    finalSeen: 1,
  });

  write(memory, inPtr, Buffer.from([0x01, 0x01, 0x00, 0xff, 0xff, 0x41]));
  assert.strictEqual(exports.deflate_stored_scan(inPtr, 6, recPtr), 3);

  write(memory, inPtr, Buffer.from([0x01, 0x01, 0x00]));
  assert.strictEqual(exports.deflate_stored_scan(inPtr, 3, recPtr), 1);

  write(memory, inPtr, Buffer.from([0x01, 0x05, 0x00, 0xfa, 0xff, 0x41]));
  assert.strictEqual(exports.deflate_stored_scan(inPtr, 6, recPtr), 1);

  write(memory, inPtr, Buffer.from([0x07]));
  assert.strictEqual(exports.deflate_stored_scan(inPtr, 1, recPtr), 3);

  write(memory, inPtr, Buffer.from([0x03]));
  assert.strictEqual(exports.deflate_stored_scan(inPtr, 1, recPtr), 7);

  write(memory, inPtr, Buffer.from([0x00, 0x00, 0x00, 0xff, 0xff]));
  assert.strictEqual(exports.deflate_stored_scan(inPtr, 5, recPtr), 1);

  write(memory, inPtr, hello);
  assert.deepStrictEqual(unpack(exports.deflate_stored_encode(inPtr, hello.length, outPtr, 8)), {
    status: 2,
    written: 0,
  });

  console.log(
    JSON.stringify(
      {
        unit: "deflate-stored",
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
