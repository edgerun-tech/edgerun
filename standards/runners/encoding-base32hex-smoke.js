#!/usr/bin/env node

const fs = require("fs");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  "standards/build/wasm/codec-primitives/encoding-base32hex.wat";
const wasmPath = process.argv[3] || "/tmp/encoding-base32hex-smoke.wasm";

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
  assert(exports.proto_standard_id() === 300052, "unexpected standard id");

  for (const [plain, encoded] of [
    ["", ""],
    [Buffer.from([0xab]), "lc"],
    ["f", "co"],
    ["fo", "cpng"],
    ["foo", "cpnmu"],
    ["foob", "cpnmuog"],
    ["fooba", "cpnmuoj1"],
    ["foobar", "cpnmuoj1e8"],
  ]) {
    const input = Buffer.isBuffer(plain) ? plain : Buffer.from(plain, "ascii");
    const enc = callCodec(exports, "base32hex_encode", input);
    assert(enc.status === 0, `base32hex encode failed for ${encoded}`);
    assert(enc.written === encoded.length, `base32hex encode length mismatch for ${encoded}`);
    assert(enc.output.toString("ascii") === encoded, `base32hex encode mismatch for ${encoded}`);

    const dec = callCodec(exports, "base32hex_decode", Buffer.from(encoded, "ascii"));
    assert(dec.status === 0, `base32hex decode failed for ${encoded}`);
    assert(dec.written === input.length, `base32hex decode length mismatch for ${encoded}`);
    assert(dec.output.equals(input), `base32hex decode mismatch for ${encoded}`);
  }

  const upper = callCodec(exports, "base32hex_decode", Buffer.from("CPNMUOG", "ascii"));
  assert(upper.status === 0, "uppercase decode should be accepted");
  assert(upper.output.toString("ascii") === "foob", "uppercase decode mismatch");

  for (const input of ["c", "cpn", "cpnmuo", "cpnmuoj1e"]) {
    const invalid = callCodec(
      exports,
      "base32hex_decode",
      Buffer.from(input, "ascii"),
    );
    assert(invalid.status === 3, `invalid length should fail for ${input}`);
  }

  const invalidChar = callCodec(
    exports,
    "base32hex_decode",
    Buffer.from("cpnm!", "ascii"),
  );
  assert(invalidChar.status === 3, "invalid character should fail");

  const shortEncode = callCodec(
    exports,
    "base32hex_encode",
    Buffer.from("foo", "ascii"),
    4,
  );
  assert(shortEncode.status === 2, "encode should reject small output cap");

  const shortDecode = callCodec(
    exports,
    "base32hex_decode",
    Buffer.from("cpnmu", "ascii"),
    2,
  );
  assert(shortDecode.status === 2, "decode should reject small output cap");

  const result = {
    unit: "encoding-base32hex",
    abi_version: exports.proto_abi_version(),
    standard_id: exports.proto_standard_id(),
    ok: true,
    cases: [
      "empty",
      "single_byte_ab",
      "rfc4648_f",
      "rfc4648_fo",
      "rfc4648_foo",
      "rfc4648_foob",
      "rfc4648_fooba",
      "rfc4648_foobar",
      "uppercase_decode",
      "invalid_length",
      "invalid_character",
      "encode_output_short",
      "decode_output_short",
    ],
  };

  console.log(JSON.stringify(result, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
