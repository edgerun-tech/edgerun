#!/usr/bin/env node

const fs = require("fs");
const os = require("os");
const path = require("path");
const { spawnSync } = require("child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/json-value-core.wat";
const tapeWatPath = "standards/build/wasm/codec-primitives/json-tape.wat";
const wasmPath = path.join(os.tmpdir(), `json-value-core-${process.pid}.wasm`);
const tapeWasmPath = path.join(os.tmpdir(), `json-tape-for-value-${process.pid}.wasm`);

function compileWat(input, output) {
  const result = spawnSync("wat2wasm", [input, "-o", output], {
    encoding: "utf8",
  });
  if (result.status !== 0) {
    throw new Error(`wat2wasm failed for ${input}\n${result.stdout || ""}${result.stderr || ""}`);
  }
  return fs.readFileSync(output);
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

(async () => {
  const wasm = compileWat(watPath, wasmPath);
  const tapeWasm = compileWat(tapeWatPath, tapeWasmPath);
  assert(WebAssembly.validate(wasm), "json-value-core wasm did not validate");
  assert(WebAssembly.validate(tapeWasm), "json-tape wasm did not validate");

  const { instance } = await WebAssembly.instantiate(wasm, {});
  const exports = instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);

  assert(exports.proto_abi_version() === 2, "unexpected ABI version");
  assert(exports.proto_standard_id() === 300041, "unexpected standard id");

  assert(exports.json_value_variant_for_tape_kind(7) === 0, "null variant");
  assert(exports.json_value_variant_for_tape_kind(6) === 1, "bool variant");
  assert(exports.json_value_variant_for_tape_kind(5) === 2, "number variant");
  assert(exports.json_value_variant_for_tape_kind(3) === 3, "string variant");
  assert(exports.json_value_variant_for_tape_kind(2) === 4, "array variant");
  assert(exports.json_value_variant_for_tape_kind(1) === 5, "object variant");
  assert(exports.json_value_variant_for_tape_kind(99) === -1, "unknown variant");

  let len = writeAscii(memory, 1024, "-2147483648");
  let caps = unpack(exports.json_number_caps(1024, len));
  assert(caps.status === 0 && (caps.value & 1) && (caps.value & 8), "negative i32 caps");
  assert((caps.value & 2) === 0, "negative number converted to u64");

  len = writeAscii(memory, 1024, "4294967295");
  caps = unpack(exports.json_number_caps(1024, len));
  assert(caps.status === 0 && (caps.value & 2) && (caps.value & 16), "u32 caps");
  assert((caps.value & 8) === 0, "u32 max converted to i32");

  len = writeAscii(memory, 1024, "18446744073709551615");
  caps = unpack(exports.json_number_caps(1024, len));
  assert(caps.status === 0 && (caps.value & 2) && (caps.value & 32), "u64 max caps");
  assert((caps.value & 1) === 0, "u64 max converted to i64");

  len = writeAscii(memory, 1024, "1.25e2");
  caps = unpack(exports.json_number_caps(1024, len));
  assert(caps.status === 0 && caps.value === 260, "float caps");

  len = writeAscii(memory, 1024, "18446744073709551616");
  caps = unpack(exports.json_number_caps(1024, len));
  assert(caps.status === 4, "u64 overflow accepted");

  assert(exports.json_value_can_convert(2, 255, 6) === 1, "number to i64");
  assert(exports.json_value_can_convert(3, 0, 3) === 1, "string to string");
  assert(exports.json_value_can_convert(3, 0, 6) === 0, "string to i64");
  assert(exports.json_value_can_convert(5, 0, 5) === 1, "object to object");

  const { instance: tapeInstance } = await WebAssembly.instantiate(tapeWasm, {});
  const tapeExports = tapeInstance.exports;
  const tapeMemory = new Uint8Array(tapeExports.memory.buffer);
  const json = '{"a":1,"dup":2,"dup":3,"nested":{"dup":4},"arr":[5]}';
  const jsonLen = writeAscii(tapeMemory, 1024, json);
  const parsed = unpack(tapeExports.json_parse_tape(1024, jsonLen, 2048, 64, 4096, 1024));
  assert(parsed.status === 0, `json_parse_tape failed with ${parsed.status}`);

  memory.set(tapeMemory.subarray(1024, 1024 + jsonLen), 4096);
  memory.set(tapeMemory.subarray(2048, 2048 + parsed.value * 20), 8192);
  writeAscii(memory, 2048, "dup");
  let found = unpack(exports.json_object_find_field(4096, jsonLen, 8192, parsed.value, 0, 2048, 3));
  assert(found.status === 0 && found.value === 4, `first dup lookup returned ${JSON.stringify(found)}`);
  const copiedKinds = tokenKinds(memory, 8192, parsed.value);
  assert(copiedKinds[found.value] === 5, "dup value token should be number");

  writeAscii(memory, 2048, "missing");
  found = unpack(exports.json_object_find_field(4096, jsonLen, 8192, parsed.value, 0, 2048, 7));
  assert(found.status === 1, "missing field should not be found");

  writeAscii(memory, 2048, "arr");
  found = unpack(exports.json_object_find_field(4096, jsonLen, 8192, parsed.value, 0, 2048, 3));
  assert(found.status === 0 && copiedKinds[found.value] === 2, "array field lookup failed");

  console.log(
    JSON.stringify(
      {
        unit: "json-value-core",
        abi_version: exports.proto_abi_version(),
        standard_id: exports.proto_standard_id(),
        ok: true,
        token_count: parsed.value,
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
    for (const file of [wasmPath, tapeWasmPath]) {
      try {
        fs.unlinkSync(file);
      } catch (_) {
        // Temp output is best-effort cleanup only.
      }
    }
  });
