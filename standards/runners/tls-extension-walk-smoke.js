#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/tls-extension-walk.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  return execFileSync("wat2wasm", [file, "-o", "-"]);
}

function u16(n) {
  return [(n >>> 8) & 0xff, n & 0xff];
}

function u32(memory, offset) {
  return (
    memory[offset] |
    (memory[offset + 1] << 8) |
    (memory[offset + 2] << 16) |
    (memory[offset + 3] << 24)
  ) >>> 0;
}

function walkRecord(memory, offset) {
  return {
    extensionCount: u32(memory, offset),
    firstSniDataOffset: u32(memory, offset + 4),
    firstSniDataLen: u32(memory, offset + 8),
    alpnPresent: u32(memory, offset + 12),
    finalOffset: u32(memory, offset + 16),
  };
}

function ext(type, data) {
  return [...u16(type), ...u16(data.length), ...data];
}

function sni(host) {
  const bytes = [...Buffer.from(host, "ascii")];
  const entry = [0, ...u16(bytes.length), ...bytes];
  return [...u16(entry.length), ...entry];
}

function alpn(protocols) {
  const list = protocols.flatMap((protocol) => {
    const bytes = [...Buffer.from(protocol, "ascii")];
    return [bytes.length, ...bytes];
  });
  return [...u16(list.length), ...list];
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const exports = instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300039);

  const sniPayload = sni("example.com");
  const alpnPayload = alpn(["h3", "acme-tls/1"]);
  const supportedVersionsPayload = [2, 0x03, 0x04];
  const extensions = [
    ...ext(43, supportedVersionsPayload),
    ...ext(0, sniPayload),
    ...ext(16, alpnPayload),
  ];
  memory.set(extensions, 1024);
  assert.strictEqual(exports.tls_extension_walk(1024, extensions.length, 2048), 0);
  assert.deepStrictEqual(walkRecord(memory, 2048), {
    extensionCount: 3,
    firstSniDataOffset: 4 + supportedVersionsPayload.length + 4,
    firstSniDataLen: sniPayload.length,
    alpnPresent: 1,
    finalOffset: extensions.length,
  });

  const sniBytes = memory.slice(1024 + u32(memory, 2048 + 4), 1024 + u32(memory, 2048 + 4) + u32(memory, 2048 + 8));
  assert.deepStrictEqual(Array.from(sniBytes), sniPayload);

  const noSni = [
    ...ext(10, [0, 2, 0, 23]),
    ...ext(16, alpn(["http/1.1"])),
  ];
  memory.set(noSni, 1024);
  assert.strictEqual(exports.tls_extension_walk(1024, noSni.length, 2048), 0);
  assert.deepStrictEqual(walkRecord(memory, 2048), {
    extensionCount: 2,
    firstSniDataOffset: 0,
    firstSniDataLen: 0,
    alpnPresent: 1,
    finalOffset: noSni.length,
  });

  memory.set([], 1024);
  assert.strictEqual(exports.tls_extension_walk(1024, 0, 2048), 0);
  assert.deepStrictEqual(walkRecord(memory, 2048), {
    extensionCount: 0,
    firstSniDataOffset: 0,
    firstSniDataLen: 0,
    alpnPresent: 0,
    finalOffset: 0,
  });

  memory.set([0, 0, 0], 1024);
  assert.strictEqual(exports.tls_extension_walk(1024, 3, 2048), 5);

  memory.set([0, 16, 0, 5, 0, 3, 2, 0x68], 1024);
  assert.strictEqual(exports.tls_extension_walk(1024, 8, 2048), 5);

  const trailing = [...ext(0, sni("a.test")), 0xaa, 0xbb];
  memory.set(trailing, 1024);
  assert.strictEqual(exports.tls_extension_walk(1024, trailing.length, 2048), 5);

  const duplicateSni = [
    ...ext(0, sni("first.test")),
    ...ext(0, sni("second.test")),
  ];
  memory.set(duplicateSni, 1024);
  assert.strictEqual(exports.tls_extension_walk(1024, duplicateSni.length, 2048), 0);
  const duplicateRecord = walkRecord(memory, 2048);
  assert.strictEqual(duplicateRecord.extensionCount, 2);
  assert.strictEqual(duplicateRecord.firstSniDataOffset, 4);
  assert.strictEqual(duplicateRecord.firstSniDataLen, sni("first.test").length);

  const emptyFirstSni = [
    ...ext(0, []),
    ...ext(0, sni("later.test")),
  ];
  memory.set(emptyFirstSni, 1024);
  assert.strictEqual(exports.tls_extension_walk(1024, emptyFirstSni.length, 2048), 0);
  assert.deepStrictEqual(walkRecord(memory, 2048), {
    extensionCount: 2,
    firstSniDataOffset: 4,
    firstSniDataLen: 0,
    alpnPresent: 0,
    finalOffset: emptyFirstSni.length,
  });

  console.log(JSON.stringify({
    unit: "tls-extension-walk",
    abi_version: exports.proto_abi_version(),
    standard_id: exports.proto_standard_id(),
    ok: true,
  }, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
