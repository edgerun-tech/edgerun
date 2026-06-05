#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/yaml-scan.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  return execFileSync("wat2wasm", [file, "-o", "-"]);
}

function write(memory, ptr, text) {
  const bytes = Buffer.from(text, "utf8");
  memory.fill(0, ptr, ptr + bytes.length + 64);
  memory.set(bytes, ptr);
  return bytes.length;
}

function u32s(view, ptr, count) {
  return Array.from({ length: count }, (_, index) => view.getUint32(ptr + index * 4, true));
}

(async () => {
  const wasm = compileWat(watPath);
  assert(WebAssembly.validate(wasm), "compiled wasm did not validate");
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const exports = instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);
  const view = new DataView(memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300042);

  let len = write(
    memory,
    1024,
    "---\n# comment\nname: edge # trimmed\n  enabled: true\nitems:\n  - one\n  - \"two:still\"\n...\n",
  );

  assert.strictEqual(exports.yaml_scan_line(1024, len, 0, 4096), 0);
  assert.deepStrictEqual(u32s(view, 4096, 8), [5, 0, 0, 3, 0, 0, 0, 0]);

  assert.strictEqual(exports.yaml_scan_line(1024, len, 4, 4096), 0);
  assert.deepStrictEqual(u32s(view, 4096, 8), [1, 0, 4, 9, 0, 0, 0, 0]);

  assert.strictEqual(exports.yaml_scan_line(1024, len, 14, 4096), 0);
  assert.deepStrictEqual(u32s(view, 4096, 8), [2, 0, 14, 20, 14, 4, 20, 4]);

  assert.strictEqual(exports.yaml_scan_line(1024, len, 35, 4096), 0);
  assert.deepStrictEqual(u32s(view, 4096, 8), [2, 2, 35, 15, 37, 7, 46, 4]);

  assert.strictEqual(exports.yaml_scan_line(1024, len, 58, 4096), 0);
  assert.deepStrictEqual(u32s(view, 4096, 8), [3, 2, 58, 7, 60, 1, 62, 3]);

  assert.strictEqual(exports.yaml_scan_line(1024, len, 66, 4096), 0);
  assert.deepStrictEqual(u32s(view, 4096, 8), [3, 2, 66, 15, 68, 1, 70, 11]);

  assert.strictEqual(exports.yaml_scan_line(1024, len, 82, 4096), 0);
  assert.deepStrictEqual(u32s(view, 4096, 8), [6, 0, 82, 3, 0, 0, 0, 0]);

  assert.strictEqual(exports.yaml_scan_document(1024, len, 4096), 0);
  assert.deepStrictEqual(u32s(view, 4096, 8), [8, 8, 1, 2, 3, 2, 1, len]);

  for (const [text, kind, flags] of [
    ["", 0, 0],
    ["null", 1, 0],
    ["~", 1, 0],
    ["true", 2, 1],
    ["off", 2, 0],
    ["123", 3, 0],
    ["-1.25e2", 4, 0],
    ['"quoted"', 5, 1],
    ["'literal'", 5, 2],
    ["[one, two]", 6, 0],
    ["{name: edge}", 7, 0],
    ["plain scalar", 8, 0],
  ]) {
    len = write(memory, 1024, text);
    assert.strictEqual(exports.yaml_scan_scalar(1024, len, 4096), 0, `${text} status`);
    const out = u32s(view, 4096, 4);
    assert.strictEqual(out[0], kind, `${text} kind`);
    assert.strictEqual(out[3], flags, `${text} flags`);
  }

  len = write(memory, 1024, '"unterminated');
  assert.strictEqual(exports.yaml_scan_scalar(1024, len, 4096), 5);

  len = write(memory, 1024, "[unterminated");
  assert.strictEqual(exports.yaml_scan_scalar(1024, len, 4096), 5);

  len = write(memory, 1024, "\tbad: indent");
  assert.strictEqual(exports.yaml_scan_line(1024, len, 0, 4096), 3);

  len = write(memory, 1024, '"not: key": value');
  assert.strictEqual(exports.yaml_scan_line(1024, len, 0, 4096), 0);
  assert.deepStrictEqual(u32s(view, 4096, 8), [2, 0, 0, 17, 0, 10, 12, 5]);

  console.log(
    JSON.stringify(
      {
        unit: "yaml-scan",
        abi_version: exports.proto_abi_version(),
        standard_id: exports.proto_standard_id(),
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
