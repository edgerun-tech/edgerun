#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/crypto-bigint-limb.wat";

function compileWat(file) {
  const wasmPath = path.join(os.tmpdir(), `edgerun-${path.basename(file)}-${process.pid}.wasm`);
  execFileSync("wat2wasm", [file, "-o", wasmPath], { stdio: "inherit" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "inherit" });
  const bytes = fs.readFileSync(wasmPath);
  fs.unlinkSync(wasmPath);
  return bytes;
}

function unpack(packed) {
  const value = BigInt.asUintN(64, packed);
  return {
    status: Number(value & 0xffff_ffffn),
    written: Number((value >> 32n) & 0xffff_ffffn),
  };
}

function maskFor(limbs) {
  return (1n << (64n * BigInt(limbs))) - 1n;
}

function writeValue(view, ptr, limbs, value) {
  let v = BigInt.asUintN(64 * limbs, value);
  for (let i = 0; i < limbs; i += 1) {
    view.setBigUint64(ptr + i * 8, v & 0xffff_ffff_ffff_ffffn, true);
    v >>= 64n;
  }
}

function readValue(view, ptr, limbs) {
  let value = 0n;
  for (let i = limbs - 1; i >= 0; i -= 1) {
    value <<= 64n;
    value |= view.getBigUint64(ptr + i * 8, true);
  }
  return value;
}

(async () => {
  const { instance } = await WebAssembly.instantiate(compileWat(watPath), {});
  const e = instance.exports;
  const view = new DataView(e.memory.buffer);
  const aPtr = 1024;
  const bPtr = 2048;
  const outPtr = 4096;
  const cases = [];

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300078);

  const vectors = {
    1: [
      [0n, 0n],
      [1n, 2n],
      [0xffff_ffff_ffff_ffffn, 1n],
      [0x8000_0000_0000_0000n, 0x7fff_ffff_ffff_ffffn],
    ],
    2: [
      [0x1_0000_0000n, 0xffff_ffffn],
      [0xffff_ffff_ffff_ffff_ffff_ffff_ffff_ffffn, 1n],
      [0x1234_5678_9abc_def0_1357_9bdf_2468_ace0n, 0xfedc_ba98_7654_3210n],
    ],
    4: [
      [0x1n << 191n, 3n],
      [
        0x0123_4567_89ab_cdef_fedc_ba98_7654_3210_1111_2222_3333_4444_5555_6666_7777_8888n,
        0xffff_eeee_dddd_cccc_bbbb_aaaa_9999_8888_7777_6666_5555_4444_3333_2222_1111_0001n,
      ],
    ],
  };

  for (const limbs of [1, 2, 4]) {
    const mask = maskFor(limbs);
    for (const [a, b] of vectors[limbs]) {
      writeValue(view, aPtr, limbs, a);
      writeValue(view, bPtr, limbs, b);

      const expectedCmp = a < b ? -1 : a > b ? 1 : 0;
      const actualCmp = e.cmp(aPtr, bPtr, limbs);
      assert.equal(actualCmp === -1 || actualCmp === 0xffffffff ? -1 : actualCmp, expectedCmp);

      let packed = unpack(e.add(aPtr, bPtr, limbs, outPtr));
      assert.deepEqual(packed, { status: 0, written: limbs });
      assert.equal(readValue(view, outPtr, limbs), (a + b) & mask);
      assert.equal(e.carry(), Number((a + b) >> (64n * BigInt(limbs))));

      packed = unpack(e.sub(aPtr, bPtr, limbs, outPtr));
      assert.deepEqual(packed, { status: 0, written: limbs });
      const expectedBorrow = a < b ? 1 : 0;
      assert.equal(readValue(view, outPtr, limbs), (a - b) & mask);
      assert.equal(e.carry(), expectedBorrow);

      packed = unpack(e.mul_low(aPtr, bPtr, limbs, outPtr));
      assert.deepEqual(packed, { status: 0, written: limbs });
      assert.equal(readValue(view, outPtr, limbs), (a * b) & mask);

      cases.push(`${limbs}x${cases.length}`);
    }
  }

  assert.deepEqual(unpack(e.add(aPtr, bPtr, 0, outPtr)), { status: 1, written: 0 });
  assert.deepEqual(unpack(e.mul_low(aPtr, bPtr, 17, outPtr)), { status: 1, written: 0 });

  console.log(
    JSON.stringify({
      unit: "crypto-bigint-limb",
      abi_version: e.proto_abi_version(),
      standard_id: e.proto_standard_id(),
      ok: true,
      limb_width: 64,
      checked_limb_counts: [1, 2, 4],
      cases: cases.length,
    }),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
