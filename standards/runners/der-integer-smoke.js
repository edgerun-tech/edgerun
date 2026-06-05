#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/der-integer.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  const wasm = execFileSync("wat2wasm", [file, "-o", "-"]);
  execFileSync("wasm-validate", ["-"], { input: wasm });
  return wasm;
}

function packed(value) {
  return {
    status: Number(value & 0xffffffffn),
    written: Number((value >> 32n) & 0xffffffffn),
  };
}

function write(memory, bytes, ptr = 1024) {
  memory.fill(0, ptr, ptr + 64);
  memory.set(bytes, ptr);
  return ptr;
}

function read(memory, ptr, len) {
  return Array.from(memory.slice(ptr, ptr + len));
}

function expectPayload(exports, memory, der, payload) {
  const inPtr = write(memory, der);
  const outPtr = 2048;
  const result = packed(exports.der_integer_payload(inPtr, der.length, outPtr, payload.length));
  assert.deepStrictEqual(result, { status: 0, written: payload.length });
  assert.deepStrictEqual(read(memory, outPtr, payload.length), payload);
}

function expectEmit(exports, memory, value, der) {
  const valuePtr = write(memory, value);
  const outPtr = 2048;
  const result = packed(exports.der_integer_emit_unsigned(valuePtr, value.length, outPtr, der.length));
  assert.deepStrictEqual(result, { status: 0, written: der.length });
  assert.deepStrictEqual(read(memory, outPtr, der.length), der);
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const exports = instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300801);

  const zero = [0x02, 0x01, 0x00];
  const i7f = [0x02, 0x01, 0x7f];
  const i80 = [0x02, 0x02, 0x00, 0x80];

  for (const der of [zero, i7f, i80]) {
    assert.strictEqual(exports.der_integer_validate(write(memory, der), der.length), 0);
  }

  expectPayload(exports, memory, zero, [0x00]);
  expectPayload(exports, memory, i7f, [0x7f]);
  expectPayload(exports, memory, i80, [0x80]);

  expectEmit(exports, memory, [], zero);
  expectEmit(exports, memory, [0x00], zero);
  expectEmit(exports, memory, [0x7f], i7f);
  expectEmit(exports, memory, [0x80], i80);

  assert.deepStrictEqual(
    packed(exports.der_integer_emit_unsigned(write(memory, [0x80]), 1, 2048, 3)),
    { status: 2, written: 0 },
  );

  assert.strictEqual(exports.der_integer_validate(write(memory, [0x02, 0x01, 0x80]), 3), 3);
  assert.strictEqual(exports.der_integer_validate(write(memory, [0x02, 0x02, 0x00, 0x7f]), 4), 3);
  assert.strictEqual(exports.der_integer_validate(write(memory, [0x02, 0x02, 0x00]), 3), 1);

  assert.deepStrictEqual(
    packed(exports.der_integer_payload(write(memory, i80), i80.length, 2048, 0)),
    { status: 2, written: 0 },
  );

  console.log(
    JSON.stringify({
      unit: "der-integer",
      abi_version: exports.proto_abi_version(),
      standard_id: exports.proto_standard_id(),
      ok: true,
      cases: [
        "zero",
        "7f",
        "80-leading-zero",
        "emit-80-leading-zero",
        "negative-reject",
        "nonminimal-leading-zero-reject",
        "length-mismatch-reject",
        "output-short",
      ],
    }),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
