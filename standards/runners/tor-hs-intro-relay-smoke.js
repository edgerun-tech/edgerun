#!/usr/bin/env node

const assert = require("node:assert/strict");
const { execFileSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const root = path.resolve(__dirname, "../..");
const relayWat = path.join(root, "standards/build/wasm/app-primitives/tor-hs-intro-relay/tor-hs-intro-relay.wat");
const authWat = path.join(root, "standards/build/wasm/app-primitives/tor-hs-intro-auth/tor-hs-intro-auth.wat");
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "tor-hs-intro-relay-"));
  const relayWasm = path.join(tmp, "relay.wasm");
  const authWasm = path.join(tmp, "auth.wasm");

for (const [wat, wasm] of [[relayWat, relayWasm], [authWat, authWasm]]) {
  execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "inherit" });
  execFileSync("wasm-validate", [wasm], { stdio: "inherit" });
}

function put(mem, ptr, bytes) {
  mem.set(bytes, ptr);
}

function take(mem, ptr, len) {
  return Buffer.from(mem.slice(ptr, ptr + len));
}

function u32(mem, ptr) {
  return new DataView(mem.buffer).getUint32(ptr, true);
}

(async () => {
  const [{ instance: relayInst }, { instance: authInst }] = await Promise.all([
    WebAssembly.instantiate(fs.readFileSync(relayWasm), {}),
    WebAssembly.instantiate(fs.readFileSync(authWasm), {}),
  ]);
  const relay = relayInst.exports;
  const auth = authInst.exports;
  const rm = new Uint8Array(relay.memory.buffer);
  const am = new Uint8Array(auth.memory.buffer);

  assert.equal(relay.proto_standard_id(), 300217);
  assert.equal(relay.proto_abi_version(), 1);
  assert.equal(relay.tor_hs_intro_relay_max_body_len(), 490);
  assert.equal(relay.tor_hs_intro_relay_init(), 0);

  const seed = Buffer.from(Array.from({ length: 32 }, (_, i) => 0x20 + i));
  const kh = Buffer.from(Array.from({ length: 32 }, (_, i) => 0x90 + i));
  put(am, 1000, seed);
  put(am, 1040, kh);
  const estLen = auth.tor_hs_intro_auth_sign(1000, 1040, 0, 0, 2000);
  assert.equal(estLen, 134);
  const establish = take(am, 2000, estLen);
  const authKey = establish.subarray(3, 35);
  put(rm, 1000, establish);
  assert.equal(relay.tor_hs_intro_relay_parse_establish_intro_auth(1000, estLen, 2000), 0);
  assert.deepEqual(take(rm, 2000, 32), authKey);
  assert.equal(relay.tor_hs_intro_relay_register(101, 202, 1000, estLen), 0);
  const rec = relay.tor_hs_intro_relay_record_ptr(101);
  assert.notEqual(rec, 0);
  assert.equal(u32(rm, rec), 1);
  assert.equal(u32(rm, rec + 4), 101);
  assert.equal(u32(rm, rec + 8), 202);
  assert.deepEqual(take(rm, rec + 16, 32), authKey);

  // Use tor-ntor's own public-key export would require another module; for this
  // relay smoke the encrypted field is opaque, so synthetic bytes are enough.
  const encrypted = Buffer.from(Array.from({ length: 120 }, (_, i) => 0x30 + (i % 200)));
  put(rm, 3000, authKey);
  put(rm, 3040, encrypted);
  // Build INTRODUCE1 prefix: legacy zeros, auth type/len/key, no top extensions, encrypted.
  const introPtr = 9000;
  rm.fill(0, introPtr, introPtr + 20);
  rm[introPtr + 20] = 2;
  rm[introPtr + 21] = 0;
  rm[introPtr + 22] = 32;
  rm.set(authKey, introPtr + 23);
  rm[introPtr + 55] = 0;
  rm.set(encrypted, introPtr + 56);
  const introLen = 56 + encrypted.length;
  assert.equal(relay.tor_hs_intro_relay_validate_introduce1(introPtr, introLen, 5000), 0);
  assert.equal(u32(rm, 5000), 202);
  assert.equal(u32(rm, rec + 12), 1);
  assert.equal(relay.tor_hs_intro_relay_build_introduce2(6000, introPtr, introLen), introLen);
  assert.deepEqual(take(rm, 6000, introLen), take(rm, introPtr, introLen));
  assert.equal(relay.tor_hs_intro_relay_build_introduce_ack(7000, 0), 3);
  assert.deepEqual(take(rm, 7000, 3), Buffer.from([0, 0, 0]));
  assert.equal(relay.tor_hs_intro_relay_build_relay_header(7100, 35, 0, introLen), 11);
  assert.deepEqual(take(rm, 7100, 11), Buffer.from([35, 0, 0, 0, 0, 0, 0, 0, 0, introLen >> 8, introLen & 0xff]));

  rm[introPtr + 23] ^= 0xff;
  assert.equal(relay.tor_hs_intro_relay_validate_introduce1(introPtr, introLen, 5000), -3);
  rm[introPtr + 23] ^= 0xff;
  rm[introPtr] = 1;
  assert.equal(relay.tor_hs_intro_relay_validate_introduce1(introPtr, introLen, 5000), -1);
  rm[introPtr] = 0;
  assert.equal(relay.tor_hs_intro_relay_validate_introduce1(introPtr, 491, 5000), -2);

  console.log("tor hs intro relay smoke passed");
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
