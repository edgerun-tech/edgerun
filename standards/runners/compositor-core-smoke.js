#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/compositor-core.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  const wasm = execFileSync("wat2wasm", [file, "-o", "-"]);
  execFileSync("wasm-validate", ["-"], { input: wasm });
  return wasm;
}

function writeAscii(memory, value, ptr) {
  memory.fill(0, ptr, ptr + Math.max(128, value.length + 1));
  memory.set(Buffer.from(value, "ascii"), ptr);
  return ptr;
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const e = instance.exports;
  const memory = new Uint8Array(e.memory.buffer);
  const str = (fn, value) => {
    const ptr = writeAscii(memory, value, 4096);
    return e[fn](ptr, value.length);
  };

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300130);

  assert.equal(str("compositor_global_code", "wl_compositor"), 1);
  assert.equal(str("compositor_global_code", "wl_shm"), 2);
  assert.equal(str("compositor_global_code", "xdg_wm_base"), 4);
  assert.equal(str("compositor_global_code", "zwp_linux_dmabuf_v1"), 6);
  assert.equal(str("compositor_global_code", "wp_linux_drm_syncobj"), 21);
  assert.equal(str("compositor_global_code", "zwlr_layer_shell_v1"), 33);

  assert.equal(e.compositor_surface_commit_result(0, 0), 0);
  assert.equal(e.compositor_surface_commit_result(1, 1), 1);
  assert.equal(e.compositor_surface_commit_result(4, 0), 2);
  assert.equal(e.compositor_surface_commit_result(1, 0), 3);
  assert.equal(e.compositor_buffer_layout_len(0, 10, 10, 40), 400);
  assert.equal(e.compositor_buffer_layout_len(0, 10, 10, 4), -1);

  assert.equal(e.compositor_logical_width(200, 100, 0, 2, -1), 100);
  assert.equal(e.compositor_logical_height(200, 100, 0, 2, -1), 50);
  assert.equal(e.compositor_logical_width(200, 100, 1, 2, -1), 50);
  assert.equal(e.compositor_logical_height(200, 100, 1, 2, -1), 100);
  assert.equal(e.compositor_logical_width(200, 100, 0, 2, 80), 80);

  assert.equal(e.compositor_sample_coord_x(0, 0, 0, 0, 2, 2, 2), 1);
  assert.equal(e.compositor_sample_coord_y(0, 0, 0, 0, 2, 2, 2), 1);
  assert.equal(e.compositor_sample_coord_x(1, 0, 10, 20, 4, 3, 1), 13);
  assert.equal(e.compositor_sample_coord_y(1, 0, 10, 20, 4, 3, 1), 21);

  assert.equal(e.compositor_render_phase(2, 0), 1);
  assert.equal(e.compositor_render_phase(1, 0), 2);
  assert.equal(e.compositor_render_phase(2, 3), 3);
  assert.equal(e.compositor_render_phase(3, 0), 4);
  assert.equal(e.compositor_render_phase(4, 0), 5);

  assert.equal(e.compositor_pixel_format_code(0x34325258), 1);
  assert.equal(e.compositor_pixel_format_code(0x34325241), 2);
  assert.equal(e.compositor_pixel_format_code(0x34324241), 3);
  assert.equal(e.compositor_pixel_format_code(0), 1);
  assert.equal(e.compositor_blend_channel(255, 0, 128), 128);
  assert.equal(e.compositor_blend_channel(64, 10, 0), 10);
  assert.equal(e.compositor_blend_channel(64, 10, 255), 64);

  assert.equal(e.compositor_toplevel_state_mask(0, 0, 0), 1);
  assert.equal(e.compositor_toplevel_state_mask(1, 0, 0), 2);
  assert.equal(e.compositor_toplevel_state_mask(0, 1, 1), 12);

  assert.equal(e.compositor_layer_width(1920, 12, 0, 10, 20), 1890);
  assert.equal(e.compositor_layer_height(1080, 3, 0, 5, 15), 1060);
  assert.equal(e.compositor_layer_x(1920, 800, 4, 10, 20), 10);
  assert.equal(e.compositor_layer_x(1920, 800, 8, 10, 20), 1100);
  assert.equal(e.compositor_layer_y(1080, 100, 0, 5, 15), 490);

  assert.equal(e.compositor_surface_accepts_point(1, 5, 5, 10, 10, 0, 0, 0, 0, 0), 1);
  assert.equal(e.compositor_surface_accepts_point(1, 11, 5, 10, 10, 0, 0, 0, 0, 0), 0);
  assert.equal(e.compositor_surface_accepts_point(1, 5, 5, 10, 10, 1, 3, 3, 4, 4), 1);
  assert.equal(e.compositor_surface_accepts_point(1, 1, 1, 10, 10, 1, 3, 3, 4, 4), 0);

  assert.equal(e.compositor_focus_changed(5, 5), 0);
  assert.equal(e.compositor_focus_changed(5, 6), 1);
  assert.equal(e.compositor_frontmost_tearing(1, 2, 1), 1);
  assert.equal(e.compositor_frontmost_tearing(1, 1, 2), 0);
  assert.equal(e.compositor_frontmost_tearing(0, 1, 2), 1);
  assert.equal(Number(e.compositor_presentation_next_seq(41n)), 42);

  console.log(JSON.stringify({ unit: "compositor-core", standard_id: e.proto_standard_id(), ok: true }));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
