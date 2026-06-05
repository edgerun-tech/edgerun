const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/codec-primitives/bytes-bump-core.wat");
const wasm = path.join(os.tmpdir(), `bytes-bump-core-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm]);
const bytes = fs.readFileSync(wasm);

(async () => {
  const { instance } = await WebAssembly.instantiate(bytes);
  const e = instance.exports;

  assert.strictEqual(e.proto_abi_version(), 2);
  assert.strictEqual(e.proto_standard_id(), 300135);

  assert.strictEqual(e.bytes_buf_advance_remaining(11, 6), 5);
  assert.strictEqual(e.bytes_buf_advance_remaining(3, 4), -1);
  assert.strictEqual(e.bytes_buf_checked_read_remaining(8, 8), 0);
  assert.strictEqual(e.bytes_buf_checked_read_remaining(7, 8), -1);
  assert.strictEqual(e.bytes_buf_copy_first_chunk(11, 8, 5), 5);
  assert.strictEqual(e.bytes_buf_copy_first_chunk(4, 8, 4), -1);

  assert.strictEqual(e.bytes_mut_split_to_field(11, 64, 5, 1), 6);
  assert.strictEqual(e.bytes_mut_split_to_field(11, 64, 5, 2), 59);
  assert.strictEqual(e.bytes_mut_split_to_field(11, 64, 5, 3), 5);
  assert.strictEqual(e.bytes_mut_split_to_field(11, 64, 12, 0), -1);

  assert.strictEqual(e.bytes_mut_split_off_field(11, 64, 5, 1), 5);
  assert.strictEqual(e.bytes_mut_split_off_field(11, 64, 5, 3), 6);
  assert.strictEqual(e.bytes_mut_split_off_field(11, 64, 5, 4), 59);
  assert.strictEqual(e.bytes_mut_split_off_field(11, 64, 65, 0), -1);

  assert.strictEqual(e.bytes_mut_reserve_action(1, 8, 16, 0, 4, 1, 16, 1), 0);
  assert.strictEqual(e.bytes_mut_reserve_action(1, 8, 16, 8, 16, 1, 16, 1), 1);
  assert.strictEqual(e.bytes_mut_reserve_action(1, 12, 16, 2, 16, 1, 16, 1), 2);
  assert.strictEqual(e.bytes_mut_reserve_action(0, 8, 16, 8, 24, 1, 64, 1), 1);
  assert.strictEqual(e.bytes_mut_reserve_action(0, 8, 16, 1, 64, 0, 64, 1), 3);
  assert.strictEqual(e.bytes_mut_reserve_action(0, 8, 16, 1, 64, 0, 64, 0), 4);

  assert.strictEqual(e.bytes_mut_unsplit_field(6, 16, 5, 48, 1, 1, 1), 11);
  assert.strictEqual(e.bytes_mut_unsplit_field(6, 16, 5, 48, 1, 1, 2), 64);
  assert.strictEqual(e.bytes_mut_unsplit_field(6, 16, 5, 48, 0, 1, 0), -1);

  assert.strictEqual(e.bump_fast_alloc_field(128, 5, 4, 8, 0, 0), 0);
  assert.strictEqual(e.bump_fast_alloc_field(128, 5, 4, 8, 0, 1), 120);
  assert.strictEqual(e.bump_fast_alloc_field(128, 5, 4, 8, 0, 2), 8);
  assert.strictEqual(e.bump_fast_alloc_field(128, 5, 32, 8, 16, 1), 80);
  assert.strictEqual(e.bump_fast_alloc_field(12, 8, 8, 8, 0, 0), 0);
  assert.strictEqual(e.bump_fast_alloc_field(7, 8, 8, 8, 0, 0), -1);

  assert.strictEqual(e.bump_new_chunk_field(0, 16, 8, 1, 1), 448);
  assert.strictEqual(e.bump_new_chunk_field(0, 600, 8, 1, 1), 960);
  assert.strictEqual(e.bump_new_chunk_field(5000, 8, 32, 8, 2), 32);
  assert.strictEqual(e.bump_limit_fits(0, 0, 8192), 1);
  assert.strictEqual(e.bump_limit_fits(1, 1024, 960), 1);
  assert.strictEqual(e.bump_limit_fits(1, 512, 960), 0);

  assert.strictEqual(e.bump_allocation_path(128, 16, 8, 8, 0, 0, 0, 448), 1);
  assert.strictEqual(e.bump_allocation_path(4, 64, 8, 8, 0, 0, 0, 448), 2);
  assert.strictEqual(e.bump_allocation_path(4, 64, 8, 8, 0, 1, 128, 448), -1);
  assert.strictEqual(e.bump_reset_field(1, 960, 1), 1);
  assert.strictEqual(e.bump_reset_field(1, 960, 2), 960);
  assert.strictEqual(e.bump_reset_field(0, 0, 1), 0);

  fs.unlinkSync(wasm);
  console.log("bytes-bump-core smoke ok");
})().catch((err) => {
  try { fs.unlinkSync(wasm); } catch (_) {}
  throw err;
});
