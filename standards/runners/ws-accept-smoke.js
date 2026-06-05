#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const root = path.resolve(__dirname, "..", "..");
const watPath = process.argv[2] || path.join(root, "standards/build/wasm/codec-primitives/ws-accept.wat");
const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "ws-accept-"));
const wasmPath = path.join(tmpDir, "ws-accept.wasm");

process.on("exit", () => {
  fs.rmSync(tmpDir, { recursive: true, force: true });
});

execFileSync("wat2wasm", [watPath, "-o", wasmPath], { stdio: "inherit" });
execFileSync("wasm-validate", [wasmPath], { stdio: "inherit" });

function unpack(result) {
  const value = BigInt.asUintN(64, result);
  return {
    status: Number(value & 0xffff_ffffn),
    written: Number((value >> 32n) & 0xffff_ffffn),
  };
}

function putAscii(mem, ptr, text) {
  mem.fill(0, ptr, ptr + text.length + 64);
  mem.set(Buffer.from(text, "ascii"), ptr);
}

(async () => {
  const wasm = fs.readFileSync(wasmPath);
  const { instance } = await WebAssembly.instantiate(wasm);
  const e = instance.exports;
  const mem = new Uint8Array(e.memory.buffer);

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300048);

  const inPtr = 1024;
  const outPtr = 2048;

  putAscii(mem, inPtr, "dGhlIHNhbXBsZSBub25jZQ==");
  let packed = unpack(e.ws_accept_key(inPtr, 24, outPtr, 28));
  assert.deepEqual(packed, { status: 0, written: 28 }, "RFC example status");
  assert.equal(Buffer.from(mem.slice(outPtr, outPtr + 28)).toString("ascii"), "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");

  putAscii(mem, inPtr, "dGhlIHNhbXBsZSBub25jZ!==");
  packed = unpack(e.ws_accept_key(inPtr, 24, outPtr, 28));
  assert.deepEqual(packed, { status: 3, written: 0 }, "invalid base64 status");

  putAscii(mem, inPtr, "dGhlIHNhbXBsZSBub25jZQ==AA==");
  packed = unpack(e.ws_accept_key(inPtr, 28, outPtr, 28));
  assert.deepEqual(packed, { status: 4, written: 0 }, "decoded length status");

  putAscii(mem, inPtr, "dGhlIHNhbXBsZSBub25jZQ==");
  packed = unpack(e.ws_accept_key(inPtr, 24, outPtr, 27));
  assert.deepEqual(packed, { status: 2, written: 0 }, "short output status");

  console.log(
    JSON.stringify(
      {
        unit: "ws-accept",
        abi_version: e.proto_abi_version(),
        standard_id: e.proto_standard_id(),
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
