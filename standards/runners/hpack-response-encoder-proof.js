#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const prefixWat =
  process.argv[2] || "standards/build/wasm/codec-primitives/http-prefix-int.wat";
const stringWat =
  process.argv[3] || "standards/build/wasm/codec-primitives/hpack-string.wat";

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
    value: Number((packed >> 32n) & 0xffffffffn),
  };
}

function appendPrefix(prefix, value, prefixBits, highBits, expected) {
  const ptr = 1024;
  const memory = new Uint8Array(prefix.exports.memory.buffer);
  memory.fill(0, ptr, ptr + 32);
  const got = unpack(prefix.exports.hpack_prefix_int_encode(BigInt(value), prefixBits, highBits, ptr, 32));
  assert.deepStrictEqual(got, { status: 0, value: expected.length });
  assert.deepStrictEqual(Array.from(memory.slice(ptr, ptr + expected.length)), expected);
  return expected;
}

function proveRawString(hpackString, encoded, value) {
  const ptr = 2048;
  const outPtr = 3072;
  const metaPtr = 4096;
  const memory = new Uint8Array(hpackString.exports.memory.buffer);
  memory.fill(0, ptr, ptr + 256);
  memory.fill(0, outPtr, outPtr + 256);
  memory.set(encoded, ptr);
  const got = unpack(
    hpackString.exports.hpack_string_decode(ptr, encoded.length, outPtr, 256, metaPtr),
  );
  assert.deepStrictEqual(got, { status: 0, value: value.length });
  assert.deepStrictEqual(Array.from(memory.slice(outPtr, outPtr + value.length)), value);
}

(async () => {
  const prefix = (await WebAssembly.instantiate(compileWat(prefixWat), {})).instance;
  const hpackString = (await WebAssembly.instantiate(compileWat(stringWat), {})).instance;

  assert.strictEqual(prefix.exports.proto_abi_version(), 2);
  assert.strictEqual(prefix.exports.proto_standard_id(), 300011);
  assert.strictEqual(hpackString.exports.proto_abi_version(), 2);
  assert.strictEqual(hpackString.exports.proto_standard_id(), 300019);

  const textPlain = Array.from(Buffer.from("text/plain", "ascii"));
  const bodyLen = Array.from(Buffer.from("2", "ascii"));

  const status200 = appendPrefix(prefix, 8, 7, 0b1, [0x88]);
  const contentTypeName = appendPrefix(prefix, 31, 4, 0b1, [0x1f, 0x10]);
  const contentTypeValuePrefix = appendPrefix(prefix, textPlain.length, 7, 0, [0x0a]);
  const contentLengthName = appendPrefix(prefix, 28, 4, 0b1, [0x1f, 0x0d]);
  const contentLengthValuePrefix = appendPrefix(prefix, bodyLen.length, 7, 0, [0x01]);

  proveRawString(hpackString, [...contentTypeValuePrefix, ...textPlain], textPlain);
  proveRawString(hpackString, [...contentLengthValuePrefix, ...bodyLen], bodyLen);

  const directEncoderBytes = [
    ...status200,
    ...contentTypeName,
    ...contentTypeValuePrefix,
    ...textPlain,
    ...contentLengthName,
    ...contentLengthValuePrefix,
    ...bodyLen,
  ];

  assert.deepStrictEqual(directEncoderBytes, [
    0x88,
    0x1f, 0x10,
    0x0a, 0x74, 0x65, 0x78, 0x74, 0x2f, 0x70, 0x6c, 0x61, 0x69, 0x6e,
    0x1f, 0x0d,
    0x01, 0x32,
  ]);

  console.log(JSON.stringify({
    unit: "hpack-response-encoder-proof",
    ok: true,
    proof: "direct 200 response bytes are composed from http-prefix-int.wat prefixes and hpack-string.wat raw string records",
    bytes: Buffer.from(directEncoderBytes).toString("hex"),
  }, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
