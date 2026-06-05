#!/usr/bin/env node

const assert = require("node:assert/strict");
const { execFileSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/app-primitives/tor-cgo-crypto/tor-cgo-crypto.wat");
const wasm = path.join(os.tmpdir(), `tor-cgo-crypto-${process.pid}.wasm`);

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
  const dv = new DataView(e.memory.buffer);

  assert.equal(e.proto_standard_id(), 300210);
  assert.equal(e.proto_abi_version(), 1);
  assert.equal(e.tor_cgo_msg_len(), 509);
  assert.equal(e.tor_cgo_body_len(), 493);
  assert.equal(e.tor_cgo_key_len(), 80);
  assert.equal(e.tor_cgo_state_len(), 96);

  put(mem, 1024, Buffer.alloc(16, 0x42));
  put(mem, 2048, Buffer.alloc(256, 0x69));
  assert.equal(e.tor_cgo_polyval(1024, 2048, 256, 4096), 0);
  assert.equal(take(mem, 4096, 16).toString("hex"), "0713c82b170eef25c8955ddf72c85ccb");

  const key80 = Buffer.from(Array.from({ length: 80 }, (_, i) => (i * 7 + 3) & 0xff));
  put(mem, 5000, key80);
  assert.equal(e.tor_cgo_state_init(6000, 5000), 0);
  assert.equal(e.tor_cgo_state_init(6200, 5000), 0);

  const payload = Buffer.from("hello cgo relay body");
  put(mem, 7000, payload);
  assert.equal(e.tor_cgo_build_body(8000, 2, 99, 7000, payload.length), 0);
  assert.equal(e.tor_cgo_parse_body(8000, 9000), 0);
  assert.equal(dv.getUint32(9000, true), 2);
  assert.equal(dv.getUint32(9004, true), payload.length);
  assert.equal(dv.getUint32(9008, true), 99);
  assert.equal(dv.getUint32(9012, true), 5);

  assert.equal(e.tor_cgo_encrypt_op_dest(6000, 3, 8000, 10000), 0);
  assert.notEqual(take(mem, 10000, 509).toString("hex"), take(mem, 8000, 493).toString("hex"));
  assert.equal(e.tor_cgo_decrypt_or(6200, 3, 10000, 11000), 1);
  assert.deepEqual(take(mem, 11016, 493), take(mem, 8000, 493));

  assert.equal(e.tor_cgo_state_init(6400, 5000), 0);
  assert.equal(e.tor_cgo_state_init(6600, 5000), 0);
  assert.equal(e.tor_cgo_encrypt_or(6400, 9, 8000, 12000), 0);
  assert.equal(e.tor_cgo_proc_or(6600, 9, 12000, 13000), 0);
  assert.notEqual(take(mem, 13000, 509).toString("hex"), take(mem, 12000, 509).toString("hex"));

  mem[10020] ^= 0xff;
  assert.equal(e.tor_cgo_state_init(6800, 5000), 0);
  assert.equal(e.tor_cgo_decrypt_or(6800, 3, 10000, 14000), 0);

  console.log("tor cgo crypto smoke passed");
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
