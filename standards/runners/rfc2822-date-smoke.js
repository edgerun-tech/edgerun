#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const root = path.resolve(__dirname, "..", "..");
const watPath = process.argv[2] || path.join(root, "standards/build/wasm/codec-primitives/rfc2822-date.wat");
const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "rfc2822-date-"));
const wasmPath = path.join(tmpDir, "rfc2822-date.wasm");

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

function splitU64(value) {
  const n = BigInt(value);
  return {
    low: Number(n & 0xffff_ffffn),
    high: Number((n >> 32n) & 0xffff_ffffn),
  };
}

function read(mem, ptr, len) {
  return Buffer.from(mem.slice(ptr, ptr + len)).toString("ascii");
}

function format(e, mem, unix, cap = 31) {
  const out = 4096;
  mem.fill(0, out, out + 64);
  const parts = splitU64(unix);
  const packed = unpack(e.rfc2822_format_utc(parts.low, parts.high, out, cap));
  return {
    ...packed,
    output: read(mem, out, packed.written),
  };
}

(async () => {
  const wasm = fs.readFileSync(wasmPath);
  const { instance } = await WebAssembly.instantiate(wasm);
  const e = instance.exports;
  const mem = new Uint8Array(e.memory.buffer);

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300054);

  assert.deepEqual(format(e, mem, 0n), {
    status: 0,
    written: 31,
    output: "Thu, 01 Jan 1970 00:00:00 +0000",
  });

  assert.deepEqual(format(e, mem, 1582977600n), {
    status: 0,
    written: 31,
    output: "Sat, 29 Feb 2020 12:00:00 +0000",
  });

  assert.deepEqual(format(e, mem, 1767225599n), {
    status: 0,
    written: 31,
    output: "Wed, 31 Dec 2025 23:59:59 +0000",
  });

  assert.deepEqual(format(e, mem, 1893456000n), {
    status: 0,
    written: 31,
    output: "Tue, 01 Jan 2030 00:00:00 +0000",
  });

  assert.deepEqual(format(e, mem, 1893456000n, 30), {
    status: 2,
    written: 0,
    output: "",
  });

  console.log(JSON.stringify({
    unit: "rfc2822-date",
    abi_version: e.proto_abi_version(),
    standard_id: e.proto_standard_id(),
    ok: true,
  }, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
