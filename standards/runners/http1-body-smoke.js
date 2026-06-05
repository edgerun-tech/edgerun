#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/http1-body.wat";

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

function outU64(view, out) {
  return view.getBigUint64(out, true);
}

(async () => {
  const module = await WebAssembly.instantiate(compileWat(watPath), {});
  const exports = module.instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);
  const view = new DataView(exports.memory.buffer);
  const ptr = 1024;
  const token = 2048;
  const out = 4096;

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300009);

  let len = write(memory, ptr, "0");
  assert.strictEqual(exports.http_parse_content_length(ptr, len, out), 0);
  assert.strictEqual(outU64(view, out), 0n);

  len = write(memory, ptr, " 5\t");
  assert.strictEqual(exports.http_parse_content_length(ptr, len, out), 0);
  assert.strictEqual(outU64(view, out), 5n);

  len = write(memory, ptr, "18446744073709551615");
  assert.strictEqual(exports.http_parse_content_length(ptr, len, out), 0);
  assert.strictEqual(outU64(view, out), 18446744073709551615n);

  len = write(memory, ptr, "18446744073709551616");
  assert.strictEqual(exports.http_parse_content_length(ptr, len, out), 4);

  len = write(memory, ptr, "12x");
  assert.strictEqual(exports.http_parse_content_length(ptr, len, out), 3);

  len = write(memory, ptr, "gzip, Chunked");
  const tokenLen = write(memory, token, "chunked");
  assert.strictEqual(exports.http_has_transfer_token(ptr, len, token, tokenLen), 0);

  len = write(memory, ptr, "xchunked");
  assert.strictEqual(exports.http_has_transfer_token(ptr, len, token, tokenLen), 3);

  len = write(memory, ptr, "Content-Length: 0\r\n\r\n");
  assert.strictEqual(exports.http_classify_body_framing(ptr, len, out), 0);
  assert.strictEqual(view.getUint32(out, true), 1);
  assert.strictEqual(outU64(view, out + 4), 0n);
  assert.strictEqual(view.getUint32(out + 12, true), 1);

  len = write(memory, ptr, "Content-Length: 5\r\n\r\n");
  assert.strictEqual(exports.http_classify_body_framing(ptr, len, out), 0);
  assert.strictEqual(view.getUint32(out, true), 1);
  assert.strictEqual(outU64(view, out + 4), 5n);

  len = write(memory, ptr, "Content-Length: 5\r\ncontent-length: 5\r\n\r\n");
  assert.strictEqual(exports.http_classify_body_framing(ptr, len, out), 0);
  assert.strictEqual(view.getUint32(out, true), 1);
  assert.strictEqual(outU64(view, out + 4), 5n);
  assert.strictEqual(view.getUint32(out + 12, true), 2);

  len = write(memory, ptr, "Content-Length: 5\r\nContent-Length: 6\r\n\r\n");
  assert.strictEqual(exports.http_classify_body_framing(ptr, len, out), 3);

  len = write(memory, ptr, "Transfer-Encoding: gzip, Chunked\r\nContent-Length: 5\r\n\r\n");
  assert.strictEqual(exports.http_classify_body_framing(ptr, len, out), 0);
  assert.strictEqual(view.getUint32(out, true), 2);
  assert.strictEqual(outU64(view, out + 4), 5n);

  len = write(memory, ptr, "Transfer-Encoding: gzip, xchunked\r\n\r\n");
  assert.strictEqual(exports.http_classify_body_framing(ptr, len, out), 0);
  assert.strictEqual(view.getUint32(out, true), 0);

  len = write(memory, ptr, "Content-Length: 18446744073709551616\r\n\r\n");
  assert.strictEqual(exports.http_classify_body_framing(ptr, len, out), 4);

  len = write(memory, ptr, "Bad/Name: 1\r\n\r\n");
  assert.strictEqual(exports.http_classify_body_framing(ptr, len, out), 3);

  console.log(
    JSON.stringify(
      {
        unit: "http1-body",
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
