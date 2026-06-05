const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/codec-primitives/collection-core-state.wat");
const wasm = path.join(os.tmpdir(), `collection-core-state-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm]);
const bytes = fs.readFileSync(wasm);

(async () => {
  const { instance } = await WebAssembly.instantiate(bytes);
  const e = instance.exports;
  const pack = (low, high) => (low & 0xffff) | ((high & 0xffff) << 16);

  assert.strictEqual(e.proto_abi_version(), 2);
  assert.strictEqual(e.proto_standard_id(), 300139);

  assert.strictEqual(e.coll_inline_or_heap(4, 4), 1);
  assert.strictEqual(e.coll_inline_or_heap(5, 4), 2);
  assert.strictEqual(e.coll_tinyvec_constructor(3, 4), 1);
  assert.strictEqual(e.coll_tinyvec_constructor(8, 4), 2);
  assert.strictEqual(e.coll_reserve_spills(0, 3, 4, 1), 0);
  assert.strictEqual(e.coll_reserve_spills(0, 4, 4, 1), 1);
  assert.strictEqual(e.coll_reserve_spills(1, 2, 4, 1), 1);
  assert.strictEqual(e.coll_next_power_two(9), 16);
  assert.strictEqual(e.coll_smallvec_grow_cap(5, 1, 8), 8);
  assert.strictEqual(e.coll_smallvec_grow_cap(9, 1, 4), 16);

  assert.strictEqual(e.coll_insert_result_len(3, 3), 4);
  assert.strictEqual(e.coll_insert_result_len(3, 4), -1);
  assert.strictEqual(e.coll_remove_result_len(3, 2), 2);
  assert.strictEqual(e.coll_remove_result_len(3, 3), -1);
  assert.strictEqual(e.coll_drain_result_len(10, 2, 5), 7);
  assert.strictEqual(e.coll_drain_result_len(10, 6, 5), -1);
  assert.strictEqual(e.coll_filter_result_len(10, 4), 6);
  assert.strictEqual(e.coll_filter_result_len(10, 11), -1);

  assert.strictEqual(e.coll_slab_insert_result(2, 4, 1), pack(1, 3));
  assert.strictEqual(e.coll_slab_insert_result(4, 4, 4), pack(4, 5));
  assert.strictEqual(e.coll_slab_remove_result(3, 4, 1, 1), pack(1, 2));
  assert.strictEqual(e.coll_slab_remove_result(3, 4, 5, 1), -1);
  assert.strictEqual(e.coll_slab_remove_result(3, 4, 1, 0), -1);
  assert.strictEqual(e.coll_slab_disjoint_error(0, 1, 0), 2);
  assert.strictEqual(e.coll_slab_disjoint_error(1, 0, 0), 1);
  assert.strictEqual(e.coll_slab_disjoint_error(1, 1, 1), 3);
  assert.strictEqual(e.coll_slab_disjoint_error(1, 1, 0), 0);

  assert.strictEqual(e.coll_sharded_get_result(3, 3, 0, 2), 3);
  assert.strictEqual(e.coll_sharded_get_result(2, 3, 0, 2), -1);
  assert.strictEqual(e.coll_sharded_get_result(3, 3, 1, 2), -1);
  assert.strictEqual(e.coll_sharded_mark_release(3, 3, 0, 0), 1);
  assert.strictEqual(e.coll_sharded_mark_release(3, 3, 0, 2), 2);
  assert.strictEqual(e.coll_sharded_mark_release(2, 3, 0, 0), 0);
  assert.strictEqual(e.coll_generation_advance(7, 7), 0);
  assert.strictEqual(e.coll_generation_advance(2, 7), 3);
  assert.strictEqual(e.coll_pack_key(5, 3, 8), 773);
  assert.strictEqual(e.coll_unpack_slot(773, 255), 5);
  assert.strictEqual(e.coll_unpack_generation(773, 8, 255), 3);
  assert.strictEqual(e.coll_pool_clear_len(99), 0);

  fs.unlinkSync(wasm);
  console.log("collection-core-state smoke ok");
})().catch((err) => {
  try { fs.unlinkSync(wasm); } catch (_) {}
  throw err;
});
