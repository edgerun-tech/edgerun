#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/der-oid.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  return execFileSync("wat2wasm", [file, "-o", "-"]);
}

function write(memory, bytes, ptr = 1024) {
  memory.fill(0, ptr, ptr + 128);
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

function packed(value) {
  return {
    status: Number(value & 0xffffffffn),
    written: Number((value >> 32n) & 0xffffffffn),
  };
}

function expectRoot(exports, memory, bytes, first, second) {
  const ptr = write(memory, bytes);
  assert.strictEqual(exports.der_oid_root_decode(ptr, bytes.length, 2048), 0);
  assert.strictEqual(u32(memory, 2048), first);
  assert.strictEqual(u32(memory, 2052), second);
  assert.strictEqual(u32(memory, 2056), 1);
}

function expectArc(exports, memory, bytes, offset, arc, next) {
  const ptr = write(memory, bytes);
  assert.strictEqual(exports.der_oid_next_arc(ptr, bytes.length, offset, 2048), 0);
  assert.strictEqual(u32(memory, 2048), arc);
  assert.strictEqual(u32(memory, 2052), next);
  assert.strictEqual(u32(memory, 2056), next - offset);
}

function expectOid(exports, memory, bytes, arcs) {
  const ptr = write(memory, bytes);
  assert.strictEqual(exports.der_oid_root_decode(ptr, bytes.length, 2048), 0);
  const got = [u32(memory, 2048), u32(memory, 2052)];
  let offset = u32(memory, 2056);
  while (true) {
    const status = exports.der_oid_next_arc(ptr, bytes.length, offset, 2048);
    if (status === 5) break;
    assert.strictEqual(status, 0);
    got.push(u32(memory, 2048));
    offset = u32(memory, 2052);
  }
  assert.deepStrictEqual(got, arcs);
  assert.strictEqual(exports.der_oid_value_validate(ptr, bytes.length, 2048), 0);
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const exports = instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300021);

  expectRoot(exports, memory, [0x2a], 1, 2);
  expectRoot(exports, memory, [0x77], 2, 39);
  assert.strictEqual(exports.der_oid_root_decode(write(memory, []), 0, 2048), 1);
  assert.strictEqual(exports.der_oid_root_decode(write(memory, [0x78]), 1, 2048), 3);

  expectArc(exports, memory, [0x2a, 0x86, 0x48], 1, 840, 3);
  expectArc(exports, memory, [0x2b, 0x06, 0x01], 1, 6, 2);
  assert.strictEqual(exports.der_oid_next_arc(write(memory, [0x2a]), 1, 1, 2048), 5);
  assert.strictEqual(exports.der_oid_next_arc(write(memory, [0x2a, 0x80]), 2, 1, 2048), 1);
  assert.strictEqual(exports.der_oid_next_arc(write(memory, [0x2a, 0x80, 0x00]), 3, 1, 2048), 3);
  assert.strictEqual(
    exports.der_oid_next_arc(write(memory, [0x2a, 0xff, 0xff, 0xff, 0xff, 0x7f]), 6, 1, 2048),
    4
  );

  expectOid(exports, memory, [0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d], [
    1, 2, 840, 113549,
  ]);
  expectOid(exports, memory, [0x2b, 0x06, 0x01, 0x05, 0x05, 0x07], [
    1, 3, 6, 1, 5, 5, 7,
  ]);

  assert.deepStrictEqual(packed(exports.der_oid_root_encode(1, 2, 3072, 8)), {
    status: 0,
    written: 1,
  });
  assert.strictEqual(memory[3072], 0x2a);
  assert.deepStrictEqual(packed(exports.der_oid_root_encode(3, 1, 3072, 8)), {
    status: 3,
    written: 0,
  });
  assert.deepStrictEqual(packed(exports.der_oid_root_encode(1, 2, 3072, 0)), {
    status: 2,
    written: 0,
  });

  assert.deepStrictEqual(packed(exports.der_oid_arc_encode(840, 3072, 8)), {
    status: 0,
    written: 2,
  });
  assert.deepStrictEqual(Array.from(memory.slice(3072, 3074)), [0x86, 0x48]);
  assert.deepStrictEqual(packed(exports.der_oid_arc_encode(113549, 3072, 8)), {
    status: 0,
    written: 3,
  });
  assert.deepStrictEqual(Array.from(memory.slice(3072, 3075)), [0x86, 0xf7, 0x0d]);
  assert.deepStrictEqual(packed(exports.der_oid_arc_encode(128, 3072, 1)), {
    status: 2,
    written: 0,
  });

  console.log("der-oid smoke ok");
})();
