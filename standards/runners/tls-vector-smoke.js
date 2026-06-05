#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/tls-vector.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  return execFileSync("wat2wasm", [file, "-o", "-"]);
}

function u32(memory, offset) {
  return (
    memory[offset] |
    (memory[offset + 1] << 8) |
    (memory[offset + 2] << 16) |
    (memory[offset + 3] << 24)
  ) >>> 0;
}

function span3(memory, offset) {
  return {
    dataOffset: u32(memory, offset),
    dataLen: u32(memory, offset + 4),
    nextOffset: u32(memory, offset + 8),
  };
}

function extRecord(memory, offset) {
  return {
    type: u32(memory, offset),
    dataOffset: u32(memory, offset + 4),
    dataLen: u32(memory, offset + 8),
    nextOffset: u32(memory, offset + 12),
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

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const exports = instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300013);

  memory.set([3, 0xaa, 0xbb, 0xcc], 1024);
  assert.strictEqual(exports.tls_vector_u8_decode(1024, 4, 2048), 0);
  assert.deepStrictEqual(span3(memory, 2048), { dataOffset: 1, dataLen: 3, nextOffset: 4 });
  assert.strictEqual(exports.tls_vector_u8_decode(1024, 0, 2048), 1);
  memory.set([4, 0xaa, 0xbb], 1024);
  assert.strictEqual(exports.tls_vector_u8_decode(1024, 3, 2048), 5);

  memory.set([0, 2, 0xaa, 0xbb], 1024);
  assert.strictEqual(exports.tls_vector_u16_decode(1024, 4, 2048), 0);
  assert.deepStrictEqual(span3(memory, 2048), { dataOffset: 2, dataLen: 2, nextOffset: 4 });
  assert.strictEqual(exports.tls_vector_u16_decode(1024, 1, 2048), 1);
  memory.set([0, 3, 0xaa], 1024);
  assert.strictEqual(exports.tls_vector_u16_decode(1024, 3, 2048), 5);

  memory.set([0, 0, 1, 0xaa], 1024);
  assert.strictEqual(exports.tls_vector_u24_decode(1024, 4, 2048), 0);
  assert.deepStrictEqual(span3(memory, 2048), { dataOffset: 3, dataLen: 1, nextOffset: 4 });
  assert.strictEqual(exports.tls_vector_u24_decode(1024, 2, 2048), 1);
  memory.set([0, 0, 2, 0xaa], 1024);
  assert.strictEqual(exports.tls_vector_u24_decode(1024, 4, 2048), 5);

  // Extension list: SNI (0) with a host_name entry, supported_versions (43) with TLS 1.3.
  const sniData = [0, 10, 0, 0, 7, ...Buffer.from("example")];
  const versionsData = [2, 0x03, 0x04];
  const extensions = [
    0, 0, 0, sniData.length, ...sniData,
    0, 43, 0, versionsData.length, ...versionsData,
  ];
  memory.set(extensions, 1024);
  assert.strictEqual(exports.tls_extension_next(1024, extensions.length, 0, 2048), 0);
  let ext = extRecord(memory, 2048);
  assert.deepStrictEqual(ext, { type: 0, dataOffset: 4, dataLen: sniData.length, nextOffset: 4 + sniData.length });
  assert.strictEqual(exports.tls_extension_next(1024, extensions.length, ext.nextOffset, 2048), 0);
  ext = extRecord(memory, 2048);
  assert.deepStrictEqual(ext, {
    type: 43,
    dataOffset: 4 + sniData.length + 4,
    dataLen: versionsData.length,
    nextOffset: extensions.length,
  });
  assert.strictEqual(exports.tls_extension_next(1024, extensions.length, extensions.length, 2048), 1);

  memory.set([0, 16, 0, 5, 0, 3, 2, 0x68], 1024);
  assert.strictEqual(exports.tls_extension_next(1024, 8, 0, 2048), 5);

  const alpn = [0, 14, 2, 0x68, 0x33, 10, ...Buffer.from("acme-tls/1")];
  memory.set(alpn, 1024);
  assert.strictEqual(exports.tls_alpn_next(1024, alpn.length, 0, 2048), 0);
  let proto = alpnRecord(memory, 2048);
  assert.deepStrictEqual(proto, { protoOffset: 3, protoLen: 2, nextOffset: 5, listLen: 14 });
  assert.strictEqual(Buffer.from(memory.slice(1024 + proto.protoOffset, 1024 + proto.protoOffset + proto.protoLen)).toString(), "h3");
  assert.strictEqual(exports.tls_alpn_next(1024, alpn.length, proto.nextOffset, 2048), 0);
  proto = alpnRecord(memory, 2048);
  assert.deepStrictEqual(proto, { protoOffset: 6, protoLen: 10, nextOffset: 16, listLen: 14 });
  assert.strictEqual(Buffer.from(memory.slice(1024 + proto.protoOffset, 1024 + proto.protoOffset + proto.protoLen)).toString(), "acme-tls/1");

  memory.set([0, 3, 0, 0, 0], 1024);
  assert.strictEqual(exports.tls_alpn_next(1024, 5, 0, 2048), 3);
  memory.set([0, 4, 2, 0x68, 0x33], 1024);
  assert.strictEqual(exports.tls_alpn_next(1024, 5, 0, 2048), 5);

  console.log(JSON.stringify({
    unit: "tls-vector",
    abi_version: exports.proto_abi_version(),
    standard_id: exports.proto_standard_id(),
    ok: true,
  }, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
