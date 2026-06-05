#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath = process.argv[2] || "standards/build/wasm/codec-primitives/hpack-table-core.wat";

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
  const sizesPtr = 1024;
  const outPtr = 2048;

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300044);

  function writeSizes(sizes) {
    memory.fill(0, sizesPtr, sizesPtr + 256);
    sizes.forEach((size, i) => view.setUint32(sizesPtr + i * 4, size, true));
  }

  function insertPlan(max, current, oldestSizes, nameLen, valueLen) {
    writeSizes(oldestSizes);
    memory.fill(0, outPtr, outPtr + 32);
    const status = exports.hpack_table_insert_plan(
      max,
      current,
      sizesPtr,
      oldestSizes.length,
      nameLen,
      valueLen,
      outPtr,
    );
    return {
      status,
      entrySize: view.getUint32(outPtr, true),
      retainedSize: view.getUint32(outPtr + 4, true),
      evictExistingCount: view.getUint32(outPtr + 8, true),
      inserted: view.getUint32(outPtr + 12, true),
    };
  }

  function resizePlan(max, current, oldestSizes) {
    writeSizes(oldestSizes);
    memory.fill(0, outPtr, outPtr + 32);
    const status = exports.hpack_table_resize_plan(max, current, sizesPtr, oldestSizes.length, outPtr);
    return {
      status,
      retainedSize: view.getUint32(outPtr, true),
      evictExistingCount: view.getUint32(outPtr + 4, true),
    };
  }

  assert.deepStrictEqual(unpack(exports.hpack_table_entry_size(1, 1)), {
    status: 0,
    value: 34,
  });

  assert.deepStrictEqual(insertPlan(4096, 0, [], 1, 1), {
    status: 0,
    entrySize: 34,
    retainedSize: 34,
    evictExistingCount: 0,
    inserted: 1,
  });

  assert.deepStrictEqual(insertPlan(38, 34, [34], 3, 3), {
    status: 0,
    entrySize: 38,
    retainedSize: 38,
    evictExistingCount: 1,
    inserted: 1,
  });

  assert.deepStrictEqual(insertPlan(38, 34, [34], 3, 4), {
    status: 0,
    entrySize: 39,
    retainedSize: 0,
    evictExistingCount: 1,
    inserted: 0,
  });

  assert.deepStrictEqual(resizePlan(38, 106, [34, 38, 34]), {
    status: 0,
    retainedSize: 34,
    evictExistingCount: 2,
  });

  assert.deepStrictEqual(resizePlan(0, 106, [34, 38, 34]), {
    status: 0,
    retainedSize: 0,
    evictExistingCount: 3,
  });

  assert.deepStrictEqual(insertPlan(0, 0, [], 1, 1), {
    status: 0,
    entrySize: 34,
    retainedSize: 0,
    evictExistingCount: 0,
    inserted: 0,
  });

  assert.strictEqual(insertPlan(38, 106, [34], 1, 1).status, 3);
  assert.strictEqual(resizePlan(38, 106, [34]).status, 3);
  assert.strictEqual(unpack(exports.hpack_table_entry_size(0xffffffff, 1)).status, 4);

  console.log(JSON.stringify({ ok: true, standard_id: exports.proto_standard_id() }));
})();
