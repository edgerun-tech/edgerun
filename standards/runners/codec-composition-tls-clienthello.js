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

function clientHelloRecord(view, ptr) {
  return {
    extensionsOffset: u32(view, ptr + 40),
    extensionsLen: u32(view, ptr + 44),
  };
}

function extRecord(view, ptr) {
  return {
    type: u32(view, ptr),
    dataOffset: u32(view, ptr + 4),
    dataLen: u32(view, ptr + 8),
    nextOffset: u32(view, ptr + 12),
  };
}

function alpnRecord(view, ptr) {
  return {
    protoOffset: u32(view, ptr),
    protoLen: u32(view, ptr + 4),
    nextOffset: u32(view, ptr + 8),
    listLen: u32(view, ptr + 12),
  };
}

function span2(view, ptr) {
  return {
    offset: u32(view, ptr),
    len: u32(view, ptr + 4),
  };
}

function ext(type, data) {
  return [...u16(type), ...u16(data.length), ...data];
}

function sni(host) {
  const hostBytes = [...Buffer.from(host, "ascii")];
  const entry = [0, ...u16(hostBytes.length), ...hostBytes];
  return [...u16(entry.length), ...entry];
}

function alpn(protocols) {
  const list = protocols.flatMap((proto) => {
    const bytes = [...Buffer.from(proto, "ascii")];
    return [bytes.length, ...bytes];
  });
  return [...u16(list.length), ...list];
}

function clientHelloBody() {
  const random = Array.from({ length: 32 }, (_, i) => i);
  const cipherSuites = [0x13, 0x01, 0x13, 0x02];
  const compression = [0];
  const extensions = [
    ...ext(0, sni("Example.COM.")),
    ...ext(16, alpn(["h3", "acme-tls/1"])),
  ];
  return [
    0x03, 0x03,
    ...random,
    0,
    ...u16(cipherSuites.length), ...cipherSuites,
    compression.length, ...compression,
    ...u16(extensions.length), ...extensions,
  ];
}

(async () => {
  const tlsFrame = await load("tls-frame");
  const clientHello = await load("tls-clienthello");
  const tlsVector = await load("tls-vector");
  const tlsName = await load("tls-name");

  const body = clientHelloBody();
  const handshake = [0x01, ...u24(body.length), ...body];
  const record = [0x16, 0x03, 0x03, ...u16(handshake.length), ...handshake];

  tlsFrame.memory.set(record, 1024);
  const rec = tlsRecord(tlsFrame.exports.tls_record_header_decode(1024, record.length));
  assert.deepEqual(rec, {
    status: 0,
    contentType: 0x16,
    version: 0x0303,
    fragmentLen: handshake.length,
  });

  const hs = tlsHandshake(tlsFrame.exports.tls_handshake_header_decode(1029, rec.fragmentLen));
  assert.deepEqual(hs, {
    status: 0,
    handshakeType: 0x01,
    bodyLen: body.length,
  });

  clientHello.memory.set(tlsFrame.memory.slice(1033, 1033 + hs.bodyLen), 1024);
  assert.equal(clientHello.exports.tls_clienthello_scan(1024, hs.bodyLen, 2048), 0);
  const ch = clientHelloRecord(clientHello.view, 2048);
  const extensions = clientHello.memory.slice(1024 + ch.extensionsOffset, 1024 + ch.extensionsOffset + ch.extensionsLen);

  tlsVector.memory.set(extensions, 1024);
  assert.equal(tlsVector.exports.tls_extension_next(1024, extensions.length, 0, 2048), 0);
  let extRec = extRecord(tlsVector.view, 2048);
  assert.equal(extRec.type, 0);

  clientHello.memory.set(tlsVector.memory.slice(1024 + extRec.dataOffset, 1024 + extRec.dataOffset + extRec.dataLen), 4096);
  assert.equal(clientHello.exports.tls_clienthello_sni_host(4096, extRec.dataLen, 8192), 0);
  const host = span2(clientHello.view, 8192);
  const hostBytes = clientHello.memory.slice(4096 + host.offset, 4096 + host.offset + host.len);

  tlsName.memory.set(hostBytes, 1024);
  const packed = tlsName.exports.tls_dns_name_normalize(1024, hostBytes.length, 2048, 128);
  assert.equal(Number(packed & 0xffffffffn), 0);
  const normalizedLen = Number((packed >> 32n) & 0xffffffffn);
  assert.equal(Buffer.from(tlsName.memory.slice(2048, 2048 + normalizedLen)).toString("ascii"), "example.com");

  tlsName.memory.set(Buffer.from("example.com", "ascii"), 3072);
  tlsName.memory.set(hostBytes, 4096);
  assert.equal(tlsName.exports.tls_dns_name_matches(3072, 11, 4096, hostBytes.length), 1);

  assert.equal(tlsVector.exports.tls_extension_next(1024, extensions.length, extRec.nextOffset, 2048), 0);
  extRec = extRecord(tlsVector.view, 2048);
  assert.equal(extRec.type, 16);
  assert.equal(tlsVector.exports.tls_alpn_next(1024 + extRec.dataOffset, extRec.dataLen, 0, 2048), 0);
  let proto = alpnRecord(tlsVector.view, 2048);
  assert.equal(Buffer.from(tlsVector.memory.slice(1024 + extRec.dataOffset + proto.protoOffset, 1024 + extRec.dataOffset + proto.protoOffset + proto.protoLen)).toString("ascii"), "h3");
  assert.equal(tlsVector.exports.tls_alpn_next(1024 + extRec.dataOffset, extRec.dataLen, proto.nextOffset, 2048), 0);
  proto = alpnRecord(tlsVector.view, 2048);
  assert.equal(Buffer.from(tlsVector.memory.slice(1024 + extRec.dataOffset + proto.protoOffset, 1024 + extRec.dataOffset + proto.protoOffset + proto.protoLen)).toString("ascii"), "acme-tls/1");

  console.log(JSON.stringify({
    unit: "codec-composition-tls-clienthello",
    ok: true,
    pipeline: "tls-frame -> tls-clienthello -> tls-vector -> tls-name",
  }, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
