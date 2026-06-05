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

function h2Header(view, ptr) {
  return {
    length: view.getUint32(ptr, true),
    typeClass: view.getUint32(ptr + 4, true),
    flags: view.getUint32(ptr + 8, true),
    headerLen: view.getUint32(ptr + 20, true),
    totalLen: view.getUint32(ptr + 24, true),
  };
}

function stringMeta(view, ptr) {
  return {
    huffman: view.getUint32(ptr + 4, true),
    consumed: view.getUint32(ptr + 8, true),
    payloadOffset: view.getUint32(ptr + 12, true),
    payloadLen: view.getUint32(ptr + 16, true),
  };
}

function headerRecord(view, ptr, index = 0) {
  const base = ptr + index * 48;
  return {
    kind: view.getUint32(base, true),
    offset: view.getUint32(base + 4, true),
    consumed: view.getUint32(base + 8, true),
    flags: view.getUint32(base + 12, true),
    value: view.getBigUint64(base + 16, true),
    namePayloadOffset: view.getUint32(base + 24, true),
    namePayloadLen: view.getUint32(base + 28, true),
    nameHuffman: view.getUint32(base + 32, true),
    valuePayloadOffset: view.getUint32(base + 36, true),
    valuePayloadLen: view.getUint32(base + 40, true),
    valueHuffman: view.getUint32(base + 44, true),
  };
}

(async () => {
  const h2 = await load("http2-frame");
  const hpackHeader = await load("hpack-header-block");
  const hpackString = await load("hpack-string");
  const huffman = await load("hpack-huffman");

  const rawLiteral = [0x40, 0x03, ...Buffer.from("foo", "ascii"), 0x03, ...Buffer.from("bar", "ascii")];
  const huffmanLiteral = [0x10, 0x03, ...Buffer.from("baz", "ascii"), 0x81, 0x3f]; // value "o"
  const payload = [0x82, ...rawLiteral, ...huffmanLiteral];
  const frame = [0x00, 0x00, payload.length, 0x01, 0x04, 0x00, 0x00, 0x00, 0x01, ...payload];

  h2.memory.set(frame, 1024);
  assert.equal(h2.exports.http2_frame_header_decode(1024, frame.length, 16384, 2048), 0);
  const decoded = h2Header(h2.view, 2048);
  assert.deepEqual(decoded, {
    length: payload.length,
    typeClass: 1,
    flags: 4,
    headerLen: 9,
    totalLen: frame.length,
  });

  const headerBlock = h2.memory.slice(1024 + decoded.headerLen, 1024 + decoded.totalLen);
  hpackHeader.memory.set(headerBlock, 1024);
  assert.deepEqual(unpack(hpackHeader.exports.hpack_header_block_scan(1024, headerBlock.length, 2048, 144)), {
    status: 0,
    value: 3,
  });
  assert.deepEqual(headerRecord(hpackHeader.view, 2048, 0), {
    kind: 1,
    offset: 0,
    consumed: 1,
    flags: 1,
    value: 2n,
    namePayloadOffset: 0,
    namePayloadLen: 0,
    nameHuffman: 0,
    valuePayloadOffset: 0,
    valuePayloadLen: 0,
    valueHuffman: 0,
  });
  assert.deepEqual(headerRecord(hpackHeader.view, 2048, 1), {
    kind: 3,
    offset: 1,
    consumed: 9,
    flags: 1,
    value: 0n,
    namePayloadOffset: 2,
    namePayloadLen: 3,
    nameHuffman: 0,
    valuePayloadOffset: 6,
    valuePayloadLen: 3,
    valueHuffman: 0,
  });
  assert.deepEqual(headerRecord(hpackHeader.view, 2048, 2), {
    kind: 4,
    offset: 10,
    consumed: 7,
    flags: 1,
    value: 0n,
    namePayloadOffset: 2,
    namePayloadLen: 3,
    nameHuffman: 0,
    valuePayloadOffset: 6,
    valuePayloadLen: 1,
    valueHuffman: 1,
  });

  hpackString.memory.set(headerBlock, 1024);
  let got = unpack(hpackString.exports.hpack_string_decode(1024 + 2, headerBlock.length - 2, 2048, 128, 4096));
  assert.deepEqual(got, { status: 0, value: 3 });
  assert.equal(Buffer.from(hpackString.memory.slice(2048, 2048 + got.value)).toString("ascii"), "foo");
  assert.deepEqual(stringMeta(hpackString.view, 4096), { huffman: 0, consumed: 1, payloadOffset: 1, payloadLen: 3 });

  got = unpack(hpackString.exports.hpack_string_decode(1024 + 6, headerBlock.length - 6, 2048, 128, 4096));
  assert.deepEqual(got, { status: 0, value: 3 });
  assert.equal(Buffer.from(hpackString.memory.slice(2048, 2048 + got.value)).toString("ascii"), "bar");
  assert.deepEqual(stringMeta(hpackString.view, 4096), { huffman: 0, consumed: 1, payloadOffset: 1, payloadLen: 3 });

  got = unpack(hpackString.exports.hpack_string_decode(1024 + 15, headerBlock.length - 15, 2048, 128, 4096));
  assert.deepEqual(got, { status: 0, value: 1 });
  assert.equal(Buffer.from(hpackString.memory.slice(2048, 2049)).toString("ascii"), "o");
  assert.deepEqual(stringMeta(hpackString.view, 4096), { huffman: 1, consumed: 1, payloadOffset: 1, payloadLen: 1 });

  huffman.memory.set([0x3f], 1024);
  got = unpack(huffman.exports.hpack_huffman_decode(1024, 1, 2048, 16));
  assert.deepEqual(got, { status: 0, value: 1 });
  assert.equal(Buffer.from(huffman.memory.slice(2048, 2049)).toString("ascii"), "o");

  console.log(JSON.stringify({
    unit: "codec-composition-http2-hpack",
    ok: true,
    pipeline: "http2-frame -> hpack-header-block -> hpack-string -> hpack-huffman",
  }, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
