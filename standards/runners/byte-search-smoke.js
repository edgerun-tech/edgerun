#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/byte-search.wat";

function compileWat(file) {
  const wasmPath = path.join(os.tmpdir(), `edgerun-${path.basename(file)}-${process.pid}.wasm`);
  execFileSync("wat2wasm", [file, "-o", wasmPath], { stdio: "inherit" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "inherit" });
  const bytes = fs.readFileSync(wasmPath);
  fs.unlinkSync(wasmPath);
  return bytes;
}

function put(memory, ptr, value) {
  const bytes = Buffer.isBuffer(value) ? value : Buffer.from(value, "utf8");
  memory.fill(0, ptr, ptr + Math.max(256, bytes.length + 32));
  memory.set(bytes, ptr);
  return bytes.length;
}

(async () => {
  const { instance } = await WebAssembly.instantiate(compileWat(watPath), {});
  const exports = instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);
  const hayPtr = 1024;
  const needlePtr = 4096;
  const cases = [];

  assert.equal(exports.proto_abi_version(), 2);
  assert.equal(exports.proto_standard_id(), 300074);

  function check(name, fn, expected) {
    assert.equal(fn(), expected, name);
    cases.push(name);
  }

  let len = put(memory, hayPtr, "abcabc");
  check("memchr first byte", () => exports.memchr(hayPtr, len, "a".charCodeAt(0)), 0);
  check("memchr last byte", () => exports.memchr(hayPtr, len, "c".charCodeAt(0)), 2);
  check("memchr missing", () => exports.memchr(hayPtr, len, "z".charCodeAt(0)), -1);
  check("memrchr repeated", () => exports.memrchr(hayPtr, len, "a".charCodeAt(0)), 3);
  check("memrchr missing", () => exports.memrchr(hayPtr, len, "z".charCodeAt(0)), -1);

  len = put(memory, hayPtr, "");
  check("empty memchr", () => exports.memchr(hayPtr, len, 0), -1);
  check("empty memrchr", () => exports.memrchr(hayPtr, len, 0), -1);

  len = put(memory, hayPtr, Buffer.from([9, 8, 7, 6, 8, 7, 5]));
  check("memchr2 first of two", () => exports.memchr2(hayPtr, len, 7, 5), 2);
  check("memchr2 second needle first", () => exports.memchr2(hayPtr, len, 6, 8), 1);
  check("memrchr2 repeated", () => exports.memrchr2(hayPtr, len, 7, 8), 5);
  check("memchr2 missing", () => exports.memchr2(hayPtr, len, 1, 2), -1);
  check("memrchr2 missing", () => exports.memrchr2(hayPtr, len, 1, 2), -1);

  len = put(memory, hayPtr, Buffer.from([1, 2, 3, 4, 5, 4, 3, 2, 1]));
  check("memchr3 middle", () => exports.memchr3(hayPtr, len, 6, 5, 9), 4);
  check("memchr3 earlier alternative", () => exports.memchr3(hayPtr, len, 9, 3, 5), 2);
  check("memrchr3 reverse", () => exports.memrchr3(hayPtr, len, 2, 5, 9), 7);
  check("memchr3 missing", () => exports.memchr3(hayPtr, len, 6, 7, 8), -1);
  check("memrchr3 missing", () => exports.memrchr3(hayPtr, len, 6, 7, 8), -1);

  len = put(memory, hayPtr, "bananana");
  let needleLen = put(memory, needlePtr, "ana");
  check("memmem overlapping first", () => exports.memmem_find(hayPtr, len, needlePtr, needleLen), 1);
  check("memmem overlapping last", () => exports.memmem_rfind(hayPtr, len, needlePtr, needleLen), 5);

  needleLen = put(memory, needlePtr, "banana");
  check("memmem full prefix", () => exports.memmem_find(hayPtr, len, needlePtr, needleLen), 0);
  check("memmem reverse full prefix", () => exports.memmem_rfind(hayPtr, len, needlePtr, needleLen), 0);

  needleLen = put(memory, needlePtr, "nanaz");
  check("memmem missing", () => exports.memmem_find(hayPtr, len, needlePtr, needleLen), -1);
  check("memmem rmissing", () => exports.memmem_rfind(hayPtr, len, needlePtr, needleLen), -1);

  needleLen = put(memory, needlePtr, "");
  check("memmem empty find", () => exports.memmem_find(hayPtr, len, needlePtr, needleLen), 0);
  check("memmem empty rfind", () => exports.memmem_rfind(hayPtr, len, needlePtr, needleLen), len);

  needleLen = put(memory, needlePtr, "banananaz");
  check("memmem needle longer", () => exports.memmem_find(hayPtr, 3, needlePtr, needleLen), -1);
  check("memmem rneedle longer", () => exports.memmem_rfind(hayPtr, 3, needlePtr, needleLen), -1);

  console.log(
    JSON.stringify({
      unit: "byte-search",
      abi_version: exports.proto_abi_version(),
      standard_id: exports.proto_standard_id(),
      ok: true,
      cases,
    }),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
