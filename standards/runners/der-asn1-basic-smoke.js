#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/der-asn1-basic.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  return execFileSync("wat2wasm", [file, "-o", "-"]);
}

function write(memory, bytes, ptr = 1024) {
  memory.fill(0, ptr, ptr + 256);
  memory.set(bytes, ptr);
  return ptr;
}

function u32(memory, ptr) {
  return (
    memory[ptr] |
    (memory[ptr + 1] << 8) |
    (memory[ptr + 2] << 16) |
    (memory[ptr + 3] << 24)
  ) >>> 0;
}

function span(memory, out = 2048) {
  return {
    ptr: u32(memory, out),
    len: u32(memory, out + 4),
    headerLen: u32(memory, out + 8),
    totalLen: u32(memory, out + 12),
    aux: u32(memory, out + 16),
  };
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const exports = instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300022);

  let ptr = write(memory, [0x02, 0x01, 0x7f]);
  assert.strictEqual(exports.der_asn1_integer_decode(ptr, 3, 2048), 0);
  assert.deepStrictEqual(span(memory), {
    ptr: ptr + 2,
    len: 1,
    headerLen: 2,
    totalLen: 3,
    aux: 0,
  });

  ptr = write(memory, [0x02, 0x02, 0x00, 0x80]);
  assert.strictEqual(exports.der_asn1_integer_decode(ptr, 4, 2048), 0);
  ptr = write(memory, [0x02, 0x01, 0x80]);
  assert.strictEqual(exports.der_asn1_integer_decode(ptr, 3, 2048), 0);
  assert.strictEqual(span(memory).aux, 1);
  assert.strictEqual(exports.der_asn1_integer_decode(write(memory, [0x02, 0x00]), 2, 2048), 3);
  assert.strictEqual(exports.der_asn1_integer_decode(write(memory, [0x02, 0x02, 0x00, 0x00]), 4, 2048), 3);
  assert.strictEqual(exports.der_asn1_integer_decode(write(memory, [0x02, 0x02, 0xff, 0x80]), 4, 2048), 3);
  assert.strictEqual(exports.der_asn1_integer_decode(write(memory, [0x04, 0x01, 0x00]), 3, 2048), 3);
  assert.strictEqual(exports.der_asn1_integer_decode(write(memory, [0x02, 0x02, 0x00]), 3, 2048), 1);

  ptr = write(memory, [0x03, 0x03, 0x01, 0xaa, 0x80]);
  assert.strictEqual(exports.der_asn1_bit_string_decode(ptr, 5, 2048), 0);
  assert.deepStrictEqual(span(memory), {
    ptr: ptr + 3,
    len: 2,
    headerLen: 2,
    totalLen: 5,
    aux: 1,
  });
  assert.strictEqual(exports.der_asn1_bit_string_decode(write(memory, [0x03, 0x00]), 2, 2048), 3);
  assert.strictEqual(exports.der_asn1_bit_string_decode(write(memory, [0x03, 0x01, 0x08]), 3, 2048), 3);
  assert.strictEqual(exports.der_asn1_bit_string_decode(write(memory, [0x03, 0x01, 0x01]), 3, 2048), 3);

  ptr = write(memory, [0x04, 0x03, 0xde, 0xad, 0xbe]);
  assert.strictEqual(exports.der_asn1_octet_string_decode(ptr, 5, 2048), 0);
  assert.deepStrictEqual(span(memory), {
    ptr: ptr + 2,
    len: 3,
    headerLen: 2,
    totalLen: 5,
    aux: 1,
  });
  assert.strictEqual(exports.der_asn1_octet_string_decode(write(memory, [0x04, 0x03, 0xde]), 3, 2048), 1);

  assert.strictEqual(exports.der_asn1_null_decode(write(memory, [0x05, 0x00]), 2), 0);
  assert.strictEqual(exports.der_asn1_null_decode(write(memory, [0x05, 0x01, 0x00]), 3), 3);
  assert.strictEqual(exports.der_asn1_null_decode(write(memory, [0x04, 0x00]), 2), 3);

  ptr = write(memory, [0x30, 0x05, 0x02, 0x01, 0x01, 0x05, 0x00]);
  assert.strictEqual(exports.der_asn1_sequence_decode(ptr, 7, 2048), 0);
  assert.deepStrictEqual(span(memory), {
    ptr: ptr + 2,
    len: 5,
    headerLen: 2,
    totalLen: 7,
    aux: 1,
  });
  const bodyPtr = u32(memory, 2048);
  const bodyLen = u32(memory, 2052);
  assert.strictEqual(exports.der_asn1_sequence_next_child(bodyPtr, bodyLen, 0, 3072), 0);
  assert.deepStrictEqual(
    {
      tag: u32(memory, 3072),
      headerLen: u32(memory, 3076),
      valueLen: u32(memory, 3084),
      totalLen: u32(memory, 3088),
      nextOffset: u32(memory, 3092),
    },
    { tag: 2, headerLen: 2, valueLen: 1, totalLen: 3, nextOffset: 3 }
  );
  assert.strictEqual(exports.der_asn1_sequence_next_child(bodyPtr, bodyLen, 3, 3072), 0);
  assert.deepStrictEqual(
    {
      tag: u32(memory, 3072),
      headerLen: u32(memory, 3076),
      valueLen: u32(memory, 3084),
      totalLen: u32(memory, 3088),
      nextOffset: u32(memory, 3092),
    },
    { tag: 5, headerLen: 2, valueLen: 0, totalLen: 2, nextOffset: 5 }
  );
  assert.strictEqual(exports.der_asn1_sequence_next_child(bodyPtr, bodyLen, 5, 3072), 5);
  assert.strictEqual(exports.der_asn1_sequence_next_child(bodyPtr, bodyLen, 7, 3072), 3);
  assert.strictEqual(exports.der_asn1_sequence_decode(write(memory, [0x31, 0x00]), 2, 2048), 3);

  console.log("der-asn1-basic smoke ok");
})();
