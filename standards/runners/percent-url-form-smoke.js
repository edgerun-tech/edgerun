#!/usr/bin/env node

const fs = require("fs");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/percent-url-form.wat";
const wasmPath = process.argv[3] || "/tmp/percent-url-form-smoke.wasm";

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

function callBytes(exports, name, input, outCap = 4096, mode = 0) {
  const memory = new Uint8Array(exports.memory.buffer);
  const inPtr = 1024;
  const outPtr = 4096;
  write(memory, inPtr, input);
  const packed =
    name === "percent_encode_component"
      ? exports[name](inPtr, input.length, outPtr, outCap, mode)
      : exports[name](inPtr, input.length, outPtr, outCap);
  return {
    status: status(packed),
    written: count(packed),
    output: read(memory, outPtr, count(packed)),
  };
}

function span(memory, base) {
  const view = new DataView(memory.buffer);
  return {
    a: view.getUint32(base, true),
    b: view.getUint32(base + 4, true),
    c: view.getUint32(base + 8, true),
    d: view.getUint32(base + 12, true),
    e: view.getUint32(base + 16, true),
    f: view.getUint32(base + 20, true),
  };
}

(async () => {
  execFileSync("wat2wasm", [watPath, "-o", wasmPath], { stdio: "inherit" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "inherit" });

  const module = await WebAssembly.instantiate(fs.readFileSync(wasmPath), {});
  const exports = module.instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);

  assert(exports.proto_abi_version() === 2, "unexpected ABI version");
  assert(exports.proto_standard_id() === 300027, "unexpected standard id");

  const decoded = callBytes(
    exports,
    "percent_decode_strict",
    Buffer.from("hello%20world%2Fok", "ascii"),
  );
  assert(decoded.status === 0, "percent decode failed");
  assert(decoded.output.toString("ascii") === "hello world/ok", "percent decode mismatch");

  for (const input of ["%", "%G0", "%0G"]) {
    const invalid = callBytes(exports, "percent_decode_strict", Buffer.from(input, "ascii"));
    assert(invalid.status === 3, `percent decode should reject ${input}`);
  }

  const decodeShort = callBytes(
    exports,
    "percent_decode_strict",
    Buffer.from("%20", "ascii"),
    0,
  );
  assert(decodeShort.status === 2, "percent decode should reject small output cap");

  const component = callBytes(
    exports,
    "percent_encode_component",
    Buffer.from("a b/c?d=e&f", "ascii"),
    4096,
    0,
  );
  assert(component.status === 0, "component encode failed");
  assert(
    component.output.toString("ascii") === "a%20b%2Fc%3Fd%3De%26f",
    "component encode mismatch",
  );

  const path = callBytes(
    exports,
    "percent_encode_component",
    Buffer.from("a b/c?d", "ascii"),
    4096,
    1,
  );
  assert(path.status === 0, "path encode failed");
  assert(path.output.toString("ascii") === "a%20b/c%3Fd", "path encode mismatch");

  const query = callBytes(
    exports,
    "percent_encode_component",
    Buffer.from("a b/c?d=e&f", "ascii"),
    4096,
    2,
  );
  assert(query.status === 0, "query encode failed");
  assert(query.output.toString("ascii") === "a%20b/c?d=e&f", "query encode mismatch");

  const plusLiteral = callBytes(
    exports,
    "percent_decode_strict",
    Buffer.from("two+words", "ascii"),
  );
  assert(plusLiteral.status === 0, "plus literal decode failed");
  assert(
    plusLiteral.output.toString("ascii") === "two+words",
    "percent decode should not turn plus into space",
  );

  const plusEncoded = callBytes(
    exports,
    "percent_encode_component",
    Buffer.from("a+b c", "ascii"),
    4096,
    0,
  );
  assert(plusEncoded.status === 0, "component plus encode failed");
  assert(
    plusEncoded.output.toString("ascii") === "a%2Bb%20c",
    "component encode should emit %2B for plus and %20 for space",
  );

  const encodeShort = callBytes(
    exports,
    "percent_encode_component",
    Buffer.from(" ", "ascii"),
    2,
    0,
  );
  assert(encodeShort.status === 2, "percent encode should reject small output cap");

  const baggage = callBytes(
    exports,
    "percent_encode_baggage",
    Buffer.from('name value";,=/%+?', "ascii"),
  );
  assert(baggage.status === 0, "baggage percent encode failed");
  assert(
    baggage.output.toString("ascii") === "name%20value%22%3B%2C%3D/%+?",
    "baggage percent encode mismatch",
  );

  const baggageControls = callBytes(
    exports,
    "percent_encode_baggage",
    Buffer.from([0x00, 0x1f, 0x7f, 0x80, 0xff]),
  );
  assert(baggageControls.status === 0, "baggage control encode failed");
  assert(
    baggageControls.output.toString("ascii") === "%00%1F%7F%80%FF",
    "baggage control encode mismatch",
  );

  const baggageShort = callBytes(
    exports,
    "percent_encode_baggage",
    Buffer.from(" ", "ascii"),
    2,
  );
  assert(baggageShort.status === 2, "baggage encode should reject small output cap");

  const formInput = Buffer.from("a=1&a=2&=emptykey&b=two+words&empty=&keyonly", "ascii");
  const inPtr = 8192;
  const outPtr = 12000;
  write(memory, inPtr, formInput);

  let rc = exports.form_urlencoded_next_pair(inPtr, formInput.length, 0, outPtr);
  let rec = span(memory, outPtr);
  assert(rc === 0, "first form pair failed");
  assert(rec.a === 0 && rec.b === 1 && rec.c === 2 && rec.d === 1 && rec.e === 4, "first form pair spans");
  assert(read(memory, inPtr + rec.c, rec.d).toString("ascii") === "1", "first form value");

  rc = exports.form_urlencoded_next_pair(inPtr, formInput.length, rec.e, outPtr);
  rec = span(memory, outPtr);
  assert(rc === 0, "repeated form pair failed");
  assert(read(memory, inPtr + rec.a, rec.b).toString("ascii") === "a", "repeated form key");
  assert(read(memory, inPtr + rec.c, rec.d).toString("ascii") === "2", "repeated form value");

  rc = exports.form_urlencoded_next_pair(inPtr, formInput.length, rec.e, outPtr);
  rec = span(memory, outPtr);
  assert(rc === 0, "empty-key form pair failed");
  assert(rec.b === 0, "empty-key length");
  assert(read(memory, inPtr + rec.c, rec.d).toString("ascii") === "emptykey", "empty-key form value");

  rc = exports.form_urlencoded_next_pair(inPtr, formInput.length, rec.e, outPtr);
  rec = span(memory, outPtr);
  assert(rc === 0, "plus form pair failed");
  assert(read(memory, inPtr + rec.a, rec.b).toString("ascii") === "b", "plus form key");
  assert(read(memory, inPtr + rec.c, rec.d).toString("ascii") === "two+words", "plus remains raw form value");

  rc = exports.form_urlencoded_next_pair(inPtr, formInput.length, rec.e, outPtr);
  rec = span(memory, outPtr);
  assert(rc === 0, "empty value form pair failed");
  assert(read(memory, inPtr + rec.a, rec.b).toString("ascii") === "empty", "empty form key");
  assert(rec.d === 0, "empty form value length");

  rc = exports.form_urlencoded_next_pair(inPtr, formInput.length, rec.e, outPtr);
  rec = span(memory, outPtr);
  assert(rc === 0, "key-only form pair failed");
  assert(read(memory, inPtr + rec.a, rec.b).toString("ascii") === "keyonly", "key-only form key");
  assert(rec.d === 0 && rec.e === formInput.length, "key-only form spans");

  const badForm = Buffer.from("a=bad%GG", "ascii");
  write(memory, inPtr, badForm);
  rc = exports.form_urlencoded_next_pair(inPtr, badForm.length, 0, outPtr);
  assert(rc === 3, "form scanner should reject malformed percent");

  const target = Buffer.from("/path/to?a=1&b=2#frag", "ascii");
  write(memory, inPtr, target);
  rc = exports.uri_scan_path_query(inPtr, target.length, outPtr);
  rec = span(memory, outPtr);
  assert(rc === 0, "URI path/query scanner failed");
  assert(rec.a === 0 && rec.b === 8, "URI path span");
  assert(rec.c === 9 && rec.d === 7, "URI query span");
  assert(rec.e === 17 && rec.f === 4, "URI fragment span");

  const result = {
    unit: "percent-url-form",
    abi_version: exports.proto_abi_version(),
    standard_id: exports.proto_standard_id(),
    ok: true,
    cases: [
      "percent_decode_strict",
      "percent_decode_strict_invalid",
      "percent_decode_strict_small_cap",
      "percent_encode_component",
      "percent_encode_path_mode",
      "percent_encode_query_mode",
      "percent_encode_baggage",
      "percent_encode_baggage_controls",
      "percent_encode_baggage_small_cap",
      "percent_plus_literal_policy",
      "percent_space_encode_policy",
      "percent_encode_small_cap",
      "form_urlencoded_next_pair",
      "form_urlencoded_repeated_pair",
      "form_urlencoded_empty_key",
      "form_urlencoded_plus_raw",
      "form_urlencoded_empty_value",
      "form_urlencoded_key_only",
      "form_urlencoded_malformed_percent",
      "uri_scan_path_query",
    ],
  };

  console.log(JSON.stringify(result, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
