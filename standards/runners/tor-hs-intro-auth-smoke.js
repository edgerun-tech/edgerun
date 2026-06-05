#!/usr/bin/env node

const assert = require("node:assert/strict");
const crypto = require("node:crypto");
const { execFileSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/app-primitives/tor-hs-intro-auth/tor-hs-intro-auth.wat");
const wasm = path.join(os.tmpdir(), `tor-hs-intro-auth-${process.pid}.wasm`);

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

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = instance.exports;
  const mem = new Uint8Array(e.memory.buffer);

  assert.equal(e.proto_standard_id(), 300215);
  assert.equal(e.proto_abi_version(), 1);
  assert.equal(e.tor_hs_intro_auth_noext_len(), 134);
  assert.equal(e.tor_hs_intro_auth_domain_len(), Buffer.byteLength("Tor establish-intro cell v1"));

  const seed = Buffer.from(Array.from({ length: 32 }, (_, i) => 0x10 + i));
  const kh = Buffer.from(Array.from({ length: 32 }, (_, i) => 0x80 + i));
  put(mem, 1000, seed);
  put(mem, 1040, kh);

  assert.equal(e.tor_hs_intro_auth_public(1000, 1100), 0);
  const pub = take(mem, 1100, 32);

  assert.equal(e.tor_hs_intro_auth_derive(1040, 1100, 0, 0, 1200), 0);
  const expectedAuth = crypto
    .createHash("sha3-256")
    .update(Buffer.concat([kh, Buffer.from([2, 0, 32]), pub, Buffer.from([0])]))
    .digest();
  assert.deepEqual(take(mem, 1200, 32), expectedAuth);

  const bodyLen = e.tor_hs_intro_auth_sign(1000, 1040, 0, 0, 2000);
  assert.equal(bodyLen, 134);
  const body = take(mem, 2000, bodyLen);
  assert.equal(body[0], 2);
  assert.equal(body.readUInt16BE(1), 32);
  assert.deepEqual(body.subarray(3, 35), pub);
  assert.equal(body[35], 0);
  assert.deepEqual(body.subarray(36, 68), expectedAuth);
  assert.equal(body.readUInt16BE(68), 64);
  assert.equal(e.tor_hs_intro_auth_verify(2000, bodyLen, 1040, 3000), 0);
  assert.deepEqual(take(mem, 3000, 32), pub);

  mem[2000 + 36] ^= 0xff;
  assert.equal(e.tor_hs_intro_auth_verify(2000, bodyLen, 1040, 3000), -3);
  mem[2000 + 36] ^= 0xff;
  mem[2000 + bodyLen - 1] ^= 0xff;
  assert.equal(e.tor_hs_intro_auth_verify(2000, bodyLen, 1040, 3000), -3);
  mem[2000 + bodyLen - 1] ^= 0xff;

  const extLen = e.tor_hs_intro_auth_build_dos_extension(4000, 123, 456);
  assert.equal(extLen, 22);
  const ext = take(mem, 4000, extLen);
  assert.deepEqual(ext, Buffer.concat([
    Buffer.from([1, 1, 19, 2, 1]),
    u64be(123),
    Buffer.from([2]),
    u64be(456),
  ]));
  const dosLen = e.tor_hs_intro_auth_sign(1000, 1040, 4000, extLen, 5000);
  assert.equal(dosLen, 155);
  assert.equal(e.tor_hs_intro_auth_verify(5000, dosLen, 1040, 6000), 0);
  assert.deepEqual(take(mem, 6000, 32), pub);

  console.log("tor hs intro auth smoke passed");
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
