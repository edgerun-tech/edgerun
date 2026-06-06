  (func (export "er_ui_button_group_item_count") (result i32) i32.const 2)
  (func (export "er_ui_button_group_text_padding") (result f32) f32.const 8)

  (func (export "er_ui_button_group_active_index") (param $active i32) (result i32)
    local.get $active i32.const 2 call $er_ui_list_clamped_index)

  (func (export "er_ui_button_group_indexed_id") (param $id i32) (param $active i32) (result i32)
    local.get $id i32.const 2 i32.mul
    local.get $active i32.const 2 call $er_ui_list_clamped_index
    i32.add)

  (func (export "er_ui_button_group_hit_id") (param $id i32) (param $index i32) (result i32)
    local.get $id local.get $index i32.const 1 call $layout_min_i32_u i32.add)

  (func (export "er_ui_button_group_segment_bounds") (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    local.get $bounds local.get $index i32.const 2 f32.const 0 local.get $out call $er_ui_list_equal_segment_bounds_gap)

  (func (export "er_ui_button_group_segment_text_bounds") (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $index i32.const 2 f32.const 0 i32.const 121824 call $er_ui_list_equal_segment_bounds_gap drop
    i32.const 121824 local.get $out call $er_ui_toggle_text_bounds)

  (func (export "er_ui_button_group_measure") (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $constraints i32) (param $out i32) (result i32)
    local.get $first_ptr local.get $first_len local.get $second_ptr local.get $second_len local.get $constraints local.get $out call $er_ui_tabs_measure_two_segments)
