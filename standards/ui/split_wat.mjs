import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const src = join(__dirname, 'src');
if (!existsSync(src)) mkdirSync(src, { recursive: true });

const file = readFileSync(join(__dirname, 'ui_framework.wat'), 'utf-8');
const lines = file.split('\n');

// Parse into top-level items (data, global, func, gap)
function parseItems(lines) {
  const items = [];
  let i = 0;
  while (i < lines.length) {
    const gapStart = i;
    while (i < lines.length && (lines[i].match(/^\s*$/) || lines[i].match(/^;;/))) i++;
    if (gapStart < i) items.push({ start: gapStart, end: i, kind: 'gap' });
    if (i >= lines.length) break;

    const start = i;
    let depth = 0, inString = false;
    do {
      const l = lines[i];
      for (let j = 0; j < l.length; j++) {
        const ch = l[j];
        if (inString) { if (ch === '"' && (j === 0 || l[j-1] !== '\\')) inString = false; }
        else { if (ch === '"') inString = true; else if (ch === '(') depth++; else if (ch === ')') depth--; }
      }
      i++;
    } while (depth > 0 && i < lines.length);

    const f = lines[start];
    const kind = f.startsWith('  (data') ? 'data' : f.startsWith('  (global') ? 'global' : f.startsWith('  (func') ? 'func' : 'other';
    items.push({ start, end: i, kind });
  }
  return items;
}

function funcName(lines, item) {
  const m = lines[item.start].match(/^\s+\(func\s+\$?(\S+)/);
  if (!m) return null;
  const name = m[1];
  // If name starts with (param/result/local/etc., this is a nameless func
  if (name.startsWith('(')) return null;
  return name;
}

const items = parseItems(lines);

// Group definitions using simple prefix/suffix matchers
// Each group: { name, test: (name) => bool }
const groups = [
  // --- Utility layers ---
  { name: '01_math', test: n => ['min_f32','max_f32','clamp_f32','clamp_i32','sin_rad','cos_rad','snap_pixel','er_ui_snap_stroke_center','min_i32_u'].includes(n) },
  { name: '02_memory_io', test: n => ['store16','store32','load16','copy','zero'].includes(n) },
  { name: '03_runtime', test: n => n === 'store_runtime_state' || n.startsWith('er_ui_runtime_') },
  { name: '04_hashing', test: n => n.startsWith('fnv1a_') },
  { name: '05_ref_kind', test: n => ['record_ptr','table_start','string_ptr','string_len','string_ref','second_ref_is_packed','kind_in_mask','packed_second_ref_valid','valid_rect','preferred_w','preferred_h','emit_rect','emit_text','emit_icon_or_rect','command_set_owner_at','command_set_meta_at','record_kind_group','record_encoded_state','record_encoded_mul','finite_f32','er_ui_ref_offset','er_ui_ref_len','er_ui_string_ptr','er_ui_string_ptr_checked','er_ui_string_len_checked'].includes(n) },
  { name: '06_rect_geom', test: n => n.startsWith('rect_store') || n.startsWith('er_ui_rect_') },
  { name: '07_theme', test: n => n.startsWith('er_ui_theme_') },
  { name: '08_core_constants', test: n => ['er_ui_version','er_ui_record_kind_count','er_ui_command_size','er_ui_color_pack','er_ui_encode_unit','er_ui_decode_unit','er_ui_font_reference_body_offset','er_ui_font_glyph_record_size','er_ui_font_kern_record_size','er_ui_font_command_record_size','er_ui_font_weight_regular','er_ui_font_weight_semibold','er_ui_font_weight_bold','er_ui_font_weight_value','er_ui_font_render_px','er_ui_font_should_snap_text','er_ui_font_snap_text_position'].includes(n) },
  { name: '09_font', test: n => n.startsWith('font_') || n.startsWith('er_ui_font_') },

  // --- Icons & SVG ---
  { name: '10_icon_assets', test: n => n.startsWith('icon_pack_') || ['er_ui_icon_asset_pack_set','er_ui_icon_asset_pack_loaded','er_ui_icon_segment_size','er_ui_icon_viewbox','er_ui_icon_valid','er_ui_icon_path_ptr','er_ui_icon_path_len','er_ui_icon_ir_byte_offset','er_ui_icon_ir_byte_len','er_ui_icon_ir_ptr','er_ui_icon_ir_float_count','er_ui_icon_ir_copy','er_ui_icon_name_ptr','er_ui_icon_name_len','er_ui_icon_count','er_ui_icon_bounds'].includes(n) || (n.startsWith('er_ui_icon_') && (n.endsWith('_op_') || n.match(/er_ui_icon_(op_|stroke_|default_)/))) },
  { name: '11_svg', test: n => n.startsWith('svg_') || n.startsWith('er_ui_svg_') },
  { name: '12_icon_render', test: n => (n.startsWith('icon_') && !n.startsWith('icon_pack_')) || n.startsWith('er_ui_icon_fit_viewport') || n.startsWith('er_ui_icon_render_') || n.startsWith('er_ui_icon_stroke_width') || n.startsWith('er_ui_icon_command_tag_') || n.startsWith('er_ui_icon_segment_count') || n.startsWith('er_ui_icon_scaled_stroke') || n.startsWith('er_ui_icon_fit_rect') || n.startsWith('er_ui_icon_viewport') || n.startsWith('er_ui_icon_segments_write') || n.startsWith('er_ui_icon_rect_contains') },

  // --- Writer / layout top level / record accessors ---
  { name: '13_writer', test: n => n.startsWith('er_ui_writer_') || n === 'writer_store_used_len' },

  // er_ui_measure, er_ui_measure_packed_width/height, er_ui_measure_width/height
  { name: '14_measure_layout_top', test: n => n === 'er_ui_measure' || n.startsWith('er_ui_measure_packed') || ['er_ui_measure_width','er_ui_measure_height'].includes(n) || n.startsWith('er_ui_layout_set_buf') || n === 'er_ui_layout' || n.startsWith('er_ui_layout_get') || n.startsWith('er_ui_layout_hit_test') || n.startsWith('er_ui_render') || n.startsWith('er_ui_validate') || n.startsWith('er_ui_node_count') || n.startsWith('er_ui_root_count') || n === 'er_ui_axis' || n === 'er_ui_gap' || n === 'er_ui_padding' || n === 'record_at' || n.startsWith('er_ui_record_') },

  // Patch, BLE, command system
  { name: '15_write_patch', test: n => n.startsWith('er_ui_write_') || n.startsWith('er_ui_patch_') || n.startsWith('patch_target_') || n.startsWith('er_ui_ble_') || n.startsWith('er_ui_command_') },

  // Widget creation
  { name: '16_widget_creation', test: n => n === 'write_single_string_ref' || n === 'write_id_multiplier_string' || n.startsWith('er_ui_wasm_new_') },

  // Regions & hit events
  { name: '17_regions_hit', test: n => n.startsWith('er_ui_region_') || n.startsWith('er_ui_regions_') || n.startsWith('er_ui_hit_') },

  // Layout engine core (flexbox, constraints, measurement, views, lists)
  { name: '18_layout_engine', test: n =>
    // layout helper functions
    n.startsWith('layout_') ||
    // er_ui_layout_* functions NOT in the "top" set
    (n.startsWith('er_ui_layout_') && !n.startsWith('er_ui_layout_set_buf') && n !== 'er_ui_layout' && !n.startsWith('er_ui_layout_get') && !n.startsWith('er_ui_layout_hit_test')) ||
    n.startsWith('er_ui_list_') ||
    n.startsWith('er_ui_view_') ||
    n.startsWith('er_ui_flex_') ||
    n.startsWith('flex_') ||
    // Components that are layout operations
    ['er_ui_centered_square_bounds'].includes(n) ||
    n.startsWith('er_ui_component_size_')
  },

  // Primitives + text + utf8
  { name: '19_primitives_text', test: n => n.startsWith('er_ui_primitives_') || n.startsWith('er_ui_text_') || n.startsWith('ui_utf8_') || n === 'ui_is_ascii_space_byte' || n.startsWith('er_ui_utf8_') || n.startsWith('primitives_') || n.startsWith('textarea_') || n.startsWith('er_ui_empty_measure') || n.startsWith('er_ui_measure_intrinsic') || n.startsWith('er_ui_measure_flexible_line') },

  // --- Component constants & measurement functions ---
  { name: '20_components_simple', test: n => {
    const pfx = ['badge','button','icon_button','input','textarea','row_item','card','empty_state','checkbox','switch','slider','progress','aspect_ratio','separator','skeleton','spinner','kbd','avatar','label','tooltip'];
    return pfx.some(p => n.startsWith('er_ui_' + p + '_'))
  }},

  { name: '21_components_toggle_alert_tabs', test: n => {
    const pfx = ['toggle','alert','tabs','radio','breadcrumb'];
    return pfx.some(p => n.startsWith('er_ui_' + p + '_'))
  }},

  { name: '22_components_accordion_etc', test: n => {
    const pfx = ['accordion','button_group','field','input_otp','pagination','select','combobox','navigation_menu','menubar','carousel','direction','input_group','resizable','scroll_area'];
    return pfx.some(p => n.startsWith('er_ui_' + p + '_'))
  }},

  { name: '23_components_popups', test: n => {
    const pfx = ['popover','context_menu','hover_card','chart','calendar','dialog','toast','drawer','sheet','sidebar','dropdown_menu'];
    return pfx.some(p => n.startsWith('er_ui_' + p + '_'))
  }},

  { name: '24_components_rest', test: n => {
    const pfx = ['timeline','workspace','graph','app','table'];
    return pfx.some(p => n.startsWith('er_ui_' + p + '_'))
  }},

  // Tree, object, stack, semantic
  { name: '25_tree_object_semantic', test: n => n.startsWith('er_ui_tree_') || n.startsWith('er_ui_object_') || n.startsWith('er_ui_stack_') || n.startsWith('er_ui_semantic_') || n.startsWith('tree_codec_') || n.startsWith('er_ui_slot_tree_') || n.startsWith('er_ui_tree_descriptor_') },

  // Gallery
  { name: '26_gallery', test: n => n.startsWith('er_ui_gallery_') },
];

// Collect all non-gap items into groups
const globalsGroup = { name: '00_globals', items: [] };
const groupItems = groups.map(() => []);

// First pass: sort data/global to globals, route funcs to groups
for (let i = 0; i < items.length; i++) {
  const item = items[i];
  if (item.kind === 'gap') continue;
  if (item.kind === 'data' || item.kind === 'global') {
    globalsGroup.items.push(i);
    continue;
  }
  if (item.kind === 'other') {
    // Comment block or unexpected form — attach to next func's group
    let found = false;
    for (let j = i + 1; j < items.length; j++) {
      if (items[j].kind === 'gap') continue;
      if (items[j].kind === 'func') {
        for (let g = 0; g < groups.length; g++) {
          const n2 = funcName(lines, items[j]);
          if (n2 && groups[g].test(n2)) {
            const pg = i > 0 && items[i-1].kind === 'gap' ? [i-1, i] : [i];
            groupItems[g].push(...pg);
            found = true; break;
          }
        }
      }
      if (!found) globalsGroup.items.push(i);
      break;
    }
    if (!found) globalsGroup.items.push(i);
    continue;
  }
  const name = funcName(lines, item);
  if (!name) {
    // Nameless function (thunk/alias) — infer group from the function it calls
    const body = lines.slice(item.start, item.end).join(' ');
    const callM = body.match(/call \$(\w+)/);
    const callee = callM ? callM[1] : null;
    if (callee) {
      let found = false;
      for (let g = 0; g < groups.length; g++) {
        if (groups[g].test(callee)) {
          if (i > 0 && items[i-1].kind === 'gap') groupItems[g].push(i-1);
          groupItems[g].push(i);
          found = true; break;
        }
      }
      if (found) continue;
    }
    console.error('UNASSIGNABLE nameless func at line ' + item.start + ': ' + lines[item.start].substring(0, 80));
    continue;
  }
  let assigned = false;
  for (let g = 0; g < groups.length; g++) {
    if (groups[g].test(name)) {
      if (i > 0 && items[i-1].kind === 'gap') groupItems[g].push(i-1);
      groupItems[g].push(i);
      assigned = true;
      break;
    }
  }
  if (!assigned) {
    console.error('UNASSIGNED: line ' + item.start + ': ' + lines[item.start].substring(0, 80));
  }
}

// Attach remaining gaps to their following group
for (let i = 0; i < items.length; i++) {
  if (items[i].kind !== 'gap') continue;
  let taken = false;
  for (const gi of groupItems) { if (gi.includes(i)) { taken = true; break; } }
  if (taken || globalsGroup.items.includes(i)) continue;
  for (let j = i + 1; j < items.length; j++) {
    if (items[j].kind === 'gap') continue;
    if (items[j].kind === 'func') {
      for (let g = 0; g < groups.length; g++) {
        if (groupItems[g].includes(j)) {
          const idx = groupItems[g].indexOf(j);
          groupItems[g].splice(idx, 0, i);
          break;
        }
      }
    } else {
      globalsGroup.items.push(i);
    }
    break;
  }
}

function padR(s, n) { return String(s).padEnd(n); }
function padL(s, n) { return String(s).padStart(n); }

function writeGroup(name, indices) {
  if (indices.length === 0) return null;
  const text = indices.map(idx => lines.slice(items[idx].start, items[idx].end).join('\n')).join('\n');
  writeFileSync(join(src, name + '.wat'), text + '\n');
  const lc = text.split('\n').length;
  const fc = indices.filter(idx => items[idx].kind === 'func').length;
  console.log('  ' + padR(name + '.wat', 34) + ' ' + padL(lc, 6) + ' lines, ' + padL(fc, 4) + ' funcs');
  return { funcs: fc, lines: lc };
}

console.log('Split:');
const gs = writeGroup('00_globals', globalsGroup.items);
let total = gs || { funcs: 0, lines: 0 };
for (let g = 0; g < groups.length; g++) {
  const s = writeGroup(groups[g].name, groupItems[g]);
  if (s) total = { funcs: total.funcs + s.funcs, lines: total.lines + s.lines };
}
console.log('  ' + padR('---------------------------------', 34) + ' ' + padL('------', 6) + ' ' + padL('----', 4));
console.log('  ' + padR('Total', 34) + ' ' + padL(total.lines, 6) + ' lines, ' + padL(total.funcs, 4) + ' funcs');

// Verify
const allFuncs = new Set(items.map((_, i) => i).filter(i => items[i].kind === 'func'));
const taken = new Set();
for (const gi of groupItems) for (const idx of gi) taken.add(idx);
for (const idx of globalsGroup.items) taken.add(idx);
const missing = [...allFuncs].filter(i => !taken.has(i));
if (missing.length > 0) {
  console.error('\n\u26a0  ' + missing.length + ' functions unassigned!');
  for (const idx of missing) console.error('  line ' + items[idx].start + ': ' + lines[items[idx].start].substring(0, 80));
  process.exit(1);
} else {
  console.log('\n\u2713 All ' + allFuncs.size + ' functions assigned');
}
