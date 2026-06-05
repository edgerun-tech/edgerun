#!/usr/bin/env node

const fs = require("fs");
const os = require("os");
const path = require("path");
const { spawnSync } = require("child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/json-tape.wat";
const wasmPath = path.join(os.tmpdir(), `json-tape-${process.pid}.wasm`);

function compileWat() {
  const result = spawnSync("wat2wasm", [watPath, "-o", wasmPath], {
    encoding: "utf8",
  });
  if (result.status !== 0) {
    throw new Error(
      `wat2wasm failed\n${result.stdout || ""}${result.stderr || ""}`,
    );
  }
  return fs.readFileSync(wasmPath);
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function unpack(packed) {
  return {
    status: Number(packed & 0xffffffffn),
    value: Number((packed >> 32n) & 0xffffffffn),
  };
}

function writeAscii(memory, ptr, text) {
  const bytes = Buffer.from(text, "ascii");
  memory.fill(0, ptr, ptr + bytes.length + 16);
  memory.set(bytes, ptr);
  return bytes.length;
}

function tokenKinds(memory, tokenPtr, count) {
  const view = new DataView(memory.buffer);
  const kinds = [];
  for (let i = 0; i < count; i += 1) {
    kinds.push(view.getUint32(tokenPtr + i * 20, true));
  }
  return kinds;
}

function parseJson(exports, memory, text, tokenCap = 32) {
  const len = writeAscii(memory, 1024, text);
  return unpack(exports.json_parse_tape(1024, len, 2048, tokenCap, 4096, 1024));
}

function assertParseError(exports, memory, text, message, status = 3) {
  const result = parseJson(exports, memory, text);
  assert(
    result.status === status,
    `${message}: expected status ${status}, got ${result.status}`,
  );
}

(async () => {
  const wasm = compileWat();
  assert(WebAssembly.validate(wasm), "compiled wasm did not validate");

  const { instance } = await WebAssembly.instantiate(wasm, {});
  const exports = instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);

  assert(exports.proto_abi_version() === 2, "unexpected ABI version");
  assert(exports.proto_standard_id() === 300007, "unexpected standard id");

  let len = writeAscii(memory, 1024, '"x"');
  let result = unpack(exports.json_scan_string(1024, len, 0));
  assert(result.status === 0 && result.value === 3, "string scan failed");

  len = writeAscii(memory, 1024, '"\\x"');
  result = unpack(exports.json_scan_string(1024, len, 0));
  assert(result.status === 3, "invalid string escape was accepted");

  len = writeAscii(memory, 1024, '"\\u12G4"');
  result = unpack(exports.json_scan_string(1024, len, 0));
  assert(result.status === 3, "invalid unicode escape was accepted");

  len = writeAscii(memory, 1024, '"\\u123"');
  result = unpack(exports.json_scan_string(1024, len, 0));
  assert(result.status !== 0, "truncated unicode escape was accepted");

  len = writeAscii(memory, 1024, "123");
  result = unpack(exports.json_scan_number(1024, len, 0));
  assert(result.status === 0 && result.value === 3, "number scan failed");

  len = writeAscii(memory, 1024, "00");
  result = unpack(exports.json_scan_number(1024, len, 0));
  assert(result.status === 3, "invalid leading-zero number was accepted");

  const json = '{"a":[1,"x"],"b":true}';
  len = writeAscii(memory, 1024, json);
  result = unpack(exports.json_parse_tape(1024, len, 2048, 32, 4096, 1024));
  assert(result.status === 0, `json_parse_tape failed with ${result.status}`);
  assert(result.value >= 7, "json_parse_tape wrote too few tokens");

  const expected = [1, 3, 2, 5, 4, 3, 6];
  const actual = tokenKinds(memory, 2048, expected.length);
  assert(
    expected.every((kind, index) => actual[index] === kind),
    `unexpected token kind order: ${actual.join(",")}`,
  );

  assertParseError(exports, memory, "[1,]", "array trailing comma accepted");
  assertParseError(exports, memory, '{"a":1,}', "object trailing comma accepted");
  assertParseError(exports, memory, "[1 2]", "array missing comma accepted");
  assertParseError(exports, memory, '{"a":1 "b":2}', "object missing comma accepted");
  assertParseError(exports, memory, "{a:1}", "bare object key accepted");
  assertParseError(exports, memory, "{1:2}", "numeric object key accepted");
  assertParseError(exports, memory, "{}x", "object trailing junk accepted");
  assertParseError(exports, memory, "true false", "literal trailing junk accepted");
  assertParseError(exports, memory, '"\\x"', "invalid parse escape accepted");
  assertParseError(exports, memory, '"\\u12G4"', "invalid parse unicode accepted");

  result = parseJson(exports, memory, '{"a":[1]}', 2);
  assert(result.status === 2, `too-small token cap returned ${result.status}`);

  console.log(
    JSON.stringify(
      {
        unit: "json-tape",
        abi_version: exports.proto_abi_version(),
        standard_id: exports.proto_standard_id(),
        ok: true,
        tokens_written: result.value,
        token_kinds: actual,
      },
      null,
      2,
    ),
  );
})()
  .catch((error) => {
    console.error(error.stack || String(error));
    process.exit(1);
  })
  .finally(() => {
    try {
      fs.unlinkSync(wasmPath);
    } catch (_) {
      // Temp output is best-effort cleanup only.
    }
  });
