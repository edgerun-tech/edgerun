#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/tls-frame.wat");

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

function tlsRecordParts(value) {
  return {
    status: Number(value & 0xffffn),
    contentType: Number((value >> 16n) & 0xffn),
    version: Number((value >> 24n) & 0xffffn),
    fragmentLen: Number((value >> 40n) & 0xffffn),
  };
}

function tlsHandshakeParts(value) {
  return {
    status: Number(value & 0xffffn),
    handshakeType: Number((value >> 16n) & 0xffn),
    bodyLen: Number((value >> 24n) & 0xffffffn),
  };
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const exports = instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300006);

  memory.set([0x17, 0x03, 0x03, 0x00, 0x05], 1024);
  assert.deepStrictEqual(tlsRecordParts(exports.tls_record_header_decode(1024, 5)), {
    status: 0,
    contentType: 0x17,
    version: 0x0303,
    fragmentLen: 5,
  });

  let packed = exports.tls_record_header_encode(0x17, 0x0303, 5, 2048, 5);
  assert.deepStrictEqual(low32High32(packed), { status: 0, written: 5 });
  assert.deepStrictEqual(Array.from(memory.slice(2048, 2053)), [0x17, 0x03, 0x03, 0x00, 0x05]);

  memory.set([0x01, 0x00, 0x00, 0x20], 1024);
  assert.deepStrictEqual(tlsHandshakeParts(exports.tls_handshake_header_decode(1024, 4)), {
    status: 0,
    handshakeType: 0x01,
    bodyLen: 0x20,
  });

  packed = exports.tls_handshake_header_encode(0x01, 0x20, 2048, 4);
  assert.deepStrictEqual(low32High32(packed), { status: 0, written: 4 });
  assert.deepStrictEqual(Array.from(memory.slice(2048, 2052)), [0x01, 0x00, 0x00, 0x20]);

  assert.strictEqual(Number(exports.tls_record_header_decode(1024, 4) & 0xffffn), 1);
  assert.strictEqual(Number(exports.tls_handshake_header_decode(1024, 3) & 0xffffn), 1);
  assert.deepStrictEqual(low32High32(exports.tls_record_header_encode(0x17, 0x0303, 5, 2048, 4)), {
    status: 2,
    written: 0,
  });
  assert.deepStrictEqual(low32High32(exports.tls_handshake_header_encode(0x01, 0x20, 2048, 3)), {
    status: 2,
    written: 0,
  });
  assert.deepStrictEqual(low32High32(exports.tls_handshake_header_encode(0x01, 0x1000000, 2048, 4)), {
    status: 4,
    written: 0,
  });

  memory.set([0x17, 0x03, 0x03, 0x00, 0x05], 1024);
  assert.strictEqual(tlsRecordParts(exports.tls_record_header_decode(1024, 5)).status, 0);

  memory.set([0x13, 0x03, 0x03, 0x00, 0x05], 1024);
  assert.strictEqual(tlsRecordParts(exports.tls_record_header_decode(1024, 5)).status, 3);
  memory.set([0x17, 0x02, 0xff, 0x00, 0x05], 1024);
  assert.strictEqual(tlsRecordParts(exports.tls_record_header_decode(1024, 5)).status, 3);
  memory.set([0x17, 0x03, 0x05, 0x00, 0x05], 1024);
  assert.strictEqual(tlsRecordParts(exports.tls_record_header_decode(1024, 5)).status, 3);
  memory.set([0x17, 0x03, 0x03, 0x40, 0x01], 1024);
  assert.strictEqual(tlsRecordParts(exports.tls_record_header_decode(1024, 5)).status, 3);

  assert.deepStrictEqual(low32High32(exports.tls_record_header_encode(0x13, 0x0303, 5, 2048, 5)), {
    status: 3,
    written: 0,
  });
  assert.deepStrictEqual(low32High32(exports.tls_record_header_encode(0x17, 0x0305, 5, 2048, 5)), {
    status: 3,
    written: 0,
  });
  assert.deepStrictEqual(low32High32(exports.tls_record_header_encode(0x17, 0x0303, 0x4001, 2048, 5)), {
    status: 3,
    written: 0,
  });
  assert.deepStrictEqual(low32High32(exports.tls_record_header_encode(0x100, 0x0303, 5, 2048, 5)), {
    status: 4,
    written: 0,
  });

  const result = {
    unit: "tls-frame",
    abi_version: exports.proto_abi_version(),
    standard_id: exports.proto_standard_id(),
    ok: true,
    cases: ["record-decode", "record-encode", "handshake-decode", "handshake-encode"],
  };
  console.log(JSON.stringify(result, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
