#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/tls-certificate-list.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  return execFileSync("wat2wasm", [file, "-o", "-"]);
}

function u16(n) {
  return [(n >>> 8) & 0xff, n & 0xff];
}

function u24(n) {
  return [(n >>> 16) & 0xff, (n >>> 8) & 0xff, n & 0xff];
}

function u32(memory, offset) {
  return (
    memory[offset] |
    (memory[offset + 1] << 8) |
    (memory[offset + 2] << 16) |
    (memory[offset + 3] << 24)
  ) >>> 0;
}

function listRecord(memory, offset) {
  return {
    contextOffset: u32(memory, offset),
    contextLen: u32(memory, offset + 4),
    listOffset: u32(memory, offset + 8),
    listLen: u32(memory, offset + 12),
    nextOffset: u32(memory, offset + 16),
  };
}

function entryRecord(memory, offset) {
  return {
    certOffset: u32(memory, offset),
    certLen: u32(memory, offset + 4),
    extensionsOffset: u32(memory, offset + 8),
    extensionsLen: u32(memory, offset + 12),
    nextOffset: u32(memory, offset + 16),
  };
}

function certBody(certDer, extensions = []) {
  const entry = [...u24(certDer.length), ...certDer, ...u16(extensions.length), ...extensions];
  return [0, ...u24(entry.length), ...entry];
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const exports = instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300024);

  const der = [0x30, 0x03, 0x02, 0x01, 0x01];
  const body = certBody(der);
  memory.set(body, 1024);
  assert.strictEqual(exports.tls_certificate_list_scan(1024, body.length, 2048), 0);
  const list = listRecord(memory, 2048);
  assert.deepStrictEqual(list, {
    contextOffset: 1,
    contextLen: 0,
    listOffset: 4,
    listLen: 10,
    nextOffset: body.length,
  });

  assert.strictEqual(exports.tls_certificate_entry_next(1024 + list.listOffset, list.listLen, 0, 2048), 0);
  let entry = entryRecord(memory, 2048);
  assert.deepStrictEqual(entry, {
    certOffset: 3,
    certLen: der.length,
    extensionsOffset: 10,
    extensionsLen: 0,
    nextOffset: 10,
  });
  assert.strictEqual(Buffer.from(memory.slice(1024 + list.listOffset + entry.certOffset, 1024 + list.listOffset + entry.certOffset + entry.certLen)).toString("hex"), "3003020101");
  assert.strictEqual(exports.tls_certificate_entry_next(1024 + list.listOffset, list.listLen, entry.nextOffset, 2048), 1);

  assert.strictEqual(exports.tls_certificate_list_scan(1024, 2, 2048), 1);

  const truncatedList = body.slice(0, body.length - 1);
  memory.set(truncatedList, 1024);
  assert.strictEqual(exports.tls_certificate_list_scan(1024, truncatedList.length, 2048), 5);

  const truncatedEntry = [...u24(der.length), ...der.slice(0, 2)];
  memory.set(truncatedEntry, 1024);
  assert.strictEqual(exports.tls_certificate_entry_next(1024, truncatedEntry.length, 0, 2048), 5);

  const badExt = [...u24(der.length), ...der, 0, 3, 0xaa];
  memory.set(badExt, 1024);
  assert.strictEqual(exports.tls_certificate_entry_next(1024, badExt.length, 0, 2048), 5);

  const emptyCert = [...u24(0), 0, 0];
  memory.set(emptyCert, 1024);
  assert.strictEqual(exports.tls_certificate_entry_next(1024, emptyCert.length, 0, 2048), 3);

  console.log(JSON.stringify({
    unit: "tls-certificate-list",
    abi_version: exports.proto_abi_version(),
    standard_id: exports.proto_standard_id(),
    ok: true,
  }, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
