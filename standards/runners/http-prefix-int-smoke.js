#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath = process.argv[2] || "standards/build/wasm/codec-primitives/http-prefix-int.wat";

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

function readDecoded(view, ptr) {
  return {
    flags: view.getUint32(ptr, true),
    consumed: view.getUint32(ptr + 4, true),
    value: view.getBigUint64(ptr + 8, true),
  };
}

(async () => {
  const module = await WebAssembly.instantiate(compileWat(watPath), {});
  const exports = module.instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);
  const view = new DataView(exports.memory.buffer);
  const ptr = 1024;
  const outPtr = 2048;

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300011);

  let packed = unpack(exports.hpack_prefix_int_encode(10n, 5, 0b101, ptr, 16));
  assert.deepStrictEqual(packed, { status: 0, value: 1 });
  assert.deepStrictEqual(Array.from(memory.slice(ptr, ptr + 1)), [0b1010_1010]);
  assert.strictEqual(exports.hpack_prefix_int_decode(ptr, 1, 5, outPtr), 0);
  assert.deepStrictEqual(readDecoded(view, outPtr), { flags: 0b101, consumed: 1, value: 10n });

  packed = unpack(exports.hpack_prefix_int_encode(1337n, 5, 0b010, ptr, 16));
  assert.deepStrictEqual(packed, { status: 0, value: 3 });
  assert.deepStrictEqual(Array.from(memory.slice(ptr, ptr + 3)), [0b0101_1111, 154, 10]);
  assert.strictEqual(exports.hpack_prefix_int_decode(ptr, 3, 5, outPtr), 0);
  assert.deepStrictEqual(readDecoded(view, outPtr), { flags: 0b010, consumed: 3, value: 1337n });

  packed = unpack(exports.hpack_prefix_int_encode(31n, 5, 0b010, ptr, 16));
  assert.deepStrictEqual(packed, { status: 0, value: 2 });
  assert.deepStrictEqual(Array.from(memory.slice(ptr, ptr + 2)), [0b0101_1111, 0]);
  assert.strictEqual(exports.hpack_prefix_int_decode(ptr, 2, 5, outPtr), 0);
  assert.deepStrictEqual(readDecoded(view, outPtr), { flags: 0b010, consumed: 2, value: 31n });

  packed = unpack(exports.hpack_prefix_int_encode(42n, 8, 0, ptr, 16));
  assert.deepStrictEqual(packed, { status: 0, value: 1 });
  assert.deepStrictEqual(Array.from(memory.slice(ptr, ptr + 1)), [42]);
  assert.strictEqual(exports.hpack_prefix_int_decode(ptr, 1, 8, outPtr), 0);
  assert.deepStrictEqual(readDecoded(view, outPtr), { flags: 0, consumed: 1, value: 42n });

  packed = unpack(exports.qpack_prefix_int_encode(424242n, 8, 0, ptr, 16));
  assert.deepStrictEqual(packed, { status: 0, value: 4 });
  assert.deepStrictEqual(Array.from(memory.slice(ptr, ptr + 4)), [255, 179, 240, 25]);
  assert.strictEqual(exports.qpack_prefix_int_decode(ptr, 4, 8, outPtr), 0);
  assert.deepStrictEqual(readDecoded(view, outPtr), { flags: 0, consumed: 4, value: 424242n });

  packed = unpack(exports.qpack_prefix_int_encode(0x80_00_00_00_00_00_00_fen, 8, 0, ptr, 16));
  assert.deepStrictEqual(packed, { status: 0, value: 10 });
  assert.deepStrictEqual(
    Array.from(memory.slice(ptr, ptr + 10)),
    [255, 255, 255, 255, 255, 255, 255, 255, 255, 127],
  );
  assert.strictEqual(exports.qpack_prefix_int_decode(ptr, 10, 8, outPtr), 0);
  assert.deepStrictEqual(readDecoded(view, outPtr), {
    flags: 0,
    consumed: 10,
    value: 0x80_00_00_00_00_00_00_fen,
  });

  assert.deepStrictEqual(unpack(exports.hpack_prefix_int_encode(1337n, 5, 0b010, ptr, 2)), {
    status: 2,
    value: 2,
  });
  assert.deepStrictEqual(unpack(exports.hpack_prefix_int_encode(0n, 5, 0b1000, ptr, 16)), {
    status: 3,
    value: 0,
  });
  assert.deepStrictEqual(unpack(exports.hpack_prefix_int_encode(0n, 0, 0, ptr, 16)), {
    status: 3,
    value: 0,
  });
  assert.strictEqual(exports.hpack_prefix_int_decode(ptr, 0, 5, outPtr), 1);
  assert.strictEqual(exports.hpack_prefix_int_decode(ptr, 1, 0, outPtr), 3);

  memory.set([0b0101_1111, 0x80], ptr);
  assert.strictEqual(exports.hpack_prefix_int_decode(ptr, 2, 5, outPtr), 1);
  memory.set([0b0101_1111, 0x80, 0x80, 0x80, 0x80], ptr);
  assert.strictEqual(exports.hpack_prefix_int_decode(ptr, 5, 5, outPtr), 4);
  memory.set([255, 128, 254, 255, 255, 255, 255, 255, 255, 255, 255, 1], ptr);
  assert.strictEqual(exports.qpack_prefix_int_decode(ptr, 12, 8, outPtr), 4);

  packed = unpack(exports.qpack_prefix_int_encode(143n, 4, 0b0001, ptr, 16));
  assert.deepStrictEqual(packed, { status: 0, value: 3 });
  assert.deepStrictEqual(Array.from(memory.slice(ptr, ptr + 3)), [31, 128, 1]);
  assert.strictEqual(exports.qpack_prefix_int_decode(ptr, 3, 4, outPtr), 0);
  assert.deepStrictEqual(readDecoded(view, outPtr), { flags: 0b0001, consumed: 3, value: 143n });

  console.log(
    JSON.stringify(
      {
        unit: "http-prefix-int",
        abi_version: exports.proto_abi_version(),
        standard_id: exports.proto_standard_id(),
        ok: true,
      },
      null,
      2,
    ),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
