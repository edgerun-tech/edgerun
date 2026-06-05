#!/usr/bin/env node

const assert = require("node:assert/strict");
const crypto = require("node:crypto");
const { execFileSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/app-primitives/tor-hs-pow-v1/tor-hs-pow-v1.wat");
const wasm = path.join(os.tmpdir(), `tor-hs-pow-v1-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "inherit" });
execFileSync("wasm-validate", [wasm], { stdio: "inherit" });

function take(mem, ptr, len) {
  return Buffer.from(mem.slice(ptr, ptr + len));
}

function put(mem, ptr, bytes) {
  mem.set(bytes, ptr);
}

function u32(mem, ptr) {
  return new DataView(mem.buffer).getUint32(ptr, true);
}

function u32be(n) {
  const out = Buffer.alloc(4);
  out.writeUInt32BE(n >>> 0);
  return out;
}

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = instance.exports;
  let mem = new Uint8Array(e.memory.buffer);

  assert.equal(e.proto_standard_id(), 300219);
  assert.equal(e.proto_abi_version(), 1);
  assert.equal(e.tor_hs_pow_v1_challenge_len(), 100);
  assert.equal(e.tor_hs_pow_v1_solution_len(), 16);
  assert.equal(e.tor_hs_pow_v1_replay_capacity(), 256);
  assert.equal(e.tor_hs_pow_v1_replay_reset(), 0);

  const identity = Buffer.from(Array.from({ length: 32 }, (_, i) => 0x20 + i));
  const seed = Buffer.from(Array.from({ length: 32 }, (_, i) => 0x60 + i));
  const nonce = Buffer.from(Array.from({ length: 16 }, (_, i) => 0xa0 + i));
  const effort = 1;
  const idPtr = 1024;
  const seedPtr = 1088;
  const noncePtr = 1152;
  const solutionPtr = 1216;
  const challengePtr = 1280;
  const rPtr = 1408;
  put(mem, idPtr, identity);
  put(mem, seedPtr, seed);
  put(mem, noncePtr, nonce);

  const challengeLen = e.tor_hs_pow_v1_challenge(idPtr, seedPtr, noncePtr, effort, challengePtr);
  mem = new Uint8Array(e.memory.buffer);
  const expectedChallenge = Buffer.concat([Buffer.from("Tor hs intro v1\0"), identity, seed, nonce, u32be(effort)]);
  assert.equal(challengeLen, expectedChallenge.length);
  assert.deepEqual(take(mem, challengePtr, challengeLen), expectedChallenge);

  assert.equal(e.tor_hs_pow_v1_solve_first(idPtr, seedPtr, noncePtr, effort, solutionPtr, rPtr), 0);
  mem = new Uint8Array(e.memory.buffer);
  const solution = take(mem, solutionPtr, 16);
  const expectedDigestHex = execFileSync("python3", [
    "-c",
    "import hashlib,sys; print(hashlib.blake2b(bytes.fromhex(sys.argv[1]), digest_size=4).hexdigest())",
    Buffer.concat([expectedChallenge, solution]).toString("hex"),
  ], { encoding: "utf8" }).trim();
  const expectedR = Buffer.from(expectedDigestHex, "hex").readUInt32BE(0);
  assert.equal(u32(mem, rPtr), expectedR);
  assert.equal(e.tor_hs_pow_v1_blake2b_result(challengePtr, challengeLen, solutionPtr, rPtr), 0);
  mem = new Uint8Array(e.memory.buffer);
  assert.equal(u32(mem, rPtr), expectedR);
  assert.equal(e.tor_hs_pow_v1_equix_verify(challengePtr, challengeLen, solutionPtr), 0);

  assert.equal(e.tor_hs_pow_v1_verify(idPtr, seedPtr, noncePtr, effort, seedPtr, solutionPtr, rPtr), 0);
  mem = new Uint8Array(e.memory.buffer);
  assert.equal(e.tor_hs_pow_v1_replay_count(), 1);
  assert.equal(e.tor_hs_pow_v1_verify(idPtr, seedPtr, noncePtr, effort, seedPtr, solutionPtr, rPtr), -5);

  mem[solutionPtr] ^= 1;
  assert.equal(e.tor_hs_pow_v1_replay_reset(), 0);
  assert.equal(e.tor_hs_pow_v1_verify(idPtr, seedPtr, noncePtr, effort, seedPtr, solutionPtr, rPtr), -3);
  mem[solutionPtr] ^= 1;

  mem[seedPtr] ^= 0xff;
  assert.equal(e.tor_hs_pow_v1_verify(idPtr, seedPtr, noncePtr, effort, seedPtr + 1, solutionPtr, rPtr), -3);

  console.log("tor hs pow v1 smoke passed");
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
}).finally(() => {
  try { fs.unlinkSync(wasm); } catch {}
});
