#!/usr/bin/env node

const assert = require("node:assert/strict");
const crypto = require("node:crypto");
const { execFileSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/app-primitives/tor-hs-descriptor-crypto/tor-hs-descriptor-crypto.wat");
const wasm = path.join(os.tmpdir(), `tor-hs-descriptor-crypto-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "inherit" });
execFileSync("wasm-validate", [wasm], { stdio: "inherit" });

function put(mem, ptr, bytes) {
  mem.set(bytes, ptr);
}

function take(mem, ptr, len) {
  return Buffer.from(mem.slice(ptr, ptr + len));
}

function u64be(n) {
  const b = Buffer.alloc(8);
  b.writeBigUInt64BE(BigInt(n));
  return b;
}

function derive(secret, subcred, revision, salt, constant) {
  return crypto
    .createHash("shake256", { outputLength: 80 })
    .update(Buffer.concat([secret, subcred, u64be(revision), salt, Buffer.from(constant)]))
    .digest();
}

function aesCtr(key, iv, input) {
  const cipher = crypto.createCipheriv("aes-256-ctr", key, iv);
  return Buffer.concat([cipher.update(input), cipher.final()]);
}

function mac(macKey, salt, encrypted) {
  return crypto
    .createHash("sha3-256")
    .update(Buffer.concat([u64be(macKey.length), macKey, u64be(salt.length), salt, encrypted]))
    .digest();
}

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = instance.exports;
  const mem = new Uint8Array(e.memory.buffer);

  assert.equal(e.proto_standard_id(), 300212);
  assert.equal(e.proto_abi_version(), 1);
  assert.equal(e.tor_hs_desc_salt_len(), 16);
  assert.equal(e.tor_hs_desc_mac_len(), 32);
  assert.equal(e.tor_hs_desc_key_material_len(), 80);

  const blinded = Buffer.from(Array.from({ length: 32 }, (_, i) => 0x10 + i));
  const subcred = Buffer.from(Array.from({ length: 32 }, (_, i) => 0x40 + i));
  const cookie = Buffer.from(Array.from({ length: 32 }, (_, i) => 0x80 + i));
  const salt = Buffer.from(Array.from({ length: 16 }, (_, i) => 0xa0 + i));
  const plain = Buffer.from("proto Relay=5 Relay=6\ncreate2-formats 2 3\nintroduction-point abc\n");
  const revision = 9;

  put(mem, 1000, blinded);
  put(mem, 1040, subcred);
  put(mem, 1080, cookie);
  put(mem, 1120, salt);
  put(mem, 2000, plain);

  assert.equal(e.tor_hs_desc_derive_first(1000, 1040, 0, revision, 1120, 3000), 0);
  const firstKeys = derive(blinded, subcred, revision, salt, "hsdir-superencrypted-data");
  assert.deepEqual(take(mem, 3000, 80), firstKeys);

  const firstLen = e.tor_hs_desc_encrypt_first(1000, 1040, 0, revision, 1120, 2000, plain.length, 4000);
  assert.equal(firstLen, 16 + plain.length + 32);
  const expectedFirstEnc = aesCtr(firstKeys.subarray(0, 32), firstKeys.subarray(32, 48), plain);
  assert.deepEqual(take(mem, 4016, plain.length), expectedFirstEnc);
  assert.deepEqual(take(mem, 4016 + plain.length, 32), mac(firstKeys.subarray(48, 80), salt, expectedFirstEnc));
  assert.equal(e.tor_hs_desc_decrypt_first(1000, 1040, 0, revision, 4000, firstLen, 5000), plain.length);
  assert.deepEqual(take(mem, 5000, plain.length), plain);

  mem[4017] ^= 0xff;
  assert.equal(e.tor_hs_desc_decrypt_first(1000, 1040, 0, revision, 4000, firstLen, 5000), -3);
  mem[4017] ^= 0xff;

  assert.equal(e.tor_hs_desc_derive_second(1000, 1080, 32, 1040, 0, revision, 1120, 6000), 0);
  const secondKeys = derive(Buffer.concat([blinded, cookie]), subcred, revision, salt, "hsdir-encrypted-data");
  assert.deepEqual(take(mem, 6000, 80), secondKeys);
  const secondLen = e.tor_hs_desc_encrypt_second(1000, 1080, 32, 1040, 0, revision, 1120, 2000, plain.length, 7000);
  assert.equal(e.tor_hs_desc_decrypt_second(1000, 1080, 32, 1040, 0, revision, 7000, secondLen, 8000), plain.length);
  assert.deepEqual(take(mem, 8000, plain.length), plain);

  const secretSeed = Buffer.from(Array.from({ length: 32 }, (_, i) => 0xc0 + i));
  put(mem, 9000, secretSeed);
  assert.equal(e.tor_hs_desc_cookie_keys(1040, 9000, 9100, 9120), 0);
  const cookieKeys = crypto.createHash("shake256", { outputLength: 40 }).update(Buffer.concat([subcred, secretSeed])).digest();
  assert.deepEqual(take(mem, 9100, 8), cookieKeys.subarray(0, 8));
  assert.deepEqual(take(mem, 9120, 32), cookieKeys.subarray(8, 40));
  assert.equal(e.tor_hs_desc_wrap_cookie(9120, 1120, 1080, 9200), 0);
  assert.deepEqual(take(mem, 9200, 32), aesCtr(cookieKeys.subarray(8, 40), salt, cookie));

  console.log("tor hs descriptor crypto smoke passed");
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
