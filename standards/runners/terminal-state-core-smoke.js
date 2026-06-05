#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  "standards/build/wasm/codec-primitives/terminal-state-core.wat";

function compileWat(file) {
  const wasmPath = path.join(os.tmpdir(), `edgerun-${path.basename(file)}-${process.pid}.wasm`);
  execFileSync("wat2wasm", [file, "-o", wasmPath], { stdio: "inherit" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "inherit" });
  const bytes = fs.readFileSync(wasmPath);
  fs.unlinkSync(wasmPath);
  return bytes;
}

function unpackCount(packed) {
  return {
    status: Number(packed & 0xffffffffn),
    count: Number((packed >> 32n) & 0xffffffffn),
  };
}

function unpackCursor(packed) {
  return {
    col: Number(packed & 0xffffffffn),
    row: Number((packed >> 32n) & 0xffffffffn),
  };
}

function write(memory, ptr, text) {
  const bytes = Buffer.from(text, "ascii");
  memory.fill(0, ptr, ptr + bytes.length + 64);
  memory.set(bytes, ptr);
  return bytes.length;
}

function paramRecord(view, base) {
  return {
    value: view.getUint32(base, true),
    flags: view.getUint32(base + 4, true),
  };
}

(async () => {
  const module = await WebAssembly.instantiate(compileWat(watPath), {});
  const exports = module.instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);
  const view = new DataView(exports.memory.buffer);
  const inPtr = 1024;
  const outPtr = 8192;

  function parseCsi(body, cap = 16) {
    const len = write(memory, inPtr, body);
    const packed = unpackCount(exports.terminal_csi_parse_params(inPtr, len, outPtr, cap));
    const params = [];
    for (let i = 0; i < packed.count; i += 1) {
      params.push(paramRecord(view, outPtr + i * 8));
    }
    return { ...packed, params, final: exports.terminal_csi_final(inPtr, len) };
  }

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300098);

  assert.strictEqual(exports.terminal_classify_byte("A".charCodeAt(0)), 1);
  assert.strictEqual(exports.terminal_classify_byte(0x20), 1);
  assert.strictEqual(exports.terminal_classify_byte(0x80), 1);
  assert.strictEqual(exports.terminal_classify_byte(0x1b), 2);
  assert.strictEqual(exports.terminal_classify_byte(0x7f), 2);
  assert.strictEqual(exports.terminal_control_action(0x0a), 1);
  assert.strictEqual(exports.terminal_control_action(0x0d), 2);
  assert.strictEqual(exports.terminal_control_action(0x08), 3);
  assert.strictEqual(exports.terminal_control_action(0x7f), 4);
  assert.strictEqual(exports.terminal_control_action(0x09), 5);
  assert.strictEqual(exports.terminal_control_action(0x1b), 0);

  let result = parseCsi("?25;2h");
  assert.strictEqual(result.status, 0);
  assert.strictEqual(result.final, "h".charCodeAt(0));
  assert.deepStrictEqual(result.params, [
    { value: 25, flags: 3 },
    { value: 2, flags: 3 },
  ]);

  result = parseCsi("38:2:221:221:221m");
  assert.strictEqual(result.status, 0);
  assert.strictEqual(result.final, "m".charCodeAt(0));
  assert.deepStrictEqual(result.params, [
    { value: 38, flags: 5 },
    { value: 2, flags: 5 },
    { value: 221, flags: 5 },
    { value: 221, flags: 5 },
    { value: 221, flags: 1 },
  ]);

  result = parseCsi("31");
  assert.strictEqual(result.status, 1);
  assert.strictEqual(result.count, 0);
  assert.strictEqual(result.final, 0);

  result = parseCsi("1;2;3m", 2);
  assert.strictEqual(result.status, 2);
  assert.strictEqual(result.count, 2);

  assert.deepStrictEqual(
    unpackCursor(exports.terminal_cursor_set(80, 24, 5, 70, 2, 20, 0, 90, 99)),
    { col: 70, row: 23 },
  );
  assert.deepStrictEqual(
    unpackCursor(exports.terminal_cursor_set(80, 24, 5, 70, 2, 20, 1, 0, 99)),
    { col: 5, row: 20 },
  );
  assert.deepStrictEqual(
    unpackCursor(exports.terminal_cursor_move(80, 24, 5, 70, 2, 20, 1, 10, 3, 0, 9)),
    { col: 10, row: 2 },
  );
  assert.deepStrictEqual(
    unpackCursor(exports.terminal_cursor_move(80, 24, 5, 70, 2, 20, 0, 68, 9, 2, 99)),
    { col: 70, row: 9 },
  );
  assert.deepStrictEqual(
    unpackCursor(exports.terminal_cursor_move(80, 24, 5, 70, 2, 20, 0, 6, 9, 3, 99)),
    { col: 5, row: 9 },
  );

  let attrs = 0;
  for (const param of [1, 2, 5, 3, 4, 9, 7, 53, 8]) {
    attrs = exports.terminal_sgr_apply(attrs, param);
  }
  assert.strictEqual(attrs, 0x1ff);
  attrs = exports.terminal_sgr_apply(attrs, 22);
  attrs = exports.terminal_sgr_apply(attrs, 25);
  attrs = exports.terminal_sgr_apply(attrs, 23);
  attrs = exports.terminal_sgr_apply(attrs, 24);
  attrs = exports.terminal_sgr_apply(attrs, 29);
  attrs = exports.terminal_sgr_apply(attrs, 27);
  attrs = exports.terminal_sgr_apply(attrs, 55);
  attrs = exports.terminal_sgr_apply(attrs, 28);
  assert.strictEqual(attrs, 0);

  assert.strictEqual(exports.terminal_grid_index(80, 24, 7, 3), 247);
  assert.strictEqual(exports.terminal_grid_index(80, 24, 80, 3), -1);
  assert.deepStrictEqual(
    unpackCursor(exports.terminal_grid_pixel_origin(10, 20, 9, 18, 7, 3)),
    { col: 73, row: 74 },
  );

  console.log(
    JSON.stringify(
      {
        unit: "terminal-state-core",
        abi_version: exports.proto_abi_version(),
        standard_id: exports.proto_standard_id(),
        ok: true,
        cases: [
          "printable-control-classification",
          "control-actions",
          "csi-params-status",
          "cursor-clamp-move",
          "sgr-attr-packing",
          "grid-coordinate-addressing",
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
