#!/usr/bin/env node

const assert = require("assert");
const { execFileSync } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/codec-primitives/fs-walk-core.wat");
const wasm = path.join(os.tmpdir(), `fs-walk-core-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "pipe" });

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = instance.exports;

  assert.strictEqual(e.walk_default_flags(), 2);
  assert.strictEqual(e.walk_default_max_open(), 10);
  assert.strictEqual(e.walk_min_depth_clamp(9, 4), 4);
  assert.strictEqual(e.walk_min_depth_clamp(2, 4), 2);
  assert.strictEqual(e.walk_max_depth_clamp(1, 3), 3);
  assert.strictEqual(e.walk_max_depth_clamp(8, 3), 8);
  assert.strictEqual(e.walk_max_open_clamp(0), 1);
  assert.strictEqual(e.walk_max_open_clamp(7), 7);

  assert.strictEqual(e.walk_depth_visible(0, 0, 3), 1);
  assert.strictEqual(e.walk_depth_visible(0, 1, 3), 0);
  assert.strictEqual(e.walk_should_descend(1, 2, 3, 1), 1);
  assert.strictEqual(e.walk_should_descend(1, 3, 3, 1), 0);
  assert.strictEqual(e.walk_should_descend(0, 1, 3, 1), 0);

  assert.strictEqual(e.walk_entry_decision(1, 0, 0, 3, 0, 1), 3);
  assert.strictEqual(e.walk_entry_decision(1, 0, 0, 3, 1, 1), 4);
  assert.strictEqual(e.walk_entry_decision(1, 0, 1, 3, 0, 1), 2);
  assert.strictEqual(e.walk_entry_decision(0, 2, 0, 3, 0, 1), 1);
  assert.strictEqual(e.walk_entry_decision(3, 2, 0, 3, 0, 1), 5);

  assert.strictEqual(e.walk_symlink_kind(1, 1, 1, 0, 1), 1);
  assert.strictEqual(e.walk_symlink_kind(0, 1, 1, 0, 1), 2);
  assert.strictEqual(e.walk_symlink_kind(0, 1, 1, 1, 0), 1);
  assert.strictEqual(e.walk_symlink_kind(0, 0, 1, 0, 0), 1);
  assert.strictEqual(e.walk_loop_error(1, 1), 1);
  assert.strictEqual(e.walk_loop_error(0, 1), 0);

  assert.strictEqual(e.walk_same_file_system_descend(0, 1n, 2n), 1);
  assert.strictEqual(e.walk_same_file_system_descend(1, 5n, 5n), 1);
  assert.strictEqual(e.walk_same_file_system_descend(1, 5n, 6n), 0);
  assert.strictEqual(e.walk_fd_spill(10, 10), 1);
  assert.strictEqual(e.walk_fd_spill(9, 10), 0);
  assert.strictEqual(e.walk_filter_skip_descendants(1, 0, 0), 1);
  assert.strictEqual(e.walk_filter_skip_descendants(1, 0, 1), 0);

  assert.strictEqual(e.same_file_unix_equal(1n, 2n, 1n, 2n), 1);
  assert.strictEqual(e.same_file_unix_equal(1n, 2n, 1n, 3n), 0);
  assert.strictEqual(e.same_file_std_drop_closes(0), 1);
  assert.strictEqual(e.same_file_std_drop_closes(1), 0);

  assert.strictEqual(e.tempfile_retry_allowed(0, 1), 1);
  assert.strictEqual(e.tempfile_retry_allowed(128, 1), 0);
  assert.strictEqual(e.tempfile_terminal_status(0, 1, 0), 1);
  assert.strictEqual(e.tempfile_terminal_status(0, 0, 1), 0);
  assert.strictEqual(e.tempfile_terminal_status(127, 0, 1), 2);
  assert.strictEqual(e.tempfile_terminal_status(0, 0, 0), 3);
  assert.strictEqual(e.tempfile_drop_action(0), 1);
  assert.strictEqual(e.tempfile_drop_action(1), 2);

  console.log("fs walk core smoke passed");
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
