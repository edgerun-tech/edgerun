#!/usr/bin/env node

const assert = require("node:assert/strict");
const crypto = require("node:crypto");
const { execFileSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/app-primitives/tor-hs-identity/tor-hs-identity.wat");
const wasm = path.join(os.tmpdir(), `tor-hs-identity-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "inherit" });
execFileSync("wasm-validate", [wasm], { stdio: "inherit" });

function put(mem, ptr, bytes) {
  mem.set(typeof bytes === "string" ? Buffer.from(bytes, "ascii") : bytes, ptr);
}

function take(mem, ptr, len) {
  return Buffer.from(mem.slice(ptr, ptr + len));
}

function credential(pubkey) {
  return crypto.createHash("sha3-256").update(Buffer.concat([Buffer.from("credential"), pubkey])).digest();
}

function subcredential(pubkey, blinded) {
  return crypto.createHash("sha3-256").update(Buffer.concat([Buffer.from("subcredential"), credential(pubkey), blinded])).digest();
}

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = instance.exports;
  const mem = new Uint8Array(e.memory.buffer);

  assert.equal(e.proto_standard_id(), 300211);
  assert.equal(e.proto_abi_version(), 1);
  assert.equal(e.tor_hs_identity_onion_len(), 62);
  assert.equal(e.tor_hs_identity_raw_len(), 35);

  const sample = "pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd.onion";
  put(mem, 1024, sample);
  assert.equal(e.tor_hs_identity_validate_onion(1024, sample.length, 2048), 0);
  const samplePub = take(mem, 2048, 32);
  assert.equal(e.tor_hs_identity_build_onion(2048, 3000), 0);
  assert.equal(take(mem, 3000, 62).toString("ascii"), sample);

  mem[1024] = "q".charCodeAt(0);
  assert.equal(e.tor_hs_identity_validate_onion(1024, sample.length, 2048), -1);

  assert.equal(e.tor_hs_identity_credential(2048, 4096), 0);
  assert.deepEqual(take(mem, 4096, 32), credential(samplePub));

  const blinded = Buffer.from(Array.from({ length: 32 }, (_, i) => 0xa0 + i));
  put(mem, 5000, blinded);
  assert.equal(e.tor_hs_identity_subcredential(2048, 5000, 6000), 0);
  assert.deepEqual(take(mem, 6000, 32), subcredential(samplePub, blinded));

  const pubkey = Buffer.from(Array.from({ length: 32 }, (_, i) => (i * 11 + 5) & 0xff));
  put(mem, 7000, pubkey);
  assert.equal(e.tor_hs_identity_build_onion(7000, 7100), 0);
  assert.equal(take(mem, 7100, 62).toString("ascii").endsWith(".onion"), true);

  console.log("tor hs identity smoke passed");
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
