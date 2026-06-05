#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/x509-certificate-core.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  return execFileSync("wat2wasm", [file, "-o", "-"]);
}

function len(n) {
  if (n < 0x80) return [n];
  if (n <= 0xff) return [0x81, n];
  return [0x82, (n >>> 8) & 0xff, n & 0xff];
}

function tlv(tag, value) {
  return [tag, ...len(value.length), ...value];
}

function seq(...children) {
  return tlv(0x30, children.flat());
}

function int(bytes) {
  return tlv(0x02, bytes);
}

function oid(...bytes) {
  return tlv(0x06, bytes);
}

function nul() {
  return [0x05, 0x00];
}

function bitString(bytes, unused = 0) {
  return tlv(0x03, [unused, ...bytes]);
}

function utc(value) {
  return tlv(0x17, [...Buffer.from(value, "ascii")]);
}

function explicit(tag, value) {
  return tlv(tag, value);
}

function u32(memory, offset) {
  return (
    memory[offset] |
    (memory[offset + 1] << 8) |
    (memory[offset + 2] << 16) |
    (memory[offset + 3] << 24)
  ) >>> 0;
}

function bytes(memory, ptr, len) {
  return Array.from(memory.slice(ptr, ptr + len));
}

function write(memory, data, ptr = 1024) {
  memory.fill(0, ptr, ptr + 4096);
  memory.set(data, ptr);
  return ptr;
}

function certRecord(memory, out = 8192) {
  return {
    certBodyPtr: u32(memory, out),
    certBodyLen: u32(memory, out + 4),
    certHeaderLen: u32(memory, out + 8),
    certTotalLen: u32(memory, out + 12),
    tbsPtr: u32(memory, out + 16),
    tbsTotalLen: u32(memory, out + 20),
    tbsHeaderLen: u32(memory, out + 24),
    tbsBodyLen: u32(memory, out + 28),
    signatureAlgorithmPtr: u32(memory, out + 32),
    signatureAlgorithmTotalLen: u32(memory, out + 36),
    signatureAlgorithmHeaderLen: u32(memory, out + 40),
    signatureAlgorithmBodyLen: u32(memory, out + 44),
    signatureValuePtr: u32(memory, out + 48),
    signatureValueTotalLen: u32(memory, out + 52),
    signatureValueHeaderLen: u32(memory, out + 56),
    signatureValuePayloadPtr: u32(memory, out + 60),
    signatureValuePayloadLen: u32(memory, out + 64),
    signatureValueUnusedBits: u32(memory, out + 68),
  };
}

function span(memory, offset) {
  return {
    ptr: u32(memory, offset),
    len: u32(memory, offset + 4),
    headerLen: u32(memory, offset + 8),
    totalLen: u32(memory, offset + 12),
  };
}

function tbsRecord(memory, out = 8192) {
  return {
    tbs: span(memory, out),
    version: span(memory, out + 16),
    versionPresent: u32(memory, out + 32),
    serial: span(memory, out + 36),
    signature: span(memory, out + 52),
    issuer: span(memory, out + 68),
    validity: span(memory, out + 84),
    subject: span(memory, out + 100),
    spki: span(memory, out + 116),
    extensions: span(memory, out + 132),
    extensionsPresent: u32(memory, out + 148),
  };
}

function fixture({ version = true, extensions = true } = {}) {
  const alg = seq(oid(0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b), nul());
  const name = seq();
  const validity = seq(utc("240101000000Z"), utc("250101000000Z"));
  const spki = seq(
    seq(oid(0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01), nul()),
    bitString([0x30, 0x00])
  );
  const tbsChildren = [];
  if (version) tbsChildren.push(explicit(0xa0, int([0x02])));
  tbsChildren.push(int([0x01]), alg, name, validity, name, spki);
  if (extensions) tbsChildren.push(explicit(0xa3, seq()));
  const tbs = seq(...tbsChildren);
  const cert = seq(tbs, alg, bitString([0xaa, 0xbb, 0xcc]));
  return { alg, cert, tbs };
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const exports = instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300028);

  const full = fixture();
  let ptr = write(memory, full.cert);
  assert.strictEqual(exports.x509_certificate_scan(ptr, full.cert.length, 8192), 0);
  let cert = certRecord(memory);
  assert.deepStrictEqual(bytes(memory, cert.tbsPtr, cert.tbsTotalLen), full.tbs);
  assert.strictEqual(cert.certTotalLen, full.cert.length);
  assert.strictEqual(cert.tbsHeaderLen, full.tbs.length - cert.tbsBodyLen);
  assert.strictEqual(cert.signatureAlgorithmTotalLen, full.alg.length);
  assert.strictEqual(cert.signatureValuePayloadLen, 3);
  assert.strictEqual(cert.signatureValueUnusedBits, 0);

  assert.strictEqual(exports.x509_tbs_certificate_scan(cert.tbsPtr, cert.tbsTotalLen, 8192), 0);
  let tbs = tbsRecord(memory);
  assert.strictEqual(tbs.versionPresent, 1);
  assert.strictEqual(tbs.serial.len, 1);
  assert.strictEqual(tbs.signature.totalLen, full.alg.length);
  assert.strictEqual(tbs.issuer.totalLen, 2);
  assert.strictEqual(tbs.subject.totalLen, 2);
  assert.strictEqual(tbs.extensionsPresent, 1);
  assert.deepStrictEqual(bytes(memory, cert.tbsPtr, cert.tbsTotalLen), full.tbs);

  const v1 = fixture({ version: false, extensions: false });
  ptr = write(memory, v1.cert);
  assert.strictEqual(exports.x509_certificate_scan(ptr, v1.cert.length, 8192), 0);
  cert = certRecord(memory);
  assert.deepStrictEqual(bytes(memory, cert.tbsPtr, cert.tbsTotalLen), v1.tbs);
  assert.strictEqual(exports.x509_tbs_certificate_scan(cert.tbsPtr, cert.tbsTotalLen, 8192), 0);
  tbs = tbsRecord(memory);
  assert.strictEqual(tbs.versionPresent, 0);
  assert.strictEqual(tbs.extensionsPresent, 0);

  assert.strictEqual(exports.x509_certificate_scan(ptr, v1.cert.length - 1, 8192), 1);
  assert.strictEqual(exports.x509_certificate_scan(ptr, v1.cert.length + 1, 8192), 3);
  assert.strictEqual(exports.x509_certificate_scan(write(memory, [0x31, 0x00]), 2, 8192), 3);

  const missingSpki = seq(int([1]), full.alg, seq(), seq(utc("240101000000Z"), utc("250101000000Z")), seq());
  assert.strictEqual(exports.x509_tbs_certificate_scan(write(memory, missingSpki), missingSpki.length, 8192), 1);

  const badSigBits = seq(v1.tbs, v1.alg, tlv(0x03, [8, 0xaa]));
  assert.strictEqual(exports.x509_certificate_scan(write(memory, badSigBits), badSigBits.length, 8192), 3);

  console.log(JSON.stringify({
    unit: "x509-certificate-core",
    abi_version: exports.proto_abi_version(),
    standard_id: exports.proto_standard_id(),
    ok: true,
  }, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
