#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/cesu8-mutf8.wat";

function compileWat(file) {
  const wasmPath = path.join(os.tmpdir(), `edgerun-${path.basename(file)}-${process.pid}.wasm`);
  execFileSync("wat2wasm", [file, "-o", wasmPath], { stdio: "pipe" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "pipe" });
  const bytes = fs.readFileSync(wasmPath);
  fs.unlinkSync(wasmPath);
  return bytes;
}

function unpack(packed) {
  return {
    status: Number(packed & 0xffffffffn),
    count: Number((packed >> 32n) & 0xffffffffn),
  };
}

(async () => {
  const module = await WebAssembly.instantiate(compileWat(watPath), {});
  const exports = module.instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300069);

  const inPtr = 1024;
  const outPtr = 8192;

  function call(name, bytes, cap = 512) {
    memory.fill(0, inPtr, inPtr + 2048);
    memory.fill(0, outPtr, outPtr + 2048);
    memory.set(Uint8Array.from(bytes), inPtr);
    const result = unpack(exports[name](inPtr, bytes.length, outPtr, cap));
    return {
      ...result,
      bytes: Array.from(memory.slice(outPtr, outPtr + result.count)),
      text: Buffer.from(memory.slice(outPtr, outPtr + result.count)).toString("utf8"),
    };
  }

  const asciiBmp = Buffer.from("Hello, ȅ, €", "utf8");
  const gothic = Buffer.from("𐐀", "utf8");
  const sparkles = Buffer.from("A💖B", "utf8");
  const cesuGothic = [0xed, 0xa0, 0x81, 0xed, 0xb0, 0x80];
  const cesuHeart = [0x41, 0xed, 0xa0, 0xbd, 0xed, 0xb2, 0x96, 0x42];

  assert.deepStrictEqual(call("cesu8_encode_utf8", asciiBmp).bytes, Array.from(asciiBmp));
  assert.deepStrictEqual(call("mutf8_encode_utf8", asciiBmp).bytes, Array.from(asciiBmp));
  assert.deepStrictEqual(call("cesu8_decode_strict", asciiBmp).bytes, Array.from(asciiBmp));
  assert.deepStrictEqual(call("mutf8_decode_strict", asciiBmp).bytes, Array.from(asciiBmp));

  assert.deepStrictEqual(call("cesu8_encode_utf8", gothic).bytes, cesuGothic);
  assert.deepStrictEqual(call("mutf8_encode_utf8", gothic).bytes, cesuGothic);
  assert.deepStrictEqual(call("cesu8_decode_strict", cesuGothic).bytes, Array.from(gothic));
  assert.deepStrictEqual(call("cesu8_encode_utf8", sparkles).bytes, cesuHeart);
  assert.deepStrictEqual(call("cesu8_decode_strict", cesuHeart).text, "A💖B");

  assert.deepStrictEqual(call("mutf8_encode_utf8", [0x00]).bytes, [0xc0, 0x80]);
  assert.deepStrictEqual(call("mutf8_decode_strict", [0xc0, 0x80]).bytes, [0x00]);
  assert.deepStrictEqual(call("cesu8_decode_strict", [0x00]).bytes, [0x00]);
  assert.strictEqual(call("mutf8_decode_strict", [0x00]).status, 3);
  assert.strictEqual(call("cesu8_decode_strict", [0xc0, 0x80]).status, 3);

  assert.strictEqual(call("cesu8_decode_strict", [0xf0, 0x9f, 0x92, 0x96]).status, 3);
  assert.strictEqual(call("cesu8_decode_strict", [0xed, 0xa0, 0xbd]).status, 5);
  assert.strictEqual(call("cesu8_decode_strict", [0xed, 0xb2, 0x96]).status, 3);
  assert.strictEqual(call("cesu8_decode_strict", [0xed, 0xa0, 0xbd, 0xed, 0x80, 0x80]).status, 3);
  assert.strictEqual(call("mutf8_decode_strict", [0xc0]).status, 5);
  assert.strictEqual(call("mutf8_decode_strict", [0xc0, 0x81]).status, 3);

  assert.strictEqual(call("cesu8_encode_utf8", sparkles, 3).status, 2);
  assert.strictEqual(call("cesu8_decode_strict", cesuHeart, 2).status, 2);
  assert.strictEqual(call("cesu8_encode_utf8", [0xf0, 0x28, 0x8c, 0x28]).status, 3);

  const cases = [
    "ascii_bmp_passthrough",
    "supplementary_encode_decode",
    "mutf8_null",
    "literal_utf8_four_byte_rejected_by_strict_decode",
    "truncated_unpaired_surrogate_rejected",
    "output_capacity_rejected",
  ];

  console.log(
    JSON.stringify(
      {
        unit: "cesu8-mutf8",
        abi_version: exports.proto_abi_version(),
        standard_id: exports.proto_standard_id(),
        ok: true,
        cases,
      },
      null,
      2,
    ),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
