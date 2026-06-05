#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { spawnSync } = require("child_process");

const root = path.resolve(__dirname, "../..");
const watRoot = path.join(root, "standards/build/wasm/codec-primitives");
const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "codec-composition-http3-qpack-huffman-"));
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
    view: new DataView(instance.exports.memory.buffer),
  };
}

function unpack(value) {
  return {
    status: Number(value & 0xffffffffn),
    value: Number((value >> 32n) & 0xffffffffn),
  };
}

function readU32(memory, offset) {
  return (
    memory[offset] |
    (memory[offset + 1] << 8) |
    (memory[offset + 2] << 16) |
    (memory[offset + 3] << 24)
  ) >>> 0;
}

function http3Record(memory, offset) {
  return {
    frameType: BigInt(readU32(memory, offset)) | (BigInt(readU32(memory, offset + 4)) << 32n),
    payloadLen: BigInt(readU32(memory, offset + 8)) | (BigInt(readU32(memory, offset + 12)) << 32n),
    headerLen: readU32(memory, offset + 16),
    classification: readU32(memory, offset + 20),
  };
}

function stringMeta(view, offset) {
  return {
    huffman: view.getUint32(offset + 4, true),
    consumed: view.getUint32(offset + 8, true),
    payloadOffset: view.getUint32(offset + 12, true),
    payloadLen: view.getUint32(offset + 16, true),
  };
}

(async () => {
  const http3 = await instantiate("http3-frame");
  const qpackString = await instantiate("qpack-string");
  const huffman = await instantiate("hpack-huffman");

  const payload = [0x81, 0x3f];
  const frame = [0x01, payload.length, ...payload];

  http3.memory.set(frame, 1024);
  assert.strictEqual(http3.exports.http3_frame_header_decode(1024, frame.length, 2048), 0);
  const decodedFrame = http3Record(http3.memory, 2048);
  assert.deepStrictEqual(decodedFrame, {
    frameType: 1n,
    payloadLen: BigInt(payload.length),
    headerLen: 2,
    classification: 1,
  });

  qpackString.memory.set(frame.slice(decodedFrame.headerLen), 1024);
  assert.strictEqual(qpackString.exports.qpack_string_scan(8, 1024, Number(decodedFrame.payloadLen), 2048), 0);
  const meta = stringMeta(qpackString.view, 2048);
  assert.deepStrictEqual(meta, { huffman: 1, consumed: 1, payloadOffset: 1, payloadLen: 1 });

  const decodedString = unpack(qpackString.exports.qpack_string_decode(8, 1024, Number(decodedFrame.payloadLen), 3072, 128, 2048));
  assert.deepStrictEqual(decodedString, { status: 0, value: 1 });
  assert.strictEqual(qpackString.memory[3072], 0x6f);

  huffman.memory.set(Array.from(qpackString.memory.slice(1024 + meta.payloadOffset, 1024 + meta.payloadOffset + meta.payloadLen)), 1024);
  const decodedHuffman = unpack(huffman.exports.hpack_huffman_decode(1024, meta.payloadLen, 2048, 128));
  assert.deepStrictEqual(decodedHuffman, { status: 0, value: 1 });
  assert.strictEqual(huffman.memory[2048], 0x6f);

  console.log(JSON.stringify({
    runner: "codec-composition-http3-qpack-huffman",
    pipeline: "http3-frame -> qpack-string -> hpack-huffman",
    ok: true,
    value: "o",
  }, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
