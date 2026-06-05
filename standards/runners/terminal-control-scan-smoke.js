#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  "standards/build/wasm/codec-primitives/terminal-control-scan.wat";

const KIND = {
  print: 1,
  execute: 2,
  esc: 3,
  csi: 4,
  osc: 5,
  dcsHook: 6,
  dcsData: 7,
  dcsEnd: 8,
  incomplete: 9,
};

function compileWat(file) {
  const wasmPath = path.join(os.tmpdir(), `edgerun-${path.basename(file)}-${process.pid}.wasm`);
  execFileSync("wat2wasm", [file, "-o", wasmPath], { stdio: "inherit" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "inherit" });
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

function write(memory, ptr, bytes) {
  memory.fill(0, ptr, ptr + bytes.length + 128);
  memory.set(bytes, ptr);
}

function record(view, base) {
  return {
    kind: view.getUint32(base, true),
    start: view.getUint32(base + 4, true),
    len: view.getUint32(base + 8, true),
    paramStart: view.getUint32(base + 12, true),
    paramLen: view.getUint32(base + 16, true),
    final: view.getUint32(base + 20, true),
    flags: view.getUint32(base + 24, true),
    status: view.getUint32(base + 28, true),
  };
}

(async () => {
  const module = await WebAssembly.instantiate(compileWat(watPath), {});
  const exports = module.instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);
  const view = new DataView(exports.memory.buffer);
  const inPtr = 1024;
  const outPtr = 8192;

  function scan(input, cap = 64) {
    const bytes = Buffer.isBuffer(input) ? input : Buffer.from(input, "utf8");
    write(memory, inPtr, bytes);
    const packed = unpack(exports.terminal_control_scan(inPtr, bytes.length, outPtr, cap));
    const records = [];
    for (let i = 0; i < packed.count; i += 1) {
      records.push(record(view, outPtr + i * 32));
    }
    return { ...packed, records, bytes };
  }

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300112);

  let result = scan("hello\tworld");
  assert.strictEqual(result.status, 0);
  assert.deepStrictEqual(result.records.map((r) => r.kind), [
    KIND.print,
    KIND.execute,
    KIND.print,
  ]);
  assert.deepStrictEqual(result.records[0], {
    kind: KIND.print,
    start: 0,
    len: 5,
    paramStart: 0,
    paramLen: 5,
    final: 0,
    flags: 0,
    status: 0,
  });
  assert.strictEqual(result.records[1].final, 9);

  result = scan("\x1b(0");
  assert.strictEqual(result.status, 0);
  assert.deepStrictEqual(result.records[0], {
    kind: KIND.esc,
    start: 0,
    len: 3,
    paramStart: 1,
    paramLen: 1,
    final: "0".charCodeAt(0),
    flags: 0,
    status: 0,
  });

  result = scan("\x1b[?25;2h");
  assert.strictEqual(result.status, 0);
  assert.strictEqual(result.records[0].kind, KIND.csi);
  assert.strictEqual(result.records[0].start, 0);
  assert.strictEqual(result.records[0].len, 8);
  assert.strictEqual(result.records[0].paramStart, 2);
  assert.strictEqual(result.records[0].paramLen, 5);
  assert.strictEqual(result.records[0].final, "h".charCodeAt(0));

  result = scan("\x1b]0;title\x07x");
  assert.strictEqual(result.status, 0);
  assert.deepStrictEqual(result.records.map((r) => r.kind), [KIND.osc, KIND.print]);
  assert.strictEqual(result.records[0].paramStart, 2);
  assert.strictEqual(result.records[0].paramLen, 7);
  assert.strictEqual(result.records[0].final, 7);
  assert.strictEqual(result.records[0].flags, 1);

  result = scan("\x1b]52;c;AAAA\x1b\\");
  assert.strictEqual(result.status, 0);
  assert.strictEqual(result.records[0].kind, KIND.osc);
  assert.strictEqual(result.records[0].final, "\\".charCodeAt(0));
  assert.strictEqual(result.records[0].flags, 2);

  result = scan("\x1bP1;2qabc\x1b\\");
  assert.strictEqual(result.status, 0);
  assert.deepStrictEqual(result.records.map((r) => r.kind), [
    KIND.dcsHook,
    KIND.dcsData,
    KIND.dcsEnd,
  ]);
  assert.strictEqual(result.records[0].paramStart, 2);
  assert.strictEqual(result.records[0].paramLen, 3);
  assert.strictEqual(result.records[0].final, "q".charCodeAt(0));
  assert.strictEqual(result.records[1].start, 6);
  assert.strictEqual(result.records[1].len, 3);

  result = scan("\x1b[31");
  assert.strictEqual(result.status, 0);
  assert.strictEqual(result.records[0].kind, KIND.incomplete);
  assert.strictEqual(result.records[0].status, 1);
  assert.strictEqual(result.records[0].flags, 1);

  result = scan("a\x1b[31mb", 1);
  assert.strictEqual(result.status, 2);
  assert.strictEqual(result.count, 1);
  assert.strictEqual(result.records[0].kind, KIND.print);

  console.log(
    JSON.stringify(
      {
        unit: "terminal-control-scan",
        abi_version: exports.proto_abi_version(),
        standard_id: exports.proto_standard_id(),
        ok: true,
        cases: [
          "printable/control",
          "esc",
          "csi-params",
          "osc-bel",
          "osc-st",
          "dcs",
          "incomplete",
          "output-cap",
        ],
      },
      null,
      2,
    ),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
