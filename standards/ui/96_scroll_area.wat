(func (export "er_ui_scroll_area_radius") (result f32) f32.const 7)
  (func (export "er_ui_scroll_area_padding") (result f32) f32.const 8)
  (func (export "er_ui_scroll_area_content_y") (result f32) f32.const 6)
  (func (export "er_ui_scroll_area_text_h") (result f32) f32.const 14)
  (func (export "er_ui_scroll_area_scrollbar_w") (result f32) f32.const 10)
  (func (export "er_ui_scroll_area_track_inset_x") (result f32) f32.const 6)
  (func (export "er_ui_scroll_area_track_inset_y") (result f32) f32.const 5)
  (func (export "er_ui_scroll_area_track_w") (result f32) f32.const 3)
  (func (export "er_ui_scroll_area_track_radius") (result f32) f32.const 2)
  (func (export "er_ui_scroll_area_thumb_min_h") (result f32) f32.const 12)
  (func (export "er_ui_scroll_area_thumb_ratio") (result f32) f32.const 0.45)
  (func (export "er_ui_scroll_area_label_len") (result i32) i32.const 18)
  (func (export "er_ui_scroll_area_label_max_lines") (result i32) i32.const 1)

  (func $er_ui_scroll_area_write_label (result i32)
    i32.const 123140 i32.const 83 i32.store8
    i32.const 123141 i32.const 99 i32.store8
    i32.const 123142 i32.const 114 i32.store8
    i32.const 123143 i32.const 111 i32.store8
    i32.const 123144 i32.const 108 i32.store8
    i32.const 123145 i32.const 108 i32.store8
    i32.const 123146 i32.const 97 i32.store8
    i32.const 123147 i32.const 98 i32.store8
    i32.const 123148 i32.const 108 i32.store8
    i32.const 123149 i32.const 101 i32.store8
    i32.const 123150 i32.const 32 i32.store8
    i32.const 123151 i32.const 99 i32.store8
    i32.const 123152 i32.const 111 i32.store8
    i32.const 123153 i32.const 110 i32.store8
    i32.const 123154 i32.const 116 i32.store8
    i32.const 123155 i32.const 101 i32.store8
    i32.const 123156 i32.const 110 i32.store8
    i32.const 123157 i32.const 116 i32.store8
    i32.const 123140)

  (func (export "er_ui_scroll_area_viewport_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load f32.const 8 f32.add
    local.get $bounds i32.const 4 i32.add f32.load f32.const 5 f32.add
    local.get $bounds i32.const 8 i32.add f32.load f32.const 26 f32.sub f32.const 1 call $max_f32
    local.get $bounds i32.const 12 i32.add f32.load f32.const 10 f32.sub f32.const 1 call $max_f32
    call $rect_store)

  (func $er_ui_scroll_area_track_bounds (export "er_ui_scroll_area_track_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add f32.const 6 f32.sub
    local.get $bounds i32.const 4 i32.add f32.load f32.const 5 f32.add
    f32.const 3
    local.get $bounds i32.const 12 i32.add f32.load f32.const 10 f32.sub f32.const 1 call $max_f32
    call $rect_store)

  (func (export "er_ui_scroll_area_default_metrics") (param $bounds i32) (param $out i32) (result i32)
    (local $viewport_h f32) (local $content_h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 12 i32.add f32.load f32.const 10 f32.sub f32.const 1 call $max_f32 local.set $viewport_h
    local.get $viewport_h f32.const 0.45 f32.div local.set $content_h
    local.get $out local.get $viewport_h f32.store
    local.get $out i32.const 4 i32.add local.get $content_h f32.store
    local.get $out i32.const 8 i32.add f32.const 0 f32.store
    i32.const 1)

  (func (export "er_ui_scroll_area_metrics") (param $viewport_h f32) (param $content_h f32) (param $offset_y f32) (param $out i32) (result i32)
    (local $vp f32) (local $content f32) (local $max_offset f32) (local $offset f32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $viewport_h f32.const 1 call $max_f32 local.set $vp
    local.get $content_h local.get $vp call $max_f32 local.set $content
    local.get $content local.get $vp f32.sub f32.const 0 call $max_f32 local.set $max_offset
    local.get $offset_y f32.const 0 call $max_f32 local.get $max_offset call $min_f32 local.set $offset
    local.get $out local.get $vp f32.store
    local.get $out i32.const 4 i32.add local.get $content f32.store
    local.get $out i32.const 8 i32.add local.get $offset f32.store
    i32.const 1)

  (func $er_ui_scroll_area_metrics_max_offset (export "er_ui_scroll_area_metrics_max_offset") (param $metrics i32) (result f32)
    local.get $metrics i32.eqz
    if f32.const 0 return end
    local.get $metrics i32.const 4 i32.add f32.load local.get $metrics f32.load f32.sub f32.const 0 call $max_f32)

  (func (export "er_ui_scroll_area_thumb_bounds") (param $track i32) (param $metrics i32) (param $out i32) (result i32)
    (local $ratio f32) (local $thumb_h f32) (local $travel f32) (local $max_offset f32) (local $offset_ratio f32)
    local.get $track i32.eqz local.get $metrics i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $metrics f32.load
    local.get $metrics f32.load local.get $metrics i32.const 4 i32.add f32.load call $max_f32
    f32.div
    f32.const 0 call $max_f32 f32.const 1 call $min_f32
    local.set $ratio
    local.get $track i32.const 12 i32.add f32.load local.get $ratio f32.mul f32.const 12 call $max_f32
    local.get $track i32.const 12 i32.add f32.load call $min_f32
    local.set $thumb_h
    local.get $track i32.const 12 i32.add f32.load local.get $thumb_h f32.sub f32.const 0 call $max_f32 local.set $travel
    local.get $metrics call $er_ui_scroll_area_metrics_max_offset local.set $max_offset
    local.get $max_offset f32.const 0 f32.eq
    if
      f32.const 0 local.set $offset_ratio
    else
      local.get $metrics i32.const 8 i32.add f32.load local.get $max_offset f32.div local.set $offset_ratio
    end
    local.get $out
    local.get $track f32.load
    local.get $track i32.const 4 i32.add f32.load local.get $travel local.get $offset_ratio f32.mul f32.add
    local.get $track i32.const 8 i32.add f32.load
    local.get $thumb_h
    call $rect_store)

  (func (export "er_ui_scroll_area_measure") (param $constraints i32) (param $out i32) (result i32)
    (local $label_w f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    call $er_ui_scroll_area_write_label i32.const 18 f32.const 14 call $er_ui_font_text_width local.set $label_w
    local.get $label_w f32.const 26 f32.add
    f32.const 24
    local.get $constraints i32.const 123164 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 123164 f32.load local.set $pref_w
    i32.const 123168 f32.load local.set $pref_h
    f32.const 27 local.get $pref_w call $min_f32
    f32.const 24 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
