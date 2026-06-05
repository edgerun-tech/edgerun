#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/dns-section-walk.wat");

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

function u32(v) {
  return [(v >>> 24) & 0xff, (v >>> 16) & 0xff, (v >>> 8) & 0xff, v & 0xff];
}

function writeBytes(memory, offset, bytes) {
  memory.set(Uint8Array.from(bytes), offset);
  return bytes.length;
}

function readU32(view, offset) {
  return view.getUint32(offset, true);
}

function summary(view, offset) {
  return {
    qStart: readU32(view, offset),
    qEnd: readU32(view, offset + 4),
    anStart: readU32(view, offset + 8),
    anEnd: readU32(view, offset + 12),
    nsStart: readU32(view, offset + 16),
    nsEnd: readU32(view, offset + 20),
    arStart: readU32(view, offset + 24),
    arEnd: readU32(view, offset + 28),
    qCount: readU32(view, offset + 32),
    rrCount: readU32(view, offset + 36),
    optCount: readU32(view, offset + 40),
    errorOffset: readU32(view, offset + 44),
  };
}

function question(view, offset) {
  return {
    nameStart: readU32(view, offset),
    nameWireLen: readU32(view, offset + 4),
    qtype: readU32(view, offset + 8),
    qclass: readU32(view, offset + 12),
    next: readU32(view, offset + 16),
  };
}

function rr(view, offset) {
  return {
    nameStart: readU32(view, offset),
    nameWireLen: readU32(view, offset + 4),
    type: readU32(view, offset + 8),
    klass: readU32(view, offset + 12),
    ttl: readU32(view, offset + 16),
    rdlen: readU32(view, offset + 20),
    rdataStart: readU32(view, offset + 24),
    next: readU32(view, offset + 28),
    isOpt: readU32(view, offset + 32),
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
  assert.equal(e.proto_standard_id(), 300034);

  const questionBytes = [...wireName(["Example", "COM"]), ...u16(1), ...u16(1)];
  const answerOffset = 12 + questionBytes.length;
  const answerBytes = [
    0xc0, 0x0c,
    ...u16(1),
    ...u16(1),
    ...u32(60),
    ...u16(4),
    127, 0, 0, 1,
  ];
  const optOffset = answerOffset + answerBytes.length;
  const optBytes = [
    0,
    ...u16(41),
    ...u16(1232),
    ...u32(0),
    ...u16(0),
  ];
  const msg = [
    ...u16(0x1234),
    ...u16(0x8180),
    ...u16(1),
    ...u16(1),
    ...u16(0),
    ...u16(1),
    ...questionBytes,
    ...answerBytes,
    ...optBytes,
  ];
  const len = writeBytes(memory, inPtr, msg);
  assert.equal(e.dns_section_walk(inPtr, len, outPtr, 48), 0);
  assert.deepEqual(summary(view, outPtr), {
    qStart: 12,
    qEnd: answerOffset,
    anStart: answerOffset,
    anEnd: optOffset,
    nsStart: optOffset,
    nsEnd: optOffset,
    arStart: optOffset,
    arEnd: len,
    qCount: 1,
    rrCount: 2,
    optCount: 1,
    errorOffset: 0,
  });

  assert.equal(e.dns_question_next(inPtr, len, 12, outPtr), 0);
  assert.deepEqual(question(view, outPtr), {
    nameStart: 12,
    nameWireLen: wireName(["Example", "COM"]).length,
    qtype: 1,
    qclass: 1,
    next: answerOffset,
  });

  assert.equal(e.dns_rr_next(inPtr, len, answerOffset, outPtr), 0);
  assert.deepEqual(rr(view, outPtr), {
    nameStart: answerOffset,
    nameWireLen: 2,
    type: 1,
    klass: 1,
    ttl: 60,
    rdlen: 4,
    rdataStart: answerOffset + 12,
    next: optOffset,
    isOpt: 0,
  });

  assert.equal(e.dns_rr_next(inPtr, len, optOffset, outPtr), 0);
  assert.deepEqual(rr(view, outPtr), {
    nameStart: optOffset,
    nameWireLen: 1,
    type: 41,
    klass: 1232,
    ttl: 0,
    rdlen: 0,
    rdataStart: optOffset + 11,
    next: len,
    isOpt: 1,
  });

  assert.equal(e.dns_section_walk(inPtr, len - 1, outPtr, 48), 1);
  assert.equal(readU32(view, outPtr + 44), optOffset);

  const trailing = [...msg, 0];
  const trailingLen = writeBytes(memory, inPtr, trailing);
  assert.equal(e.dns_section_walk(inPtr, trailingLen, outPtr, 48), 3);
  assert.equal(readU32(view, outPtr + 44), len);

  const badPointer = [
    ...u16(1), ...u16(0), ...u16(0), ...u16(1), ...u16(0), ...u16(0),
    0xc0, 0x0c, ...u16(1), ...u16(1), ...u32(0), ...u16(0),
  ];
  const badLen = writeBytes(memory, inPtr, badPointer);
  assert.equal(e.dns_section_walk(inPtr, badLen, outPtr, 48), 3);
  assert.equal(readU32(view, outPtr + 44), 12);

  assert.equal(e.dns_section_walk(inPtr, 11, outPtr, 48), 1);
  assert.equal(e.dns_section_walk(inPtr, len, outPtr, 47), 2);

  console.log(JSON.stringify({
    unit: "dns-section-walk",
    abi_version: e.proto_abi_version(),
    standard_id: e.proto_standard_id(),
    ok: true,
  }, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
