#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/rust-syn-core.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  const wasm = execFileSync("wat2wasm", [file, "-o", "-"]);
  execFileSync("wasm-validate", ["-"], { input: wasm });
  return wasm;
}

function put(memory, value, ptr = 1024) {
  const bytes = Buffer.from(value, "ascii");
  memory.fill(0, ptr, ptr + Math.max(64, bytes.length + 1));
  memory.set(bytes, ptr);
  return [ptr, bytes.length];
}

function code(e, memory, fn, value) {
  return e[fn](...put(memory, value));
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const e = instance.exports;
  const memory = new Uint8Array(e.memory.buffer);

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300133);
  assert.equal(code(e, memory, "rust_syn_keyword_code", "pub"), 1);
  assert.equal(code(e, memory, "rust_syn_keyword_code", "fn"), 2);
  assert.equal(code(e, memory, "rust_syn_keyword_code", "struct"), 3);
  assert.equal(code(e, memory, "rust_syn_keyword_code", "unsafe"), 18);
  assert.equal(code(e, memory, "rust_syn_keyword_code", "self"), 22);
  assert.equal(e.rust_syn_keyword_family(1), 1);
  assert.equal(e.rust_syn_keyword_family(3), 2);
  assert.equal(e.rust_syn_keyword_family(6), 3);
  assert.equal(e.rust_syn_keyword_family(14), 4);
  assert.equal(e.rust_syn_keyword_family(28), 5);
  assert.equal(e.rust_syn_punct_width(...put(memory, ">>=")), 3);
  assert.equal(code(e, memory, "rust_syn_binop_precedence", "||"), 4);
  assert.equal(code(e, memory, "rust_syn_binop_precedence", "&&"), 5);
  assert.equal(code(e, memory, "rust_syn_binop_precedence", "=="), 6);
  assert.equal(code(e, memory, "rust_syn_binop_precedence", "|"), 7);
  assert.equal(code(e, memory, "rust_syn_binop_precedence", "^"), 8);
  assert.equal(code(e, memory, "rust_syn_binop_precedence", "&"), 9);
  assert.equal(code(e, memory, "rust_syn_binop_precedence", "<<"), 10);
  assert.equal(code(e, memory, "rust_syn_binop_precedence", "+"), 11);
  assert.equal(code(e, memory, "rust_syn_binop_precedence", "*"), 12);
  assert.equal(code(e, memory, "rust_syn_binop_precedence", "="), 2);
  assert.equal(e.rust_syn_delimiter_code("(".charCodeAt(0)), 1);
  assert.equal(e.rust_syn_delimiter_code("{".charCodeAt(0)), 2);
  assert.equal(e.rust_syn_delimiter_code("[".charCodeAt(0)), 3);
  assert.equal(code(e, memory, "rust_syn_literal_kind", "\"x\""), 1);
  assert.equal(code(e, memory, "rust_syn_literal_kind", "b\"x\""), 2);
  assert.equal(code(e, memory, "rust_syn_literal_kind", "c\"x\""), 3);
  assert.equal(code(e, memory, "rust_syn_literal_kind", "b'x'"), 4);
  assert.equal(code(e, memory, "rust_syn_literal_kind", "'x'"), 5);
  assert.equal(code(e, memory, "rust_syn_literal_kind", "123u64"), 6);
  assert.equal(code(e, memory, "rust_syn_literal_kind", "1.0f32"), 7);
  assert.equal(code(e, memory, "rust_syn_literal_kind", "true"), 8);
  assert.equal(e.rust_syn_type_family(1), 1);
  assert.equal(e.rust_syn_type_family(2), 2);
  assert.equal(e.rust_syn_type_family(14), 3);
  assert.equal(e.rust_syn_type_family(9), 4);
  assert.equal(e.rust_syn_item_family(1), 1);
  assert.equal(e.rust_syn_item_family(10), 2);
  assert.equal(e.rust_syn_item_family(4), 3);
  assert.equal(e.rust_syn_item_family(6), 4);

  console.log(JSON.stringify({ unit: "rust-syn-core", standard_id: e.proto_standard_id(), ok: true }));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
