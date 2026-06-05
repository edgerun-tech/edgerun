#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/codex-patch-core.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  const wasm = execFileSync("wat2wasm", [file, "-o", "-"]);
  execFileSync("wasm-validate", ["-"], { input: wasm });
  return wasm;
}

function writeAscii(memory, value, ptr = 2048) {
  memory.fill(0, ptr, ptr + Math.max(256, value.length + 1));
  memory.set(Buffer.from(value, "ascii"), ptr);
  return ptr;
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const e = instance.exports;
  const memory = new Uint8Array(e.memory.buffer);
  const code = (fn, value) => {
    const ptr = writeAscii(memory, value);
    return e[fn](ptr, value.length);
  };

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300113);
  assert.equal(code("codex_patch_marker_code", "*** Begin Patch"), 1);
  assert.equal(code("codex_patch_marker_code", "*** End Patch"), 2);
  assert.equal(code("codex_patch_marker_code", "*** Add File: README.md"), 3);
  assert.equal(code("codex_patch_marker_code", "*** Delete File: src/lib.rs"), 4);
  assert.equal(code("codex_patch_marker_code", "*** Update File: src/lib.rs"), 5);
  assert.equal(code("codex_patch_marker_code", "*** Move to: src/new.rs"), 6);
  assert.equal(code("codex_patch_marker_code", "*** End of File"), 7);
  assert.equal(code("codex_patch_marker_code", "@@ fn main()"), 8);
  assert.equal(code("codex_patch_marker_code", "@@"), 8);

  assert.equal(code("codex_patch_line_class", "+new"), 1);
  assert.equal(code("codex_patch_line_class", "-old"), 2);
  assert.equal(code("codex_patch_line_class", " context"), 3);
  assert.equal(code("codex_patch_line_class", "@@ context"), 4);
  assert.equal(code("codex_patch_line_class", "*** End of File"), 5);

  assert.equal(code("codex_patch_invocation_code", "apply_patch"), 1);
  assert.equal(code("codex_patch_invocation_code", "applypatch"), 1);
  assert.equal(code("codex_patch_invocation_code", "bash"), 2);
  assert.equal(code("codex_patch_invocation_code", "zsh"), 2);
  assert.equal(code("codex_patch_invocation_code", "sh"), 2);

  assert.equal(code("codex_patch_outcome_code", "body"), 1);
  assert.equal(code("codex_patch_outcome_code", "verify"), 2);
  assert.equal(code("codex_patch_outcome_code", "not_apply_patch"), 4);
  assert.equal(code("codex_patch_outcome_code", "shell_parse_err"), 5);
  assert.equal(code("codex_patch_error_code", "parse_error"), 1);
  assert.equal(code("codex_patch_error_code", "implicit_invocation"), 3);

  console.log(JSON.stringify({ unit: "codex-patch-core", standard_id: e.proto_standard_id(), ok: true }));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
