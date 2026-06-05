#!/usr/bin/env node

const assert = require("node:assert/strict");
const { execFileSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/app-primitives/tor-ntor-crypto/tor-ntor-crypto.wat");
const wasm = path.join(os.tmpdir(), `tor-ntor-crypto-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "inherit" });
execFileSync("wasm-validate", [wasm], { stdio: "inherit" });

function hex(s) {
  return Uint8Array.from(Buffer.from(s, "hex"));
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
  const memory = new Uint8Array(e.memory.buffer);

  assert.equal(e.proto_standard_id(), 300206);
  assert.equal(e.proto_abi_version(), 1);

  const xSecret = 1024;
  const xPublic = 1088;
  const xOut = 1152;
  put(memory, xSecret, hex("a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4"));
  put(memory, xPublic, hex("e6db6867583030db3594c1a424b15f7c726624ec26b3353b10a903a6d0ab1c4c"));
  assert.equal(e.tor_x25519_shared(xSecret, xPublic, xOut), 0);
  assert.equal(take(memory, xOut, 32).toString("hex"), "c3da55379de9c6908e94ea4df28d084f32eccf03491c71f754b4075577a28552");

  const hKey = 2048;
  const hMsg = 2080;
  const hOut = 2200;
  put(memory, hKey, Buffer.from("key"));
  put(memory, hMsg, Buffer.from("The quick brown fox jumps over the lazy dog"));
  assert.equal(e.tor_hmac_sha256(hKey, 3, hMsg, 43, hOut), 0);
  assert.equal(take(memory, hOut, 32).toString("hex"), "f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8");

  const node = 3000;
  const onionSecret = 3040;
  const onionPublic = 3080;
  const ySecret = 3120;
  const yPublic = 3160;
  const clientSecret = 3200;
  const clientPublic = 3240;
  const handshake = 3300;
  const reply = 3400;
  const keys = 3500;
  const clientHandshake = 3700;
  const clientKeys = 3900;
  for (let i = 0; i < 20; i += 1) memory[node + i] = 1 + i;
  for (let i = 0; i < 32; i += 1) {
    memory[onionSecret + i] = 0x11 + i;
    memory[ySecret + i] = 0x41 + i;
    memory[clientSecret + i] = 0x81 + i;
  }
  assert.equal(e.tor_x25519_public(onionSecret, onionPublic), 0);
  assert.equal(e.tor_x25519_public(ySecret, yPublic), 0);
  assert.equal(e.tor_x25519_public(clientSecret, clientPublic), 0);
  assert.equal(e.tor_ntor_client_handshake_seeded(node, onionPublic, clientSecret, clientHandshake, clientPublic), 0);
  memory.set(memory.slice(clientHandshake, clientHandshake + 84), handshake);
  memory.set(memory.slice(node, node + 20), handshake);
  memory.set(memory.slice(onionPublic, onionPublic + 32), handshake + 20);
  memory.set(memory.slice(clientPublic, clientPublic + 32), handshake + 52);
  assert.deepEqual(take(memory, clientHandshake, 84), take(memory, handshake, 84));

  assert.equal(e.tor_ntor_server_handshake_seeded(handshake, node, onionPublic, onionSecret, ySecret, reply, keys), 0);
  assert.equal(take(memory, reply, 32).toString("hex"), take(memory, yPublic, 32).toString("hex"));
  assert.notEqual(take(memory, reply + 32, 32).toString("hex"), "00".repeat(32));
  assert.notEqual(take(memory, keys, 92).toString("hex"), "00".repeat(92));
  assert.equal(e.tor_ntor_client_process(reply, node, onionPublic, clientSecret, clientPublic, clientKeys), 0);
  assert.deepEqual(take(memory, clientKeys, 92), take(memory, keys, 92));
  memory[reply + 40] ^= 0xff;
  assert.equal(e.tor_ntor_client_process(reply, node, onionPublic, clientSecret, clientPublic, clientKeys), -1);
  memory[reply + 40] ^= 0xff;

  memory[node] ^= 0xff;
  assert.equal(e.tor_ntor_server_handshake_seeded(handshake, node, onionPublic, onionSecret, ySecret, reply, keys), -1);

  console.log("tor ntor crypto smoke passed");
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
