#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/http1-scan.wat";

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

function cstr(memory, offset, text) {
  const bytes = Buffer.from(text, "utf8");
  memory.set(bytes, offset);
  return bytes.length;
}

(async () => {
  const module = await WebAssembly.instantiate(compileWat(watPath), {});
  const exports = module.instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);
  const view = new DataView(exports.memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300003);

  const chunkPtr = 1024;
  const chunkLen = cstr(memory, chunkPtr, "5\r\nHello\r\n0\r\n\r\n");
  const outPtr = 2048;

  let status = exports.http_parse_chunk_step(chunkPtr, chunkLen, 0, outPtr);
  assert.strictEqual(status, 0);
  assert.strictEqual(view.getUint32(outPtr, true), 5);
  assert.strictEqual(view.getUint32(outPtr + 4, true), 3);
  assert.strictEqual(view.getUint32(outPtr + 8, true), 5);
  assert.strictEqual(view.getUint32(outPtr + 12, true), 10);
  assert.strictEqual(view.getUint32(outPtr + 16, true), 0);

  status = exports.http_parse_chunk_step(chunkPtr, chunkLen, 10, outPtr);
  assert.strictEqual(status, 0);
  assert.strictEqual(view.getUint32(outPtr, true), 0);
  assert.strictEqual(view.getUint32(outPtr + 12, true), chunkLen);
  assert.strictEqual(view.getUint32(outPtr + 16, true), 1);

  let found = unpack(exports.http_find_crlf(chunkPtr, chunkLen, 0));
  assert.deepStrictEqual(found, { status: 0, value: 1 });
  found = unpack(exports.http_find_double_crlf(chunkPtr, chunkLen, 0));
  assert.deepStrictEqual(found, { status: 0, value: 11 });

  const valuePtr = 4096;
  const tokenPtr = 4200;
  const valueLen = cstr(memory, valuePtr, "gzip, Chunked");
  const tokenLen = cstr(memory, tokenPtr, "chunked");
  assert.strictEqual(exports.http_value_has_token(valuePtr, valueLen, tokenPtr, tokenLen), 0);

  const badLen = cstr(memory, valuePtr, "xchunked");
  assert.strictEqual(exports.http_value_has_token(valuePtr, badLen, tokenPtr, tokenLen), 3);

  assert.strictEqual(exports.http_validate_header_name(tokenPtr, tokenLen), 0);
  assert.strictEqual(exports.http_validate_header_value(valuePtr, badLen), 0);
  const invalidNameLen = cstr(memory, tokenPtr, "bad/name");
  assert.strictEqual(exports.http_validate_header_name(tokenPtr, invalidNameLen), 3);
  memory[valuePtr] = 0x80;
  assert.strictEqual(exports.http_validate_header_value(valuePtr, 1), 3);

  console.log(
    JSON.stringify(
      {
        unit: "http1-scan",
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
