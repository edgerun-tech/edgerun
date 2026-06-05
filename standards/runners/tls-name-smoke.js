#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/tls-name.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  return execFileSync("wat2wasm", [file, "-o", "-"]);
}

function low32High32(value) {
  return {
    status: Number(value & 0xffffffffn),
    written: Number((value >> 32n) & 0xffffffffn),
  };
}

function writeAscii(memory, offset, text) {
  const bytes = Buffer.from(text, "ascii");
  memory.set(bytes, offset);
  return bytes.length;
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const exports = instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300014);

  let len = writeAscii(memory, 1024, "Example.COM.");
  let packed = exports.tls_dns_name_normalize(1024, len, 2048, 256);
  assert.deepStrictEqual(low32High32(packed), { status: 0, written: 11 });
  assert.strictEqual(Buffer.from(memory.slice(2048, 2048 + 11)).toString("ascii"), "example.com");

  packed = exports.tls_dns_name_normalize(1024, len, 2048, 4);
  assert.deepStrictEqual(low32High32(packed), { status: 2, written: 0 });

  for (const bad of [
    "localhost",
    "127.0.0.1",
    "::1",
    "bad label.example",
    "-bad.example",
    "bad-.example",
    "bad..example",
    "exa\tmple.com",
  ]) {
    len = writeAscii(memory, 1024, bad);
    packed = exports.tls_dns_name_normalize(1024, len, 2048, 256);
    assert.strictEqual(low32High32(packed).status, 3, bad);
  }

  let patternLen = writeAscii(memory, 1024, "example.com");
  let hostLen = writeAscii(memory, 1536, "Example.COM.");
  assert.strictEqual(exports.tls_dns_name_matches(1024, patternLen, 1536, hostLen), 1);

  patternLen = writeAscii(memory, 1024, "*.example.com");
  hostLen = writeAscii(memory, 1536, "www.example.com");
  assert.strictEqual(exports.tls_dns_name_matches(1024, patternLen, 1536, hostLen), 1);

  hostLen = writeAscii(memory, 1536, "a.b.example.com");
  assert.strictEqual(exports.tls_dns_name_matches(1024, patternLen, 1536, hostLen), 0);

  hostLen = writeAscii(memory, 1536, "example.com");
  assert.strictEqual(exports.tls_dns_name_matches(1024, patternLen, 1536, hostLen), 0);

  patternLen = writeAscii(memory, 1024, "*.*.example.com");
  hostLen = writeAscii(memory, 1536, "www.example.com");
  assert.strictEqual(exports.tls_dns_name_matches(1024, patternLen, 1536, hostLen), 0);

  patternLen = writeAscii(memory, 1024, "*.com");
  hostLen = writeAscii(memory, 1536, "example.com");
  assert.strictEqual(exports.tls_dns_name_matches(1024, patternLen, 1536, hostLen), 0);

  hostLen = writeAscii(memory, 1536, "127.0.0.1");
  patternLen = writeAscii(memory, 1024, "*.0.0.1");
  assert.strictEqual(exports.tls_dns_name_matches(1024, patternLen, 1536, hostLen), 0);

  console.log(JSON.stringify({
    unit: "tls-name",
    abi_version: exports.proto_abi_version(),
    standard_id: exports.proto_standard_id(),
    ok: true,
  }, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
