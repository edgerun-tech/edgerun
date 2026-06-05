#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/acme-core.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  const wasm = execFileSync("wat2wasm", [file, "-o", "-"]);
  execFileSync("wasm-validate", ["-"], { input: wasm });
  return wasm;
}

function put(memory, value, ptr = 1024) {
  const bytes = Buffer.from(value, "ascii");
  memory.fill(0, ptr, ptr + Math.max(64, bytes.length + 1));
  memory.set(bytes, ptr);
  return [ptr, bytes.length];
}

function code(e, memory, fn, value) {
  return e[fn](...put(memory, value));
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const e = instance.exports;
  const memory = new Uint8Array(e.memory.buffer);

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300132);
  assert.equal(code(e, memory, "acme_account_status_code", "valid"), 1);
  assert.equal(code(e, memory, "acme_account_status_code", "deactivated"), 2);
  assert.equal(code(e, memory, "acme_account_status_code", "revoked"), 3);
  assert.equal(code(e, memory, "acme_order_status_code", "pending"), 1);
  assert.equal(code(e, memory, "acme_order_status_code", "processing"), 3);
  assert.equal(code(e, memory, "acme_order_status_code", "invalid"), 5);
  assert.equal(code(e, memory, "acme_authorization_status_code", "expired"), 5);
  assert.equal(code(e, memory, "acme_challenge_status_code", "valid"), 3);
  assert.equal(code(e, memory, "acme_challenge_type_code", "http-01"), 1);
  assert.equal(code(e, memory, "acme_challenge_type_code", "dns-01"), 2);
  assert.equal(code(e, memory, "acme_challenge_type_code", "tls-alpn-01"), 3);
  assert.equal(code(e, memory, "acme_challenge_type_code", "other-01"), 4);
  assert.equal(e.acme_challenge_material_kind(1), 1);
  assert.equal(e.acme_challenge_material_kind(2), 2);
  assert.equal(e.acme_challenge_material_kind(3), 3);
  assert.equal(e.acme_key_authorization_parts_valid(5, 10), 1);
  assert.equal(e.acme_key_authorization_parts_valid(0, 10), 0);
  assert.equal(code(e, memory, "acme_directory_builtin_code", "letsencrypt"), 1);
  assert.equal(code(e, memory, "acme_directory_builtin_code", "letsencryptstaging"), 2);
  assert.equal(e.acme_jwk_kind(1, 1, 0, 0, 0), 1);
  assert.equal(e.acme_jwk_kind(0, 0, 1, 1, 1), 2);
  assert.equal(e.acme_jwk_kind(0, 0, 1, 1, 0), 0);

  console.log(JSON.stringify({ unit: "acme-core", standard_id: e.proto_standard_id(), ok: true }));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
