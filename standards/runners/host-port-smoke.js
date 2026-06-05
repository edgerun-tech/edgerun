#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/host-port.wat";

function compileWat(file) {
  const wasmPath = path.join(os.tmpdir(), `edgerun-${path.basename(file)}-${process.pid}.wasm`);
  execFileSync("wat2wasm", [file, "-o", wasmPath], { stdio: "pipe" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "pipe" });
  const bytes = fs.readFileSync(wasmPath);
  fs.unlinkSync(wasmPath);
  return bytes;
}

function write(memory, ptr, text) {
  const bytes = Buffer.from(text, "ascii");
  memory.fill(0, ptr, ptr + bytes.length + 64);
  memory.set(bytes, ptr);
  return bytes.length;
}

function record(view, outPtr) {
  return {
    hostOff: view.getUint32(outPtr, true),
    hostLen: view.getUint32(outPtr + 4, true),
    port: view.getUint32(outPtr + 8, true),
    hasExplicitPort: view.getUint32(outPtr + 12, true),
  };
}

(async () => {
  const module = await WebAssembly.instantiate(compileWat(watPath), {});
  const exports = module.instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);
  const view = new DataView(exports.memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300060);

  const inPtr = 1024;
  const outPtr = 4096;

  let len = write(memory, inPtr, "localhost:8080");
  assert.strictEqual(exports.host_port_scan(inPtr, len, 443, outPtr), 0);
  assert.deepStrictEqual(record(view, outPtr), {
    hostOff: 0,
    hostLen: 9,
    port: 8080,
    hasExplicitPort: 1,
  });

  len = write(memory, inPtr, "192.168.1.1:443");
  assert.strictEqual(exports.host_port_scan(inPtr, len, 80, outPtr), 0);
  assert.deepStrictEqual(record(view, outPtr), {
    hostOff: 0,
    hostLen: 11,
    port: 443,
    hasExplicitPort: 1,
  });

  len = write(memory, inPtr, "example.com");
  assert.strictEqual(exports.host_port_scan(inPtr, len, 443, outPtr), 0);
  assert.deepStrictEqual(record(view, outPtr), {
    hostOff: 0,
    hostLen: 11,
    port: 443,
    hasExplicitPort: 0,
  });

  len = write(memory, inPtr, "example.com");
  assert.strictEqual(exports.host_port_scan(inPtr, len, 0, outPtr), 0);
  assert.deepStrictEqual(record(view, outPtr), {
    hostOff: 0,
    hostLen: 11,
    port: 0,
    hasExplicitPort: 0,
  });

  len = write(memory, inPtr, "");
  assert.strictEqual(exports.host_port_scan(inPtr, len, 443, outPtr), 3);

  len = write(memory, inPtr, "localhost:");
  assert.strictEqual(exports.host_port_scan(inPtr, len, 443, outPtr), 3);

  len = write(memory, inPtr, "localhost:http");
  assert.strictEqual(exports.host_port_scan(inPtr, len, 443, outPtr), 3);

  len = write(memory, inPtr, "localhost:65536");
  assert.strictEqual(exports.host_port_scan(inPtr, len, 443, outPtr), 3);

  len = write(memory, inPtr, "a:b:123");
  assert.strictEqual(exports.host_port_scan(inPtr, len, 443, outPtr), 0);
  assert.deepStrictEqual(record(view, outPtr), {
    hostOff: 0,
    hostLen: 3,
    port: 123,
    hasExplicitPort: 1,
  });

  console.log(
    JSON.stringify(
      {
        unit: "host-port",
        abi_version: exports.proto_abi_version(),
        standard_id: exports.proto_standard_id(),
        ok: true,
      },
      null,
      2,
    ),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
