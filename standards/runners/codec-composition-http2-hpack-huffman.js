#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { spawnSync } = require("child_process");

const root = path.resolve(__dirname, "../..");
const watRoot = path.join(root, "standards/build/wasm/codec-primitives");
const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "codec-composition-http2-hpack-huffman-"));
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

function header(view, offset) {
  return {
    length: view.getUint32(offset, true),
    typeClass: view.getUint32(offset + 4, true),
    flags: view.getUint32(offset + 8, true),
    streamId: view.getUint32(offset + 16, true),
    headerLen: view.getUint32(offset + 20, true),
    totalLen: view.getUint32(offset + 24, true),
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
  const http2 = await instantiate("http2-frame");
  const hpackString = await instantiate("hpack-string");
  const huffman = await instantiate("hpack-huffman");

  const payload = [0x81, 0x3f];
  const frame = [0x00, 0x00, payload.length, 0x01, 0x04, 0x00, 0x00, 0x00, 0x01, ...payload];

  http2.memory.set(frame, 1024);
  assert.strictEqual(http2.exports.http2_frame_header_decode(1024, frame.length, 16384, 2048), 0);
  const decodedFrame = header(http2.view, 2048);
  assert.deepStrictEqual(decodedFrame, {
    length: payload.length,
    typeClass: 1,
    flags: 4,
    streamId: 1,
    headerLen: 9,
    totalLen: frame.length,
  });

  hpackString.memory.set(frame.slice(decodedFrame.headerLen, decodedFrame.totalLen), 1024);
  assert.strictEqual(hpackString.exports.hpack_string_scan(1024, decodedFrame.length, 2048), 0);
  const meta = stringMeta(hpackString.view, 2048);
  assert.deepStrictEqual(meta, { huffman: 1, consumed: 1, payloadOffset: 1, payloadLen: 1 });

  const decodedString = unpack(hpackString.exports.hpack_string_decode(1024, decodedFrame.length, 3072, 128, 2048));
  assert.deepStrictEqual(decodedString, { status: 0, value: 1 });
  assert.strictEqual(hpackString.memory[3072], 0x6f);

  huffman.memory.set(Array.from(hpackString.memory.slice(1024 + meta.payloadOffset, 1024 + meta.payloadOffset + meta.payloadLen)), 1024);
  const decodedHuffman = unpack(huffman.exports.hpack_huffman_decode(1024, meta.payloadLen, 2048, 128));
  assert.deepStrictEqual(decodedHuffman, { status: 0, value: 1 });
  assert.strictEqual(huffman.memory[2048], 0x6f);

  console.log(JSON.stringify({
    runner: "codec-composition-http2-hpack-huffman",
    pipeline: "http2-frame -> hpack-string -> hpack-huffman",
    ok: true,
    value: "o",
  }, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
