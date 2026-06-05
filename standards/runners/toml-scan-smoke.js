#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/toml-scan.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  return execFileSync("wat2wasm", [file, "-o", "-"]);
}

function write(memory, ptr, text) {
  const bytes = Buffer.from(text, "utf8");
  memory.fill(0, ptr, ptr + bytes.length + 16);
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
  assert.strictEqual(exports.proto_standard_id(), 300017);

  let len = write(memory, 1024, "\n# comment\nkey = 42\n[server]\n[[users]]\n");
  assert.strictEqual(exports.toml_scan_line(1024, len, 0, 2048), 0);
  assert.strictEqual(u32s(view, 2048, 4)[0], 0);
  assert.strictEqual(exports.toml_scan_line(1024, len, 1, 2048), 0);
  assert.strictEqual(u32s(view, 2048, 4)[0], 1);
  assert.strictEqual(exports.toml_scan_line(1024, len, 11, 2048), 0);
  assert.strictEqual(u32s(view, 2048, 4)[0], 2);
  assert.strictEqual(exports.toml_scan_line(1024, len, 20, 2048), 0);
  assert.strictEqual(u32s(view, 2048, 4)[0], 3);
  assert.strictEqual(exports.toml_scan_line(1024, len, 29, 2048), 0);
  assert.strictEqual(u32s(view, 2048, 4)[0], 4);

  len = write(memory, 1024, "name = \"edge\"");
  assert.strictEqual(exports.toml_scan_key_value(1024, len, 2048), 0);
  assert.deepStrictEqual(u32s(view, 2048, 4), [0, 4, 7, 6]);

  len = write(memory, 1024, "missing");
  assert.strictEqual(exports.toml_scan_key_value(1024, len, 2048), 3);

  len = write(memory, 1024, "[table]");
  assert.strictEqual(exports.toml_scan_table_header(1024, len, 2048), 0);
  assert.deepStrictEqual(u32s(view, 2048, 4), [3, 1, 5, 7]);

  len = write(memory, 1024, "[[array]]");
  assert.strictEqual(exports.toml_scan_table_header(1024, len, 2048), 0);
  assert.deepStrictEqual(u32s(view, 2048, 4), [4, 2, 5, 9]);

  len = write(memory, 1024, "[broken");
  assert.strictEqual(exports.toml_scan_table_header(1024, len, 2048), 3);

  for (const [text, kind] of [
    ['"quoted\\n"', 1],
    ["'literal'", 2],
    ["true", 3],
    ["false", 3],
    ["12345", 4],
    ["0x2a", 4],
    ["0o52", 4],
    ["0b101010", 4],
    ["1.25", 5],
    ["6e7", 5],
    ["[1, true, 'x']", 6],
  ]) {
    len = write(memory, 1024, text);
    assert.strictEqual(exports.toml_scan_scalar(1024, len, 2048), 0, `${text} status`);
    assert.strictEqual(u32s(view, 2048, 2)[0], kind, `${text} kind`);
  }

  len = write(memory, 1024, '"unterminated');
  assert.strictEqual(exports.toml_scan_scalar(1024, len, 2048), 5);

  len = write(memory, 1024, "[1, 2");
  assert.strictEqual(exports.toml_scan_scalar(1024, len, 2048), 5);

  for (const text of ["0b102", "0o78", "0xzz"]) {
    len = write(memory, 1024, text);
    assert.strictEqual(exports.toml_scan_scalar(1024, len, 2048), 3, `${text} should reject`);
  }

  console.log(
    JSON.stringify(
      {
        unit: "toml-scan",
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
