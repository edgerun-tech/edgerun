#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/json-scalar.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  return execFileSync("wat2wasm", [file, "-o", "-"]);
}

function unpack(value) {
  return {
    status: Number(value & 0xffffffffn),
    value: Number((value >> 32n) & 0xffffffffn),
  };
}

function write(memory, ptr, text, encoding = "utf8") {
  const bytes = Buffer.from(text, encoding);
  memory.fill(0, ptr, ptr + bytes.length + 16);
  memory.set(bytes, ptr);
  return bytes.length;
}

function readBytes(memory, ptr, len) {
  return Buffer.from(memory.slice(ptr, ptr + len));
}

(async () => {
  const wasm = compileWat(watPath);
  assert(WebAssembly.validate(wasm), "compiled wasm did not validate");
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const exports = instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);
  const view = new DataView(memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300016);

  let len = write(memory, 1024, '"\\uD834\\uDD1E"');
  assert.deepStrictEqual(unpack(exports.json_scan_string_strict(1024, len, 0)), {
    status: 0,
    value: len,
  });

  len = write(memory, 1024, '"\\uD834"');
  assert.strictEqual(unpack(exports.json_scan_string_strict(1024, len, 0)).status, 3);

  len = write(memory, 1024, '"\\uDD1E"');
  assert.strictEqual(unpack(exports.json_scan_string_strict(1024, len, 0)).status, 3);

  len = write(memory, 1024, '"a\u0001b"');
  assert.strictEqual(unpack(exports.json_scan_string_strict(1024, len, 0)).status, 3);

  len = write(memory, 1024, '"a\\n\\t\\u00A9\\uD834\\uDD1E"');
  let result = unpack(exports.json_unescape_string(1024, len, 2048, 64));
  assert.strictEqual(result.status, 0);
  assert.strictEqual(readBytes(memory, 2048, result.value).toString("utf8"), "a\n\t©𝄞");

  result = unpack(exports.json_unescape_string(1024, len, 2048, 4));
  assert.strictEqual(result.status, 2);

  for (const [text, end] of [
    ["0", 1],
    ["-12", 3],
    ["1.25", 4],
    ["1e10", 4],
    ["-2.5E-3", 7],
  ]) {
    len = write(memory, 1024, text);
    assert.deepStrictEqual(unpack(exports.json_scan_number_strict(1024, len, 0)), {
      status: 0,
      value: end,
    });
  }

  for (const text of ["01", "-", "1.", "1e", "1e+", "x"]) {
    len = write(memory, 1024, text);
    assert.strictEqual(
      unpack(exports.json_scan_number_strict(1024, len, 0)).status,
      text === "-" ? 5 : 3,
      `${text} should reject`,
    );
  }

  len = write(memory, 1024, "-9223372036854775808", "ascii");
  assert.strictEqual(exports.json_parse_i64(1024, len, 2048), 0);
  assert.strictEqual(view.getBigInt64(2048, true), -9223372036854775808n);

  len = write(memory, 1024, "9223372036854775807", "ascii");
  assert.strictEqual(exports.json_parse_i64(1024, len, 2048), 0);
  assert.strictEqual(view.getBigInt64(2048, true), 9223372036854775807n);

  len = write(memory, 1024, "9223372036854775808", "ascii");
  assert.strictEqual(exports.json_parse_i64(1024, len, 2048), 4);

  len = write(memory, 1024, "18446744073709551615", "ascii");
  assert.strictEqual(exports.json_parse_u64(1024, len, 2048), 0);
  assert.strictEqual(view.getBigUint64(2048, true), 18446744073709551615n);

  len = write(memory, 1024, "18446744073709551616", "ascii");
  assert.strictEqual(exports.json_parse_u64(1024, len, 2048), 4);

  len = write(memory, 1024, "00", "ascii");
  assert.strictEqual(exports.json_parse_u64(1024, len, 2048), 3);

  console.log(
    JSON.stringify(
      {
        unit: "json-scalar",
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
