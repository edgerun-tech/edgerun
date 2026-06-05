#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/tls-clienthello.wat");

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

function clientHelloRecord(memory, offset) {
  return {
    legacyVersion: u32(memory, offset),
    randomOffset: u32(memory, offset + 4),
    randomLen: u32(memory, offset + 8),
    sessionOffset: u32(memory, offset + 12),
    sessionLen: u32(memory, offset + 16),
    cipherSuitesOffset: u32(memory, offset + 20),
    cipherSuitesLen: u32(memory, offset + 24),
    cipherSuiteCount: u32(memory, offset + 28),
    compressionOffset: u32(memory, offset + 32),
    compressionLen: u32(memory, offset + 36),
    extensionsOffset: u32(memory, offset + 40),
    extensionsLen: u32(memory, offset + 44),
  };
}

function span2(memory, offset) {
  return {
    offset: u32(memory, offset),
    len: u32(memory, offset + 4),
  };
}

function span4(memory, offset) {
  return {
    offset: u32(memory, offset),
    len: u32(memory, offset + 4),
    nextOffset: u32(memory, offset + 8),
    kind: u32(memory, offset + 12),
  };
}

function alpnRecord(memory, offset) {
  return {
    protoOffset: u32(memory, offset),
    protoLen: u32(memory, offset + 4),
    nextOffset: u32(memory, offset + 8),
    listLen: u32(memory, offset + 12),
  };
}

function ext(type, data) {
  return [...u16(type), ...u16(data.length), ...data];
}

function sni(host) {
  const h = [...Buffer.from(host)];
  const entry = [0, ...u16(h.length), ...h];
  return [...u16(entry.length), ...entry];
}

function alpn(protocols) {
  const list = protocols.flatMap((p) => {
    const bytes = [...Buffer.from(p)];
    return [bytes.length, ...bytes];
  });
  return [...u16(list.length), ...list];
}

function minimalClientHello({ badCipherLen = false, badExtensionLen = false } = {}) {
  const random = Array.from({ length: 32 }, (_, i) => i);
  const session = [];
  const ciphers = badCipherLen ? [0x13] : [0x13, 0x01, 0x13, 0x02];
  const compression = [0];
  const extensions = [
    ...ext(0, sni("example.com")),
    ...ext(16, alpn(["h3", "acme-tls/1"])),
  ];
  if (badExtensionLen) {
    extensions[2] = 0xff;
    extensions[3] = 0xff;
  }
  return [
    0x03, 0x03,
    ...random,
    session.length, ...session,
    ...u16(ciphers.length), ...ciphers,
    compression.length, ...compression,
    ...u16(extensions.length), ...extensions,
  ];
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const exports = instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300023);

  const body = minimalClientHello();
  memory.set(body, 1024);
  assert.strictEqual(exports.tls_clienthello_scan(1024, body.length, 2048), 0);
  const ch = clientHelloRecord(memory, 2048);
  assert.deepStrictEqual(ch, {
    legacyVersion: 0x0303,
    randomOffset: 2,
    randomLen: 32,
    sessionOffset: 35,
    sessionLen: 0,
    cipherSuitesOffset: 37,
    cipherSuitesLen: 4,
    cipherSuiteCount: 2,
    compressionOffset: 42,
    compressionLen: 1,
    extensionsOffset: 45,
    extensionsLen: body.length - 45,
  });

  assert.strictEqual(exports.tls_clienthello_find_extension(1024 + ch.extensionsOffset, ch.extensionsLen, 0, 2048), 0);
  let extRec = span4(memory, 2048);
  assert.deepStrictEqual(extRec, { offset: 4, len: 16, nextOffset: 20, kind: 0 });
  assert.strictEqual(exports.tls_clienthello_sni_host(1024 + ch.extensionsOffset + extRec.offset, extRec.len, 2048), 0);
  let host = span2(memory, 2048);
  assert.strictEqual(
    Buffer.from(memory.slice(1024 + ch.extensionsOffset + extRec.offset + host.offset, 1024 + ch.extensionsOffset + extRec.offset + host.offset + host.len)).toString(),
    "example.com"
  );

  assert.strictEqual(exports.tls_clienthello_find_extension(1024 + ch.extensionsOffset, ch.extensionsLen, 16, 2048), 0);
  extRec = span4(memory, 2048);
  assert.strictEqual(exports.tls_clienthello_alpn_next(1024 + ch.extensionsOffset + extRec.offset, extRec.len, 0, 2048), 0);
  let proto = alpnRecord(memory, 2048);
  assert.strictEqual(Buffer.from(memory.slice(1024 + ch.extensionsOffset + extRec.offset + proto.protoOffset, 1024 + ch.extensionsOffset + extRec.offset + proto.protoOffset + proto.protoLen)).toString(), "h3");
  assert.strictEqual(exports.tls_clienthello_alpn_next(1024 + ch.extensionsOffset + extRec.offset, extRec.len, proto.nextOffset, 2048), 0);
  proto = alpnRecord(memory, 2048);
  assert.strictEqual(Buffer.from(memory.slice(1024 + ch.extensionsOffset + extRec.offset + proto.protoOffset, 1024 + ch.extensionsOffset + extRec.offset + proto.protoOffset + proto.protoLen)).toString(), "acme-tls/1");

  assert.strictEqual(exports.tls_clienthello_find_extension(1024 + ch.extensionsOffset, ch.extensionsLen, 43, 2048), 3);

  assert.strictEqual(exports.tls_clienthello_scan(1024, 10, 2048), 1);

  const oddCipher = minimalClientHello({ badCipherLen: true });
  memory.set(oddCipher, 1024);
  assert.strictEqual(exports.tls_clienthello_scan(1024, oddCipher.length, 2048), 3);

  const badExt = minimalClientHello({ badExtensionLen: true });
  memory.set(badExt, 1024);
  assert.strictEqual(exports.tls_clienthello_scan(1024, badExt.length, 2048), 0);
  const badExtScan = clientHelloRecord(memory, 2048);
  assert.strictEqual(exports.tls_clienthello_find_extension(1024 + badExtScan.extensionsOffset, badExtScan.extensionsLen, 0, 2048), 5);

  const truncatedSession = minimalClientHello();
  truncatedSession[34] = 8;
  memory.set(truncatedSession.slice(0, 40), 1024);
  assert.strictEqual(exports.tls_clienthello_scan(1024, 40, 2048), 5);

  memory.set([0, 5, 0, 0, 10], 1024);
  assert.strictEqual(exports.tls_clienthello_sni_host(1024, 5, 2048), 5);

  console.log(JSON.stringify({
    unit: "tls-clienthello",
    abi_version: exports.proto_abi_version(),
    standard_id: exports.proto_standard_id(),
    ok: true,
  }, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
