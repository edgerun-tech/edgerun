#!/usr/bin/env node
const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { spawnSync } = require("node:child_process");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/codec-primitives/dkim-body.wat");
const wasm = path.join(os.tmpdir(), `dkim-body-${process.pid}.wasm`);

function run(cmd, args) {
  const result = spawnSync(cmd, args, { encoding: "utf8" });
  if (result.status !== 0) {
    process.stderr.write(result.stdout);
    process.stderr.write(result.stderr);
    process.exit(result.status ?? 1);
  }
}

function unpack(value) {
  const n = BigInt.asUintN(64, value);
  return {
    status: Number(n & 0xffffffffn),
    written: Number((n >> 32n) & 0xffffffffn),
  };
}

function write(memory, ptr, text) {
  const bytes = Buffer.from(text, "utf8");
  new Uint8Array(memory.buffer, ptr, Math.max(512, bytes.length + 64)).fill(0);
  new Uint8Array(memory.buffer, ptr, bytes.length).set(bytes);
  return bytes.length;
}

function read(memory, ptr, len) {
  return Buffer.from(new Uint8Array(memory.buffer, ptr, len)).toString("utf8");
}

(async () => {
  run("wat2wasm", [wat, "-o", wasm]);
  run("wasm-validate", [wasm]);

  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm));
  const e = instance.exports;
  const memory = e.memory;
  const inPtr = 1024;
  const outPtr = 4096;

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300047);

  function canonicalize(fn, input, cap = 256) {
    const len = write(memory, inPtr, input);
    const result = unpack(e[fn](inPtr, len, outPtr, cap));
    return { ...result, text: read(memory, outPtr, result.written) };
  }

  assert.deepEqual(canonicalize("dkim_body_simple", "Hello"), {
    status: 0,
    written: 7,
    text: "Hello\r\n",
  });

  assert.deepEqual(canonicalize("dkim_body_simple", "Hello\r\n\r\n\r\n"), {
    status: 0,
    written: 7,
    text: "Hello\r\n",
  });

  assert.deepEqual(canonicalize("dkim_body_relaxed", "A \t B\t\tC  \r\n  D\t E \t\r\n\r\n"), {
    status: 0,
    written: 12,
    text: "A B C\r\nD E\r\n",
  });

  assert.deepEqual(canonicalize("dkim_body_simple", "A\n\nB\n\n"), {
    status: 0,
    written: 8,
    text: "A\r\n\r\nB\r\n",
  });

  assert.deepEqual(canonicalize("dkim_body_relaxed", "A\t\tB\n \t \nC  "), {
    status: 0,
    written: 10,
    text: "A B\r\n\r\nC\r\n",
  });

  assert.deepEqual(canonicalize("dkim_body_simple", ""), {
    status: 0,
    written: 2,
    text: "\r\n",
  });

  assert.deepEqual(canonicalize("dkim_body_relaxed", " \t \r\n\r\n"), {
    status: 0,
    written: 2,
    text: "\r\n",
  });

  assert.deepEqual(canonicalize("dkim_body_simple", "Hello", 6), {
    status: 2,
    written: 6,
    text: "Hello\r",
  });

  assert.deepEqual(canonicalize("dkim_body_relaxed", "A B", 4), {
    status: 2,
    written: 4,
    text: "A B\r",
  });

  fs.rmSync(wasm, { force: true });
  console.log(JSON.stringify({ ok: true, standard_id: e.proto_standard_id() }));
})().catch((error) => {
  fs.rmSync(wasm, { force: true });
  console.error(error);
  process.exit(1);
});
