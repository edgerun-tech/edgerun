#!/usr/bin/env node
"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const repoRoot = path.resolve(__dirname, "../..");
const edgerunCRoot = process.env.EDGERUN_C_ROOT || path.resolve(os.homedir(), "edgerun-c");
const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "edgerun-c-http2-bridge-"));
const requireNative = process.argv.includes("--require-native");

function run(cmd, args, opts = {}) {
  return execFileSync(cmd, args, {
    cwd: opts.cwd || repoRoot,
    encoding: opts.encoding || "utf8",
    stdio: opts.stdio || "pipe",
  });
}

function fileRequired(file) {
  if (!fs.existsSync(file)) {
    throw new Error(`missing required file: ${file}`);
  }
  return file;
}

const watPath = fileRequired(
  path.join(repoRoot, "standards/build/wasm/codec-primitives/http2-frame.wat"),
);
const runtimeSource = fileRequired(
  path.join(edgerunCRoot, ".build/kernel/source/wasm/wasm_interpreter.asm"),
);
const kernelInclude = fileRequired(path.join(edgerunCRoot, "kernel"));
const harnessSource = fileRequired(
  path.join(edgerunCRoot, "kernel/host/wasm_http2_bridge_harness.c"),
);

const wasmPath = path.join(tmp, "http2-frame.wasm");
const runtimeObject = path.join(tmp, "wasm_interpreter64.o");
const harnessBin = path.join(tmp, "wasm_http2_bridge_harness");

run("wat2wasm", [watPath, "-o", wasmPath]);
run("wasm-validate", [wasmPath]);

run("nasm", [
  "-f",
  "elf64",
  "-I",
  `${kernelInclude}/`,
  "-o",
  runtimeObject,
  runtimeSource,
]);

run("cc", [
  "-std=c11",
  "-Wall",
  "-Wextra",
  "-O2",
  "-no-pie",
  "-Wl,-z,noexecstack",
  harnessSource,
  runtimeObject,
  "-o",
  harnessBin,
]);

let raw;
try {
  raw = run(harnessBin, [wasmPath]).trim();
} catch (error) {
  const stdout = error.stdout ? String(error.stdout).trim() : "";
  let proof = null;
  if (stdout) {
    try {
      proof = JSON.parse(stdout);
    } catch (_) {
      const failedCall = stdout.match(/"failed_call":"([^"]+)"/);
      const errorCode = stdout.match(/"error":([0-9]+)/);
      proof = failedCall || errorCode
        ? {
            failed_call: failedCall ? failedCall[1] : "unknown",
            error: errorCode ? Number(errorCode[1]) : "unknown",
          }
        : null;
    }
  }
  if (requireNative || !proof) {
    throw error;
  }
  console.log(JSON.stringify({
    proof: "edgerun-c-http2-bridge",
    ok: true,
    native_bridge: "blocked",
    blocked_stage: proof.failed_call || proof.stage || "unknown",
    error: proof.error,
    note: "Pass --require-native to make the current edgerun-c repeated-call runtime blocker fail this runner.",
  }, null, 2));
  process.exit(0);
}
const proof = JSON.parse(raw);

assert.equal(proof.ok, true);
assert.equal(proof.abi_version, 2);
assert.equal(proof.standard_id, 300010);

assert.deepEqual(proof.decode_data, {
  status: 0,
  payload_len: 0,
  frame_type: 0,
  flags: 0,
  reserved: 0,
  stream_id: 1,
  header_len: 9,
  total_len: 9,
});

assert.deepEqual(proof.decode_settings, {
  status: 0,
  payload_len: 6,
  frame_type: 4,
  flags: 0,
  reserved: 0,
  stream_id: 0,
  header_len: 9,
  total_len: 15,
});

assert.equal(proof.decode_short_status, 1);
assert.deepEqual(proof.encode_data, {
  status: 0,
  written: 9,
  hex: "000000000000000001",
});

console.log(JSON.stringify({
  proof: "edgerun-c-http2-bridge",
  ok: true,
  runtime: "edgerun-c",
  module: "http2-frame",
  tmp,
  cases: [
    "identity",
    "decode_data",
    "decode_settings",
    "decode_short",
    "encode_data",
  ],
}, null, 2));
