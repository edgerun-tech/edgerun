#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const root = path.resolve(__dirname, "..", "..");
const hmacWat = path.join(root, "standards/build/wasm/codec-primitives/crypto-hmac-hkdf.wat");
const sha256Wat = path.join(root, "standards/build/wasm/codec-primitives/crypto-sha256.wat");
const sha512Wat = path.join(root, "standards/build/wasm/codec-primitives/crypto-sha512.wat");
const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "crypto-hmac-hkdf-"));

process.on("exit", () => {
  fs.rmSync(tmpDir, { recursive: true, force: true });
});

function compileWat(file) {
  const wasmPath = path.join(tmpDir, `${path.basename(file)}.wasm`);
  execFileSync("wat2wasm", [file, "-o", wasmPath], { stdio: "inherit" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "inherit" });
  return fs.readFileSync(wasmPath);
}

function unpack(result) {
  const value = BigInt.asUintN(64, result);
  return {
    status: Number(value & 0xffff_ffffn),
    written: Number((value >> 32n) & 0xffff_ffffn),
  };
}

function write(memory, ptr, bytes) {
  memory.fill(0, ptr, ptr + bytes.length + 256);
  memory.set(bytes, ptr);
}

function read(memory, ptr, len) {
  return Buffer.from(memory.slice(ptr, ptr + len));
}

function hex(value) {
  return Buffer.from(value).toString("hex");
}

function hmacProfile(exports, memory, alg) {
  const ptr = 8192;
  assert.equal(exports.crypto_hmac_profile(alg, ptr), 0);
  const view = new DataView(memory.buffer);
  return {
    block: view.getUint32(ptr, true),
    digest: view.getUint32(ptr + 4, true),
    maxHkdf: view.getUint32(ptr + 8, true),
  };
}

function digestWithWat(sha, alg, input) {
  const memory = new Uint8Array(sha.exports.memory.buffer);
  const inPtr = 1024;
  const outPtr = 16384;
  const name = alg === 384 ? "sha384" : "sha256";
  const outLen = alg === 384 ? 48 : 32;
  write(memory, inPtr, input);
  memory.fill(0, outPtr, outPtr + 64);
  const packed = unpack(sha.exports[name](inPtr, input.length, outPtr));
  assert.deepEqual(packed, { status: 0, written: outLen }, `${name} digest status`);
  return read(memory, outPtr, outLen);
}

function hmacWithWat(hmac, sha, alg, key, message) {
  const memory = new Uint8Array(hmac.exports.memory.buffer);
  const profile = hmacProfile(hmac.exports, memory, alg);
  const keyBytes = key.length > profile.block ? digestWithWat(sha, alg, key) : Buffer.from(key);
  const keyPtr = 1024;
  const ipadPtr = 4096;
  const opadPtr = 4352;
  write(memory, keyPtr, keyBytes);
  memory.fill(0, ipadPtr, ipadPtr + profile.block);
  memory.fill(0, opadPtr, opadPtr + profile.block);
  assert.equal(
    hmac.exports.crypto_hmac_key_pad(alg, keyPtr, keyBytes.length, ipadPtr, opadPtr),
    0,
    "key pad status",
  );
  const innerInput = Buffer.concat([read(memory, ipadPtr, profile.block), Buffer.from(message)]);
  const inner = digestWithWat(sha, alg, innerInput);
  const outerInput = Buffer.concat([read(memory, opadPtr, profile.block), inner]);
  return digestWithWat(sha, alg, outerInput);
}

function hkdfSha256WithWat(hmac, sha, ikm, salt, info, length) {
  const prk = hmacWithWat(hmac, sha, 256, salt, ikm);
  let previous = Buffer.alloc(0);
  let okm = Buffer.alloc(0);
  let counter = 1;
  const memory = new Uint8Array(hmac.exports.memory.buffer);
  const prevPtr = 1024;
  const infoPtr = 2048;
  const inputPtr = 4096;

  while (okm.length < length) {
    write(memory, prevPtr, previous);
    write(memory, infoPtr, info);
    const packed = unpack(
      hmac.exports.crypto_hkdf_expand_input(
        256,
        prevPtr,
        previous.length,
        infoPtr,
        info.length,
        counter,
        inputPtr,
        512,
      ),
    );
    assert.equal(packed.status, 0, "expand input status");
    const input = read(memory, inputPtr, packed.written);
    previous = hmacWithWat(hmac, sha, 256, prk, input);
    okm = Buffer.concat([okm, previous]);
    counter += 1;
  }

  return okm.subarray(0, length);
}

(async () => {
  const [hmacModule, sha256Module, sha512Module] = await Promise.all([
    WebAssembly.instantiate(compileWat(hmacWat), {}),
    WebAssembly.instantiate(compileWat(sha256Wat), {}),
    WebAssembly.instantiate(compileWat(sha512Wat), {}),
  ]);
  const hmac = hmacModule.instance;
  const sha256 = sha256Module.instance;
  const sha512 = sha512Module.instance;
  const e = hmac.exports;
  const memory = new Uint8Array(e.memory.buffer);

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300076);
  assert.deepEqual(hmacProfile(e, memory, 256), { block: 64, digest: 32, maxHkdf: 8160 });
  assert.deepEqual(hmacProfile(e, memory, 384), { block: 128, digest: 48, maxHkdf: 12240 });
  assert.equal(e.crypto_hmac_profile(1, 8192), 1);
  assert.equal(e.crypto_hmac_digest_required_status(), 4);

  const keyPtr = 1024;
  const ipadPtr = 4096;
  const opadPtr = 4352;
  write(memory, keyPtr, Buffer.from("key", "ascii"));
  assert.equal(e.crypto_hmac_key_pad(256, keyPtr, 3, ipadPtr, opadPtr), 0);
  const expectedIpad = Buffer.alloc(64, 0x36);
  const expectedOpad = Buffer.alloc(64, 0x5c);
  expectedIpad[0] = "k".charCodeAt(0) ^ 0x36;
  expectedIpad[1] = "e".charCodeAt(0) ^ 0x36;
  expectedIpad[2] = "y".charCodeAt(0) ^ 0x36;
  expectedOpad[0] = "k".charCodeAt(0) ^ 0x5c;
  expectedOpad[1] = "e".charCodeAt(0) ^ 0x5c;
  expectedOpad[2] = "y".charCodeAt(0) ^ 0x5c;
  assert.equal(hex(read(memory, ipadPtr, 64)), hex(expectedIpad));
  assert.equal(hex(read(memory, opadPtr, 64)), hex(expectedOpad));

  write(memory, keyPtr, Buffer.alloc(65, 0xaa));
  assert.equal(e.crypto_hmac_key_pad(256, keyPtr, 65, ipadPtr, opadPtr), 4);

  write(memory, 1024, Buffer.from([1, 2, 3]));
  write(memory, 2048, Buffer.from([0xf0, 0xf1]));
  assert.deepEqual(unpack(e.crypto_hkdf_expand_input(256, 1024, 3, 2048, 2, 7, 4096, 6)), {
    status: 0,
    written: 6,
  });
  assert.equal(hex(read(memory, 4096, 6)), "010203f0f107");
  assert.equal(unpack(e.crypto_hkdf_expand_input(256, 1024, 33, 2048, 2, 1, 4096, 64)).status, 3);
  assert.equal(unpack(e.crypto_hkdf_expand_input(256, 1024, 0, 2048, 2, 0, 4096, 64)).status, 3);
  assert.equal(unpack(e.crypto_hkdf_expand_input(256, 1024, 0, 2048, 2, 1, 4096, 2)).status, 2);

  const hmacSha256 = hmacWithWat(
    hmac,
    sha256,
    256,
    Buffer.alloc(20, 0x0b),
    Buffer.from("Hi There", "ascii"),
  );
  assert.equal(hex(hmacSha256), "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7");

  const hmacSha384 = hmacWithWat(
    hmac,
    sha512,
    384,
    Buffer.alloc(20, 0x0b),
    Buffer.from("Hi There", "ascii"),
  );
  assert.equal(
    hex(hmacSha384),
    "afd03944d84895626b0825f4ab46907f15f9dadbe4101ec682aa034c7cebc59cfaea9ea9076ede7f4af152e8b2fa9cb6",
  );

  const okm = hkdfSha256WithWat(
    hmac,
    sha256,
    Buffer.alloc(22, 0x0b),
    Buffer.from("000102030405060708090a0b0c", "hex"),
    Buffer.from("f0f1f2f3f4f5f6f7f8f9", "hex"),
    42,
  );
  assert.equal(
    hex(okm),
    "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865",
  );

  console.log(
    JSON.stringify(
      {
        unit: "crypto-hmac-hkdf",
        abi_version: e.proto_abi_version(),
        standard_id: e.proto_standard_id(),
        ok: true,
        cases: ["profiles", "key_pad", "expand_input", "hmac_sha256", "hmac_sha384", "hkdf_sha256"],
      },
      null,
      2,
    ),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
