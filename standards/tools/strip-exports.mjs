import { readFileSync, writeFileSync } from 'fs';

const KEEP = new Set([
  'er_ui_writer_begin',
  'er_ui_writer_string',
  'er_ui_render',
  'er_ui_wasm_new_card',
  'er_ui_wasm_new_badge',
  'er_ui_layout_set_buf',
  'er_ui_layout',
  'er_ui_layout_hit_test',
  'er_ui_runtime_hover',
  'er_ui_runtime_focus',
  'er_ui_runtime_active',
  'er_ui_runtime_pointer_x',
  'er_ui_runtime_pointer_y',
  'er_ui_runtime_overlay',
  'er_ui_runtime_load_state',
]);

const src = readFileSync(process.argv[2], 'utf8');

// Match (export "er_ui_NAME") and remove it if NAME is not in KEEP
const result = src.replace(
  /\(export "(er_ui_[a-z0-9_]+)"\)/g,
  (match, name) => KEEP.has(name) ? match : ''
);

writeFileSync(process.argv[2], result);
console.log(`Stripped exports from ${process.argv[2]}`);
