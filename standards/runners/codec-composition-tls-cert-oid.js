#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { spawnSync } = require("child_process");

const root = path.resolve(__dirname, "../..");
const watRoot = path.join(root, "standards/build/wasm/codec-primitives");
const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "codec-composition-tls-cert-oid-"));
const wat2wasm = resolveTool("wat2wasm");
const wasmValidate = resolveTool("wasm-validate");

process.on("exit", () => {
  fs.rmSync(tmpRoot, { recursive: true, force: true });
});

function resolveTool(name) {
  for (const dir of (process.env.PATH || "").split(path.delimiter)) {
    const candidate = path.join(dir, name);
    try {
      fs.accessSync(candidate, fs.constants.X_OK);
      return candidate;
    } catch (_) {
      // Keep scanning PATH.
    }
  }
  return name;
}

function compileWat(name) {
  const watPath = path.join(watRoot, `${name}.wat`);
  const wasmPath = path.join(tmpRoot, `${name}.wasm`);
  fs.accessSync(watPath, fs.constants.R_OK);
  runTool(wat2wasm, [watPath, "-o", wasmPath]);
  runTool(wasmValidate, [wasmPath]);
  return fs.readFileSync(wasmPath);
}

function runTool(file, args) {
  const result = spawnSync(file, args, { stdio: "pipe" });
  if (result.status !== 0) {
    throw result.error || new Error(`${file} failed with status ${result.status}`);
  }
}

async function instantiate(name) {
  const { instance } = await WebAssembly.instantiate(compileWat(name), {});
  return {
    exports: instance.exports,
    memory: new Uint8Array(instance.exports.memory.buffer),
  };
}

function u16(n) {
  return [(n >>> 8) & 0xff, n & 0xff];
}

function u24(n) {
  return [(n >>> 16) & 0xff, (n >>> 8) & 0xff, n & 0xff];
}

function u32(memory, offset) {
  return (
    memory[offset] |
    (memory[offset + 1] << 8) |
    (memory[offset + 2] << 16) |
    (memory[offset + 3] << 24)
  ) >>> 0;
}

function tlsRecordParts(value) {
  return {
    status: Number(value & 0xffffn),
    contentType: Number((value >> 16n) & 0xffn),
    version: Number((value >> 24n) & 0xffffn),
    fragmentLen: Number((value >> 40n) & 0xffffn),
  };
}

function tlsHandshakeParts(value) {
  return {
    status: Number(value & 0xffffn),
    handshakeType: Number((value >> 16n) & 0xffn),
    bodyLen: Number((value >> 24n) & 0xffffffn),
  };
}

function listRecord(memory, offset) {
  return {
    listOffset: u32(memory, offset + 8),
    listLen: u32(memory, offset + 12),
  };
}

function entryRecord(memory, offset) {
  return {
    certOffset: u32(memory, offset),
    certLen: u32(memory, offset + 4),
  };
}

function derHeaderParts(value) {
  return {
    status: Number(value & 0xffffn),
    consumed: Number((value >> 16n) & 0xffffn),
    tag: Number((value >> 32n) & 0xffn),
    length: Number((value >> 40n) & 0xffffffn),
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

function sequenceChild(memory, offset) {
  return {
    tag: u32(memory, offset),
    headerLen: u32(memory, offset + 4),
    valueOffset: u32(memory, offset + 8),
    valueLen: u32(memory, offset + 12),
    totalLen: u32(memory, offset + 16),
    nextOffset: u32(memory, offset + 20),
  };
}

(async () => {
  const tlsFrame = await instantiate("tls-frame");
  const certList = await instantiate("tls-certificate-list");
  const derTlv = await instantiate("der-tlv");
  const derAsn1 = await instantiate("der-asn1-basic");
  const derOid = await instantiate("der-oid");

  const der = [0x30, 0x05, 0x06, 0x03, 0x2a, 0x03, 0x04];
  const certEntry = [...u24(der.length), ...der, ...u16(0)];
  const certBody = [0, ...u24(certEntry.length), ...certEntry];
  const record = [0x16, 0x03, 0x03, ...u16(certBody.length + 4), 0x0b, ...u24(certBody.length), ...certBody];

  tlsFrame.memory.set(record, 1024);
  const recordHeader = tlsRecordParts(tlsFrame.exports.tls_record_header_decode(1024, record.length));
  assert.deepStrictEqual(recordHeader, {
    status: 0,
    contentType: 0x16,
    version: 0x0303,
    fragmentLen: certBody.length + 4,
  });
  const handshake = tlsHandshakeParts(tlsFrame.exports.tls_handshake_header_decode(1029, recordHeader.fragmentLen));
  assert.deepStrictEqual(handshake, { status: 0, handshakeType: 11, bodyLen: certBody.length });

  certList.memory.set(record.slice(9), 1024);
  assert.strictEqual(certList.exports.tls_certificate_list_scan(1024, handshake.bodyLen, 2048), 0);
  const list = listRecord(certList.memory, 2048);
  assert.strictEqual(certList.exports.tls_certificate_entry_next(1024 + list.listOffset, list.listLen, 0, 2048), 0);
  const entry = entryRecord(certList.memory, 2048);
  assert.deepStrictEqual({ certLen: entry.certLen }, { certLen: der.length });
  const certDer = Array.from(certList.memory.slice(1024 + list.listOffset + entry.certOffset, 1024 + list.listOffset + entry.certOffset + entry.certLen));

  derTlv.memory.set(certDer, 1024);
  assert.deepStrictEqual(derHeaderParts(derTlv.exports.der_header_decode(1024, certDer.length)), {
    status: 0,
    consumed: 2,
    tag: 0x30,
    length: 5,
  });

  derAsn1.memory.set(certDer, 1024);
  assert.strictEqual(derAsn1.exports.der_asn1_sequence_decode(1024, certDer.length, 2048), 0);
  const sequence = span(derAsn1.memory, 2048);
  assert.deepStrictEqual(sequence, { ptr: 1026, len: 5, headerLen: 2, totalLen: 7 });
  assert.strictEqual(derAsn1.exports.der_asn1_sequence_next_child(sequence.ptr, sequence.len, 0, 3072), 0);
  const oidChild = sequenceChild(derAsn1.memory, 3072);
  assert.deepStrictEqual(
    { tag: oidChild.tag, headerLen: oidChild.headerLen, valueLen: oidChild.valueLen, totalLen: oidChild.totalLen },
    { tag: 6, headerLen: 2, valueLen: 3, totalLen: 5 },
  );

  const oidValue = Array.from(derAsn1.memory.slice(sequence.ptr + oidChild.headerLen, sequence.ptr + oidChild.headerLen + oidChild.valueLen));
  derOid.memory.set(oidValue, 1024);
  assert.strictEqual(derOid.exports.der_oid_value_validate(1024, oidValue.length, 2048), 0);
  assert.strictEqual(derOid.exports.der_oid_root_decode(1024, oidValue.length, 2048), 0);
  assert.deepStrictEqual([u32(derOid.memory, 2048), u32(derOid.memory, 2052)], [1, 2]);
  assert.strictEqual(derOid.exports.der_oid_next_arc(1024, oidValue.length, u32(derOid.memory, 2056), 2048), 0);
  assert.strictEqual(u32(derOid.memory, 2048), 3);
  assert.strictEqual(derOid.exports.der_oid_next_arc(1024, oidValue.length, u32(derOid.memory, 2052), 2048), 0);
  assert.strictEqual(u32(derOid.memory, 2048), 4);

  console.log(JSON.stringify({
    runner: "codec-composition-tls-cert-oid",
    pipeline: "tls-frame -> tls-certificate-list -> der-tlv -> der-asn1-basic -> der-oid",
    ok: true,
    oid: "1.2.3.4",
  }, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
