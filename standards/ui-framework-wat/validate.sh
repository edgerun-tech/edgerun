#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

fail() {
  printf 'FAIL: %s\n' "$*" >&2
  exit 1
}

command -v wat2wasm >/dev/null 2>&1 || fail "wat2wasm missing"
command -v wasm-validate >/dev/null 2>&1 || fail "wasm-validate missing"
command -v node >/dev/null 2>&1 || fail "node missing"

[[ -s "$root/build_wat.mjs" ]] || fail "build_wat.mjs missing"
[[ -s "$root/fragments.mjs" ]] || fail "fragment manifest missing"
[[ -s "$root/test_ui_framework.mjs" ]] || fail "test_ui_framework.mjs missing"
[[ -s "$root/source-commit.txt" ]] || fail "source-commit.txt missing"

node "$root/build_wat.mjs"
[[ -s "$root/ui_framework.wat" ]] || fail "ui_framework.wat missing"
wat2wasm "$root/ui_framework.wat" -o "$root/ui_framework.wasm"
wasm-validate "$root/ui_framework.wasm"
node "$root/test_ui_framework.mjs"

grep -q 'er_ui_writer_begin' "$root/ui_framework.wat" || fail "writer ABI missing"
grep -q 'er_ui_writer_record_child' "$root/ui_framework.wat" || fail "writer child-record ABI missing"
grep -q 'er_ui_render' "$root/ui_framework.wat" || fail "render ABI missing"
grep -q 'er_ui_measure' "$root/ui_framework.wat" || fail "measure ABI missing"
grep -q 'er_ui_command_hit_find' "$root/ui_framework.wat" || fail "command hit-find ABI missing"
grep -q 'er_ui_command_hit_id' "$root/ui_framework.wat" || fail "command hit-id ABI missing"
grep -q 'er_ui_command_text_len' "$root/ui_framework.wat" || fail "command accessor ABI missing"
grep -q 'er_ui_command_owner_id' "$root/ui_framework.wat" || fail "command owner ABI missing"
grep -q 'er_ui_command_tag_at' "$root/ui_framework.wat" || fail "bounded command accessor ABI missing"
grep -q 'er_ui_command_valid_at' "$root/ui_framework.wat" || fail "command valid-at ABI missing"
grep -q 'er_ui_command_write_rect' "$root/ui_framework.wat" || fail "command rect builder ABI missing"
grep -q 'er_ui_command_write_text' "$root/ui_framework.wat" || fail "command text builder ABI missing"
grep -q 'er_ui_record_parent' "$root/ui_framework.wat" || fail "record parent ABI missing"
grep -q 'er_ui_record_set_parent' "$root/ui_framework.wat" || fail "record parent setter ABI missing"
grep -q 'er_ui_string_ptr_checked' "$root/ui_framework.wat" || fail "checked string pointer ABI missing"
grep -q 'er_ui_string_len_checked' "$root/ui_framework.wat" || fail "checked string length ABI missing"
grep -q 'ERuI' "$root/test_ui_framework.mjs" || fail "ERuI codec test missing"
grep -q 'er_ui_wasm_new_button' "$root/ui_framework.wat" || fail "component constructor ABI missing"
grep -q 'er_ui_regions_hit_test' "$root/ui_framework.wat" || fail "region hit-test ABI missing"
grep -q 'er_ui_layout_grid_child' "$root/ui_framework.wat" || fail "layout ABI missing"
grep -q 'er_ui_layout_scratch_base' "$root/ui_framework.wat" || fail "layout scratch ABI missing"
grep -q 'er_ui_layout_masonry_child' "$root/ui_framework.wat" || fail "masonry layout ABI missing"
grep -q 'er_ui_layout_bento_child' "$root/ui_framework.wat" || fail "bento layout ABI missing"
grep -q 'er_ui_hit_id' "$root/ui_framework.wat" || fail "hit-id ABI missing"
grep -q 'er_ui_hit_event_encode_point' "$root/ui_framework.wat" || fail "hit event ABI missing"
grep -q 'er_ui_hit_event_validate' "$root/ui_framework.wat" || fail "hit event validation ABI missing"
grep -q 'er_ui_runtime_state_size' "$root/ui_framework.wat" || fail "runtime state ABI missing"
grep -q 'er_ui_patch_encode_two_strings_bool' "$root/ui_framework.wat" || fail "two-string bool patch ABI missing"
grep -q 'er_ui_patch_apply_two_strings' "$root/ui_framework.wat" || fail "two-string patch apply ABI missing"
grep -q 'er_ui_patch_apply_two_strings_checked' "$root/ui_framework.wat" || fail "checked two-string patch apply ABI missing"
grep -q 'er_ui_patch_apply_bool_ref' "$root/ui_framework.wat" || fail "bool patch apply ABI missing"
grep -q 'er_ui_patch_apply_second_bool_checked' "$root/ui_framework.wat" || fail "checked bool patch apply ABI missing"
grep -q 'er_ui_validate_deep' "$root/ui_framework.wat" || fail "deep validation ABI missing"
grep -q 'er_ui_wasm_new_switch' "$root/ui_framework.wat" || fail "switch constructor ABI missing"
grep -q 'er_ui_wasm_new_alert_dialog' "$root/ui_framework.wat" || fail "alert dialog constructor ABI missing"
grep -q 'er_ui_wasm_new_combobox' "$root/ui_framework.wat" || fail "combobox constructor ABI missing"
grep -q 'er_ui_wasm_new_select' "$root/ui_framework.wat" || fail "select constructor ABI missing"
grep -q 'er_ui_wasm_new_table' "$root/ui_framework.wat" || fail "table constructor ABI missing"
grep -q 'er_ui_wasm_new_dialog' "$root/ui_framework.wat" || fail "dialog constructor ABI missing"
grep -q 'er_ui_wasm_new_dropdown_menu' "$root/ui_framework.wat" || fail "dropdown menu constructor ABI missing"
grep -q 'er_ui_wasm_new_pagination' "$root/ui_framework.wat" || fail "pagination constructor ABI missing"
grep -q 'er_ui_wasm_new_resizable' "$root/ui_framework.wat" || fail "resizable constructor ABI missing"
grep -q 'er_ui_wasm_new_textarea' "$root/ui_framework.wat" || fail "textarea constructor ABI missing"
grep -q 'er_ui_wasm_new_tooltip' "$root/ui_framework.wat" || fail "tooltip constructor ABI missing"
grep -q 'er_ui_wasm_new_toast' "$root/ui_framework.wat" || fail "toast constructor ABI missing"
grep -q 'er_ui_wasm_new_sidebar' "$root/ui_framework.wat" || fail "sidebar constructor ABI missing"
grep -q 'er_ui_ble_decode_manufacturer_ad' "$root/ui_framework.wat" || fail "BLE manufacturer decode ABI missing"

printf 'OK ui-framework-wat\n'
