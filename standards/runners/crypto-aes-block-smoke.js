#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/crypto-aes-block.wat";

const STATUS_OK = 0;
const STATUS_INVALID = 2;

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

function writeHex(memory, ptr, hex) {
  const bytes = Buffer.from(hex, "hex");
  memory.fill(0, ptr, ptr + bytes.length + 32);
  memory.set(bytes, ptr);
  return bytes.length;
}

function readHex(memory, ptr, len) {
  return Buffer.from(memory.slice(ptr, ptr + len)).toString("hex");
}

(async () => {
  const { instance } = await WebAssembly.instantiate(compileWat(watPath), {});
  const e = instance.exports;
  const memory = new Uint8Array(e.memory.buffer);
  const keyPtr = 1024;
  const blockPtr = 2048;
  const outPtr = 4096;
  const cases = [];

  const key128 = "000102030405060708090a0b0c0d0e0f";
  const key256 = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
  const plaintext = "00112233445566778899aabbccddeeff";
  const cipher128 = "69c4e0d86a7b0430d8cdb78070b4c55a";
  const cipher256 = "8ea2b7ca516745bfeafc49904b496089";

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300083);

  writeHex(memory, keyPtr, key128);
  writeHex(memory, blockPtr, plaintext);
  assert.deepEqual(unpack(e.aes128_encrypt(keyPtr, 16, blockPtr, 16, outPtr)), {
    status: STATUS_OK,
    written: 16,
  });
  assert.equal(readHex(memory, outPtr, 16), cipher128);
  cases.push("aes128_encrypt_nist");

  writeHex(memory, keyPtr, key128);
  writeHex(memory, blockPtr, cipher128);
  assert.deepEqual(unpack(e.aes128_decrypt(keyPtr, 16, blockPtr, 16, outPtr)), {
    status: STATUS_OK,
    written: 16,
  });
  assert.equal(readHex(memory, outPtr, 16), plaintext);
  cases.push("aes128_decrypt_nist");

  writeHex(memory, keyPtr, key256);
  writeHex(memory, blockPtr, plaintext);
  assert.deepEqual(unpack(e.aes256_encrypt(keyPtr, 32, blockPtr, 16, outPtr)), {
    status: STATUS_OK,
    written: 16,
  });
  assert.equal(readHex(memory, outPtr, 16), cipher256);
  cases.push("aes256_encrypt_nist");

  assert.deepEqual(unpack(e.aes128_encrypt(keyPtr, 15, blockPtr, 16, outPtr)), {
    status: STATUS_INVALID,
    written: 0,
  });
  cases.push("bad_aes128_key_len");

  assert.deepEqual(unpack(e.aes256_encrypt(keyPtr, 32, blockPtr, 15, outPtr)), {
    status: STATUS_INVALID,
    written: 0,
  });
  cases.push("bad_block_len");

  assert.deepEqual(unpack(e.aes128_decrypt(keyPtr, 16, blockPtr, 16, 131068)), {
    status: STATUS_INVALID,
    written: 0,
  });
  cases.push("bad_output_range");

  console.log(
    JSON.stringify({
      unit: "crypto-aes-block",
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
