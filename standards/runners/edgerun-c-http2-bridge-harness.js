#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync, spawnSync } = require("node:child_process");

const root = path.resolve(__dirname, "..", "..");
const watPath = path.join(root, "standards/build/wasm/codec-primitives/http2-frame.wat");
const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "edgerun-c-http2-bridge-"));
const defaultBridge = "/home/ken/edgerun-c/.build/host/edgerun-c-wasm-call";
const bridgePath = process.env.EDGERUN_C_WASM_CALL || defaultBridge;
const requireNative = process.argv.includes("--require-native");
const keepTmp = process.argv.includes("--keep");

process.on("exit", () => {
  if (!keepTmp) {
    fs.rmSync(tmpRoot, { recursive: true, force: true });
  }
});

function compileWat() {
  const wasmPath = path.join(tmpRoot, "http2-frame.wasm");
  execFileSync("wat2wasm", [watPath, "-o", wasmPath], { stdio: "pipe" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "pipe" });
  return wasmPath;
}

async function instantiate(wasmPath) {
  const bytes = fs.readFileSync(wasmPath);
  const { instance } = await WebAssembly.instantiate(bytes, {});
  return {
    e: instance.exports,
    mem: new Uint8Array(instance.exports.memory.buffer),
    view: new DataView(instance.exports.memory.buffer),
  };
}

function writeBytes(mem, ptr, bytes) {
  mem.fill(0, ptr, ptr + Math.max(64, bytes.length + 16));
  mem.set(bytes, ptr);
}

function readHeader(view, ptr) {
  return {
    payload_len: view.getUint32(ptr, true),
    type_class: view.getUint32(ptr + 4, true),
    flags: view.getUint32(ptr + 8, true),
    reserved: view.getUint32(ptr + 12, true),
    stream_id: view.getUint32(ptr + 16, true),
    header_len: view.getUint32(ptr + 20, true),
    total_len: view.getUint32(ptr + 24, true),
  };
}

function decodeVector(wat, name, bytes, maxFrameSize) {
  const inPtr = 1024;
  const outPtr = 2048;
  writeBytes(wat.mem, inPtr, bytes);
  const status = wat.e.http2_frame_header_decode(inPtr, bytes.length, maxFrameSize, outPtr);
  return {
    op: "decode",
    name,
    input_hex: Buffer.from(bytes).toString("hex"),
    max_frame_size: maxFrameSize,
    status,
    record: status === 0 ? readHeader(wat.view, outPtr) : null,
  };
}

function encodeVector(wat, name, payloadLen, type, flags, streamId) {
  const outPtr = 3072;
  const packed = BigInt.asUintN(
    64,
    wat.e.http2_frame_header_encode(payloadLen, type, flags, streamId, outPtr, 16),
  );
  const status = Number(packed & 0xffff_ffffn);
  const written = Number((packed >> 32n) & 0xffff_ffffn);
  return {
    op: "encode",
    name,
    payload_len: payloadLen,
    type,
    flags,
    stream_id: streamId,
    status,
    written,
    output_hex: Buffer.from(wat.mem.slice(outPtr, outPtr + written)).toString("hex"),
  };
}

function runNativeBridge(wasmPath, vectorsPath) {
  if (!fs.existsSync(bridgePath)) {
    if (requireNative) {
      throw new Error(`missing edgerun-c bridge executable: ${bridgePath}`);
    }
    return {
      available: false,
      path: bridgePath,
      reason: "missing executable; set EDGERUN_C_WASM_CALL or build the userspace edgerun-c bridge",
    };
  }
  const result = spawnSync(bridgePath, [wasmPath, vectorsPath], {
    cwd: root,
    encoding: "utf8",
    maxBuffer: 1024 * 1024,
  });
  return {
    available: true,
    path: bridgePath,
    status: result.status,
    stdout: result.stdout,
    stderr: result.stderr,
  };
}

(async () => {
  const wasmPath = compileWat();
  const wat = await instantiate(wasmPath);

  assert.equal(wat.e.proto_abi_version(), 2);
  assert.equal(wat.e.proto_standard_id(), 300010);

  const vectors = {
    contract: "edgerun-c-wasm-call/v1",
    module: "http2-frame",
    abi_version: wat.e.proto_abi_version(),
    standard_id: wat.e.proto_standard_id(),
    expected_cli: `${bridgePath} ${wasmPath} vectors.json`,
    calls: [
      decodeVector(wat, "data-empty-stream1", [0, 0, 0, 0, 0, 0, 0, 0, 1], 16_384),
      decodeVector(wat, "headers-len5-reserved", [0, 0, 5, 1, 4, 0x80, 0, 0, 3, 1, 2, 3, 4, 5], 16_384),
      decodeVector(wat, "too-short", [0, 0, 0, 0, 0, 0, 0, 0], 16_384),
      decodeVector(wat, "frame-too-large", [0, 0, 17, 0, 0, 0, 0, 0, 1], 16),
      encodeVector(wat, "settings-empty", 0, 4, 0, 0),
      encodeVector(wat, "headers-stream3", 5, 1, 4, 3),
    ],
  };

  const vectorsPath = path.join(tmpRoot, "http2-frame-vectors.json");
  fs.writeFileSync(vectorsPath, `${JSON.stringify(vectors, null, 2)}\n`);
  const native = runNativeBridge(wasmPath, vectorsPath);

  if (native.available && native.status !== 0) {
    throw new Error(`edgerun-c bridge exited ${native.status}: ${native.stderr || native.stdout}`);
  }

  console.log(
    JSON.stringify(
      {
        unit: "edgerun-c-http2-bridge-harness",
        wasm_path: wasmPath,
        vectors_path: vectorsPath,
        temp_files_kept: keepTmp,
        vector_count: vectors.calls.length,
        native_bridge: native,
        ok: native.available ? native.status === 0 : !requireNative,
      },
      null,
      2,
    ),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
