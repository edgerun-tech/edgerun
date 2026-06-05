#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/crypto-x25519-scalar.wat";

const STATUS_OK = 0;
const STATUS_LENGTH = 2;
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

function put(memory, ptr, bytes) {
  memory.fill(0, ptr, ptr + bytes.length + 64);
  memory.set(bytes, ptr);
}

(async () => {
  const { instance } = await WebAssembly.instantiate(compileWat(watPath), {});
  const e = instance.exports;
  const memory = new Uint8Array(e.memory.buffer);
  const inPtr = 1024;
  const outPtr = 2048;
  const cases = [];

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300080);

  const scalar = Uint8Array.from(Array.from({ length: 32 }, (_, i) => i));
  put(memory, inPtr, scalar);
  assert.deepEqual(unpack(e.x25519_clamp_scalar(inPtr, 32, outPtr)), {
    status: STATUS_OK,
    written: 32,
  });
  assert.equal(
    Buffer.from(memory.slice(outPtr, outPtr + 32)).toString("hex"),
    "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e5f",
  );
  cases.push("clamp_00_1f");

  assert.deepEqual(unpack(e.x25519_clamp_scalar(inPtr, 31, outPtr)), {
    status: STATUS_LENGTH,
    written: 0,
  });
  assert.deepEqual(unpack(e.x25519_clamp_scalar(inPtr, 33, outPtr)), {
    status: STATUS_LENGTH,
    written: 0,
  });
  cases.push("reject_len_31_33");

  const zero = new Uint8Array(32);
  const nonzero = new Uint8Array(32);
  nonzero[0] = 9;
  put(memory, inPtr, zero);
  assert.equal(e.x25519_public_key_status(inPtr, 32), STATUS_INVALID);
  assert.equal(e.x25519_shared_secret_status(inPtr, 32), STATUS_INVALID);
  cases.push("reject_zero_public_shared");

  put(memory, inPtr, nonzero);
  assert.equal(e.x25519_public_key_status(inPtr, 32), STATUS_OK);
  assert.equal(e.x25519_shared_secret_status(inPtr, 32), STATUS_OK);
  assert.equal(e.x25519_public_key_status(inPtr, 31), STATUS_LENGTH);
  assert.equal(e.x25519_public_key_status(inPtr, 33), STATUS_LENGTH);
  cases.push("accept_nonzero");

  console.log(
    JSON.stringify({
      unit: "crypto-x25519-scalar",
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
