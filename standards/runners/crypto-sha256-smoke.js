#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/crypto-sha256.wat";

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
  return Buffer.from(memory.slice(outPtr, outPtr + 32)).toString("hex");
}

(async () => {
  const { instance } = await WebAssembly.instantiate(compileWat(watPath), {});
  const e = instance.exports;
  const memory = new Uint8Array(e.memory.buffer);
  const inPtr = 1024;
  const outPtr = 4096;
  const cases = [];

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300075);

  function checkVector(name, input, expectedHex) {
    const len = put(memory, inPtr, input);
    const packed = unpack(e.sha256(inPtr, len, outPtr));
    assert.deepEqual(packed, { status: STATUS_OK, written: 32 }, `${name} status`);
    assert.equal(digestHex(memory, outPtr), expectedHex, `${name} digest`);
    cases.push(name);
  }

  checkVector("empty", "", "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
  checkVector("abc", "abc", "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
  checkVector(
    "long-known",
    "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
    "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
  );

  assert.deepEqual(unpack(e.sha256(inPtr, 3, 65520)), {
    status: STATUS_OUTPUT_SHORT,
    written: 0,
  }, "output short status");

  assert.deepEqual(unpack(e.sha256(65520, 64, outPtr)), {
    status: STATUS_INVALID,
    written: 0,
  }, "invalid input bounds status");

  console.log(
    JSON.stringify({
      unit: "crypto-sha256",
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
