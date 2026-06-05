#!/usr/bin/env node

const assert = require("node:assert/strict");
const crypto = require("node:crypto");
const { execFileSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/app-primitives/tor-relay-full/tor-relay-full.wat");
const ntorWat = path.join(root, "standards/build/wasm/app-primitives/tor-ntor-crypto/tor-ntor-crypto.wat");
const ntorV3Wat = path.join(root, "standards/build/wasm/app-primitives/tor-ntor-v3-crypto/tor-ntor-v3-crypto.wat");
const relayCryptoWat = path.join(root, "standards/build/wasm/app-primitives/tor-relay-crypto/tor-relay-crypto.wat");
const wasm = path.join(os.tmpdir(), `tor-relay-full-${process.pid}.wasm`);
const ntorWasm = path.join(os.tmpdir(), `tor-ntor-crypto-${process.pid}.wasm`);
const ntorV3Wasm = path.join(os.tmpdir(), `tor-ntor-v3-crypto-${process.pid}.wasm`);
const relayCryptoWasm = path.join(os.tmpdir(), `tor-relay-crypto-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "inherit" });
execFileSync("wat2wasm", [ntorWat, "-o", ntorWasm], { stdio: "inherit" });
execFileSync("wat2wasm", [ntorV3Wat, "-o", ntorV3Wasm], { stdio: "inherit" });
execFileSync("wat2wasm", [relayCryptoWat, "-o", relayCryptoWasm], { stdio: "inherit" });
execFileSync("wasm-validate", [wasm], { stdio: "inherit" });
execFileSync("wasm-validate", [ntorWasm], { stdio: "inherit" });
execFileSync("wasm-validate", [ntorV3Wasm], { stdio: "inherit" });
execFileSync("wasm-validate", [relayCryptoWasm], { stdio: "inherit" });

let memory;
let ntorExports;
let ntorMemory;
let ntorV3Exports;
let ntorV3Memory;
let relayCryptoExports;
let relayCryptoMemory;
const sent = [];
const connected = [];

const NTOR3_PROTOID = Buffer.from("ntor3-curve25519-sha3_256-1");
const NTOR3_T_MSGKDF = Buffer.from("ntor3-curve25519-sha3_256-1:kdf_phase1");
const NTOR3_T_MSGMAC = Buffer.from("ntor3-curve25519-sha3_256-1:msg_mac");

function encap(buf) {
  const len = Buffer.alloc(8);
  len.writeBigUInt64BE(BigInt(buf.length));
  return Buffer.concat([len, buf]);
}

function shakeKdf(s, tag, len) {
  return crypto.createHash("shake256", { outputLength: len }).update(Buffer.concat([encap(tag), s])).digest();
}

function macTag(key, msg, tag) {
  return crypto.createHash("sha3-256").update(Buffer.concat([encap(tag), encap(key), msg])).digest();
}

function aes256ctr(key, msg) {
  const cipher = crypto.createCipheriv("aes-256-ctr", key, Buffer.alloc(16));
  return Buffer.concat([cipher.update(msg), cipher.final()]);
}

const imports = {
  "tor.crypto": {
    tor_relay_digest_update20_sha1(statePtr, payloadPtr, len, outPtr) {
      const state = 1024;
      const payload = 2048;
      const out = 4096;
      relayCryptoMemory.set(memory.slice(statePtr, statePtr + 20), state);
      relayCryptoMemory.set(memory.slice(payloadPtr, payloadPtr + len), payload);
      const rc = relayCryptoExports.tor_relay_digest_update20_sha1(state, payload, len, out);
      if (rc !== 0) return rc;
      memory.set(relayCryptoMemory.slice(out, out + 20), outPtr);
      return 0;
    },
    tor_aes_ctr_crypt(keyPtr, ivPtr, inputPtr, len, outputPtr) {
      const key = 4096;
      const iv = 4128;
      const input = 4160;
      const output = 8192;
      relayCryptoMemory.set(memory.slice(keyPtr, keyPtr + 16), key);
      relayCryptoMemory.set(memory.slice(ivPtr, ivPtr + 16), iv);
      relayCryptoMemory.set(memory.slice(inputPtr, inputPtr + len), input);
      const rc = relayCryptoExports.tor_relay_aes128_ctr_crypt(key, iv, input, len, output);
      if (rc !== 0) return rc;
      memory.set(relayCryptoMemory.slice(output, output + len), outputPtr);
      memory.set(relayCryptoMemory.slice(iv, iv + 16), ivPtr);
      return 0;
    },
    tor_ntor_server_handshake_seeded(handshakePtr, nodePtr, onionPublicPtr, onionSecretPtr, ySecretPtr, replyPtr, keyMaterialPtr) {
      const handshake = 1024;
      const node = 1120;
      const onionPublic = 1160;
      const onionSecret = 1200;
      const ySecret = 1240;
      const reply = 1300;
      const keys = 1400;
      ntorMemory.set(memory.slice(handshakePtr, handshakePtr + 84), handshake);
      ntorMemory.set(memory.slice(nodePtr, nodePtr + 20), node);
      ntorMemory.set(memory.slice(onionPublicPtr, onionPublicPtr + 32), onionPublic);
      ntorMemory.set(memory.slice(onionSecretPtr, onionSecretPtr + 32), onionSecret);
      ntorMemory.set(memory.slice(ySecretPtr, ySecretPtr + 32), ySecret);
      const rc = ntorExports.tor_ntor_server_handshake_seeded(handshake, node, onionPublic, onionSecret, ySecret, reply, keys);
      if (rc !== 0) return rc;
      memory.set(ntorMemory.slice(reply, reply + 64), replyPtr);
      memory.set(ntorMemory.slice(keys, keys + 92), keyMaterialPtr);
      return 0;
    },
    tor_ntor_v3_server_handshake_seeded(handshakePtr, handshakeLen, nodePtr, onionPublicPtr, onionSecretPtr, ySecretPtr, verPtr, verLen, serverMsgPtr, serverMsgLen, replyPtr, replyLenPtr, keyMaterialPtr, keyMaterialLen, clientMsgPtr, clientMsgLenPtr) {
      const handshake = 2048;
      const node = 2600;
      const onionPublic = 2640;
      const onionSecret = 2680;
      const ySecret = 2720;
      const ver = 2760;
      const serverMsg = 3000;
      const reply = 3600;
      const replyLen = 3520;
      const keys = 3800;
      const clientMsg = 4400;
      const clientMsgLen = 4520;
      ntorV3Memory.set(memory.slice(handshakePtr, handshakePtr + handshakeLen), handshake);
      ntorV3Memory.set(memory.slice(nodePtr, nodePtr + 32), node);
      ntorV3Memory.set(memory.slice(onionPublicPtr, onionPublicPtr + 32), onionPublic);
      ntorV3Memory.set(memory.slice(onionSecretPtr, onionSecretPtr + 32), onionSecret);
      ntorV3Memory.set(memory.slice(ySecretPtr, ySecretPtr + 32), ySecret);
      if (verLen) ntorV3Memory.set(memory.slice(verPtr, verPtr + verLen), ver);
      if (serverMsgLen) ntorV3Memory.set(memory.slice(serverMsgPtr, serverMsgPtr + serverMsgLen), serverMsg);
      const rc = ntorV3Exports.tor_ntor_v3_server_handshake_seeded(handshake, handshakeLen, node, onionPublic, onionSecret, ySecret, verLen ? ver : 0, verLen, serverMsgLen ? serverMsg : 0, serverMsgLen, reply, replyLen, keys, keyMaterialLen, clientMsg, clientMsgLen);
      if (rc !== 0) return rc;
      const outReplyLen = new DataView(ntorV3Exports.memory.buffer).getUint32(replyLen, true);
      const outClientLen = new DataView(ntorV3Exports.memory.buffer).getUint32(clientMsgLen, true);
      memory.set(ntorV3Memory.slice(reply, reply + outReplyLen), replyPtr);
      new DataView(memory.buffer).setUint32(replyLenPtr, outReplyLen, true);
      memory.set(ntorV3Memory.slice(keys, keys + keyMaterialLen), keyMaterialPtr);
      memory.set(ntorV3Memory.slice(clientMsg, clientMsg + outClientLen), clientMsgPtr);
      new DataView(memory.buffer).setUint32(clientMsgLenPtr, outClientLen, true);
      return 0;
    },
  },
  "edgerun.io": {
    tor_relay_connect(ip, port, nodePtr, handshakePtr) {
      connected.push({ ip, port, node: nodePtr, handshake: handshakePtr });
      return 77;
    },
    tor_relay_send_cell(conn, cellPtr, len) {
      sent.push({ conn, cellPtr, len });
      return 0;
    },
  },
};

function putU16BE(p, v) {
  memory[p] = (v >>> 8) & 0xff;
  memory[p + 1] = v & 0xff;
}

function putU32LE(p, v) {
  memory[p] = v & 0xff;
  memory[p + 1] = (v >>> 8) & 0xff;
  memory[p + 2] = (v >>> 16) & 0xff;
  memory[p + 3] = (v >>> 24) & 0xff;
}

function getU32LE(p) {
  return (memory[p] | (memory[p + 1] << 8) | (memory[p + 2] << 16) | (memory[p + 3] << 24)) >>> 0;
}

(async () => {
  const { instance: ntorInstance } = await WebAssembly.instantiate(fs.readFileSync(ntorWasm), {});
  ntorExports = ntorInstance.exports;
  ntorMemory = new Uint8Array(ntorExports.memory.buffer);
  const { instance: ntorV3Instance } = await WebAssembly.instantiate(fs.readFileSync(ntorV3Wasm), {});
  ntorV3Exports = ntorV3Instance.exports;
  ntorV3Memory = new Uint8Array(ntorV3Exports.memory.buffer);
  const { instance: relayCryptoInstance } = await WebAssembly.instantiate(fs.readFileSync(relayCryptoWasm), {});
  relayCryptoExports = relayCryptoInstance.exports;
  relayCryptoMemory = new Uint8Array(relayCryptoExports.memory.buffer);

  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), imports);
  const e = instance.exports;
  memory = new Uint8Array(e.memory.buffer);

  assert.equal(e.proto_standard_id(), 300205);
  assert.equal(e.proto_abi_version(), 1);
  assert.equal(e.tor_relay_full_cell_len(), 514);
  assert.equal(e.tor_relay_full_data_max(), 498);
  assert.equal(e.er_tor_relay_init(), 0);

  const cell = 1024;
  const out = 2048;
  const key = 4096;
  const node = 5000;
  const onionSecret = 5040;
  const onionPublic = 5080;
  const ySecret = 5120;
  const clientSecret = 5160;
  const clientPublic = 5200;
  const node32 = 5240;
  const body = 8192;

  assert.equal(e.er_tor_relay_alloc_circuit(123, 10, 20, 3), 0);
  assert.equal(e.er_tor_relay_get_state(123), 2);
  assert.equal(e.er_tor_relay_set_keys(123, key, key + 16, key + 32, key + 48), 0);

  memory.fill(0, cell, cell + 514);
  putU32LE(cell, 123);
  memory[cell + 4] = 10;
  putU16BE(cell + 5, 2);
  putU16BE(cell + 7, 84);
  for (let i = 0; i < 20; i += 1) memory[node + i] = 1 + i;
  for (let i = 0; i < 32; i += 1) {
    memory[onionSecret + i] = 0x11 + i;
    memory[ySecret + i] = 0x41 + i;
    memory[clientSecret + i] = 0x81 + i;
  }
  ntorMemory.set(memory.slice(onionSecret, onionSecret + 32), 2048);
  assert.equal(ntorExports.tor_x25519_public(2048, 2080), 0);
  memory.set(ntorMemory.slice(2080, 2112), onionPublic);
  ntorMemory.set(memory.slice(clientSecret, clientSecret + 32), 2048);
  assert.equal(ntorExports.tor_x25519_public(2048, 2080), 0);
  memory.set(ntorMemory.slice(2080, 2112), clientPublic);
  memory.set(memory.slice(node, node + 20), cell + 9);
  memory.set(memory.slice(onionPublic, onionPublic + 32), cell + 29);
  memory.set(memory.slice(clientPublic, clientPublic + 32), cell + 61);
  assert.equal(e.er_tor_relay_handle_create2(cell, 514, node, onionPublic, onionSecret, ySecret, out), 0);
  assert.equal(getU32LE(out), 123);
  assert.equal(memory[out + 4], 11);
  assert.equal(memory[out + 5], 0);
  assert.equal(memory[out + 6], 2);
  assert.equal(memory[out + 7], 0);
  assert.equal(memory[out + 8], 64);

  for (let i = 0; i < 32; i += 1) memory[node32 + i] = 0x01 + i;
  const clientMsg = Buffer.from([1, 3, 2, 2, 6]);
  const serverMsg = Buffer.from([1, 2, 1, 100]);
  ntorMemory.set(memory.slice(clientSecret, clientSecret + 32), 2048);
  ntorMemory.set(memory.slice(onionPublic, onionPublic + 32), 2080);
  assert.equal(ntorExports.tor_x25519_shared(2048, 2080, 2120), 0);
  const bx = Buffer.from(ntorMemory.slice(2120, 2152));
  const nodeId = Buffer.from(memory.slice(node32, node32 + 32));
  const onionPub = Buffer.from(memory.slice(onionPublic, onionPublic + 32));
  const clientPub = Buffer.from(memory.slice(clientPublic, clientPublic + 32));
  const secretPhase1 = Buffer.concat([bx, nodeId, clientPub, onionPub, NTOR3_PROTOID, encap(Buffer.alloc(0))]);
  const phase1 = shakeKdf(secretPhase1, NTOR3_T_MSGKDF, 64);
  const encryptedClientMsg = aes256ctr(phase1.subarray(0, 32), clientMsg);
  const clientMac = macTag(phase1.subarray(32, 64), Buffer.concat([nodeId, onionPub, clientPub, encryptedClientMsg]), NTOR3_T_MSGMAC);
  const v3Handshake = Buffer.concat([nodeId, onionPub, clientPub, encryptedClientMsg, clientMac]);
  memory.fill(0, cell, cell + 514);
  putU32LE(cell, 125);
  memory[cell + 4] = 10;
  putU16BE(cell + 5, 3);
  putU16BE(cell + 7, v3Handshake.length);
  memory.set(v3Handshake, cell + 9);
  memory.set(serverMsg, body);
  assert.equal(e.er_tor_relay_handle_create2_v3(cell, 514, node32, onionPublic, onionSecret, ySecret, body, serverMsg.length, out), 0);
  assert.equal(getU32LE(out), 125);
  assert.equal(memory[out + 4], 11);
  assert.equal(memory[out + 5], 0);
  assert.equal(memory[out + 6], 3);
  assert.equal(memory[out + 8], 64 + serverMsg.length);

  memory.fill(0, body, body + 160);
  memory[body] = 2;
  memory[body + 1] = 0;
  memory[body + 2] = 6;
  putU32LE(body + 3, 0x01020304);
  putU16BE(body + 7, 9001);
  memory[body + 9] = 2;
  memory[body + 10] = 20;
  for (let i = 0; i < 20; i += 1) memory[body + 11 + i] = 0x30 + i;
  putU16BE(body + 31, 2);
  putU16BE(body + 33, 84);
  for (let i = 0; i < 84; i += 1) memory[body + 35 + i] = 0x80 + i;
  assert.equal(e.er_tor_relay_handle_extend2(123, body, 119, out), 5);
  assert.equal(connected.length, 1);
  assert.equal(e.er_tor_relay_get_state(123), 3);

  assert.equal(e.er_tor_relay_alloc_circuit(124, 10, 20, 3), 0);
  assert.equal(e.er_tor_relay_build_relay_cell(cell, 124, 1, 7, body, 12), 0);
  assert.equal(e.er_tor_relay_process_recognized(124, cell + 5, 509, out), 6);
  assert.equal(e.er_tor_relay_get_last_action(124), 6);

  assert.equal(e.er_tor_relay_build_sendme(out, 124, 7, 0), 0);
  assert.equal(memory[out + 5], 5);

  assert.equal(e.er_tor_relay_close_circuit(124, 6, out), 4);
  assert.equal(e.er_tor_relay_get_state(124), 0);
  assert.equal(memory[out + 4], 4);

  console.log("tor relay full smoke passed");
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
