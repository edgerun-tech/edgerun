#!/usr/bin/env node

const assert = require("node:assert/strict");
const crypto = require("node:crypto");
const { execFileSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/app-primitives/tor-directory-rsa/tor-directory-rsa.wat");
const wasm = path.join(os.tmpdir(), `tor-directory-rsa-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "inherit" });
execFileSync("wasm-validate", [wasm], { stdio: "inherit" });

function put(mem, ptr, bytes) { mem.set(bytes, ptr); }

function derLen(buf, offset) {
  const first = buf[offset];
  if (first < 0x80) return { len: first, bytes: 1 };
  const n = first & 0x7f;
  let len = 0;
  for (let i = 0; i < n; i += 1) len = (len << 8) | buf[offset + 1 + i];
  return { len, bytes: 1 + n };
}

function readInteger(buf, offset) {
  assert.equal(buf[offset], 0x02);
  const l = derLen(buf, offset + 1);
  const start = offset + 1 + l.bytes;
  let out = buf.slice(start, start + l.len);
  while (out.length > 0 && out[0] === 0) out = out.slice(1);
  return { value: out, end: start + l.len };
}

function rsaComponents(publicKey) {
  const der = publicKey.export({ type: "pkcs1", format: "der" });
  assert.equal(der[0], 0x30);
  const seq = derLen(der, 1);
  const modulus = readInteger(der, 1 + seq.bytes);
  const exponent = readInteger(der, modulus.end);
  return { modulus: modulus.value, exponent: exponent.value };
}

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = instance.exports;
  const mem = new Uint8Array(e.memory.buffer);

  assert.equal(e.proto_standard_id(), 300223);
  assert.equal(e.proto_abi_version(), 1);
  assert.equal(e.tor_dir_rsa_algorithm_sha1_pkcs1(), 1);
  assert.equal(e.tor_dir_rsa_algorithm_sha256_pkcs1(), 2);

  const { publicKey, privateKey } = crypto.generateKeyPairSync("rsa", { modulusLength: 1024, publicExponent: 0x10001 });
  const { modulus, exponent } = rsaComponents(publicKey);
  assert.equal(modulus.length, 128);
  assert.equal(exponent.toString("hex"), "010001");
  const document = Buffer.from("network-status-version 3\nvote-status consensus\ndirectory-signature ", "ascii");
  const sig256 = crypto.sign("sha256", document, { key: privateKey, padding: crypto.constants.RSA_PKCS1_PADDING });
  const sig1 = crypto.sign("sha1", document, { key: privateKey, padding: crypto.constants.RSA_PKCS1_PADDING });

  put(mem, 1000, document);
  put(mem, 2000, exponent);
  put(mem, 2040, modulus);
  put(mem, 2300, sig256);
  put(mem, 2500, sig1);
  assert.equal(e.tor_dir_rsa_verify_pkcs1(1000, document.length, 2000, exponent.length, 2040, modulus.length, 2300, sig256.length, 2), 0);
  assert.equal(e.tor_dir_rsa_verify_pkcs1(1000, document.length, 2000, exponent.length, 2040, modulus.length, 2500, sig1.length, 1), 0);
  mem[1000 + 10] ^= 1;
  assert.equal(e.tor_dir_rsa_verify_pkcs1(1000, document.length, 2000, exponent.length, 2040, modulus.length, 2300, sig256.length, 2), -3);
  mem[1000 + 10] ^= 1;
  mem[2300] ^= 1;
  assert.equal(e.tor_dir_rsa_verify_pkcs1(1000, document.length, 2000, exponent.length, 2040, modulus.length, 2300, sig256.length, 2), -3);
  mem[2300] ^= 1;
  assert.equal(e.tor_dir_rsa_verify_pkcs1(1000, document.length, 2000, exponent.length, 2040, modulus.length, 2300, sig256.length - 1, 2), -2);
  assert.equal(e.tor_dir_rsa_verify_pkcs1(1000, document.length, 2000, exponent.length, 2040, modulus.length, 2300, sig256.length, 99), -4);

  console.log("tor directory rsa smoke passed");
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
}).finally(() => {
  try { fs.unlinkSync(wasm); } catch {}
});
