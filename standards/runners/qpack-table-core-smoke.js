#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const root = path.resolve(__dirname, "..", "..");
const watPath =
  process.argv[2] || path.join(root, "standards/build/wasm/codec-primitives/qpack-table-core.wat");
const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "qpack-table-core-"));
const wasmPath = path.join(tmpDir, "qpack-table-core.wasm");

process.on("exit", () => {
  fs.rmSync(tmpDir, { recursive: true, force: true });
});

execFileSync("wat2wasm", [watPath, "-o", wasmPath], { stdio: "inherit" });
execFileSync("wasm-validate", [wasmPath], { stdio: "inherit" });

function unpack(value) {
  const bits = BigInt.asUintN(64, value);
  return {
    status: Number(bits & 0xffff_ffffn),
    value: Number((bits >> 32n) & 0xffff_ffffn),
  };
}

function readPrefix(view, ptr) {
  return {
    flags: view.getUint32(ptr, true),
    consumed: view.getUint32(ptr + 4, true),
    value: view.getBigUint64(ptr + 8, true),
  };
}

function readStringMeta(view, ptr) {
  return {
    flags: view.getUint32(ptr, true),
    huffman: view.getUint32(ptr + 4, true),
    consumed: view.getUint32(ptr + 8, true),
    payloadOffset: view.getUint32(ptr + 12, true),
    payloadLen: view.getUint32(ptr + 16, true),
  };
}

function readInsertPlan(view, ptr) {
  return {
    entrySize: view.getUint32(ptr, true),
    retainedSize: view.getUint32(ptr + 4, true),
    evictCount: view.getUint32(ptr + 8, true),
    inserted: view.getUint32(ptr + 12, true),
  };
}

function readResizePlan(view, ptr) {
  return {
    retainedSize: view.getUint32(ptr, true),
    evictCount: view.getUint32(ptr + 4, true),
  };
}

function readAckPlan(view, ptr) {
  return {
    newKnown: view.getUint32(ptr, true),
    acked: view.getUint32(ptr + 4, true),
    remaining: view.getUint32(ptr + 8, true),
  };
}

function writeU32s(view, ptr, values) {
  values.forEach((value, index) => view.setUint32(ptr + index * 4, value, true));
}

(async () => {
  const wasm = fs.readFileSync(wasmPath);
  const { instance } = await WebAssembly.instantiate(wasm);
  const e = instance.exports;
  const mem = new Uint8Array(e.memory.buffer);
  const view = new DataView(e.memory.buffer);

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300103);

  const ptr = 1024;
  const out = 2048;

  let packed = unpack(e.qpack_prefix_int_encode(1337n, 5, 0b010, ptr, 16));
  assert.deepEqual(packed, { status: 0, value: 3 });
  assert.deepEqual(Array.from(mem.slice(ptr, ptr + 3)), [0b0101_1111, 154, 10]);
  assert.equal(e.qpack_prefix_int_decode(ptr, 3, 5, out), 0);
  assert.deepEqual(readPrefix(view, out), { flags: 0b010, consumed: 3, value: 1337n });

  packed = unpack(e.qpack_prefix_int_encode(0x80_00_00_00_00_00_00_fen, 8, 0, ptr, 16));
  assert.deepEqual(packed, { status: 0, value: 10 });
  assert.deepEqual(
    Array.from(mem.slice(ptr, ptr + 10)),
    [255, 255, 255, 255, 255, 255, 255, 255, 255, 127],
  );
  assert.equal(e.qpack_prefix_int_decode(ptr, 10, 8, out), 0);
  assert.deepEqual(readPrefix(view, out), {
    flags: 0,
    consumed: 10,
    value: 0x80_00_00_00_00_00_00_fen,
  });

  mem.set([0x05, ...Buffer.from("hello", "ascii")], ptr);
  assert.equal(e.qpack_string_scan(8, ptr, 6, out), 0);
  assert.deepEqual(readStringMeta(view, out), {
    flags: 0,
    huffman: 0,
    consumed: 1,
    payloadOffset: 1,
    payloadLen: 5,
  });
  mem.set([0x85, 0xaa, 0xbb, 0xcc, 0xdd, 0xee], ptr);
  assert.equal(e.qpack_string_scan(8, ptr, 6, out), 0);
  assert.deepEqual(readStringMeta(view, out), {
    flags: 1,
    huffman: 1,
    consumed: 1,
    payloadOffset: 1,
    payloadLen: 5,
  });
  mem.set([0x06, 1, 2], ptr);
  assert.equal(e.qpack_string_scan(8, ptr, 3, out), 1);

  assert.deepEqual(unpack(e.qpack_table_entry_size(4, 5)), { status: 0, value: 41 });
  assert.deepEqual(unpack(e.qpack_table_entry_size(0xffff_ffff, 1)), { status: 4, value: 0 });

  const sizes = 4096;
  const tracked = 4200;
  writeU32s(view, sizes, [42, 41]);
  writeU32s(view, tracked, [0, 0]);
  assert.equal(e.qpack_table_insert_plan(95, 83, sizes, tracked, 2, 3, 4, out), 0);
  assert.deepEqual(readInsertPlan(view, out), {
    entrySize: 39,
    retainedSize: 80,
    evictCount: 1,
    inserted: 1,
  });

  writeU32s(view, tracked, [1, 0]);
  assert.equal(e.qpack_table_insert_plan(95, 83, sizes, tracked, 2, 3, 4, out), 7);
  assert.equal(e.qpack_table_insert_plan(38, 0, sizes, tracked, 2, 3, 4, out), 6);

  writeU32s(view, tracked, [0, 0]);
  assert.equal(e.qpack_table_duplicate_plan(95, 83, sizes, tracked, 2, 41, out), 0);
  assert.deepEqual(readInsertPlan(view, out), {
    entrySize: 41,
    retainedSize: 82,
    evictCount: 1,
    inserted: 1,
  });

  assert.equal(e.qpack_table_resize_plan(41, 83, sizes, tracked, 2, out), 0);
  assert.deepEqual(readResizePlan(view, out), { retainedSize: 41, evictCount: 1 });
  assert.equal(e.qpack_table_resize_plan(1_073_741_824, 83, sizes, tracked, 2, out), 5);

  assert.equal(e.qpack_blocked_max_validate(65_534), 0);
  assert.equal(e.qpack_blocked_max_validate(65_535), 8);
  assert.deepEqual(unpack(e.qpack_blocked_register(1, 2, 3, 0)), { status: 0, value: 2 });
  assert.deepEqual(unpack(e.qpack_blocked_register(2, 2, 4, 0)), { status: 9, value: 2 });
  assert.deepEqual(unpack(e.qpack_blocked_register(2, 2, 2, 3)), { status: 0, value: 2 });

  const largestRefs = 5000;
  const counts = 5100;
  writeU32s(view, largestRefs, [2, 3, 5]);
  writeU32s(view, counts, [1, 2, 1]);
  assert.equal(e.qpack_blocked_ack_plan(0, 3, largestRefs, counts, 3, out), 0);
  assert.deepEqual(readAckPlan(view, out), { newKnown: 3, acked: 3, remaining: 1 });
  assert.equal(e.qpack_blocked_ack_plan(0xffff_ffff, 1, largestRefs, counts, 3, out), 4);

  assert.equal(e.qpack_encoder_instruction_classify(0b0010_1010), 1);
  assert.equal(e.qpack_encoder_instruction_classify(0b1100_0000), 2);
  assert.equal(e.qpack_encoder_instruction_classify(0b0100_0011), 3);
  assert.equal(e.qpack_encoder_instruction_classify(0b0001_1111), 4);
  assert.equal(e.qpack_encoder_instruction_classify(0b0110_0000), 3);

  assert.equal(e.qpack_decoder_instruction_classify(0b1000_0001), 1);
  assert.equal(e.qpack_decoder_instruction_classify(0b0100_0010), 2);
  assert.equal(e.qpack_decoder_instruction_classify(0b0010_1010), 3);

  console.log(
    JSON.stringify(
      {
        unit: "qpack-table-core",
        abi_version: e.proto_abi_version(),
        standard_id: e.proto_standard_id(),
        ok: true,
      },
      null,
      2,
    ),
  );
})();
