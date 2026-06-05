#!/usr/bin/env node

const assert = require("node:assert/strict");
const { execFileSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/app-primitives/tor-identity-seal/tor-identity-seal.wat");
const wasm = path.join(os.tmpdir(), `tor-identity-seal-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "inherit" });
execFileSync("wasm-validate", [wasm], { stdio: "inherit" });

function put(mem, ptr, bytes) {
  mem.set(bytes, ptr);
}

function take(mem, ptr, len) {
  return Buffer.from(mem.slice(ptr, ptr + len));
}

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = instance.exports;
  const mem = new Uint8Array(e.memory.buffer);

  assert.equal(e.proto_standard_id(), 300220);
  assert.equal(e.proto_abi_version(), 1);
  assert.equal(e.tor_identity_seal_overhead(), 80);

  const sender = Buffer.from(Array.from({ length: 32 }, (_, i) => 0x10 + i));
  const recipientSecret = Buffer.from(Array.from({ length: 32 }, (_, i) => 0x40 + i));
  const ephSecret = Buffer.from(Array.from({ length: 32 }, (_, i) => 0x80 + i));
  const iv = Buffer.from(Array.from({ length: 16 }, (_, i) => 0xc0 + i));
  const plain = Buffer.from("sealed identity payload over host tor relay", "utf8");

  const senderPtr = 1024;
  const recipientSecretPtr = 1088;
  const recipientPubPtr = 1152;
  const ephSecretPtr = 1216;
  const ivPtr = 1280;
  const plainPtr = 1344;
  const boxPtr = 4096;
  const openPtr = 8192;
  put(mem, senderPtr, sender);
  put(mem, recipientSecretPtr, recipientSecret);
  put(mem, ephSecretPtr, ephSecret);
  put(mem, ivPtr, iv);
  put(mem, plainPtr, plain);

  assert.equal(e.tor_identity_seal_public(recipientSecretPtr, recipientPubPtr), 0);
  const sealedLen = e.tor_identity_seal(senderPtr, recipientPubPtr, ephSecretPtr, ivPtr, plainPtr, plain.length, boxPtr);
  assert.equal(sealedLen, plain.length + 80);
  assert.notDeepEqual(take(mem, boxPtr + 48, plain.length), plain);

  const openedLen = e.tor_identity_open(senderPtr, recipientSecretPtr, recipientPubPtr, boxPtr, sealedLen, openPtr);
  assert.equal(openedLen, plain.length);
  assert.deepEqual(take(mem, openPtr, plain.length), plain);

  mem[boxPtr + 52] ^= 1;
  assert.equal(e.tor_identity_open(senderPtr, recipientSecretPtr, recipientPubPtr, boxPtr, sealedLen, openPtr), -3);
  mem[boxPtr + 52] ^= 1;
  mem[senderPtr] ^= 1;
  assert.equal(e.tor_identity_open(senderPtr, recipientSecretPtr, recipientPubPtr, boxPtr, sealedLen, openPtr), -3);

  console.log("tor identity seal smoke passed");
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
}).finally(() => {
  try { fs.unlinkSync(wasm); } catch {}
});
