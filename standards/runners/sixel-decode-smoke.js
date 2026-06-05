#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath = process.argv[2] || "standards/build/wasm/codec-primitives/sixel-decode.wat";

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

(async () => {
  const module = await WebAssembly.instantiate(compileWat(watPath), {});
  const exports = module.instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);
  const view = new DataView(exports.memory.buffer);
  const inPtr = 1024;
  const outPtr = 8192;
  const metaPtr = 4096;
  const cases = [];

  function decode(name, text, outCap = 65536, zeroColor = 0, gridSize = 0) {
    const input = Buffer.from(text, "ascii");
    memory.fill(0, inPtr, inPtr + input.length + 64);
    memory.fill(0xaa, outPtr, outPtr + Math.min(outCap, 4096));
    memory.fill(0, metaPtr, metaPtr + 12);
    memory.set(input, inPtr);
    const packed = unpack(
      exports.sixel_decode_rgba(inPtr, input.length, zeroColor, gridSize, outPtr, outCap, metaPtr),
    );
    const result = {
      name,
      status: packed.status,
      written: packed.value,
      width: view.getUint32(metaPtr, true),
      height: view.getUint32(metaPtr + 4, true),
      required: view.getUint32(metaPtr + 8, true),
    };
    cases.push(result);
    return result;
  }

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300071);

  let basic = decode("basic", '"1;1;2;2#0;2;0;0;0#0~~');
  assert.strictEqual(basic.status, 0);
  assert.strictEqual(basic.width, 2);
  assert.ok(basic.height >= 6);
  assert.strictEqual(basic.written, basic.width * basic.height * 4);
  assert.deepStrictEqual(Array.from(memory.slice(outPtr, outPtr + 4)), [0, 0, 0, 255]);
  assert.deepStrictEqual(Array.from(memory.slice(outPtr + 4, outPtr + 8)), [0, 0, 0, 255]);

  let repeat = decode("repeat", "#0!5~");
  assert.strictEqual(repeat.status, 0);
  assert.strictEqual(repeat.width, 5);
  assert.strictEqual(repeat.height, 6);
  assert.strictEqual(repeat.written, 5 * 6 * 4);

  let move = decode("movement", "~-~");
  assert.strictEqual(move.status, 0);
  assert.strictEqual(move.width, 1);
  assert.strictEqual(move.height, 12);

  let color = decode("color", "#1;2;100;0;0#1@");
  assert.strictEqual(color.status, 0);
  assert.deepStrictEqual(Array.from(memory.slice(outPtr, outPtr + 4)), [255, 0, 0, 255]);

  let colorOrder = decode("color-order", "#1@#1;2;100;0;0#1@");
  assert.strictEqual(colorOrder.status, 0);
  assert.deepStrictEqual(Array.from(memory.slice(outPtr, outPtr + 4)), [51, 51, 204, 255]);
  assert.deepStrictEqual(Array.from(memory.slice(outPtr + 4, outPtr + 8)), [255, 0, 0, 255]);

  let empty = decode("empty", "");
  assert.strictEqual(empty.status, 1);

  for (const input of ["!", "!12", "!x~"]) {
    const invalidRepeat = decode(`invalid-repeat:${input}`, input);
    assert.strictEqual(invalidRepeat.status, 3);
  }

  for (const input of ["#", "#x", "#1;", "#1;3;10;20;30"]) {
    const invalidColor = decode(`invalid-color:${input}`, input);
    assert.strictEqual(invalidColor.status, 3);
  }

  let tooSmall = decode("output-cap", "#0!5~", 5);
  assert.strictEqual(tooSmall.status, 2);
  assert.strictEqual(tooSmall.required, 5 * 6 * 4);

  console.log(
    JSON.stringify(
      {
        unit: "sixel-decode",
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
