import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const src = join(__dirname, 'src');

if (!existsSync(src)) mkdirSync(src, { recursive: true });

const file = readFileSync(join(__dirname, 'ui_framework.wat'), 'utf-8');
const lines = file.split('\n');

// Parse into top-level items, preserving gaps (blank/comment lines between items)
function parseItems(lines) {
  const items = [];
  let i = 0;
  while (i < lines.length) {
    // Capture gap (blank lines, comments) before this item
    const gapStart = i;
    while (i < lines.length && (lines[i].match(/^\s*$/) || lines[i].match(/^;;/))) i++;
    if (gapStart < i) {
      items.push({ start: gapStart, end: i, kind: 'gap' });
    }
    if (i >= lines.length) break;

    // Top-level content form starts with `  (`
    const start = i;
    let depth = 0;
    let inString = false;
    do {
      const l = lines[i];
      for (let j = 0; j < l.length; j++) {
        const ch = l[j];
        if (inString) {
          if (ch === '"' && (j === 0 || l[j-1] !== '\\')) inString = false;
        } else {
          if (ch === '"') inString = true;
          else if (ch === '(') depth++;
          else if (ch === ')') depth--;
        }
      }
      i++;
    } while (depth > 0 && i < lines.length);

    const first = lines[start];
    const kind = first.startsWith('  (data') ? 'data' :
                 first.startsWith('  (global') ? 'global' :
                 first.startsWith('  (func') ? 'func' : 'other';
    items.push({ start, end: i, kind });
  }
  return items;
}

function funcName(lines, item) {
  const m = lines[item.start].match(/^\s+\(func\s+\$(\S+)/);
  return m ? m[1] : null;
}

const items = parseItems(lines);

// Helper: safe match on nullable name
const fmatch = (name, pattern) => name ? !!name.match(pattern) : false;

// Group definitions: each group has a name and a matcher function (kind, name) => bool
// Only 'func' items are matched by name; 'data'/'global' go to header.
const groups = [
  { name: '00_header', match: (kind, name) => kind === 'data' || kind === 'global' },
  { name: '01_math', match: (kind, name) => kind === 'func' && fmatch(name, /^(min_|max_|clamp_|sin_rad|cos_rad|snap_pixel|er_ui_snap_stroke_center|min_i32_u)$/) },
  { name: '02_memory_io', match: (kind, name) => kind === 'func' && fmatch(name, /^(store16|store32|load16|copy|zero)$/) },
  { name: '03_runtime_state', match: (kind, name) => kind === 'func' && fmatch(name, /^er_ui_runtime_/) },
  { name: '04_store_state', match: (kind, name) => kind === 'func' && name === 'store_runtime_state' },
  { name: '05_hashing', match: (kind, name) => kind === 'func' && fmatch(name, /^fnv1a_/) },
  { name: '06_ref_kind', match: (kind, name) => kind === 'func' && fmatch(name, /^(record_ptr|table_start|string_ptr|string_len|string_ref|second_ref_is_packed|kind_in_mask|packed_second_ref_valid|valid_rect|preferred_[wh]|emit_rect|emit_text|emit_icon_or_rect|command_set_owner_at|command_set_meta_at|record_kind_group|record_encoded_state|record_encoded_mul|finite_f32)$/) },
  { name: '07_rect_geom', match: (kind, name) => kind === 'func' && fmatch(name, /^(rect_store|er_ui_rect_)/) },
  { name: '08_theme', match: (kind, name) => kind === 'func' && fmatch(name, /^er_ui_theme_/) },
  { name: '09_core_constants', match: (kind, name) => kind === 'func' && fmatch(name, /^(er_ui_version|er_ui_record_kind_count|er_ui_command_size|er_ui_color_pack|er_ui_encode_unit|er_ui_decode_unit|er_ui_font_reference_body_offset|er_ui_font_glyph_record_size|er_ui_font_kern_record_size|er_ui_font_command_record_size|er_ui_font_weight_|er_ui_font_render_px|er_ui_font_should_snap_text|er_ui_font_snap_text_position)$/) },
  { name: '10_font', match: (kind, name) => kind === 'func' && fmatch(name, /^(font_|er_ui_font_)/) },
  { name: '11_icon_assets', match: (kind, name) => kind === 'func' && fmatch(name, /^(icon_pack_|er_ui_icon_(asset_|segment_size|viewbox|valid|path_|ir_|name_|op_|stroke_|default_))/) },
  { name: '12_svg', match: (kind, name) => kind === 'func' && fmatch(name, /^(svg_|er_ui_svg_)/) },
  { name: '13_icon_render', match: (kind, name) => kind === 'func' && fmatch(name, /^(icon_|er_ui_icon_(render_|stroke_width|command_tag|segment_count|scaled_stroke|fit_rect|viewport|segments_write|rect_contains))/) && !fmatch(name, /^icon_pack_/) },
  { name: '14_writer', match: (kind, name) => kind === 'func' && fmatch(name, /^er_ui_writer_/) },
  { name: '15_measure_layout_top', match: (kind, name) => kind === 'func' && fmatch(name, /^(er_ui_measure$|er_ui_measure_packed|er_ui_layout_set_buf|er_ui_layout$|er_ui_layout_get|er_ui_layout_hit_test|er_ui_render|er_ui_validate|er_ui_node_count|er_ui_root_count|er_ui_axis|er_ui_gap|er_ui_padding|record_at|er_ui_record_)/) },
  { name: '16_write_patch', match: (kind, name) => kind === 'func' && fmatch(name, /^(er_ui_write_|er_ui_patch_|patch_target_|er_ui_ble_|er_ui_command_)/) },
  { name: '17_widget_creation', match: (kind, name) => kind === 'func' && fmatch(name, /^(write_single_string_ref|write_id_multiplier_string|er_ui_wasm_new_)/) },
  { name: '18_regions_hit', match: (kind, name) => kind === 'func' && fmatch(name, /^(er_ui_region_|er_ui_hit_)/) },
  { name: '19_layout_engine', match: (kind, name) => kind === 'func' && fmatch(name, /^(er_ui_layout_(?!writer_|measure$|measure_packed|set_buf$|get$|hit_test|render|validate|node_count|root_count|axis|gap|padding|record_)|er_ui_list_|er_ui_view_|layout_sanitize_|er_ui_layout_axis_|er_ui_layout_insets_|layout_shrink_|er_ui_layout_constraints_|layout_measurement_|er_ui_flex_|flex_)/) },
  { name: '20_primitives_text', match: (kind, name) => kind === 'func' && fmatch(name, /^(er_ui_primitives_|er_ui_text_|ui_utf8_|ui_is_ascii_|er_ui_utf8_|layout_measure_)/) },
  { name: '21_components_simple', match: (kind, name) => kind === 'func' && fmatch(name, /^(er_ui_(badge|button|icon_button|input|textarea|row_item|card|empty_state|checkbox|switch|slider|progress|aspect_ratio|separator|skeleton|spinner|kbd|avatar|label|measure_intrinsic|measure_flexible_line|tooltip)_)/) },
  { name: '22_components_toggle_alert_tabs', match: (kind, name) => kind === 'func' && fmatch(name, /^(er_ui_(toggle|alert|tabs|radio|breadcrumb)_)/) },
  { name: '23_components_accordion_etc', match: (kind, name) => kind === 'func' && fmatch(name, /^(er_ui_(accordion|button_group|field|input_otp|pagination|select|combobox|navigation_menu|menubar|carousel|direction|input_group|resizable|scroll_area)_)/) },
  { name: '24_components_popups', match: (kind, name) => kind === 'func' && fmatch(name, /^(er_ui_(popover|context_menu|hover_card|chart|calendar|dialog|toast|drawer|sheet|sidebar|dropdown_menu)_)/) },
  { name: '25_components_rest', match: (kind, name) => kind === 'func' && fmatch(name, /^(er_ui_(timeline|workspace|graph|app|table)_)/) },
  { name: '26_tree_object_semantic', match: (kind, name) => kind === 'func' && fmatch(name, /^(er_ui_tree_|er_ui_object_|er_ui_stack_|er_ui_semantic_)/) },
  { name: '27_gallery', match: (kind, name) => kind === 'func' && fmatch(name, /^er_ui_gallery_/) },
];

// Assign items to groups
const groupItems = groups.map(() => []);

for (let i = 0; i < items.length; i++) {
  const item = items[i];
  if (item.kind === 'gap') continue; // gaps handled by last-active flow or header
  
  const name = item.kind === 'func' ? funcName(lines, item) : null;
  
  let assigned = false;
  for (let g = 0; g < groups.length; g++) {
    if (groups[g].match(item.kind, name)) {
      groupItems[g].push(i);
      assigned = true;
      break;
    }
  }
  
  if (!assigned) {
    // Walk backwards to find last non-gap assigned item and attach there
    // This handles misc items that are closely related to the previous group
    let lastAssigned = -1;
    let lastG = -1;
    for (let j = i - 1; j >= 0; j--) {
      if (items[j].kind !== 'gap') {
        for (let g = groups.length - 1; g >= 0; g--) {
          if (groupItems[g].includes(j)) { lastG = g; break; }
        }
        break;
      }
    }
    if (lastG >= 0) {
      groupItems[lastG].push(i);
    } else {
      console.error(`UNMATCHED item at line ${items[i].start}: ${lines[items[i].start].substring(0, 100)}`);
    }
  }
}

// Also attach gaps to their following group
// For each gap, find which group has the next non-gap item
for (let i = 0; i < items.length; i++) {
  if (items[i].kind !== 'gap') continue;
  // find the next non-gap item
  for (let j = i + 1; j < items.length; j++) {
    if (items[j].kind !== 'gap') {
      for (let g = 0; g < groups.length; g++) {
        if (groupItems[g].includes(j)) {
          // Insert gap before the next item in the group
          const idx = groupItems[g].indexOf(j);
          groupItems[g].splice(idx, 0, i);
          break;
        }
      }
      break;
    }
  }
}

// Write each group to a file
let totalLines = 0;
let totalFuncs = 0;

for (let g = 0; g < groups.length; g++) {
  if (groupItems[g].length === 0) continue;
  
  const parts = [];
  for (const idx of groupItems[g]) {
    const item = items[idx];
    const txt = lines.slice(item.start, item.end).join('\n');
    parts.push(txt);
  }
  
  const content = parts.join('\n');
  const filePath = join(src, groups[g].name + '.wat');
  writeFileSync(filePath, content + (content.endsWith('\n') ? '' : '\n'));
  const lineCount = content.split('\n').length;
  const funcCount = groupItems[g].filter(idx => items[idx].kind === 'func').length;
  totalLines += lineCount;
  totalFuncs += funcCount;
  
  console.log(`${groups[g].name}.wat: ${filePath.split('/').pop().padEnd(28)} ${String(lineCount).padStart(6)} lines, ${String(funcCount).padStart(4)} funcs`);
}

console.log(`\nTotal: ${String(totalLines).padStart(6)} lines, ${String(totalFuncs).padStart(4)} funcs`);
console.log(`Original: 28561 lines, 1923 funcs`);

// Verify coverage
const assignedSet = new Set();
for (const gi of groupItems) for (const idx of gi) assignedSet.add(idx);
const contentItems = items.filter(i => i.kind !== 'gap');
const unassigned = contentItems.filter((_, i) => {
  const realIdx = items.indexOf(contentItems[i]);
  return !assignedSet.has(realIdx);
});
if (unassigned.length > 0) {
  console.error(`\n⚠  ${unassigned.length} content items unassigned:`);
  for (const item of unassigned) {
    console.error(`  line ${item.start}: ${lines[item.start].substring(0, 100)}`);
  }
} else {
  console.log(`\n✓ All content items assigned`);
}
