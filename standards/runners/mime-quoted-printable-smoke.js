#!/usr/bin/env node

const fs = require("fs");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  "standards/build/wasm/codec-primitives/mime-quoted-printable.wat";
const wasmPath = process.argv[3] || "/tmp/mime-quoted-printable-smoke.wasm";

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
  memory.fill(0, ptr, ptr + bytes.length + 128);
  memory.set(bytes, ptr);
}

function read(memory, ptr, len) {
  return Buffer.from(memory.slice(ptr, ptr + len));
}

function encode(exports, input, outCap = 4096) {
  const memory = new Uint8Array(exports.memory.buffer);
  const inPtr = 1024;
  const outPtr = 4096;
  write(memory, inPtr, input);
  const packed = exports.quoted_printable_encode(
    inPtr,
    input.length,
    outPtr,
    outCap,
  );
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
  assert(exports.proto_standard_id() === 300050, "unexpected standard id");

  for (const [name, input, expected] of [
    ["empty", "", ""],
    ["ascii", "Hello, World!", "Hello, World!"],
    ["equals", "a=b", "a=3Db"],
    ["nul", "Hello\u0000World", "Hello=00World"],
    ["lf", "Hello\nWorld", "Hello\nWorld"],
    ["crlf", "Hello\r\nWorld", "Hello\r\nWorld"],
    ["tab_space", "a \tb", "a \tb"],
  ]) {
    const result = encode(exports, Buffer.from(input, "binary"));
    assert(result.status === 0, `${name} encode failed`);
    assert(result.written === expected.length, `${name} length mismatch`);
    assert(result.output.toString("binary") === expected, `${name} output mismatch`);
  }

  const longLine = encode(exports, Buffer.from("A".repeat(80), "ascii"));
  assert(longLine.status === 0, "long line encode failed");
  assert(
    longLine.output.toString("ascii") === `${"A".repeat(73)}=\r\n${"A".repeat(7)}`,
    "long line soft break mismatch",
  );

  const escapedBreak = encode(
    exports,
    Buffer.concat([Buffer.from("A".repeat(72), "ascii"), Buffer.from([0])]),
  );
  assert(escapedBreak.status === 0, "escaped break encode failed");
  assert(
    escapedBreak.output.toString("ascii") === `${"A".repeat(72)}=00=\r\n`,
    "encoded byte soft break mismatch",
  );

  const shortOut = encode(exports, Buffer.from("a=b", "ascii"), 3);
  assert(shortOut.status === 2, "short output should fail with status 2");
  assert(shortOut.written === 0, "short output should report zero bytes");

  const result = {
    unit: "mime-quoted-printable",
    abi_version: exports.proto_abi_version(),
    standard_id: exports.proto_standard_id(),
    ok: true,
    cases: [
      "ascii_pass_through",
      "equals_escape",
      "nul_escape",
      "newline_preserve",
      "soft_line_break",
      "encoded_soft_line_break",
      "output_short",
    ],
  };

  console.log(JSON.stringify(result, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
