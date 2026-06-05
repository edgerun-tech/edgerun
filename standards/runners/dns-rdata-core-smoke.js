#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/dns-rdata-core.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  return execFileSync("wat2wasm", [file, "-o", "-"]);
}

function wireName(labels) {
  const out = [];
  for (const label of labels) {
    const bytes = Buffer.from(label, "ascii");
    out.push(bytes.length, ...bytes);
  }
  out.push(0);
  return out;
}

function u16(v) {
  return [(v >> 8) & 0xff, v & 0xff];
}

function writeBytes(memory, offset, bytes) {
  memory.fill(0, offset, offset + Math.max(bytes.length, 256));
  memory.set(Uint8Array.from(bytes), offset);
  return bytes.length;
}

function readU32(view, offset) {
  return view.getUint32(offset, true);
}

function readWords(view, offset, count) {
  return Array.from({ length: count }, (_, i) => readU32(view, offset + i * 4));
}

function nameRecord(view, offset) {
  const labels = readU32(view, offset + 12);
  return {
    rdataStart: readU32(view, offset),
    rdlen: readU32(view, offset + 4),
    consumed: readU32(view, offset + 8),
    labelCount: labels,
    normalizedLen: readU32(view, offset + 16),
    pointerCount: readU32(view, offset + 20),
    terminalOffset: readU32(view, offset + 24),
    labels: Array.from({ length: labels }, (_, i) => ({
      offset: readU32(view, offset + 28 + i * 8),
      len: readU32(view, offset + 32 + i * 8),
    })),
  };
}

function mxRecord(view, offset) {
  return {
    preference: readU32(view, offset),
    exchangeStart: readU32(view, offset + 4),
    exchangeWireLen: readU32(view, offset + 8),
    labelCount: readU32(view, offset + 12),
    normalizedLen: readU32(view, offset + 16),
    pointerCount: readU32(view, offset + 20),
    terminalOffset: readU32(view, offset + 24),
  };
}

function txtRecord(view, offset) {
  const count = readU32(view, offset);
  return {
    stringCount: count,
    totalTextBytes: readU32(view, offset + 4),
    firstOffset: readU32(view, offset + 8),
    firstLen: readU32(view, offset + 12),
    lastOffset: readU32(view, offset + 16),
    lastLen: readU32(view, offset + 20),
    strings: Array.from({ length: count }, (_, i) => ({
      offset: readU32(view, offset + 24 + i * 8),
      len: readU32(view, offset + 28 + i * 8),
    })),
  };
}

(async () => {
  const instance = await WebAssembly.instantiate(await WebAssembly.compile(compileWat(watPath)), {});
  const e = instance.exports;
  const memory = new Uint8Array(e.memory.buffer);
  const view = new DataView(e.memory.buffer);
  const inPtr = 1024;
  const outPtr = 4096;

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300035);

  let len = writeBytes(memory, inPtr, [192, 0, 2, 1]);
  assert.equal(e.dns_rdata_a_decode(inPtr, len, 0, 4, outPtr), 0);
  assert.deepEqual(readWords(view, outPtr, 5), [192, 0, 2, 1, 0xc0000201]);
  assert.equal(e.dns_rdata_a_decode(inPtr, len, 0, 3, outPtr), 3);
  assert.equal(e.dns_rdata_a_decode(inPtr, 3, 0, 4, outPtr), 1);

  len = writeBytes(memory, inPtr, [
    0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 1,
  ]);
  assert.equal(e.dns_rdata_aaaa_decode(inPtr, len, 0, 16, outPtr), 0);
  assert.deepEqual(readWords(view, outPtr, 8), [0x2001, 0x0db8, 0, 0, 0, 0, 0, 1]);
  assert.equal(e.dns_rdata_aaaa_decode(inPtr, len, 0, 15, outPtr), 3);
  assert.equal(e.dns_rdata_aaaa_decode(inPtr, 15, 0, 16, outPtr), 1);

  const owner = wireName(["Example", "COM"]);
  const aliasOffset = owner.length;
  const msg = [...owner, 3, 0x57, 0x57, 0x57, 0xc0, 0x00];
  len = writeBytes(memory, inPtr, msg);
  assert.equal(e.dns_rdata_name_decode(inPtr, len, aliasOffset, 6, outPtr, 64), 0);
  assert.deepEqual(nameRecord(view, outPtr), {
    rdataStart: aliasOffset,
    rdlen: 6,
    consumed: 6,
    labelCount: 3,
    normalizedLen: 15,
    pointerCount: 1,
    terminalOffset: 12,
    labels: [
      { offset: aliasOffset + 1, len: 3 },
      { offset: 1, len: 7 },
      { offset: 9, len: 3 },
    ],
  });
  assert.equal(e.dns_rdata_name_decode(inPtr, len, aliasOffset, 5, outPtr, 64), 3);
  assert.equal(e.dns_rdata_name_decode(inPtr, len, aliasOffset, 6, outPtr, 27), 2);

  len = writeBytes(memory, inPtr, [0xc0, 0x00]);
  assert.equal(e.dns_rdata_name_decode(inPtr, len, 0, 2, outPtr, 64), 3);

  const mxExchange = wireName(["MAIL", "Example", "COM"]);
  len = writeBytes(memory, inPtr, [...u16(10), ...mxExchange]);
  assert.equal(e.dns_rdata_mx_decode(inPtr, len, 0, len, outPtr, 28), 0);
  assert.deepEqual(mxRecord(view, outPtr), {
    preference: 10,
    exchangeStart: 2,
    exchangeWireLen: mxExchange.length,
    labelCount: 3,
    normalizedLen: 16,
    pointerCount: 0,
    terminalOffset: len - 1,
  });
  assert.equal(e.dns_rdata_mx_decode(inPtr, len, 0, 2, outPtr, 28), 3);
  assert.equal(e.dns_rdata_mx_decode(inPtr, len - 1, 0, len, outPtr, 28), 1);

  len = writeBytes(memory, inPtr, [3, 0x66, 0x6f, 0x6f, 0, 3, 0x62, 0x61, 0x72]);
  assert.equal(e.dns_rdata_txt_walk(inPtr, len, 0, len, outPtr, 64), 0);
  assert.deepEqual(txtRecord(view, outPtr), {
    stringCount: 3,
    totalTextBytes: 6,
    firstOffset: 1,
    firstLen: 3,
    lastOffset: 6,
    lastLen: 3,
    strings: [
      { offset: 1, len: 3 },
      { offset: 5, len: 0 },
      { offset: 6, len: 3 },
    ],
  });
  assert.equal(e.dns_rdata_txt_walk(inPtr, len, 0, len, outPtr, 39), 2);

  len = writeBytes(memory, inPtr, [4, 0x66, 0x6f, 0x6f]);
  assert.equal(e.dns_rdata_txt_walk(inPtr, len, 0, len, outPtr, 64), 1);

  len = writeBytes(memory, inPtr, []);
  assert.equal(e.dns_rdata_txt_walk(inPtr, len, 0, 0, outPtr, 24), 0);
  assert.deepEqual(txtRecord(view, outPtr), {
    stringCount: 0,
    totalTextBytes: 0,
    firstOffset: 0,
    firstLen: 0,
    lastOffset: 0,
    lastLen: 0,
    strings: [],
  });

  console.log(JSON.stringify({
    unit: "dns-rdata-core",
    abi_version: e.proto_abi_version(),
    standard_id: e.proto_standard_id(),
    ok: true,
  }, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
