#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/codex-shell-command-core.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  const wasm = execFileSync("wat2wasm", [file, "-o", "-"]);
  execFileSync("wasm-validate", ["-"], { input: wasm });
  return wasm;
}

function writeAscii(memory, value, ptr) {
  memory.fill(0, ptr, ptr + Math.max(128, value.length + 1));
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
    const ptr = writeAscii(memory, value, 2048);
    return e[fn](ptr, value.length);
  };

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300114);
  assert.equal(code("codex_shell_command_class", "cat"), 1);
  assert.equal(code("codex_shell_command_class", "tree"), 2);
  assert.equal(code("codex_shell_command_class", "rg"), 3);
  assert.equal(code("codex_shell_command_class", "sed"), 4);
  assert.equal(code("codex_shell_command_class", "find"), 5);
  assert.equal(code("codex_shell_command_class", "printf"), 6);
  assert.equal(code("codex_shell_command_class", "xargs"), 7);
  assert.equal(code("codex_shell_command_class", "bash"), 8);
  assert.equal(code("codex_shell_command_class", "rm"), 9);

  assert.equal(code("codex_shell_connector_code", "|"), 1);
  assert.equal(code("codex_shell_connector_code", "&&"), 2);
  assert.equal(code("codex_shell_connector_code", "||"), 3);
  assert.equal(code("codex_shell_connector_code", ";"), 4);

  assert.equal(code("codex_shell_safe_builtin", "cat"), 1);
  assert.equal(code("codex_shell_safe_builtin", "grep"), 1);
  assert.equal(code("codex_shell_safe_builtin", "whoami"), 1);
  assert.equal(code("codex_shell_safe_builtin", "rm"), 0);

  let cmd = writeAscii(memory, "rm", 2048);
  let arg = writeAscii(memory, "-rf", 3072);
  assert.equal(e.codex_shell_danger_code(cmd, 2, arg, 3), 1);
  arg = writeAscii(memory, "-f", 3072);
  assert.equal(e.codex_shell_danger_code(cmd, 2, arg, 2), 1);
  cmd = writeAscii(memory, "sudo", 2048);
  assert.equal(e.codex_shell_danger_code(cmd, 4, arg, 2), 2);

  let shell = writeAscii(memory, "bash", 2048);
  let flag = writeAscii(memory, "-lc", 3072);
  assert.equal(e.codex_shell_wrapper_code(shell, 4, flag, 3), 1);
  shell = writeAscii(memory, "fish", 2048);
  assert.equal(e.codex_shell_wrapper_code(shell, 4, flag, 3), 0);

  console.log(JSON.stringify({ unit: "codex-shell-command-core", standard_id: e.proto_standard_id(), ok: true }));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
