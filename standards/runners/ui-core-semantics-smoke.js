#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/ui-core-semantics.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  const wasm = execFileSync("wat2wasm", [file, "-o", "-"]);
  execFileSync("wasm-validate", ["-"], { input: wasm });
  return wasm;
}

function put(memory, value, ptr = 4096) {
  const bytes = Buffer.from(value, "ascii");
  memory.fill(0, ptr, ptr + Math.max(128, bytes.length + 1));
  memory.set(bytes, ptr);
  return [ptr, bytes.length];
}

function code(e, memory, fn, value) {
  return e[fn](...put(memory, value));
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const e = instance.exports;
  const memory = new Uint8Array(e.memory.buffer);

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300141);

  assert.equal(code(e, memory, "ui_scene_render_layer_code", "clear"), 1);
  assert.equal(code(e, memory, "ui_scene_render_layer_code", "rects"), 2);
  assert.equal(code(e, memory, "ui_scene_render_layer_code", "icon_quads"), 3);
  assert.equal(code(e, memory, "ui_scene_render_layer_code", "text_quads"), 4);
  assert.equal(code(e, memory, "ui_scene_interaction_layer_code", "hits"), 1);
  assert.equal(code(e, memory, "ui_scene_interaction_layer_code", "drag_sources"), 2);
  assert.equal(code(e, memory, "ui_scene_interaction_layer_code", "drop_targets"), 3);
  assert.equal(e.ui_scene_budget_limit(1, 1), 2000);
  assert.equal(e.ui_scene_budget_limit(1, 7), 8000);
  assert.equal(e.ui_scene_budget_limit(2, 2), 600);
  assert.equal(e.ui_scene_budget_limit(3, 6), 640);
  assert.equal(e.ui_frame_time_budget_ms(1, 1), 3);
  assert.equal(e.ui_frame_time_budget_ms(2, 2), 6);
  assert.equal(e.ui_packed_scene_stride(1), 13);
  assert.equal(e.ui_packed_scene_stride(2), 11);
  assert.equal(e.ui_packed_scene_stride(3), 12);
  assert.equal(e.ui_packed_scene_stride(4), 6);

  assert.equal(code(e, memory, "ui_hit_kind_code", "button"), 4);
  assert.equal(code(e, memory, "ui_hit_kind_code", "workspace_tab"), 20);
  assert.equal(code(e, memory, "ui_hit_kind_code", "shell_launcher"), 23);
  assert.equal(code(e, memory, "ui_hit_kind_code", "app_launcher_item"), 24);
  assert.equal(code(e, memory, "ui_action_code", "none"), 1);
  assert.equal(code(e, memory, "ui_action_code", "hovered"), 2);
  assert.equal(code(e, memory, "ui_action_code", "activated"), 4);
  assert.equal(code(e, memory, "ui_action_code", "scroll_changed"), 13);
  assert.equal(e.ui_action_needs_redraw(1), 0);
  assert.equal(e.ui_action_needs_redraw(2), 0);
  assert.equal(e.ui_action_needs_redraw(4), 1);

  assert.equal(code(e, memory, "ui_style_class_code", "row"), 1);
  assert.equal(code(e, memory, "ui_style_class_code", "justify-between"), 9);
  assert.equal(code(e, memory, "ui_style_class_code", "rounded-lg"), 12);
  assert.equal(code(e, memory, "ui_color_token_code", "accent_text"), 12);
  assert.equal(code(e, memory, "ui_color_token_code", "danger"), 15);
  assert.equal(e.ui_spacing_px(1), 4);
  assert.equal(e.ui_spacing_px(12), 32);
  assert.equal(e.ui_spacing_px(16), 64);

  assert.equal(code(e, memory, "ui_icon_code", "activity"), 1);
  assert.equal(code(e, memory, "ui_icon_code", "chevron-right"), 6);
  assert.equal(code(e, memory, "ui_icon_code", "message-plus"), 15);
  assert.equal(code(e, memory, "ui_icon_code", "x"), 31);
  assert.equal(e.ui_icon_count(), 31);
  assert.equal(e.ui_icon_tone_count(), 7);
  assert.equal(e.ui_icon_atlas_metric(1, 1), 672);
  assert.equal(e.ui_icon_atlas_metric(2, 2), 560);
  assert.equal(e.ui_icon_atlas_metric(1, 3), 30);

  assert.equal(code(e, memory, "ui_workspace_code", "leaf"), 1);
  assert.equal(code(e, memory, "ui_workspace_code", "split_requested"), 6);
  assert.equal(code(e, memory, "ui_workspace_code", "runtime"), 7);
  assert.equal(e.ui_shell_launcher_id(), 880);
  assert.equal(e.ui_app_control(1), 0);
  assert.equal(e.ui_app_control(2), 1);
  assert.equal(e.ui_app_control(3), 2);

  assert.equal(code(e, memory, "ui_component_domain_code", "network_app_prompt"), 1);
  assert.equal(code(e, memory, "ui_component_domain_code", "trust_manager_actions"), 3);
  assert.equal(code(e, memory, "ui_component_domain_code", "receipt_payment"), 6);
  assert.equal(code(e, memory, "ui_record_error_code", "wrong_magic"), 1);
  assert.equal(code(e, memory, "ui_record_error_code", "invalid_utf8"), 6);
  assert.equal(e.ui_record_limit(1), 16 * 1024 * 1024);
  assert.equal(e.ui_record_limit(2), 65536);
  assert.equal(code(e, memory, "ui_accessibility_role_code", "button"), 4);
  assert.equal(code(e, memory, "ui_accessibility_role_code", "progressbar"), 12);
  assert.equal(code(e, memory, "ui_accessibility_role_code", "slider"), 22);

  console.log(JSON.stringify({ unit: "ui-core-semantics", standard_id: e.proto_standard_id(), ok: true }));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
