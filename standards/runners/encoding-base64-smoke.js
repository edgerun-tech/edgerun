#!/usr/bin/env node

const fs = require("fs");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/encoding-base64.wat";
const wasmPath = process.argv[3] || "/tmp/encoding-base64-smoke.wasm";

function status(packed) {
  return Number(packed & 0xffffffffn);
}

function count(packed) {
  return Number(packed >> 32n);
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function write(memory, ptr, bytes) {
  memory.fill(0, ptr, ptr + bytes.length + 64);
  memory.set(bytes, ptr);
}

function read(memory, ptr, len) {
  return Buffer.from(memory.slice(ptr, ptr + len));
}

function callCodec(exports, name, input, outCap = 4096) {
  const memory = new Uint8Array(exports.memory.buffer);
  const inPtr = 1024;
  const outPtr = 4096;
  write(memory, inPtr, input);
  const packed = exports[name](inPtr, input.length, outPtr, outCap);
  return {
    status: status(packed),
    written: count(packed),
    output: read(memory, outPtr, count(packed)),
  };
}

(async () => {
  execFileSync("wat2wasm", [watPath, "-o", wasmPath], { stdio: "inherit" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "inherit" });

  const module = await WebAssembly.instantiate(fs.readFileSync(wasmPath), {});
  const exports = module.instance.exports;

  assert(exports.proto_abi_version() === 2, "unexpected ABI version");
  assert(exports.proto_standard_id() === 300046, "unexpected standard id");

  for (const [plain, encoded] of [
    ["", ""],
    ["f", "Zg=="],
    ["fo", "Zm8="],
    ["foo", "Zm9v"],
    ["foob", "Zm9vYg=="],
    ["fooba", "Zm9vYmE="],
    ["foobar", "Zm9vYmFy"],
  ]) {
    const enc = callCodec(exports, "base64_standard_encode", Buffer.from(plain, "ascii"));
    assert(enc.status === 0, `base64 encode failed for ${plain}`);
    assert(enc.written === encoded.length, `base64 encode length mismatch for ${plain}`);
    assert(enc.output.toString("ascii") === encoded, `base64 encode mismatch for ${plain}`);

    const dec = callCodec(exports, "base64_standard_decode", Buffer.from(encoded, "ascii"));
    assert(dec.status === 0, `base64 decode failed for ${encoded}`);
    assert(dec.written === plain.length, `base64 decode length mismatch for ${encoded}`);
    assert(dec.output.toString("ascii") === plain, `base64 decode mismatch for ${encoded}`);
  }

  for (const input of ["Zg=", "Zg===", "Z=g=", "Zm=9", "====", "Zm9v\n"]) {
    const invalid = callCodec(
      exports,
      "base64_standard_decode",
      Buffer.from(input, "ascii"),
    );
    assert(invalid.status === 3, `invalid padding/input should fail for ${input}`);
  }

  for (const input of ["Zh==", "Zm9="]) {
    const noncanonical = callCodec(
      exports,
      "base64_standard_decode",
      Buffer.from(input, "ascii"),
    );
    assert(noncanonical.status === 3, `noncanonical tail should fail for ${input}`);
  }

  const shortEncode = callCodec(
    exports,
    "base64_standard_encode",
    Buffer.from("foo", "ascii"),
    3,
  );
  assert(shortEncode.status === 2, "base64 encode should reject small output cap");

  const shortDecode = callCodec(
    exports,
    "base64_standard_decode",
    Buffer.from("Zm9v", "ascii"),
    2,
  );
  assert(shortDecode.status === 2, "base64 decode should reject small output cap");

  const result = {
    unit: "encoding-base64",
    abi_version: exports.proto_abi_version(),
    standard_id: exports.proto_standard_id(),
    ok: true,
    cases: [
      "rfc4648_empty",
      "rfc4648_f",
      "rfc4648_fo",
      "rfc4648_foo",
      "rfc4648_foob",
      "rfc4648_fooba",
      "rfc4648_foobar",
      "invalid_padding",
      "noncanonical_tail_bits",
      "encode_output_short",
      "decode_output_short",
    ],
  };

  console.log(JSON.stringify(result, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
