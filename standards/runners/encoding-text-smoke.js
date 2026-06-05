#!/usr/bin/env node

const fs = require("fs");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/encoding-text.wat";
const wasmPath = process.argv[3] || "/tmp/encoding-text-smoke.wasm";

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

function callText(exports, name, input, outCap = 4096) {
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
  assert(exports.proto_standard_id() === 300002, "unexpected standard id");

  const hex = callText(
    exports,
    "hex_encode_lower",
    Buffer.from([0x01, 0x02, 0xab, 0xcd]),
  );
  assert(hex.status === 0, "hex encode failed");
  assert(hex.output.toString("ascii") === "0102abcd", "hex encode mismatch");

  const shortHexEncode = callText(
    exports,
    "hex_encode_lower",
    Buffer.from([0xff]),
    1,
  );
  assert(shortHexEncode.status === 2, "hex encode should reject small output cap");

  const oddHex = callText(exports, "hex_decode_strict", Buffer.from("0", "ascii"));
  assert(oddHex.status === 3, "odd hex length should fail");

  const invalidHex = callText(exports, "hex_decode_strict", Buffer.from("0g", "ascii"));
  assert(invalidHex.status === 3, "invalid hex character should fail");

  const decodedHex = callText(
    exports,
    "hex_decode_strict",
    Buffer.from("0102abcd", "ascii"),
  );
  assert(decodedHex.status === 0, "hex decode failed");
  assert(decodedHex.output.equals(Buffer.from([0x01, 0x02, 0xab, 0xcd])), "hex decode mismatch");

  const shortHexDecode = callText(
    exports,
    "hex_decode_strict",
    Buffer.from("ff", "ascii"),
    0,
  );
  assert(shortHexDecode.status === 2, "hex decode should reject small output cap");

  for (const [plain, encoded] of [
    ["foo", "Zm9v"],
    ["foob", "Zm9vYg"],
  ]) {
    const enc = callText(exports, "base64url_nopad_encode", Buffer.from(plain, "ascii"));
    assert(enc.status === 0, `base64url encode failed for ${plain}`);
    assert(enc.output.toString("ascii") === encoded, `base64url encode mismatch for ${plain}`);

    const dec = callText(exports, "base64url_nopad_decode", Buffer.from(encoded, "ascii"));
    assert(dec.status === 0, `base64url decode failed for ${encoded}`);
    assert(dec.output.toString("ascii") === plain, `base64url decode mismatch for ${encoded}`);
  }

  const shortBase64Encode = callText(
    exports,
    "base64url_nopad_encode",
    Buffer.from("foo", "ascii"),
    3,
  );
  assert(shortBase64Encode.status === 2, "base64url encode should reject small output cap");

  const shortBase64Decode = callText(
    exports,
    "base64url_nopad_decode",
    Buffer.from("Zm9v", "ascii"),
    2,
  );
  assert(shortBase64Decode.status === 2, "base64url decode should reject small output cap");

  for (const input of ["*", "Zm=v", "A"]) {
    const invalid = callText(
      exports,
      "base64url_nopad_decode",
      Buffer.from(input, "ascii"),
    );
    assert(invalid.status === 3, `base64url invalid input should fail for ${input}`);
  }

  for (const input of ["AB", "AAB"]) {
    const noncanonical = callText(
      exports,
      "base64url_nopad_decode",
      Buffer.from(input, "ascii"),
    );
    assert(noncanonical.status === 3, `base64url noncanonical tail should fail for ${input}`);
  }

  for (const input of ["AA", "AAA"]) {
    const canonical = callText(
      exports,
      "base64url_nopad_decode",
      Buffer.from(input, "ascii"),
    );
    assert(canonical.status === 0, `base64url canonical tail should pass for ${input}`);
  }

  const result = {
    unit: "encoding-text",
    abi_version: exports.proto_abi_version(),
    standard_id: exports.proto_standard_id(),
    ok: true,
    cases: [
      "hex_encode_lower",
      "hex_encode_lower_small_cap",
      "hex_decode_strict_odd_length",
      "hex_decode_strict_invalid",
      "hex_decode_strict",
      "hex_decode_strict_small_cap",
      "base64url_nopad_foo",
      "base64url_nopad_foob",
      "base64url_nopad_encode_small_cap",
      "base64url_nopad_decode_small_cap",
      "base64url_nopad_invalid_input",
      "base64url_nopad_noncanonical_tail",
    ],
  };

  console.log(JSON.stringify(result, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
