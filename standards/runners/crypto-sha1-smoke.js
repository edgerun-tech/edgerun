#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/crypto-sha1.wat";

const STATUS_OK = 0;
const STATUS_OUTPUT_SHORT = 2;
const STATUS_INVALID = 3;

function compileWat(file) {
  const wasmPath = path.join(os.tmpdir(), `edgerun-${path.basename(file)}-${process.pid}.wasm`);
  execFileSync("wat2wasm", [file, "-o", wasmPath], { stdio: "inherit" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "inherit" });
  const bytes = fs.readFileSync(wasmPath);
  fs.unlinkSync(wasmPath);
  return bytes;
}

function unpack(packed) {
  const value = BigInt.asUintN(64, packed);
  return {
    status: Number(value & 0xffff_ffffn),
    written: Number((value >> 32n) & 0xffff_ffffn),
  };
}

function put(memory, ptr, value) {
  const bytes = Buffer.isBuffer(value) ? value : Buffer.from(value, "utf8");
  memory.fill(0, ptr, ptr + bytes.length + 64);
  memory.set(bytes, ptr);
  return bytes.length;
}

function digestHex(memory, outPtr) {
  return Buffer.from(memory.slice(outPtr, outPtr + 20)).toString("hex");
}

(async () => {
  const { instance } = await WebAssembly.instantiate(compileWat(watPath), {});
  const e = instance.exports;
  const memory = new Uint8Array(e.memory.buffer);
  const inPtr = 1024;
  const outPtr = 4096;
  const cases = [];

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300077);

  function checkVector(name, input, expectedHex) {
    const len = put(memory, inPtr, input);
    const packed = unpack(e.sha1(inPtr, len, outPtr));
    assert.deepEqual(packed, { status: STATUS_OK, written: 20 }, `${name} status`);
    assert.equal(digestHex(memory, outPtr), expectedHex, `${name} digest`);
    cases.push(name);
  }

  checkVector("empty", "", "da39a3ee5e6b4b0d3255bfef95601890afd80709");
  checkVector("abc", "abc", "a9993e364706816aba3e25717850c26c9cd0d89d");
  checkVector(
    "long-known",
    "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
    "84983e441c3bd26ebaae4aa1f95129e5e54670f1",
  );

  assert.deepEqual(unpack(e.sha1(inPtr, 3, 65520)), {
    status: STATUS_OUTPUT_SHORT,
    written: 0,
  }, "output short status");

  assert.deepEqual(unpack(e.sha1(65520, 64, outPtr)), {
    status: STATUS_INVALID,
    written: 0,
  }, "invalid input bounds status");

  console.log(
    JSON.stringify({
      unit: "crypto-sha1",
      abi_version: e.proto_abi_version(),
      standard_id: e.proto_standard_id(),
      ok: true,
      cases,
    }),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
