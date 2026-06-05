#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/http1-lines.wat";

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
  memory.set(bytes, offset);
  return bytes.length;
}

(async () => {
  const module = await WebAssembly.instantiate(compileWat(watPath), {});
  const exports = module.instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);
  const view = new DataView(exports.memory.buffer);
  const ptr = 1024;
  const out = 4096;

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300008);

  let len = write(memory, ptr, "GET /x?q=1 HTTP/1.1\r\nHost: example.com\r\n\r\n");
  assert.strictEqual(exports.http_parse_request_line(ptr, len, out), 0);
  assert.strictEqual(view.getUint32(out, true), 0);
  assert.strictEqual(view.getUint32(out + 4, true), 3);
  assert.strictEqual(view.getUint32(out + 8, true), 4);
  assert.strictEqual(view.getUint32(out + 12, true), 6);
  assert.strictEqual(view.getUint32(out + 16, true), 1);
  assert.strictEqual(view.getUint32(out + 20, true), 1);
  assert.strictEqual(view.getUint32(out + 24, true), 21);

  assert.strictEqual(exports.http_next_header(ptr, len, 21, out), 0);
  assert.strictEqual(view.getUint32(out, true), 21);
  assert.strictEqual(view.getUint32(out + 4, true), 4);
  assert.strictEqual(view.getUint32(out + 8, true), 27);
  assert.strictEqual(view.getUint32(out + 12, true), 11);
  assert.strictEqual(view.getUint32(out + 16, true), 40);
  assert.strictEqual(view.getUint32(out + 20, true), 0);

  assert.strictEqual(exports.http_next_header(ptr, len, 40, out), 0);
  assert.strictEqual(view.getUint32(out + 20, true), 1);

  len = write(memory, ptr, "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n");
  assert.strictEqual(exports.http_parse_status_line(ptr, len, out), 0);
  assert.strictEqual(view.getUint32(out, true), 1);
  assert.strictEqual(view.getUint32(out + 4, true), 1);
  assert.strictEqual(view.getUint32(out + 8, true), 200);
  assert.strictEqual(view.getUint32(out + 12, true), 13);
  assert.strictEqual(view.getUint32(out + 16, true), 2);
  assert.strictEqual(view.getUint32(out + 20, true), 17);

  len = write(memory, ptr, "GET/x HTTP/1.1\r\n");
  assert.strictEqual(exports.http_parse_request_line(ptr, len, out), 3);

  len = write(memory, ptr, "GET /x HTTX/1.1\r\n");
  assert.strictEqual(exports.http_parse_request_line(ptr, len, out), 3);

  len = write(memory, ptr, "GET /x HTTP/1\r\n");
  assert.strictEqual(exports.http_parse_request_line(ptr, len, out), 3);

  len = write(memory, ptr, "GET /x HTTP/1.1");
  assert.strictEqual(exports.http_parse_request_line(ptr, len, out), 1);

  len = write(memory, ptr, "HTTP/1.1 099 Nope\r\n");
  assert.strictEqual(exports.http_parse_status_line(ptr, len, out), 3);

  len = write(memory, ptr, "HTTP/1.1 200OK\r\n");
  assert.strictEqual(exports.http_parse_status_line(ptr, len, out), 3);

  len = write(memory, ptr, "Bad/Name: x\r\n");
  assert.strictEqual(exports.http_next_header(ptr, len, 0, out), 3);

  len = write(memory, ptr, ": x\r\n");
  assert.strictEqual(exports.http_next_header(ptr, len, 0, out), 3);

  memory.fill(0, ptr, ptr + 16);
  memory[ptr] = "X".charCodeAt(0);
  memory[ptr + 1] = ":".charCodeAt(0);
  memory[ptr + 2] = " ".charCodeAt(0);
  memory[ptr + 3] = 0x80;
  memory[ptr + 4] = 13;
  memory[ptr + 5] = 10;
  assert.strictEqual(exports.http_next_header(ptr, 6, 0, out), 3);

  len = write(memory, ptr, "Host: example.com");
  assert.strictEqual(exports.http_next_header(ptr, len, 0, out), 1);

  console.log(
    JSON.stringify(
      {
        unit: "http1-lines",
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
