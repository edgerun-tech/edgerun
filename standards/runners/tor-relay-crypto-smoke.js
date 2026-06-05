#!/usr/bin/env node

const assert = require("node:assert/strict");
const crypto = require("node:crypto");
const { execFileSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/app-primitives/tor-relay-crypto/tor-relay-crypto.wat");
const wasm = path.join(os.tmpdir(), `tor-relay-crypto-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "inherit" });
execFileSync("wasm-validate", [wasm], { stdio: "inherit" });

(async () => {
  const mod = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = mod.instance.exports;
  const memory = new Uint8Array(e.memory.buffer);
  const view = new DataView(e.memory.buffer);

  assert.equal(e.proto_standard_id(), 300218);
  assert.equal(e.tor_relay_digest_state_len(), 20);

  const state = 1024;
  const payload = 2048;
  const out = 4096;
  for (let i = 0; i < 32; i += 1) memory[state + i] = i;
  for (let i = 0; i < 509; i += 1) memory[payload + i] = (0x80 + i) & 0xff;
  assert.equal(e.tor_relay_digest_update20_sha1(state, payload, 509, out), 0);
  const expectedSha1 = crypto.createHash("sha1")
    .update(Buffer.from(memory.slice(state, state + 20)))
    .update(Buffer.from(memory.slice(payload, payload + 509)))
    .digest();
  assert.deepEqual(Buffer.from(memory.slice(out, out + 20)), expectedSha1);
  assert.equal(e.tor_relay_digest4_le_sha1(state, payload, 509) >>> 0, expectedSha1.readUInt32LE(0));

  assert.equal(e.tor_relay_digest_update32(state, payload, 509, out), 0);
  const expected = crypto.createHash("sha256")
    .update(Buffer.from(memory.slice(state, state + 32)))
    .update(Buffer.from(memory.slice(payload, payload + 509)))
    .digest();
  assert.deepEqual(Buffer.from(memory.slice(out, out + 32)), expected);
  assert.equal(e.tor_relay_digest4_le(state, payload, 509) >>> 0, expected.readUInt32LE(0));

  const key = 8192;
  const iv = 8224;
  const input = 8256;
  const encrypted = 12288;
  for (let i = 0; i < 16; i += 1) {
    memory[key + i] = 0x10 + i;
    memory[iv + i] = i;
  }
  for (let i = 0; i < 37; i += 1) memory[input + i] = 0x40 + i;
  const expectedCipher = Buffer.concat([
    crypto.createCipheriv("aes-128-ctr", Buffer.from(memory.slice(key, key + 16)), Buffer.from(memory.slice(iv, iv + 16))).update(Buffer.from(memory.slice(input, input + 37))),
  ]);
  assert.equal(e.tor_relay_aes128_ctr_crypt(key, iv, input, 37, encrypted), 0);
  assert.deepEqual(Buffer.from(memory.slice(encrypted, encrypted + 37)), expectedCipher);
  assert.equal(view.getUint32(iv + 12, false), 0x0c0d0e12);

  console.log("tor relay crypto smoke passed");
})().finally(() => {
  try { fs.unlinkSync(wasm); } catch {}
});
