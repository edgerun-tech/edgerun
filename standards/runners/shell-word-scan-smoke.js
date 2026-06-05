#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/shell-word-scan.wat";

function compileWat(file) {
  const wasmPath = path.join(os.tmpdir(), `edgerun-${path.basename(file)}-${process.pid}.wasm`);
  execFileSync("wat2wasm", [file, "-o", wasmPath], { stdio: "inherit" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "inherit" });
  const bytes = fs.readFileSync(wasmPath);
  fs.unlinkSync(wasmPath);
  return bytes;
}

function unpack(packed) {
  return {
    status: Number(packed & 0xffffffffn),
    written: Number(packed >> 32n),
  };
}

function write(memory, ptr, text) {
  const bytes = Buffer.from(text, "utf8");
  memory.fill(0, ptr, ptr + Math.max(256, bytes.length + 64));
  memory.set(bytes, ptr);
  return bytes.length;
}

function record(view, ptr) {
  return {
    next: view.getUint32(ptr, true),
    sourceStart: view.getUint32(ptr + 4, true),
    sourceLen: view.getUint32(ptr + 8, true),
    flags: view.getUint32(ptr + 12, true),
  };
}

function decoded(memory, ptr, len) {
  return Buffer.from(memory.slice(ptr, ptr + len)).toString("utf8");
}

(async () => {
  const { instance } = await WebAssembly.instantiate(compileWat(watPath), {});
  const exports = instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);
  const view = new DataView(exports.memory.buffer);
  const inPtr = 1024;
  const outPtr = 8192;
  const recPtr = 12288;
  const cases = [];

  assert.equal(exports.proto_abi_version(), 2);
  assert.equal(exports.proto_standard_id(), 300070);

  function split(text, cap = 256) {
    const len = write(memory, inPtr, text);
    const words = [];
    let offset = 0;
    for (;;) {
      memory.fill(0, outPtr, outPtr + 256);
      const step = unpack(exports.shell_word_next(inPtr, len, offset, outPtr, cap, recPtr));
      const rec = record(view, recPtr);
      if (step.status === 5) {
        return { status: 5, words, rec };
      }
      if (step.status !== 0) {
        return { status: step.status, words, written: step.written, rec };
      }
      words.push(decoded(memory, outPtr, step.written));
      offset = rec.next;
    }
  }

  function ok(name, input, expected) {
    const got = split(input);
    assert.deepEqual(got.words, expected, name);
    cases.push(name);
  }

  ok("simple words", "git grep TODO src", ["git", "grep", "TODO", "src"]);
  ok("quoted spaces", "git grep 'foo bar' src", ["git", "grep", "foo bar", "src"]);
  ok("escaped spaces", String.raw`two\ words plain`, ["two words", "plain"]);
  ok("double quotes", String.raw`cat "pkg\src\main.rs"`, ["cat", String.raw`pkg\src\main.rs`]);
  ok("single quotes", String.raw`bash -lc 'echo "hello world"'`, [
    "bash",
    "-lc",
    'echo "hello world"',
  ]);
  ok("mixed quotes escapes", String.raw`a"b\"c"'d'\ e`, ['ab"cd e']);
  ok("empty quoted word", `'' ""`, ["", ""]);
  ok("hash is literal", "echo # not-comment", ["echo", "#", "not-comment"]);

  let bad = split("'unterminated");
  assert.equal(bad.status, 3, "unterminated single quote");
  cases.push("unterminated single quote");

  bad = split('"unterminated');
  assert.equal(bad.status, 3, "unterminated double quote");
  cases.push("unterminated double quote");

  const capped = split("abcd", 3);
  assert.equal(capped.status, 2, "output cap");
  cases.push("output cap");

  console.log(
    JSON.stringify({
      unit: "shell-word-scan",
      abi_version: exports.proto_abi_version(),
      standard_id: exports.proto_standard_id(),
      ok: true,
      cases,
    }),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
