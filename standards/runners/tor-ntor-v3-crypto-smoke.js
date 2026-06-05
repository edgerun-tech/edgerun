#!/usr/bin/env node

const assert = require("node:assert/strict");
const crypto = require("node:crypto");
const { execFileSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/app-primitives/tor-ntor-v3-crypto/tor-ntor-v3-crypto.wat");
const wasm = path.join(os.tmpdir(), `tor-ntor-v3-crypto-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "inherit" });
execFileSync("wasm-validate", [wasm], { stdio: "inherit" });

const PROTOID = Buffer.from("ntor3-curve25519-sha3_256-1");
const T_MSGKDF = Buffer.from("ntor3-curve25519-sha3_256-1:kdf_phase1");
const T_MSGMAC = Buffer.from("ntor3-curve25519-sha3_256-1:msg_mac");

function encap(buf) {
  const len = Buffer.alloc(8);
  len.writeBigUInt64BE(BigInt(buf.length));
  return Buffer.concat([len, buf]);
}

function kdf(s, tag, len) {
  return crypto.createHash("shake256", { outputLength: len }).update(Buffer.concat([encap(tag), s])).digest();
}

function mac(key, msg, tag) {
  return crypto.createHash("sha3-256").update(Buffer.concat([encap(tag), encap(key), msg])).digest();
}

function aes256ctr(key, msg) {
  const cipher = crypto.createCipheriv("aes-256-ctr", key, Buffer.alloc(16));
  return Buffer.concat([cipher.update(msg), cipher.final()]);
}

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
  const dv = new DataView(e.memory.buffer);

  assert.equal(e.proto_standard_id(), 300209);
  assert.equal(e.proto_abi_version(), 1);
  assert.equal(e.tor_ntor_v3_protoid_len(), 27);

  put(mem, 1000, Buffer.from("abc"));
  assert.equal(e.tor_sha3_256(1000, 3, 1100), 0);
  assert.equal(take(mem, 1100, 32).toString("hex"), crypto.createHash("sha3-256").update("abc").digest("hex"));
  assert.equal(e.tor_shake256(1000, 3, 1200, 64), 0);
  assert.equal(take(mem, 1200, 64).toString("hex"), crypto.createHash("shake256", { outputLength: 64 }).update("abc").digest("hex"));

  const goodExt = Buffer.from([1, 3, 2, 2, 6]);
  const badExt = Buffer.from([2, 3, 2, 2, 6]);
  put(mem, 1300, goodExt);
  put(mem, 1320, badExt);
  assert.equal(e.tor_ntor_v3_extensions_validate(1300, goodExt.length), 0);
  assert.equal(e.tor_ntor_v3_extensions_validate(1320, badExt.length), -1);

  const nodeId = Buffer.from(Array.from({ length: 32 }, (_, i) => 0x01 + i));
  const onionSecret = Buffer.from(Array.from({ length: 32 }, (_, i) => 0x21 + i));
  const clientSecret = Buffer.from(Array.from({ length: 32 }, (_, i) => 0x41 + i));
  const ySecret = Buffer.from(Array.from({ length: 32 }, (_, i) => 0x61 + i));
  const ver = Buffer.alloc(0);
  const clientMsg = Buffer.from([1, 3, 2, 2, 6]);
  const serverMsg = Buffer.from([1, 2, 1, 100]);

  put(mem, 2000, onionSecret);
  put(mem, 2040, clientSecret);
  assert.equal(e.tor_aes256_ctr(2000, 2040, 32, 2080), 0);
  assert.notEqual(take(mem, 2080, 32).toString("hex"), clientSecret.toString("hex"));

  put(mem, 2000, onionSecret);
  assert.equal(instance.exports.tor_aes256_ctr(2000, 2040, 0, 2080), 0);

  // Derive deterministic public keys via the existing ntor v2 WAT.
  const ntorV2Wat = path.join(root, "standards/build/wasm/app-primitives/tor-ntor-crypto/tor-ntor-crypto.wat");
  const ntorV2Wasm = path.join(os.tmpdir(), `tor-ntor-crypto-for-v3-${process.pid}.wasm`);
  execFileSync("wat2wasm", [ntorV2Wat, "-o", ntorV2Wasm], { stdio: "inherit" });
  const { instance: v2 } = await WebAssembly.instantiate(fs.readFileSync(ntorV2Wasm), {});
  const v2mem = new Uint8Array(v2.exports.memory.buffer);
  put(v2mem, 3000, onionSecret);
  put(v2mem, 3040, clientSecret);
  assert.equal(v2.exports.tor_x25519_public(3000, 3100), 0);
  assert.equal(v2.exports.tor_x25519_public(3040, 3140), 0);
  assert.equal(v2.exports.tor_x25519_shared(3040, 3100, 3180), 0);
  const onionPublic = take(v2mem, 3100, 32);
  const clientPublic = take(v2mem, 3140, 32);
  const bx = take(v2mem, 3180, 32);

  const secretPhase1 = Buffer.concat([bx, nodeId, clientPublic, onionPublic, PROTOID, encap(ver)]);
  const phase1 = kdf(secretPhase1, T_MSGKDF, 64);
  const encryptedClientMsg = aes256ctr(phase1.subarray(0, 32), clientMsg);
  const clientMac = mac(phase1.subarray(32, 64), Buffer.concat([nodeId, onionPublic, clientPublic, encryptedClientMsg]), T_MSGMAC);
  const handshake = Buffer.concat([nodeId, onionPublic, clientPublic, encryptedClientMsg, clientMac]);

  put(mem, 4000, handshake);
  put(mem, 4600, nodeId);
  put(mem, 4640, onionPublic);
  put(mem, 4680, onionSecret);
  put(mem, 4720, ySecret);
  put(mem, 4800, serverMsg);
  assert.equal(e.tor_ntor_v3_server_handshake_seeded(4000, handshake.length, 4600, 4640, 4680, 4720, 0, 0, 4800, serverMsg.length, 5000, 4900, 5100, 92, 5300, 4920), 0);
  const replyLen = dv.getUint32(4900, true);
  const clientLen = dv.getUint32(4920, true);
  assert.equal(replyLen, 64 + serverMsg.length);
  assert.equal(clientLen, clientMsg.length);
  assert.deepEqual(take(mem, 5300, clientLen), clientMsg);
  assert.notEqual(take(mem, 5000, 32).toString("hex"), "00".repeat(32));
  assert.notEqual(take(mem, 5032, 32).toString("hex"), "00".repeat(32));
  assert.notEqual(take(mem, 5100, 92).toString("hex"), "00".repeat(92));

  mem[4600] ^= 0xff;
  assert.equal(e.tor_ntor_v3_server_handshake_seeded(4000, handshake.length, 4600, 4640, 4680, 4720, 0, 0, 4800, serverMsg.length, 5000, 4900, 5100, 92, 5300, 4920), -1);

  console.log("tor ntor v3 crypto smoke passed");
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
