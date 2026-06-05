#!/usr/bin/env node

const assert = require("node:assert/strict");
const { execFileSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/app-primitives/tor-hs-intro-extensions/tor-hs-intro-extensions.wat");
const wasm = path.join(os.tmpdir(), `tor-hs-intro-extensions-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "inherit" });
execFileSync("wasm-validate", [wasm], { stdio: "inherit" });

function put(mem, ptr, bytes) {
  mem.set(bytes, ptr);
}

function take(mem, ptr, len) {
  return Buffer.from(mem.slice(ptr, ptr + len));
}

function u32(mem, ptr) {
  return new DataView(mem.buffer).getUint32(ptr, true);
}

function u32be(n) {
  const b = Buffer.alloc(4);
  b.writeUInt32BE(n >>> 0);
  return b;
}

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = instance.exports;
  const mem = new Uint8Array(e.memory.buffer);

  assert.equal(e.proto_standard_id(), 300216);
  assert.equal(e.proto_abi_version(), 1);
  assert.equal(e.tor_hs_intro_ext_pow_len(), 41);
  assert.equal(e.tor_hs_intro_ext_pow_challenge_len(), Buffer.byteLength("Tor hs intro v1\0") + 32 + 32 + 16 + 4);

  assert.equal(e.tor_hs_intro_ext_empty(1000), 1);
  assert.deepEqual(take(mem, 1000, 1), Buffer.from([0]));
  assert.equal(e.tor_hs_intro_ext_validate(1000, 1), 0);

  assert.equal(e.tor_hs_intro_ext_build_cc(1100), 3);
  assert.deepEqual(take(mem, 1100, 3), Buffer.from([1, 1, 0]));
  assert.equal(e.tor_hs_intro_ext_validate(1100, 3), 0);

  const nonce = Buffer.from(Array.from({ length: 16 }, (_, i) => 0x10 + i));
  const seed = Buffer.from(Array.from({ length: 32 }, (_, i) => 0x40 + i));
  const id = Buffer.from(Array.from({ length: 32 }, (_, i) => 0x80 + i));
  const solution = Buffer.from(Array.from({ length: 16 }, (_, i) => 0xc0 + i));
  const effort = 1;
  put(mem, 2000, nonce);
  put(mem, 2040, seed);
  put(mem, 2080, id);
  put(mem, 2120, solution);
  const powLen = e.tor_hs_intro_ext_build_pow(3000, 2000, effort, 2040, 2120);
  assert.equal(powLen, 44);
  assert.deepEqual(take(mem, 3000, 44), Buffer.concat([
    Buffer.from([1, 2, 41, 1]),
    nonce,
    u32be(effort),
    seed.subarray(0, 4),
    solution,
  ]));
  assert.equal(e.tor_hs_intro_ext_validate(3000, powLen), 0);
  assert.equal(e.tor_hs_intro_ext_find(3000, powLen, 2, 4000), 0);
  assert.deepEqual([u32(mem, 4000), u32(mem, 4004), u32(mem, 4008), u32(mem, 4012)], [2, 3, 41, 44]);
  assert.equal(e.tor_hs_intro_ext_parse_pow(3003, 41, 4200, 4240, 4280, 4320), 0);
  assert.deepEqual(take(mem, 4200, 16), nonce);
  assert.equal(u32(mem, 4240), effort);
  assert.deepEqual(take(mem, 4280, 4), seed.subarray(0, 4));
  assert.deepEqual(take(mem, 4320, 16), solution);

  const bothLen = e.tor_hs_intro_ext_build_cc_pow(5000, 2000, effort, 2040, 2120);
  assert.equal(bothLen, 46);
  assert.equal(e.tor_hs_intro_ext_validate(5000, bothLen), 0);
  assert.equal(e.tor_hs_intro_ext_record(5000, bothLen, 0, 5100), 0);
  assert.deepEqual([u32(mem, 5100), u32(mem, 5104), u32(mem, 5108), u32(mem, 5112)], [1, 3, 0, 3]);
  assert.equal(e.tor_hs_intro_ext_record(5000, bothLen, 1, 5100), 0);
  assert.deepEqual([u32(mem, 5100), u32(mem, 5104), u32(mem, 5108), u32(mem, 5112)], [2, 5, 41, 46]);

  const challengeLen = e.tor_hs_intro_pow_challenge(2080, 2040, 2000, effort, 6000);
  const expectedChallenge = Buffer.concat([Buffer.from("Tor hs intro v1\0"), id, seed, nonce, u32be(effort)]);
  assert.equal(challengeLen, expectedChallenge.length);
  assert.deepEqual(take(mem, 6000, challengeLen), expectedChallenge);

  const expectedDigestHex = execFileSync("python3", [
    "-c",
    "import hashlib,sys; print(hashlib.blake2b(bytes.fromhex(sys.argv[1]), digest_size=4).hexdigest())",
    Buffer.concat([expectedChallenge, solution]).toString("hex"),
  ], { encoding: "utf8" }).trim();
  const expectedR = Buffer.from(expectedDigestHex, "hex").readUInt32BE(0);
  assert.equal(e.tor_hs_intro_pow_blake2b_result(2080, 2040, 2000, effort, 2120) >>> 0, expectedR);
  assert.equal(e.tor_hs_intro_pow_seed_prefix_ok(2040, 2040), 0);
  assert.equal(e.tor_hs_intro_pow_effort_gate(2080, 2040, 2000, effort, 2040, 2120, 7000), 0);
  assert.equal(u32(mem, 7000), expectedR);
  assert.equal(e.tor_hs_intro_pow_effort_gate(2080, 2040, 2000, 0xffffffff, 2040, 2120, 7000), -3);
  mem[2040] ^= 0xff;
  assert.equal(e.tor_hs_intro_pow_seed_prefix_ok(2040, 2041), -3);
  mem[2040] ^= 0xff;

  const bad = Buffer.from([1, 1, 1, 0]);
  put(mem, 8000, bad);
  assert.equal(e.tor_hs_intro_ext_validate(8000, bad.length), -1);

  console.log("tor hs intro extensions smoke passed");
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
