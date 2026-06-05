#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/integer-decimal.wat";

const ERR_OUTPUT_SHORT = -2;

function compileWat(file) {
  const wasmPath = path.join(os.tmpdir(), `edgerun-${path.basename(file)}-${process.pid}.wasm`);
  execFileSync("wat2wasm", [file, "-o", wasmPath], { stdio: "pipe" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "pipe" });
  const bytes = fs.readFileSync(wasmPath);
  fs.unlinkSync(wasmPath);
  return bytes;
}

function readAscii(memory, ptr, len) {
  return Buffer.from(memory.slice(ptr, ptr + len)).toString("ascii");
}

function clear(memory, ptr, len) {
  memory.fill(0, ptr, ptr + len);
}

function expectFormat(memory, fn, args, expected) {
  const outPtr = 4096;
  clear(memory, outPtr, 128);
  const len = fn(...args, outPtr, expected.length);
  assert.equal(len, expected.length);
  assert.equal(readAscii(memory, outPtr, len), expected);

  if (expected.length > 0) {
    clear(memory, outPtr, 128);
    assert.equal(fn(...args, outPtr, expected.length - 1), ERR_OUTPUT_SHORT);
  }
}

function splitU128(value) {
  return {
    lo: value & ((1n << 64n) - 1n),
    hi: value >> 64n,
  };
}

(async () => {
  const module = await WebAssembly.instantiate(compileWat(watPath), {});
  const e = module.instance.exports;
  const memory = new Uint8Array(e.memory.buffer);

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300073);

  expectFormat(memory, e.integer_format_u64, [0n], "0");
  expectFormat(memory, e.integer_format_u64, [9n], "9");
  expectFormat(memory, e.integer_format_u64, [10n], "10");
  expectFormat(memory, e.integer_format_u64, [9999n], "9999");
  expectFormat(memory, e.integer_format_u64, [18446744073709551615n], "18446744073709551615");

  expectFormat(memory, e.integer_format_i64, [0n], "0");
  expectFormat(memory, e.integer_format_i64, [1n], "1");
  expectFormat(memory, e.integer_format_i64, [-1n], "-1");
  expectFormat(memory, e.integer_format_i64, [9223372036854775807n], "9223372036854775807");
  expectFormat(memory, e.integer_format_i64, [-9223372036854775808n], "-9223372036854775808");

  for (const value of [
    0n,
    1n,
    10n,
    18446744073709551615n,
    18446744073709551616n,
    340282366920938463463374607431768211455n,
  ]) {
    const { lo, hi } = splitU128(value);
    expectFormat(memory, e.integer_format_u128, [lo, hi], value.toString(10));
  }

  console.log(
    JSON.stringify({
      unit: "integer-decimal",
      abi_version: e.proto_abi_version(),
      standard_id: e.proto_standard_id(),
      ok: true,
    }),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
