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

function unpack(packed) {
  return {
    status: Number(packed & 0xffffffffn),
    value: Number((packed >> 32n) & 0xffffffffn),
  };
}

function h3Record(view, ptr) {
  const typeLow = view.getUint32(ptr, true);
  const typeHigh = view.getUint32(ptr + 4, true);
  const lenLow = view.getUint32(ptr + 8, true);
  const lenHigh = view.getUint32(ptr + 12, true);
  return {
    frameType: (BigInt(typeHigh) << 32n) | BigInt(typeLow),
    payloadLen: (BigInt(lenHigh) << 32n) | BigInt(lenLow),
    headerLen: view.getUint32(ptr + 16, true),
    classification: view.getUint32(ptr + 20, true),
  };
}

function stringMeta(view, ptr) {
  return {
    consumed: view.getUint32(ptr + 8, true),
    payloadOffset: view.getUint32(ptr + 12, true),
    payloadLen: view.getUint32(ptr + 16, true),
  };
}

(async () => {
  const h3 = await load("http3-frame");
  const qpackString = await load("qpack-string");
  const huffman = await load("hpack-huffman");

  const rawString = [0b0100_0011, ...Buffer.from("bar", "ascii")];
  const huffmanString = [0x81, 0x3f]; // QPACK size=8 prefix-string, Huffman "o".
  const payload = [...rawString, ...huffmanString];
  const frame = [0x01, payload.length, ...payload];

  h3.memory.set(frame, 1024);
  assert.equal(h3.exports.http3_frame_header_decode(1024, frame.length, 2048), 0);
  const decoded = h3Record(h3.view, 2048);
  assert.deepEqual(decoded, {
    frameType: 1n,
    payloadLen: BigInt(payload.length),
    headerLen: 2,
    classification: 1,
  });

  qpackString.memory.set(h3.memory.slice(1024 + decoded.headerLen, 1024 + decoded.headerLen + Number(decoded.payloadLen)), 1024);
  let got = unpack(qpackString.exports.qpack_string_decode(6, 1024, payload.length, 2048, 128, 4096));
  assert.deepEqual(got, { status: 0, value: 3 });
  assert.equal(Buffer.from(qpackString.memory.slice(2048, 2048 + got.value)).toString("ascii"), "bar");
  const firstMeta = stringMeta(qpackString.view, 4096);

  const secondOffset = firstMeta.consumed + firstMeta.payloadLen;
  got = unpack(qpackString.exports.qpack_string_decode(8, 1024 + secondOffset, payload.length - secondOffset, 2048, 128, 4096));
  assert.deepEqual(got, { status: 0, value: 1 });
  assert.equal(Buffer.from(qpackString.memory.slice(2048, 2049)).toString("ascii"), "o");

  huffman.memory.set([0x3f], 1024);
  got = unpack(huffman.exports.hpack_huffman_decode(1024, 1, 2048, 16));
  assert.deepEqual(got, { status: 0, value: 1 });
  assert.equal(Buffer.from(huffman.memory.slice(2048, 2049)).toString("ascii"), "o");

  console.log(JSON.stringify({
    unit: "codec-composition-http3-qpack",
    ok: true,
    pipeline: "http3-frame -> qpack-string -> hpack-huffman",
    gap: "string prefix composition only; no QPACK header-block instruction or table-state decode",
  }, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
