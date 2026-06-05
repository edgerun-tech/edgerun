#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const root = path.resolve(__dirname, "..", "..");
const codecDir = path.join(root, "standards/build/wasm/codec-primitives");

function compileWat(name) {
  const watPath = path.join(codecDir, `${name}.wat`);
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), `${name}-composition-`));
  const wasmPath = path.join(tmp, `${name}.wasm`);
  execFileSync("wat2wasm", [watPath, "-o", wasmPath], { stdio: "pipe" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "pipe" });
  const bytes = fs.readFileSync(wasmPath);
  fs.rmSync(tmp, { recursive: true, force: true });
  return bytes;
}

async function load(name) {
  const { instance } = await WebAssembly.instantiate(compileWat(name), {});
  return {
    exports: instance.exports,
    memory: new Uint8Array(instance.exports.memory.buffer),
    view: new DataView(instance.exports.memory.buffer),
  };
}

function u16(n) {
  return [(n >>> 8) & 0xff, n & 0xff];
}

function u24(n) {
  return [(n >>> 16) & 0xff, (n >>> 8) & 0xff, n & 0xff];
}

function u32(view, ptr) {
  return view.getUint32(ptr, true);
}

function tlsRecord(value) {
  return {
    status: Number(value & 0xffffn),
    contentType: Number((value >> 16n) & 0xffn),
    version: Number((value >> 24n) & 0xffffn),
    fragmentLen: Number((value >> 40n) & 0xffffn),
  };
}

function tlsHandshake(value) {
  return {
    status: Number(value & 0xffffn),
    handshakeType: Number((value >> 16n) & 0xffn),
    bodyLen: Number((value >> 24n) & 0xffffffn),
  };
}

function certListRecord(view, ptr) {
  return {
    listOffset: u32(view, ptr + 8),
    listLen: u32(view, ptr + 12),
  };
}

function certEntryRecord(view, ptr) {
  return {
    certOffset: u32(view, ptr),
    certLen: u32(view, ptr + 4),
  };
}

function asn1Span(view, ptr) {
  return {
    ptr: u32(view, ptr),
    len: u32(view, ptr + 4),
    headerLen: u32(view, ptr + 8),
    totalLen: u32(view, ptr + 12),
  };
}

function seqChild(view, ptr) {
  return {
    tag: u32(view, ptr),
    headerLen: u32(view, ptr + 4),
    valueLen: u32(view, ptr + 12),
    totalLen: u32(view, ptr + 16),
    nextOffset: u32(view, ptr + 20),
  };
}

(async () => {
  const tlsFrame = await load("tls-frame");
  const certList = await load("tls-certificate-list");
  const derTlv = await load("der-tlv");
  const derAsn1 = await load("der-asn1-basic");
  const derOid = await load("der-oid");

  const oidTlv = [0x06, 0x03, 0x2a, 0x86, 0x48]; // 1.2.840
  const der = [0x30, oidTlv.length, ...oidTlv];
  const certEntry = [...u24(der.length), ...der, ...u16(0)];
  const certBody = [0, ...u24(certEntry.length), ...certEntry];
  const handshake = [0x0b, ...u24(certBody.length), ...certBody];
  const record = [0x16, 0x03, 0x03, ...u16(handshake.length), ...handshake];

  tlsFrame.memory.set(record, 1024);
  const rec = tlsRecord(tlsFrame.exports.tls_record_header_decode(1024, record.length));
  assert.equal(rec.status, 0);
  assert.equal(rec.contentType, 0x16);
  const hs = tlsHandshake(tlsFrame.exports.tls_handshake_header_decode(1029, rec.fragmentLen));
  assert.deepEqual(hs, { status: 0, handshakeType: 0x0b, bodyLen: certBody.length });

  certList.memory.set(tlsFrame.memory.slice(1033, 1033 + hs.bodyLen), 1024);
  assert.equal(certList.exports.tls_certificate_list_scan(1024, hs.bodyLen, 2048), 0);
  const list = certListRecord(certList.view, 2048);
  assert.equal(certList.exports.tls_certificate_entry_next(1024 + list.listOffset, list.listLen, 0, 2048), 0);
  const entry = certEntryRecord(certList.view, 2048);
  const certBytes = certList.memory.slice(1024 + list.listOffset + entry.certOffset, 1024 + list.listOffset + entry.certOffset + entry.certLen);

  derTlv.memory.set(certBytes, 1024);
  const header = derTlv.exports.der_header_decode(1024, certBytes.length);
  assert.equal(Number(header & 0xffffn), 0);
  assert.equal(Number((header >> 32n) & 0xffn), 0x30);
  assert.equal(Number((header >> 40n) & 0xffffffn), oidTlv.length);

  derAsn1.memory.set(certBytes, 1024);
  assert.equal(derAsn1.exports.der_asn1_sequence_decode(1024, certBytes.length, 2048), 0);
  const seq = asn1Span(derAsn1.view, 2048);
  assert.equal(derAsn1.exports.der_asn1_sequence_next_child(seq.ptr, seq.len, 0, 3072), 0);
  const child = seqChild(derAsn1.view, 3072);
  assert.deepEqual(child, { tag: 0x06, headerLen: 2, valueLen: 3, totalLen: 5, nextOffset: 5 });

  const oidValue = derAsn1.memory.slice(seq.ptr + child.headerLen, seq.ptr + child.totalLen);
  derOid.memory.set(oidValue, 1024);
  assert.equal(derOid.exports.der_oid_root_decode(1024, oidValue.length, 2048), 0);
  assert.equal(u32(derOid.view, 2048), 1);
  assert.equal(u32(derOid.view, 2052), 2);
  assert.equal(derOid.exports.der_oid_next_arc(1024, oidValue.length, u32(derOid.view, 2056), 2048), 0);
  assert.equal(u32(derOid.view, 2048), 840);
  assert.equal(derOid.exports.der_oid_value_validate(1024, oidValue.length, 2048), 0);

  console.log(JSON.stringify({
    unit: "codec-composition-tls-certificate-der",
    ok: true,
    pipeline: "tls-frame -> tls-certificate-list -> der-tlv -> der-asn1-basic -> der-oid",
  }, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
