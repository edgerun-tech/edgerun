#!/usr/bin/env node

const assert = require("node:assert/strict");
const crypto = require("node:crypto");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const root = path.resolve(__dirname, "..", "..");
const watPath = process.argv[2] || path.join(root, "standards/build/wasm/codec-primitives/crypto-sha512.wat");
const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "crypto-sha512-"));
const wasmPath = path.join(tmpDir, "crypto-sha512.wasm");

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

function write(memory, ptr, bytes) {
  memory.fill(0, ptr, ptr + bytes.length + 128);
  memory.set(bytes, ptr);
}

function digest(exports, memory, name, text) {
  const inPtr = 1024;
  const outPtr = 4096;
  const bytes = Buffer.from(text, "ascii");
  const outLen = name === "sha512" ? 64 : 48;
  write(memory, inPtr, bytes);
  memory.fill(0, outPtr, outPtr + 64);
  const packed = unpack(exports[name](inPtr, bytes.length, outPtr));
  return {
    packed,
    hex: Buffer.from(memory.slice(outPtr, outPtr + outLen)).toString("hex"),
  };
}

(async () => {
  const wasm = fs.readFileSync(wasmPath);
  const { instance } = await WebAssembly.instantiate(wasm);
  const e = instance.exports;
  const memory = new Uint8Array(e.memory.buffer);

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300074);

  for (const text of ["", "abc"]) {
    for (const name of ["sha512", "sha384"]) {
      const actual = digest(e, memory, name, text);
      const expected = crypto.createHash(name).update(Buffer.from(text, "ascii")).digest("hex");
      assert.deepEqual(actual.packed, { status: 0, written: name === "sha512" ? 64 : 48 }, `${name} ${JSON.stringify(text)} status`);
      assert.equal(actual.hex, expected, `${name} ${JSON.stringify(text)} digest`);
    }
  }

  console.log(
    JSON.stringify(
      {
        unit: "crypto-sha512",
        abi_version: e.proto_abi_version(),
        standard_id: e.proto_standard_id(),
        ok: true,
        cases: ["sha512_empty", "sha512_abc", "sha384_empty", "sha384_abc"],
      },
      null,
      2,
    ),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
