  (func (export "er_ui_timeline_min_scale") (result f32) f32.const 0.05)
  (func (export "er_ui_timeline_min_window_w") (result f32) f32.const 0.01)
  (func (export "er_ui_timeline_pan_factor") (result f32) f32.const 0.25)
  (func (export "er_ui_timeline_min_zoom") (result f32) f32.const 1)
  (func (export "er_ui_timeline_max_zoom") (result f32) f32.const 6)
  (func (export "er_ui_timeline_zoom_out_factor") (result f32) f32.const 0.75)
  (func (export "er_ui_timeline_zoom_in_factor") (result f32) f32.const 1.35)
  (func (export "er_ui_timeline_default_label_w") (result f32) f32.const 82)
  (func (export "er_ui_timeline_default_inset") (result f32) f32.const 14)
  (func (export "er_ui_timeline_default_radius") (result f32) f32.const 8)
  (func (export "er_ui_timeline_header_h") (result f32) f32.const 24)
  (func (export "er_ui_timeline_controls_max_w") (result f32) f32.const 194)
  (func (export "er_ui_timeline_controls_gap") (result f32) f32.const 10)
  (func (export "er_ui_timeline_axis_top_gap") (result f32) f32.const 12)
  (func (export "er_ui_timeline_axis_y_offset") (result f32) f32.const 14)
  (func (export "er_ui_timeline_axis_bottom_gap") (result f32) f32.const 18)
  (func (export "er_ui_timeline_lane_top") (result f32) f32.const 18)
  (func (export "er_ui_timeline_lane_reserved_h") (result f32) f32.const 16)
  (func (export "er_ui_timeline_lane_label_x_offset") (result f32) f32.const 78)
  (func (export "er_ui_timeline_lane_label_w") (result f32) f32.const 68)
  (func (export "er_ui_timeline_lane_label_h") (result f32) f32.const 16)
  (func (export "er_ui_timeline_lane_separator_bottom") (result f32) f32.const 6)
  (func (export "er_ui_timeline_block_min_w") (result f32) f32.const 4)
  (func (export "er_ui_timeline_block_min_h") (result f32) f32.const 5)
  (func (export "er_ui_timeline_block_bottom") (result f32) f32.const 8)
  (func (export "er_ui_timeline_axis_line_y") (result f32) f32.const 12)
  (func (export "er_ui_timeline_mark_tick_y") (result f32) f32.const 7)
  (func (export "er_ui_timeline_mark_tick_h") (result f32) f32.const 11)
  (func (export "er_ui_timeline_mark_label_x_offset") (result f32) f32.const 22)
  (func (export "er_ui_timeline_mark_label_y_offset") (result f32) f32.const 7)
  (func (export "er_ui_timeline_mark_label_w") (result f32) f32.const 64)
  (func (export "er_ui_timeline_mark_label_h") (result f32) f32.const 14)

  (func $er_ui_timeline_clamp_unit (export "er_ui_timeline_clamp_unit") (param $value f32) (result f32)
    local.get $value f32.const 0 call $max_f32 f32.const 1 call $min_f32)

  (func (export "er_ui_timeline_window") (param $offset f32) (param $scale f32) (param $out i32) (result i32)
    (local $start f32) (local $width f32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $offset call $er_ui_timeline_clamp_unit local.set $start
    f32.const 1 local.get $scale f32.const 0.05 call $max_f32 f32.div local.set $width
    local.get $out local.get $start f32.store
    local.get $out i32.const 4 i32.add local.get $start f32.const 0.01 f32.add local.get $start local.get $width f32.add call $max_f32 f32.store
    i32.const 1)

  (func (export "er_ui_timeline_pan_offset") (param $offset f32) (param $scale f32) (param $direction f32) (result f32)
    (local $window_w f32) (local $max_offset f32)
    f32.const 1 local.get $scale f32.const 0.05 call $max_f32 f32.div local.set $window_w
    f32.const 0 f32.const 1 local.get $window_w f32.sub call $max_f32 local.set $max_offset
    local.get $offset local.get $direction local.get $window_w f32.mul f32.const 0.25 f32.mul f32.add
    f32.const 0 call $max_f32
    local.get $max_offset call $min_f32)

  (func (export "er_ui_timeline_zoom_state") (param $offset f32) (param $scale f32) (param $factor f32) (param $out i32) (result i32)
    (local $previous_scale f32) (local $previous_w f32) (local $center f32) (local $next_scale f32) (local $next_w f32) (local $max_offset f32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $scale f32.const 0.05 call $max_f32 local.set $previous_scale
    f32.const 1 local.get $previous_scale f32.div local.set $previous_w
    local.get $offset local.get $previous_w f32.const 0.5 f32.mul f32.add local.set $center
    f32.const 1 local.get $scale local.get $factor f32.mul call $max_f32 f32.const 6 call $min_f32 local.set $next_scale
    f32.const 1 local.get $next_scale f32.div local.set $next_w
    f32.const 0 f32.const 1 local.get $next_w f32.sub call $max_f32 local.set $max_offset
    local.get $out local.get $center local.get $next_w f32.const 0.5 f32.mul f32.sub f32.const 0 call $max_f32 local.get $max_offset call $min_f32 f32.store
    local.get $out i32.const 4 i32.add local.get $next_scale f32.store
    i32.const 1)

  (func (export "er_ui_timeline_unit_in_window") (param $value f32) (param $start f32) (param $end f32) (param $out i32) (result i32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $end local.get $start f32.le
    if i32.const 0 return end
    local.get $value local.get $start f32.lt
    local.get $value local.get $end f32.gt
    i32.or
    if i32.const 0 return end
    local.get $out local.get $value local.get $start f32.sub local.get $end local.get $start f32.sub f32.div call $er_ui_timeline_clamp_unit f32.store
    i32.const 1)

  (func (export "er_ui_timeline_block_in_window") (param $block_start f32) (param $block_end f32) (param $block_value f32) (param $start f32) (param $end f32) (param $out i32) (result i32)
    (local $mapped_start f32) (local $mapped_end f32) (local $denom f32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $end local.get $start f32.le
    if i32.const 0 return end
    local.get $start local.get $block_start call $max_f32 local.set $mapped_start
    local.get $end local.get $block_start local.get $block_end call $max_f32 call $min_f32 local.set $mapped_end
    local.get $mapped_end local.get $mapped_start f32.le
    if i32.const 0 return end
    local.get $end local.get $start f32.sub local.set $denom
    local.get $out local.get $mapped_start local.get $start f32.sub local.get $denom f32.div call $er_ui_timeline_clamp_unit f32.store
    local.get $out i32.const 4 i32.add local.get $mapped_end local.get $start f32.sub local.get $denom f32.div call $er_ui_timeline_clamp_unit f32.store
    local.get $out i32.const 8 i32.add local.get $block_value f32.store
    i32.const 1)

  (func $er_ui_timeline_lane_h (export "er_ui_timeline_lane_h") (param $axis i32) (param $lane_count i32) (result f32)
    local.get $axis i32.eqz local.get $lane_count i32.eqz i32.or
    if f32.const 0 return end
    local.get $axis i32.const 12 i32.add f32.load f32.const 16 f32.sub local.get $lane_count f32.convert_i32_u f32.div f32.const 1 call $max_f32)

  (func (export "er_ui_timeline_lane_label_bounds") (param $axis i32) (param $lane_index i32) (param $lane_count i32) (param $out i32) (result i32)
    (local $lane_h f32) (local $y f32)
    local.get $axis i32.eqz local.get $out i32.eqz i32.or local.get $lane_count i32.eqz i32.or
    if i32.const 0 return end
    local.get $axis local.get $lane_count call $er_ui_timeline_lane_h local.set $lane_h
    local.get $axis i32.const 4 i32.add f32.load f32.const 18 f32.add local.get $lane_index i32.const 0 local.get $lane_count i32.const 1 i32.sub call $clamp_i32 f32.convert_i32_u local.get $lane_h f32.mul f32.add local.set $y
    local.get $out
    local.get $axis f32.load f32.const 78 f32.sub
    local.get $y f32.const 4 f32.add
    f32.const 68
    f32.const 16
    call $rect_store)

  (func (export "er_ui_timeline_lane_separator_bounds") (param $axis i32) (param $lane_index i32) (param $lane_count i32) (param $out i32) (result i32)
    (local $lane_h f32) (local $y f32)
    local.get $axis i32.eqz local.get $out i32.eqz i32.or local.get $lane_count i32.eqz i32.or
    if i32.const 0 return end
    local.get $axis local.get $lane_count call $er_ui_timeline_lane_h local.set $lane_h
    local.get $axis i32.const 4 i32.add f32.load f32.const 18 f32.add local.get $lane_index i32.const 0 local.get $lane_count i32.const 1 i32.sub call $clamp_i32 f32.convert_i32_u local.get $lane_h f32.mul f32.add local.set $y
    local.get $out
    local.get $axis f32.load
    local.get $y local.get $lane_h f32.add f32.const 6 f32.sub
    local.get $axis i32.const 8 i32.add f32.load
    f32.const 1
    call $rect_store)

  (func (export "er_ui_timeline_lane_block_bounds") (param $axis i32) (param $lane_index i32) (param $lane_count i32) (param $block_start f32) (param $block_end f32) (param $block_value f32) (param $out i32) (result i32)
    (local $lane_h f32) (local $y f32) (local $start_x f32) (local $end_x f32) (local $w f32) (local $h f32)
    local.get $axis i32.eqz local.get $out i32.eqz i32.or local.get $lane_count i32.eqz i32.or
    if i32.const 0 return end
    local.get $axis local.get $lane_count call $er_ui_timeline_lane_h local.set $lane_h
    local.get $axis i32.const 4 i32.add f32.load f32.const 18 f32.add local.get $lane_index i32.const 0 local.get $lane_count i32.const 1 i32.sub call $clamp_i32 f32.convert_i32_u local.get $lane_h f32.mul f32.add local.set $y
    local.get $axis f32.load local.get $axis i32.const 8 i32.add f32.load local.get $block_start call $er_ui_timeline_clamp_unit f32.mul f32.add local.set $start_x
    local.get $axis f32.load local.get $axis i32.const 8 i32.add f32.load local.get $block_end call $er_ui_timeline_clamp_unit f32.mul f32.add local.set $end_x
    f32.const 4 f32.const 0 local.get $end_x local.get $start_x f32.sub call $max_f32 call $max_f32 local.set $w
    f32.const 5 local.get $lane_h f32.const 18 f32.sub local.get $block_value call $er_ui_timeline_clamp_unit f32.mul call $max_f32 local.set $h
    local.get $out local.get $start_x local.get $y local.get $lane_h f32.add f32.const 8 f32.sub local.get $h f32.sub local.get $w local.get $h call $rect_store)

  (func (export "er_ui_timeline_axis_line_bounds") (param $axis i32) (param $out i32) (result i32)
    local.get $axis i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $axis f32.load local.get $axis i32.const 4 i32.add f32.load f32.const 12 f32.add local.get $axis i32.const 8 i32.add f32.load f32.const 1 call $rect_store)

  (func (export "er_ui_timeline_mark_tick_bounds") (param $axis i32) (param $x_unit f32) (param $out i32) (result i32)
    (local $x f32)
    local.get $axis i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $axis f32.load local.get $axis i32.const 8 i32.add f32.load local.get $x_unit call $er_ui_timeline_clamp_unit f32.mul f32.add local.set $x
    local.get $out local.get $x local.get $axis i32.const 4 i32.add f32.load f32.const 7 f32.add f32.const 1 f32.const 11 call $rect_store)

  (func (export "er_ui_timeline_mark_label_bounds") (param $axis i32) (param $x_unit f32) (param $out i32) (result i32)
    (local $x f32)
    local.get $axis i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $axis f32.load local.get $axis i32.const 8 i32.add f32.load local.get $x_unit call $er_ui_timeline_clamp_unit f32.mul f32.add local.set $x
    local.get $out local.get $x f32.const 22 f32.sub local.get $axis i32.const 4 i32.add f32.load f32.const 7 f32.sub f32.const 64 f32.const 14 call $rect_store)

  (func $er_ui_timeline_inner_bounds (export "er_ui_timeline_inner_bounds") (param $bounds i32) (param $inset f32) (param $out i32) (result i32)
    local.get $bounds local.get $inset local.get $out call $er_ui_primitives_content_inset)

  (func (export "er_ui_timeline_header_text_bounds") (param $bounds i32) (param $has_header i32) (param $has_controls i32) (param $label_w f32) (param $inset f32) (param $out i32) (result i32)
    (local $controls_w f32) (local $controls_gap f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $inset i32.const 124300 call $er_ui_timeline_inner_bounds drop
    local.get $has_controls
    if
      i32.const 124308 f32.load f32.const 194 call $min_f32 local.set $controls_w
      f32.const 10 local.set $controls_gap
    else
      f32.const 0 local.set $controls_w
      f32.const 0 local.set $controls_gap
    end
    local.get $out i32.const 124300 f32.load i32.const 124304 f32.load i32.const 124308 f32.load local.get $controls_w f32.sub local.get $controls_gap f32.sub f32.const 1 call $max_f32 local.get $has_header if (result f32) f32.const 24 else f32.const 0 end call $rect_store)

  (func (export "er_ui_timeline_controls_bounds") (param $bounds i32) (param $has_controls i32) (param $inset f32) (param $out i32) (result i32)
    (local $controls_w f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $inset i32.const 124320 call $er_ui_timeline_inner_bounds drop
    local.get $has_controls i32.eqz
    if i32.const 0 return end
    i32.const 124328 f32.load f32.const 194 call $min_f32 local.set $controls_w
    local.get $out i32.const 124320 f32.load i32.const 124328 f32.load f32.add local.get $controls_w f32.sub i32.const 124324 f32.load f32.const 2 f32.sub local.get $controls_w f32.const 28 call $rect_store)

  (func (export "er_ui_timeline_axis_bounds") (param $bounds i32) (param $has_header i32) (param $label_w f32) (param $inset f32) (param $out i32) (result i32)
    (local $header_h f32) (local $resolved_label_w f32) (local $axis_y f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $inset i32.const 124340 call $er_ui_timeline_inner_bounds drop
    local.get $has_header if (result f32) f32.const 24 else f32.const 0 end local.set $header_h
    i32.const 124348 f32.load f32.const 0.42 f32.mul local.get $label_w f32.const 0 call $max_f32 call $min_f32 local.set $resolved_label_w
    i32.const 124344 f32.load local.get $header_h f32.add f32.const 12 f32.add local.set $axis_y
    local.get $out
    i32.const 124340 f32.load local.get $resolved_label_w f32.add
    local.get $axis_y f32.const 14 f32.add
    i32.const 124348 f32.load local.get $resolved_label_w f32.sub f32.const 1 call $max_f32
    i32.const 124344 f32.load i32.const 124352 f32.load f32.add local.get $axis_y f32.sub f32.const 18 f32.sub f32.const 1 call $max_f32
    call $rect_store)
