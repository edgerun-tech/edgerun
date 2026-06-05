#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/http1-header-block.wat";

function compileWat(file) {
  const wasmPath = path.join(os.tmpdir(), `edgerun-${path.basename(file)}-${process.pid}.wasm`);
  execFileSync("wat2wasm", [file, "-o", wasmPath], { stdio: "pipe" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "pipe" });
  const bytes = fs.readFileSync(wasmPath);
  fs.unlinkSync(wasmPath);
  return bytes;
}

function write(memory, offset, text) {
  const bytes = Buffer.from(text, "ascii");
  memory.fill(0, offset, offset + Math.max(bytes.length, 256));
  memory.set(bytes, offset);
  return bytes.length;
}

function readSummary(view, out) {
  return {
    headerCount: view.getUint32(out, true),
    bytesConsumed: view.getUint32(out + 4, true),
    hostCount: view.getUint32(out + 8, true),
    contentLengthCount: view.getUint32(out + 12, true),
    transferEncodingCount: view.getUint32(out + 16, true),
    flags: view.getUint32(out + 20, true),
    contentLength:
      BigInt(view.getUint32(out + 24, true)) |
      (BigInt(view.getUint32(out + 28, true)) << 32n),
  };
}

(async () => {
  const module = await WebAssembly.instantiate(compileWat(watPath), {});
  const exports = module.instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);
  const view = new DataView(exports.memory.buffer);
  const ptr = 1024;
  const out = 4096;

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300101);

  let text =
    "Host: example.com\r\n" +
    "Content-Length: 5\r\n" +
    "content-length: 5\r\n" +
    "Transfer-Encoding: gzip, Chunked\r\n" +
    "\r\n";
  let len = write(memory, ptr, text);
  assert.strictEqual(exports.http1_validate_header_block(ptr, len, 16, 1024, out), 0);
  assert.deepStrictEqual(readSummary(view, out), {
    headerCount: 4,
    bytesConsumed: len,
    hostCount: 1,
    contentLengthCount: 2,
    transferEncodingCount: 1,
    flags: 15,
    contentLength: 5n,
  });

  text = "Server: edge\r\nDate: now\r\n\r\n";
  len = write(memory, ptr, text);
  assert.strictEqual(exports.http1_validate_header_block(ptr, len, 16, 1024, out), 0);
  assert.deepStrictEqual(readSummary(view, out), {
    headerCount: 2,
    bytesConsumed: len,
    hostCount: 0,
    contentLengthCount: 0,
    transferEncodingCount: 0,
    flags: 0,
    contentLength: 0n,
  });

  len = write(memory, ptr, "Host: a\r\nHost: b\r\n\r\n");
  assert.strictEqual(exports.http1_validate_header_block(ptr, len, 16, 1024, out), 3);

  len = write(memory, ptr, "Content-Length: 5\r\nContent-Length: 6\r\n\r\n");
  assert.strictEqual(exports.http1_validate_header_block(ptr, len, 16, 1024, out), 3);

  len = write(memory, ptr, "Content-Length: 18446744073709551616\r\n\r\n");
  assert.strictEqual(exports.http1_validate_header_block(ptr, len, 16, 1024, out), 4);

  len = write(memory, ptr, "Host: a\r\n X-Folded: b\r\n\r\n");
  assert.strictEqual(exports.http1_validate_header_block(ptr, len, 16, 1024, out), 3);

  len = write(memory, ptr, "Bad/Name: x\r\n\r\n");
  assert.strictEqual(exports.http1_validate_header_block(ptr, len, 16, 1024, out), 3);

  memory.fill(0, ptr, ptr + 16);
  memory[ptr] = "X".charCodeAt(0);
  memory[ptr + 1] = ":".charCodeAt(0);
  memory[ptr + 2] = " ".charCodeAt(0);
  memory[ptr + 3] = 0x80;
  memory[ptr + 4] = 13;
  memory[ptr + 5] = 10;
  memory[ptr + 6] = 13;
  memory[ptr + 7] = 10;
  assert.strictEqual(exports.http1_validate_header_block(ptr, 8, 16, 1024, out), 3);

  len = write(memory, ptr, "A: 1\r\nB: 2\r\n\r\n");
  assert.strictEqual(exports.http1_validate_header_block(ptr, len, 1, 1024, out), 6);
  assert.strictEqual(exports.http1_validate_header_block(ptr, len, 16, len - 1, out), 6);

  len = write(memory, ptr, "Host: example.com\r\n");
  assert.strictEqual(exports.http1_validate_header_block(ptr, len, 16, 1024, out), 1);

  console.log(
    JSON.stringify(
      {
        unit: "http1-header-block",
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
