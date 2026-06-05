#!/usr/bin/env node
import assert from "assert";
import fs from "fs";
import path from "path";
import { execFileSync } from "child_process";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/utf8-scan.wat");
const wasmPath = process.argv[3] || "/tmp/utf8-scan-smoke.wasm";

execFileSync("wat2wasm", [watPath, "-o", wasmPath], { stdio: "inherit" });
const mod = new WebAssembly.Module(fs.readFileSync(wasmPath));
const instance = new WebAssembly.Instance(mod, {});
const { exports } = instance;
const memory = new Uint8Array(exports.memory.buffer);
const view = new DataView(exports.memory.buffer);

function write(ptr, bytes) {
  memory.set(Buffer.from(bytes), ptr);
}

function scan(bytes) {
  write(1024, bytes);
  const status = exports.utf8_scan(1024, bytes.length, 2048);
  return {
    status,
    valid: view.getUint32(2048, true),
    errorLen: view.getUint32(2052, true),
    suffixLen: view.getUint32(2056, true),
    expectedLen: view.getUint32(2060, true),
  };
}

function unpack(packed) {
  return {
    status: Number(packed & 0xffffffffn),
    count: Number((packed >> 32n) & 0xffffffffn),
  };
}

function repair(bytes, cap = 128) {
  write(4096, bytes);
  const result = unpack(exports.utf8_lossy_repair(4096, bytes.length, 8192, cap));
  return {
    ...result,
    text: Buffer.from(memory.slice(8192, 8192 + result.count)).toString("utf8"),
  };
}

assert.strictEqual(exports.proto_abi_version(), 2);
assert.strictEqual(exports.proto_standard_id(), 300068);

assert.deepStrictEqual(scan(Buffer.from("hello © 𝄞", "utf8")), {
  status: 0,
  valid: 13,
  errorLen: 0,
  suffixLen: 0,
  expectedLen: 0,
});

assert.deepStrictEqual(scan([0x61, 0xe2, 0x82]), {
  status: 5,
  valid: 1,
  errorLen: 0,
  suffixLen: 2,
  expectedLen: 3,
});

assert.deepStrictEqual(scan([0x61, 0xe2, 0x28, 0xa1]), {
  status: 3,
  valid: 1,
  errorLen: 1,
  suffixLen: 0,
  expectedLen: 0,
});

assert.deepStrictEqual(scan([0xed, 0xa0, 0x80]), {
  status: 3,
  valid: 0,
  errorLen: 1,
  suffixLen: 0,
  expectedLen: 0,
});

assert.deepStrictEqual(scan([0xf4, 0x90, 0x80, 0x80]), {
  status: 3,
  valid: 0,
  errorLen: 1,
  suffixLen: 0,
  expectedLen: 0,
});

assert.deepStrictEqual(repair(Buffer.from("ok", "utf8")), {
  status: 0,
  count: 2,
  text: "ok",
});

assert.deepStrictEqual(repair([0x61, 0xe2, 0x28, 0xa1, 0x62]), {
  status: 0,
  count: 9,
  text: "a�(�b",
});

assert.deepStrictEqual(repair([0x61, 0xe2, 0x82]), {
  status: 0,
  count: 4,
  text: "a�",
});

assert.deepStrictEqual(repair([0x61, 0xe2, 0x82], 3), {
  status: 2,
  count: 1,
  text: "a",
});

console.log(
  JSON.stringify(
    {
      unit: "utf8-scan",
      abi_version: exports.proto_abi_version(),
      standard_id: exports.proto_standard_id(),
      ok: true,
    },
    null,
    2
  )
);
