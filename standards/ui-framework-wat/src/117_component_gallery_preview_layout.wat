  (func (export "er_ui_gallery_selected_preview_surface_h") (result f32) f32.const 266)
  (func (export "er_ui_gallery_selected_preview_compact_surface_h") (result f32) f32.const 320)
  (func (export "er_ui_gallery_catalog_preview_h") (result f32) f32.const 38)
  (func (export "er_ui_gallery_catalog_card_pad") (result f32) f32.const 14)

  (func $er_ui_gallery_split_left_rects (param $bounds i32) (param $width f32) (param $gap f32) (param $out i32) (result i32)
    (local $first_w f32) (local $rest_x f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $width f32.const 1 call $max_f32 local.get $bounds i32.const 8 i32.add f32.load call $min_f32 local.set $first_w
    local.get $bounds f32.load local.get $first_w f32.add local.get $gap f32.add local.set $rest_x
    local.get $out local.get $bounds f32.load local.get $bounds i32.const 4 i32.add f32.load local.get $first_w local.get $bounds i32.const 12 i32.add f32.load call $rect_store drop
    local.get $out i32.const 16 i32.add local.get $rest_x local.get $bounds i32.const 4 i32.add f32.load local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add local.get $rest_x f32.sub f32.const 1 call $max_f32 local.get $bounds i32.const 12 i32.add f32.load call $rect_store)

  (func (export "er_ui_gallery_selected_is_compact") (param $bounds i32) (result i32)
    local.get $bounds i32.eqz
    if i32.const 0 return end
    local.get $bounds i32.const 8 i32.add f32.load f32.const 36 f32.sub f32.const 720 f32.lt)

  (func (export "er_ui_gallery_selected_inset_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds local.get $out f32.const 18 call $er_ui_rect_inset_uniform)

  (func (export "er_ui_gallery_selected_part_bounds") (param $bounds i32) (param $part i32) (param $out i32) (result i32)
    (local $in_x f32) (local $in_y f32) (local $in_w f32) (local $in_h f32)
    (local $first_w f32) (local $gap f32) (local $second_x f32) (local $second_w f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds f32.load f32.const 18 f32.add local.set $in_x
    local.get $bounds i32.const 4 i32.add f32.load f32.const 18 f32.add local.set $in_y
    local.get $bounds i32.const 8 i32.add f32.load f32.const 36 f32.sub local.set $in_w
    local.get $bounds i32.const 12 i32.add f32.load f32.const 36 f32.sub local.set $in_h
    local.get $in_w f32.const 720 f32.lt
    if
      local.get $part i32.const 1 i32.eq
      if local.get $out local.get $in_x local.get $in_y f32.const 38 f32.add local.get $in_w f32.const 26 call $rect_store return end
      local.get $part i32.const 2 i32.eq
      if local.get $out local.get $in_x local.get $in_y f32.const 76 f32.add local.get $in_w f32.const 54 call $rect_store return end
      local.get $part i32.const 3 i32.eq
      if local.get $out local.get $in_x local.get $in_y f32.const 148 f32.add local.get $in_w f32.const 320 call $rect_store return end
      local.get $part i32.const 4 i32.eq
      if local.get $out local.get $in_x local.get $in_y f32.const 490 f32.add local.get $in_w f32.const 210 call $rect_store return end
      local.get $part i32.const 5 i32.eq
      if local.get $out local.get $in_x local.get $in_y f32.const 718 f32.add local.get $in_w f32.const 64 call $rect_store return end
      i32.const 0 return
    end
    local.get $in_w f32.const 0.42 f32.mul f32.const 1 call $max_f32 local.get $in_w call $min_f32 local.set $first_w
    local.get $in_w f32.const 0.06 f32.mul local.set $gap
    local.get $in_x local.get $first_w f32.add local.get $gap f32.add local.set $second_x
    local.get $in_x local.get $in_w f32.add local.get $second_x f32.sub f32.const 1 call $max_f32 local.set $second_w
    local.get $part i32.const 1 i32.eq
    if local.get $out local.get $in_x local.get $in_y f32.const 38 f32.add local.get $first_w f32.const 26 call $rect_store return end
    local.get $part i32.const 2 i32.eq
    if local.get $out local.get $in_x local.get $in_y f32.const 76 f32.add local.get $first_w f32.const 54 call $rect_store return end
    local.get $part i32.const 3 i32.eq
    if local.get $out local.get $second_x local.get $in_y f32.const 10 f32.add local.get $second_w f32.const 266 call $rect_store return end
    local.get $part i32.const 4 i32.eq
    if local.get $out local.get $in_x local.get $in_y f32.const 154 f32.add local.get $first_w local.get $in_h f32.const 154 f32.sub call $rect_store return end
    local.get $part i32.const 5 i32.eq
    if local.get $out local.get $second_x local.get $in_y f32.const 298 f32.add local.get $second_w local.get $in_h f32.const 298 f32.sub call $rect_store return end
    i32.const 0)

  (func (export "er_ui_gallery_opened_preview_inner_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load f32.const 18 f32.add local.get $bounds i32.const 4 i32.add f32.load f32.const 72 f32.add local.get $bounds i32.const 8 i32.add f32.load f32.const 36 f32.sub local.get $bounds i32.const 12 i32.add f32.load f32.const 92 f32.sub call $rect_store)

  (func (export "er_ui_gallery_opened_preview_slot_bounds") (param $bounds i32) (param $slot i32) (param $out i32) (result i32)
    (local $x f32) (local $y f32) (local $w f32) (local $h f32)
    (local $main_h f32) (local $second_h f32) (local $first_w f32) (local $rest_x f32) (local $rest_w f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or local.get $slot i32.const 2 i32.gt_u i32.or
    if i32.const 0 return end
    local.get $bounds f32.load f32.const 18 f32.add local.set $x
    local.get $bounds i32.const 4 i32.add f32.load f32.const 72 f32.add local.set $y
    local.get $bounds i32.const 8 i32.add f32.load f32.const 36 f32.sub local.set $w
    local.get $bounds i32.const 12 i32.add f32.load f32.const 92 f32.sub local.set $h
    local.get $w f32.const 360 f32.lt
    if
      f32.const 112 local.get $h f32.const 0.45 f32.mul call $min_f32 local.set $main_h
      f32.const 88 local.get $h f32.const 0.32 f32.mul call $min_f32 local.set $second_h
      local.get $slot i32.eqz
      if local.get $out local.get $x local.get $y local.get $w local.get $main_h call $rect_store return end
      local.get $slot i32.const 1 i32.eq
      if local.get $out local.get $x local.get $y local.get $main_h f32.add f32.const 14 f32.add local.get $w local.get $second_h call $rect_store return end
      local.get $out local.get $x local.get $y local.get $main_h f32.add f32.const 14 f32.add local.get $second_h f32.add f32.const 14 f32.add local.get $w local.get $second_h call $rect_store return
    end
    local.get $w f32.const 0.58 f32.mul f32.const 1 call $max_f32 local.get $w call $min_f32 local.set $first_w
    local.get $x local.get $first_w f32.add f32.const 14 f32.add local.set $rest_x
    local.get $x local.get $w f32.add local.get $rest_x f32.sub f32.const 1 call $max_f32 local.set $rest_w
    local.get $slot i32.eqz
    if local.get $out local.get $x local.get $y local.get $first_w local.get $h call $rect_store return end
    local.get $h f32.const 14 f32.sub f32.const 0.5 f32.mul local.set $second_h
    local.get $slot i32.const 1 i32.eq
    if local.get $out local.get $rest_x local.get $y local.get $rest_w local.get $second_h call $rect_store return end
    local.get $out local.get $rest_x local.get $y local.get $second_h f32.add f32.const 14 f32.add local.get $rest_w local.get $second_h call $rect_store)

  (func (export "er_ui_gallery_preview_slot_content_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds local.get $out f32.const 12 f32.const 32 f32.const 12 f32.const 12 call $er_ui_rect_inset_ltrb)

  (func (export "er_ui_gallery_api_label_h") (result f32) f32.const 14)
  (func (export "er_ui_gallery_api_value_line_h") (result f32) f32.const 16)
  (func (export "er_ui_gallery_api_value_avg_w") (result f32) f32.const 7.8)
  (func (export "er_ui_gallery_api_value_max_lines") (result i32) i32.const 2)

  (func $er_ui_gallery_api_len_for_field (param $index i32) (param $field i32) (result i32)
    local.get $index i32.const 60 i32.ge_u
    if i32.const -1 return end
    local.get $field i32.const 0 i32.eq
    if i32.const 66600 local.get $index i32.add i32.load8_u return end
    local.get $field i32.const 1 i32.eq
    if i32.const 66400 local.get $index i32.add i32.load8_u return end
    local.get $field i32.const 2 i32.eq
    if i32.const 66500 local.get $index i32.add i32.load8_u return end
    i32.const -1)

  (func $er_ui_gallery_api_wrapped_line_count_for_len (export "er_ui_gallery_api_wrapped_line_count_for_len") (param $value_len i32) (param $width f32) (result i32)
    (local $cap i32) (local $lines i32)
    local.get $value_len i32.eqz
    if i32.const 0 return end
    local.get $width f32.const 7.8 f32.div f32.const 1 call $max_f32 i32.trunc_f32_u local.set $cap
    local.get $value_len local.get $cap i32.add i32.const 1 i32.sub local.get $cap i32.div_u local.set $lines
    local.get $lines i32.const 2 i32.gt_u
    if i32.const 2 return end
    local.get $lines)

  (func $er_ui_gallery_api_value_height_for_len (export "er_ui_gallery_api_value_height_for_len") (param $value_len i32) (param $width f32) (result f32)
    local.get $value_len local.get $width call $er_ui_gallery_api_wrapped_line_count_for_len f32.convert_i32_u f32.const 16 f32.mul f32.const 16 call $max_f32)

  (func $er_ui_gallery_api_field_height_for_len (export "er_ui_gallery_api_field_height_for_len") (param $value_len i32) (param $width f32) (result f32)
    f32.const 20 local.get $value_len local.get $width call $er_ui_gallery_api_value_height_for_len f32.add)

  (func $er_ui_gallery_api_field_height_for_index (export "er_ui_gallery_api_field_height_for_index") (param $index i32) (param $field i32) (param $width f32) (result f32)
    (local $value_len i32)
    local.get $index local.get $field call $er_ui_gallery_api_len_for_field local.tee $value_len i32.const 0 i32.lt_s
    if f32.const -1 return end
    local.get $value_len local.get $width call $er_ui_gallery_api_field_height_for_len)

  (func (export "er_ui_gallery_api_content_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load f32.const 18 f32.add local.get $bounds i32.const 4 i32.add f32.load f32.const 50 f32.add local.get $bounds i32.const 8 i32.add f32.load f32.const 36 f32.sub f32.const 1 call $max_f32 local.get $bounds i32.const 12 i32.add f32.load f32.const 50 f32.sub f32.const 1 call $max_f32 call $rect_store)

  (func (export "er_ui_gallery_api_field_bounds") (param $bounds i32) (param $index i32) (param $field i32) (param $out i32) (result i32)
    (local $x f32) (local $y f32) (local $w f32) (local $h f32)
    (local $h0 f32) (local $h1 f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or local.get $field i32.const 2 i32.gt_u i32.or
    if i32.const 0 return end
    local.get $bounds f32.load f32.const 18 f32.add local.set $x
    local.get $bounds i32.const 4 i32.add f32.load f32.const 50 f32.add local.set $y
    local.get $bounds i32.const 8 i32.add f32.load f32.const 36 f32.sub f32.const 1 call $max_f32 local.set $w
    local.get $index local.get $field local.get $w call $er_ui_gallery_api_field_height_for_index local.tee $h f32.const 0 f32.lt
    if i32.const 0 return end
    local.get $field i32.const 1 i32.ge_u
    if
      local.get $index i32.const 0 local.get $w call $er_ui_gallery_api_field_height_for_index local.set $h0
      local.get $y local.get $h0 f32.add f32.const 12 f32.add local.set $y
    end
    local.get $field i32.const 2 i32.eq
    if
      local.get $index i32.const 1 local.get $w call $er_ui_gallery_api_field_height_for_index local.set $h1
      local.get $y local.get $h1 f32.add f32.const 12 f32.add local.set $y
    end
    local.get $out local.get $x local.get $y local.get $w local.get $h call $rect_store)
