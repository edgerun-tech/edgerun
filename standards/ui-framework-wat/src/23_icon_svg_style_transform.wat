  (func $er_ui_svg_color_parse_rgba (export "er_ui_svg_color_parse_rgba") (param $ptr i32) (param $len i32) (result i32)
    (local $idx i32) (local $end i32) (local $r i32) (local $g i32) (local $b i32) (local $a i32)
    (local $h0 i32) (local $h1 i32) (local $h2 i32) (local $h3 i32) (local $h4 i32) (local $h5 i32) (local $h6 i32) (local $h7 i32)
    (local $idx_slot i32) (local $fv f32)
    local.get $ptr i32.eqz if i32.const -1 return end
    local.get $ptr local.get $len i32.const 0 call $svg_skip_separators local.set $idx
    local.get $len local.set $end
    block $trim_done
      loop $trim_loop
        local.get $end local.get $idx i32.le_u br_if $trim_done
        local.get $ptr local.get $end i32.const 1 i32.sub i32.add i32.load8_u call $svg_is_ws
        i32.eqz
        if
          br $trim_done
        end
        local.get $end i32.const 1 i32.sub local.set $end
        br $trim_loop
      end
    end
    local.get $end local.get $idx i32.sub local.set $len
    local.get $len i32.const 0 i32.eq if i32.const -1 return end
    local.get $len i32.const 4 i32.eq
    local.get $ptr local.get $end local.get $idx i32.const 110 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 1 i32.add i32.const 111 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 2 i32.add i32.const 110 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 3 i32.add i32.const 101 call $svg_alpha_eq i32.and
    if
      i32.const -2 return
    end
    local.get $len i32.const 12 i32.eq
    local.get $ptr local.get $end local.get $idx i32.const 99 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 1 i32.add i32.const 117 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 2 i32.add i32.const 114 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 3 i32.add i32.const 114 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 4 i32.add i32.const 101 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 5 i32.add i32.const 110 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 6 i32.add i32.const 116 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 7 i32.add i32.const 67 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 8 i32.add i32.const 111 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 9 i32.add i32.const 108 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 10 i32.add i32.const 111 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 11 i32.add i32.const 114 call $svg_alpha_eq i32.and
    if
      i32.const -3 return
    end
    local.get $len i32.const 5 i32.ge_u
    local.get $ptr local.get $end local.get $idx i32.const 114 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 1 i32.add i32.const 103 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 2 i32.add i32.const 98 call $svg_alpha_eq i32.and
    if
      local.get $ptr local.get $end local.get $idx i32.const 3 i32.add i32.const 97 call $svg_alpha_eq
      if
        local.get $ptr local.get $end local.get $idx i32.const 4 i32.add i32.const 40 call $svg_byte_eq i32.eqz if i32.const -1 return end
        i32.const 65512 local.set $idx_slot
        local.get $idx_slot local.get $idx i32.const 5 i32.add i32.store
      else
        local.get $ptr local.get $end local.get $idx i32.const 3 i32.add i32.const 40 call $svg_byte_eq i32.eqz if i32.const -1 return end
        i32.const 65512 local.set $idx_slot
        local.get $idx_slot local.get $idx i32.const 4 i32.add i32.store
      end
      local.get $ptr local.get $end local.get $idx_slot call $svg_parse_number i32.trunc_f32_s local.set $r
      local.get $ptr local.get $end local.get $idx_slot call $svg_parse_number i32.trunc_f32_s local.set $g
      local.get $ptr local.get $end local.get $idx_slot call $svg_parse_number i32.trunc_f32_s local.set $b
      local.get $idx_slot i32.load i32.const -1 i32.eq if i32.const -1 return end
      i32.const 255 local.set $a
      local.get $ptr local.get $end local.get $idx_slot i32.load call $svg_skip_separators local.set $idx
      local.get $ptr local.get $end local.get $idx i32.const 41 call $svg_byte_eq
      if
      else
        local.get $ptr local.get $end local.get $idx_slot call $svg_parse_number local.set $fv
        local.get $idx_slot i32.load i32.const -1 i32.eq if i32.const -1 return end
        local.get $fv f32.const 1 f32.le
        if
          local.get $fv f32.const 255 f32.mul i32.trunc_f32_s local.set $a
        else
          local.get $fv i32.trunc_f32_s local.set $a
        end
        local.get $ptr local.get $end local.get $idx_slot i32.load call $svg_skip_separators local.set $idx
        local.get $ptr local.get $end local.get $idx i32.const 41 call $svg_byte_eq i32.eqz if i32.const -1 return end
      end
      local.get $r i32.const 0 i32.lt_s if i32.const 0 local.set $r end
      local.get $g i32.const 0 i32.lt_s if i32.const 0 local.set $g end
      local.get $b i32.const 0 i32.lt_s if i32.const 0 local.set $b end
      local.get $a i32.const 0 i32.lt_s if i32.const 0 local.set $a end
      local.get $r i32.const 255 i32.gt_s if i32.const 255 local.set $r end
      local.get $g i32.const 255 i32.gt_s if i32.const 255 local.set $g end
      local.get $b i32.const 255 i32.gt_s if i32.const 255 local.set $b end
      local.get $a i32.const 255 i32.gt_s if i32.const 255 local.set $a end
      local.get $r local.get $g local.get $b local.get $a call $svg_color_pack return
    end
    local.get $ptr local.get $end local.get $idx i32.const 35 call $svg_byte_eq i32.eqz if i32.const -1 return end
    local.get $len i32.const 4 i32.eq
    if
      local.get $ptr local.get $idx i32.const 1 i32.add i32.add i32.load8_u call $svg_hex_value local.tee $h0 i32.const 0 i32.lt_s if i32.const -1 return end
      local.get $ptr local.get $idx i32.const 2 i32.add i32.add i32.load8_u call $svg_hex_value local.tee $h1 i32.const 0 i32.lt_s if i32.const -1 return end
      local.get $ptr local.get $idx i32.const 3 i32.add i32.add i32.load8_u call $svg_hex_value local.tee $h2 i32.const 0 i32.lt_s if i32.const -1 return end
      local.get $h0 i32.const 17 i32.mul local.set $r
      local.get $h1 i32.const 17 i32.mul local.set $g
      local.get $h2 i32.const 17 i32.mul local.set $b
      i32.const 255 local.set $a
      local.get $r local.get $g local.get $b local.get $a call $svg_color_pack return
    end
    local.get $len i32.const 5 i32.eq
    if
      local.get $ptr local.get $idx i32.const 1 i32.add i32.add i32.load8_u call $svg_hex_value local.tee $h0 i32.const 0 i32.lt_s if i32.const -1 return end
      local.get $ptr local.get $idx i32.const 2 i32.add i32.add i32.load8_u call $svg_hex_value local.tee $h1 i32.const 0 i32.lt_s if i32.const -1 return end
      local.get $ptr local.get $idx i32.const 3 i32.add i32.add i32.load8_u call $svg_hex_value local.tee $h2 i32.const 0 i32.lt_s if i32.const -1 return end
      local.get $ptr local.get $idx i32.const 4 i32.add i32.add i32.load8_u call $svg_hex_value local.tee $h3 i32.const 0 i32.lt_s if i32.const -1 return end
      local.get $h0 i32.const 17 i32.mul local.set $r
      local.get $h1 i32.const 17 i32.mul local.set $g
      local.get $h2 i32.const 17 i32.mul local.set $b
      local.get $h3 i32.const 17 i32.mul local.set $a
      local.get $r local.get $g local.get $b local.get $a call $svg_color_pack return
    end
    local.get $len i32.const 7 i32.eq
    local.get $len i32.const 9 i32.eq
    i32.or
    if
      local.get $ptr local.get $idx i32.const 1 i32.add i32.add i32.load8_u call $svg_hex_value local.tee $h0 i32.const 0 i32.lt_s if i32.const -1 return end
      local.get $ptr local.get $idx i32.const 2 i32.add i32.add i32.load8_u call $svg_hex_value local.tee $h1 i32.const 0 i32.lt_s if i32.const -1 return end
      local.get $ptr local.get $idx i32.const 3 i32.add i32.add i32.load8_u call $svg_hex_value local.tee $h2 i32.const 0 i32.lt_s if i32.const -1 return end
      local.get $ptr local.get $idx i32.const 4 i32.add i32.add i32.load8_u call $svg_hex_value local.tee $h3 i32.const 0 i32.lt_s if i32.const -1 return end
      local.get $ptr local.get $idx i32.const 5 i32.add i32.add i32.load8_u call $svg_hex_value local.tee $h4 i32.const 0 i32.lt_s if i32.const -1 return end
      local.get $ptr local.get $idx i32.const 6 i32.add i32.add i32.load8_u call $svg_hex_value local.tee $h5 i32.const 0 i32.lt_s if i32.const -1 return end
      local.get $h0 i32.const 4 i32.shl local.get $h1 i32.or local.set $r
      local.get $h2 i32.const 4 i32.shl local.get $h3 i32.or local.set $g
      local.get $h4 i32.const 4 i32.shl local.get $h5 i32.or local.set $b
      i32.const 255 local.set $a
      local.get $len i32.const 9 i32.eq
      if
        local.get $ptr local.get $idx i32.const 7 i32.add i32.add i32.load8_u call $svg_hex_value local.tee $h6 i32.const 0 i32.lt_s if i32.const -1 return end
        local.get $ptr local.get $idx i32.const 8 i32.add i32.add i32.load8_u call $svg_hex_value local.tee $h7 i32.const 0 i32.lt_s if i32.const -1 return end
        local.get $h6 i32.const 4 i32.shl local.get $h7 i32.or local.set $a
      end
      local.get $r local.get $g local.get $b local.get $a call $svg_color_pack return
    end
    i32.const -1)

  (func $er_ui_svg_opacity_parse_alpha (export "er_ui_svg_opacity_parse_alpha") (param $ptr i32) (param $len i32) (result i32)
    (local $idx i32) (local $end i32) (local $idx_slot i32) (local $value f32) (local $alpha i32)
    local.get $ptr i32.eqz if i32.const -1 return end
    local.get $ptr local.get $len i32.const 0 call $svg_skip_separators local.set $idx
    local.get $len local.set $end
    block $trim_done
      loop $trim_loop
        local.get $end local.get $idx i32.le_u br_if $trim_done
        local.get $ptr local.get $end i32.const 1 i32.sub i32.add i32.load8_u call $svg_is_ws
        i32.eqz
        if
          br $trim_done
        end
        local.get $end i32.const 1 i32.sub local.set $end
        br $trim_loop
      end
    end
    local.get $end local.get $idx i32.sub i32.const 0 i32.eq if i32.const -1 return end
    i32.const 65508 local.set $idx_slot
    local.get $idx_slot local.get $idx i32.store
    local.get $ptr local.get $end local.get $idx_slot call $svg_parse_number local.set $value
    local.get $idx_slot i32.load i32.const -1 i32.eq if i32.const -1 return end
    local.get $ptr local.get $end local.get $idx_slot i32.load call $svg_skip_separators local.set $idx
    local.get $idx local.get $end i32.ne if i32.const -1 return end
    local.get $value f32.const 1 f32.le
    if
      local.get $value f32.const 255 f32.mul i32.trunc_f32_s local.set $alpha
    else
      local.get $value i32.trunc_f32_s local.set $alpha
    end
    local.get $alpha i32.const 0 i32.lt_s if i32.const 0 local.set $alpha end
    local.get $alpha i32.const 255 i32.gt_s if i32.const 255 local.set $alpha end
    local.get $alpha)

  (func $er_ui_svg_color_apply_alpha (export "er_ui_svg_color_apply_alpha") (param $rgba i32) (param $alpha i32) (result i32)
    (local $base_alpha i32) (local $out_alpha i32)
    local.get $rgba i32.const -1 i32.eq
    local.get $rgba i32.const -2 i32.eq i32.or
    local.get $rgba i32.const -3 i32.eq i32.or
    if
      local.get $rgba return
    end
    local.get $alpha i32.const 0 i32.lt_s if i32.const 0 local.set $alpha end
    local.get $alpha i32.const 255 i32.gt_s if i32.const 255 local.set $alpha end
    local.get $rgba i32.const 24 i32.shr_u i32.const 255 i32.and local.set $base_alpha
    local.get $base_alpha local.get $alpha i32.mul i32.const 255 i32.div_u local.set $out_alpha
    local.get $rgba i32.const 0x00ffffff i32.and
    local.get $out_alpha i32.const 24 i32.shl
    i32.or)

  (func $svg_trim_start (param $ptr i32) (param $len i32) (result i32)
    local.get $ptr local.get $len i32.const 0 call $svg_skip_separators)

  (func $svg_trim_end (param $ptr i32) (param $start i32) (param $len i32) (result i32)
    block $done
      loop $loop
        local.get $len local.get $start i32.le_u br_if $done
        local.get $ptr local.get $len i32.const 1 i32.sub i32.add i32.load8_u call $svg_is_ws
        i32.eqz br_if $done
        local.get $len i32.const 1 i32.sub local.set $len
        br $loop
      end
    end
    local.get $len)

  (func $er_ui_svg_stroke_cap_parse (export "er_ui_svg_stroke_cap_parse") (param $ptr i32) (param $len i32) (result i32)
    (local $idx i32) (local $end i32) (local $n i32)
    local.get $ptr i32.eqz if i32.const -1 return end
    local.get $ptr local.get $len call $svg_trim_start local.set $idx
    local.get $ptr local.get $idx local.get $len call $svg_trim_end local.set $end
    local.get $end local.get $idx i32.sub local.set $n
    local.get $n i32.const 4 i32.eq
    local.get $ptr local.get $end local.get $idx i32.const 98 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 1 i32.add i32.const 117 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 2 i32.add i32.const 116 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 3 i32.add i32.const 116 call $svg_alpha_eq i32.and
    if i32.const 0 return end
    local.get $n i32.const 5 i32.eq
    local.get $ptr local.get $end local.get $idx i32.const 114 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 1 i32.add i32.const 111 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 2 i32.add i32.const 117 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 3 i32.add i32.const 110 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 4 i32.add i32.const 100 call $svg_alpha_eq i32.and
    if i32.const 1 return end
    local.get $n i32.const 6 i32.eq
    local.get $ptr local.get $end local.get $idx i32.const 115 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 1 i32.add i32.const 113 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 2 i32.add i32.const 117 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 3 i32.add i32.const 97 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 4 i32.add i32.const 114 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 5 i32.add i32.const 101 call $svg_alpha_eq i32.and
    if i32.const 2 return end
    i32.const -1)

  (func $er_ui_svg_stroke_join_parse (export "er_ui_svg_stroke_join_parse") (param $ptr i32) (param $len i32) (result i32)
    (local $idx i32) (local $end i32) (local $n i32)
    local.get $ptr i32.eqz if i32.const -1 return end
    local.get $ptr local.get $len call $svg_trim_start local.set $idx
    local.get $ptr local.get $idx local.get $len call $svg_trim_end local.set $end
    local.get $end local.get $idx i32.sub local.set $n
    local.get $n i32.const 5 i32.eq
    local.get $ptr local.get $end local.get $idx i32.const 109 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 1 i32.add i32.const 105 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 2 i32.add i32.const 116 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 3 i32.add i32.const 101 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 4 i32.add i32.const 114 call $svg_alpha_eq i32.and
    if i32.const 0 return end
    local.get $n i32.const 5 i32.eq
    local.get $ptr local.get $end local.get $idx i32.const 114 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 1 i32.add i32.const 111 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 2 i32.add i32.const 117 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 3 i32.add i32.const 110 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 4 i32.add i32.const 100 call $svg_alpha_eq i32.and
    if i32.const 1 return end
    local.get $n i32.const 5 i32.eq
    local.get $ptr local.get $end local.get $idx i32.const 98 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 1 i32.add i32.const 101 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 2 i32.add i32.const 118 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 3 i32.add i32.const 101 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 4 i32.add i32.const 108 call $svg_alpha_eq i32.and
    if i32.const 2 return end
    i32.const -1)

  (func $er_ui_svg_fill_rule_parse (export "er_ui_svg_fill_rule_parse") (param $ptr i32) (param $len i32) (result i32)
    (local $idx i32) (local $end i32) (local $n i32)
    local.get $ptr i32.eqz if i32.const -1 return end
    local.get $ptr local.get $len call $svg_trim_start local.set $idx
    local.get $ptr local.get $idx local.get $len call $svg_trim_end local.set $end
    local.get $end local.get $idx i32.sub local.set $n
    local.get $n i32.const 7 i32.eq
    local.get $ptr local.get $end local.get $idx i32.const 110 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 1 i32.add i32.const 111 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 2 i32.add i32.const 110 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 3 i32.add i32.const 122 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 4 i32.add i32.const 101 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 5 i32.add i32.const 114 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 6 i32.add i32.const 111 call $svg_alpha_eq i32.and
    if i32.const 0 return end
    local.get $n i32.const 7 i32.eq
    local.get $ptr local.get $end local.get $idx i32.const 101 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 1 i32.add i32.const 118 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 2 i32.add i32.const 101 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 3 i32.add i32.const 110 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 4 i32.add i32.const 111 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 5 i32.add i32.const 100 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 6 i32.add i32.const 100 call $svg_alpha_eq i32.and
    if i32.const 1 return end
    i32.const -1)

  (func (export "er_ui_svg_style_record_init") (param $out i32) (result i32)
    local.get $out i32.eqz if i32.const -1 return end
    local.get $out i32.const 0 i32.store
    local.get $out i32.const 4 i32.add i32.const -16777216 i32.store
    local.get $out i32.const 8 i32.add i32.const -2 i32.store
    local.get $out i32.const 12 i32.add f32.const 0.083333336 f32.store
    local.get $out i32.const 16 i32.add i32.const 255 i32.store
    local.get $out i32.const 20 i32.add i32.const 255 i32.store
    local.get $out i32.const 24 i32.add i32.const 255 i32.store
    local.get $out i32.const 28 i32.add i32.const 0 i32.store
    local.get $out i32.const 32 i32.add i32.const 0 i32.store
    local.get $out i32.const 36 i32.add i32.const 0 i32.store
    local.get $out i32.const 40 i32.add f32.const 4 f32.store
    local.get $out i32.const 44 i32.add f32.const 0 f32.store
    local.get $out i32.const 48 i32.add f32.const 0 f32.store
    local.get $out i32.const 52 i32.add f32.const 0 f32.store
    local.get $out i32.const 56 i32.add i32.const -16777216 i32.store
    local.get $out i32.const 60 i32.add i32.const 1 i32.store
    local.get $out i32.const 64 i32.add i32.const 1 i32.store
    i32.const 1)

  (func (export "er_ui_svg_style_record_inherit") (param $parent i32) (param $out i32) (result i32)
    local.get $parent i32.eqz
    local.get $out i32.eqz i32.or
    if i32.const -1 return end
    local.get $out
    local.get $parent
    i32.const 68
    memory.copy
    local.get $out
    i32.const 0
    i32.store
    i32.const 1)

  (func $er_ui_svg_style_resolve_paint (export "er_ui_svg_style_resolve_paint") (param $style i32) (param $which i32) (result i32)
    (local $paint i32) (local $alpha i32)
    local.get $style i32.eqz if i32.const -1 return end
    local.get $style i32.const 60 i32.add i32.load i32.eqz
    local.get $style i32.const 64 i32.add i32.load i32.eqz i32.or
    if i32.const -2 return end
    local.get $which
    if
      local.get $style i32.const 8 i32.add i32.load local.set $paint
      local.get $style i32.const 24 i32.add i32.load local.set $alpha
    else
      local.get $style i32.const 4 i32.add i32.load local.set $paint
      local.get $style i32.const 20 i32.add i32.load local.set $alpha
    end
    local.get $paint i32.const -2 i32.eq if i32.const -2 return end
    local.get $paint i32.const -3 i32.eq
    if
      local.get $style i32.const 56 i32.add i32.load local.set $paint
    end
    local.get $paint local.get $style i32.const 16 i32.add i32.load call $er_ui_svg_color_apply_alpha
    local.get $alpha call $er_ui_svg_color_apply_alpha)

  (func $svg_style_parse_number_attr (param $ptr i32) (param $len i32) (result f32)
    (local $idx i32) (local $end i32) (local $slot i32) (local $v f32)
    local.get $ptr local.get $len call $svg_trim_start local.set $idx
    local.get $ptr local.get $idx local.get $len call $svg_trim_end local.set $end
    i32.const 65504 local.set $slot
    local.get $slot local.get $idx i32.store
    local.get $ptr local.get $end local.get $slot call $svg_parse_number local.set $v
    local.get $slot i32.load i32.const -1 i32.eq if f32.const -340282346638528859811704183484516925440 return end
    local.get $ptr local.get $end local.get $slot i32.load call $svg_skip_separators local.get $end i32.ne
    if f32.const -340282346638528859811704183484516925440 return end
    local.get $v)

  (func $er_ui_svg_length_parse_normalized (export "er_ui_svg_length_parse_normalized") (param $ptr i32) (param $len i32) (param $scale f32) (result f32)
    (local $idx i32) (local $end i32) (local $slot i32) (local $v f32)
    local.get $ptr i32.eqz
    local.get $scale f32.const 0 f32.le i32.or
    if f32.const -340282346638528859811704183484516925440 return end
    local.get $ptr local.get $len call $svg_trim_start local.set $idx
    local.get $ptr local.get $idx local.get $len call $svg_trim_end local.set $end
    i32.const 65496 local.set $slot
    local.get $slot local.get $idx i32.store
    local.get $ptr local.get $end local.get $slot call $svg_parse_number local.set $v
    local.get $slot i32.load i32.const -1 i32.eq if f32.const -340282346638528859811704183484516925440 return end
    local.get $ptr local.get $end local.get $slot i32.load call $svg_skip_separators local.set $idx
    local.get $idx local.get $end i32.eq
    if
      local.get $v local.get $scale f32.div
      return
    end
    local.get $idx i32.const 1 i32.add local.get $end i32.eq
    local.get $ptr local.get $end local.get $idx i32.const 37 call $svg_byte_eq i32.and
    if
      local.get $v f32.const 100 f32.div
      return
    end
    local.get $idx i32.const 2 i32.add local.get $end i32.eq
    local.get $ptr local.get $end local.get $idx i32.const 112 call $svg_alpha_eq i32.and
    local.get $ptr local.get $end local.get $idx i32.const 1 i32.add i32.const 120 call $svg_alpha_eq i32.and
    if
      local.get $v local.get $scale f32.div
      return
    end
    f32.const -340282346638528859811704183484516925440)

  (func $er_ui_svg_style_attr_apply (export "er_ui_svg_style_attr_apply") (param $name_ptr i32) (param $name_len i32) (param $value_ptr i32) (param $value_len i32) (param $out i32) (result i32)
    (local $idx i32) (local $end i32) (local $n i32) (local $flags i32) (local $iv i32) (local $fv f32) (local $slot i32)
    local.get $name_ptr i32.eqz
    local.get $value_ptr i32.eqz i32.or
    local.get $out i32.eqz i32.or
    if i32.const -1 return end
    local.get $out i32.load local.set $flags
    local.get $name_ptr local.get $name_len call $svg_trim_start local.set $idx
    local.get $name_ptr local.get $idx local.get $name_len call $svg_trim_end local.set $end
    local.get $end local.get $idx i32.sub local.set $n
    local.get $n i32.const 4 i32.eq
    local.get $name_ptr local.get $end local.get $idx i32.const 102 call $svg_alpha_eq i32.and
    local.get $name_ptr local.get $end local.get $idx i32.const 1 i32.add i32.const 105 call $svg_alpha_eq i32.and
    local.get $name_ptr local.get $end local.get $idx i32.const 2 i32.add i32.const 108 call $svg_alpha_eq i32.and
    local.get $name_ptr local.get $end local.get $idx i32.const 3 i32.add i32.const 108 call $svg_alpha_eq i32.and
    if
      local.get $value_ptr local.get $value_len call $er_ui_svg_color_parse_rgba local.tee $iv i32.const -1 i32.eq if i32.const -1 return end
      local.get $out i32.const 4 i32.add local.get $iv i32.store
      local.get $out local.get $flags i32.const 1 i32.or i32.store
      i32.const 1 return
    end
    local.get $n i32.const 5 i32.eq
    local.get $name_ptr local.get $end local.get $idx i32.const 99 call $svg_alpha_eq i32.and
    local.get $name_ptr local.get $end local.get $idx i32.const 1 i32.add i32.const 111 call $svg_alpha_eq i32.and
    local.get $name_ptr local.get $end local.get $idx i32.const 2 i32.add i32.const 108 call $svg_alpha_eq i32.and
    local.get $name_ptr local.get $end local.get $idx i32.const 3 i32.add i32.const 111 call $svg_alpha_eq i32.and
    local.get $name_ptr local.get $end local.get $idx i32.const 4 i32.add i32.const 114 call $svg_alpha_eq i32.and
    if
      local.get $value_ptr local.get $value_len call $er_ui_svg_color_parse_rgba local.tee $iv i32.const -1 i32.eq if i32.const -1 return end
      local.get $iv i32.const -2 i32.eq if i32.const -1 return end
      local.get $out i32.const 56 i32.add local.get $iv i32.store
      local.get $out local.get $flags i32.const 4096 i32.or i32.store
      i32.const 1 return
    end
    local.get $n i32.const 6 i32.eq
    local.get $name_ptr local.get $end local.get $idx i32.const 115 call $svg_alpha_eq i32.and
    local.get $name_ptr local.get $end local.get $idx i32.const 1 i32.add i32.const 116 call $svg_alpha_eq i32.and
    local.get $name_ptr local.get $end local.get $idx i32.const 2 i32.add i32.const 114 call $svg_alpha_eq i32.and
    local.get $name_ptr local.get $end local.get $idx i32.const 3 i32.add i32.const 111 call $svg_alpha_eq i32.and
    local.get $name_ptr local.get $end local.get $idx i32.const 4 i32.add i32.const 107 call $svg_alpha_eq i32.and
    local.get $name_ptr local.get $end local.get $idx i32.const 5 i32.add i32.const 101 call $svg_alpha_eq i32.and
    if
      local.get $value_ptr local.get $value_len call $er_ui_svg_color_parse_rgba local.tee $iv i32.const -1 i32.eq if i32.const -1 return end
      local.get $out i32.const 8 i32.add local.get $iv i32.store
      local.get $out local.get $flags i32.const 2 i32.or i32.store
      i32.const 1 return
    end
    local.get $n i32.const 7 i32.eq
    local.get $name_ptr local.get $end local.get $idx i32.const 111 call $svg_alpha_eq i32.and
    local.get $name_ptr local.get $end local.get $idx i32.const 1 i32.add i32.const 112 call $svg_alpha_eq i32.and
    local.get $name_ptr local.get $end local.get $idx i32.const 2 i32.add i32.const 97 call $svg_alpha_eq i32.and
    local.get $name_ptr local.get $end local.get $idx i32.const 3 i32.add i32.const 99 call $svg_alpha_eq i32.and
    local.get $name_ptr local.get $end local.get $idx i32.const 4 i32.add i32.const 105 call $svg_alpha_eq i32.and
    local.get $name_ptr local.get $end local.get $idx i32.const 5 i32.add i32.const 116 call $svg_alpha_eq i32.and
    local.get $name_ptr local.get $end local.get $idx i32.const 6 i32.add i32.const 121 call $svg_alpha_eq i32.and
    if
      local.get $value_ptr local.get $value_len call $er_ui_svg_opacity_parse_alpha local.tee $iv i32.const -1 i32.eq if i32.const -1 return end
      local.get $out i32.const 16 i32.add local.get $iv i32.store
      local.get $out local.get $flags i32.const 8 i32.or i32.store
      i32.const 1 return
    end
    local.get $n i32.const 12 i32.eq
    local.get $name_ptr local.get $end local.get $idx i32.const 115 call $svg_alpha_eq i32.and
    local.get $name_ptr local.get $end local.get $idx i32.const 7 i32.add i32.const 119 call $svg_alpha_eq i32.and
    if
      local.get $value_ptr local.get $value_len call $svg_style_parse_number_attr local.tee $fv
      f32.const -340282346638528859811704183484516925440 f32.eq if i32.const -1 return end
      local.get $out i32.const 12 i32.add local.get $fv f32.store
      local.get $out local.get $flags i32.const 4 i32.or i32.store
      i32.const 1 return
    end
    local.get $n i32.const 12 i32.eq
    local.get $name_ptr local.get $end local.get $idx i32.const 102 call $svg_alpha_eq i32.and
    local.get $name_ptr local.get $end local.get $idx i32.const 5 i32.add i32.const 111 call $svg_alpha_eq i32.and
    if
      local.get $value_ptr local.get $value_len call $er_ui_svg_opacity_parse_alpha local.tee $iv i32.const -1 i32.eq if i32.const -1 return end
      local.get $out i32.const 20 i32.add local.get $iv i32.store
      local.get $out local.get $flags i32.const 16 i32.or i32.store
      i32.const 1 return
    end
    local.get $n i32.const 14 i32.eq
    local.get $name_ptr local.get $end local.get $idx i32.const 115 call $svg_alpha_eq i32.and
    local.get $name_ptr local.get $end local.get $idx i32.const 7 i32.add i32.const 111 call $svg_alpha_eq i32.and
    if
      local.get $value_ptr local.get $value_len call $er_ui_svg_opacity_parse_alpha local.tee $iv i32.const -1 i32.eq if i32.const -1 return end
      local.get $out i32.const 24 i32.add local.get $iv i32.store
      local.get $out local.get $flags i32.const 32 i32.or i32.store
      i32.const 1 return
    end
    local.get $n i32.const 14 i32.eq
    local.get $name_ptr local.get $end local.get $idx i32.const 115 call $svg_alpha_eq i32.and
    local.get $name_ptr local.get $end local.get $idx i32.const 7 i32.add i32.const 108 call $svg_alpha_eq i32.and
    if
      local.get $value_ptr local.get $value_len call $er_ui_svg_stroke_cap_parse local.tee $iv i32.const -1 i32.eq if i32.const -1 return end
      local.get $out i32.const 28 i32.add local.get $iv i32.store
      local.get $out local.get $flags i32.const 64 i32.or i32.store
      i32.const 1 return
    end
    local.get $n i32.const 15 i32.eq
    local.get $name_ptr local.get $end local.get $idx i32.const 115 call $svg_alpha_eq i32.and
    local.get $name_ptr local.get $end local.get $idx i32.const 11 i32.add i32.const 106 call $svg_alpha_eq i32.and
    if
      local.get $value_ptr local.get $value_len call $er_ui_svg_stroke_join_parse local.tee $iv i32.const -1 i32.eq if i32.const -1 return end
      local.get $out i32.const 32 i32.add local.get $iv i32.store
      local.get $out local.get $flags i32.const 128 i32.or i32.store
      i32.const 1 return
    end
    local.get $n i32.const 9 i32.eq
    local.get $name_ptr local.get $end local.get $idx i32.const 102 call $svg_alpha_eq i32.and
    local.get $name_ptr local.get $end local.get $idx i32.const 5 i32.add i32.const 114 call $svg_alpha_eq i32.and
    if
      local.get $value_ptr local.get $value_len call $er_ui_svg_fill_rule_parse local.tee $iv i32.const -1 i32.eq if i32.const -1 return end
      local.get $out i32.const 36 i32.add local.get $iv i32.store
      local.get $out local.get $flags i32.const 256 i32.or i32.store
      i32.const 1 return
    end
    local.get $n i32.const 17 i32.eq
    local.get $name_ptr local.get $end local.get $idx i32.const 115 call $svg_alpha_eq i32.and
    local.get $name_ptr local.get $end local.get $idx i32.const 7 i32.add i32.const 109 call $svg_alpha_eq i32.and
    if
      local.get $value_ptr local.get $value_len call $svg_style_parse_number_attr local.tee $fv
      f32.const -340282346638528859811704183484516925440 f32.eq if i32.const -1 return end
      local.get $out i32.const 40 i32.add local.get $fv f32.store
      local.get $out local.get $flags i32.const 512 i32.or i32.store
      i32.const 1 return
    end
    local.get $n i32.const 16 i32.eq
    local.get $name_ptr local.get $end local.get $idx i32.const 115 call $svg_alpha_eq i32.and
    local.get $name_ptr local.get $end local.get $idx i32.const 7 i32.add i32.const 100 call $svg_alpha_eq i32.and
    if
      i32.const 65500 local.set $slot
      local.get $value_ptr local.get $value_len call $svg_trim_start local.set $idx
      local.get $value_ptr local.get $idx local.get $value_len call $svg_trim_end local.set $end
      local.get $end local.get $idx i32.sub i32.const 4 i32.eq
      local.get $value_ptr local.get $end local.get $idx i32.const 110 call $svg_alpha_eq i32.and
      local.get $value_ptr local.get $end local.get $idx i32.const 1 i32.add i32.const 111 call $svg_alpha_eq i32.and
      local.get $value_ptr local.get $end local.get $idx i32.const 2 i32.add i32.const 110 call $svg_alpha_eq i32.and
      local.get $value_ptr local.get $end local.get $idx i32.const 3 i32.add i32.const 101 call $svg_alpha_eq i32.and
      if
        local.get $out i32.const 44 i32.add f32.const 0 f32.store
        local.get $out i32.const 48 i32.add f32.const 0 f32.store
        local.get $out
        local.get $flags
        i32.const -1025
        i32.and
        i32.store
        i32.const 1 return
      end
      local.get $slot local.get $idx i32.store
      local.get $value_ptr local.get $end local.get $slot call $svg_parse_number local.set $fv
      local.get $slot i32.load i32.const -1 i32.eq if i32.const -1 return end
      local.get $out i32.const 44 i32.add local.get $fv f32.store
      local.get $value_ptr local.get $end local.get $slot i32.load call $svg_skip_separators local.set $idx
      local.get $idx local.get $end i32.ge_u
      if
        local.get $out i32.const 48 i32.add local.get $fv f32.store
        local.get $out local.get $flags i32.const 1024 i32.or i32.store
        i32.const 1 return
      end
      local.get $value_ptr local.get $end local.get $slot call $svg_parse_number local.set $fv
      local.get $slot i32.load i32.const -1 i32.eq if i32.const -1 return end
      local.get $out i32.const 48 i32.add local.get $fv f32.store
      local.get $value_ptr local.get $end local.get $slot i32.load call $svg_skip_separators local.get $end i32.ne
      if i32.const -1 return end
      local.get $out local.get $flags i32.const 1024 i32.or i32.store
      i32.const 1 return
    end
    local.get $n i32.const 7 i32.eq
    local.get $name_ptr local.get $end local.get $idx i32.const 100 call $svg_alpha_eq i32.and
    if
      local.get $value_ptr local.get $value_len call $svg_trim_start local.set $idx
      local.get $value_ptr local.get $idx local.get $value_len call $svg_trim_end local.set $end
      local.get $end local.get $idx i32.sub i32.const 4 i32.eq
      local.get $value_ptr local.get $end local.get $idx i32.const 110 call $svg_alpha_eq i32.and
      local.get $value_ptr local.get $end local.get $idx i32.const 1 i32.add i32.const 111 call $svg_alpha_eq i32.and
      local.get $value_ptr local.get $end local.get $idx i32.const 2 i32.add i32.const 110 call $svg_alpha_eq i32.and
      local.get $value_ptr local.get $end local.get $idx i32.const 3 i32.add i32.const 101 call $svg_alpha_eq i32.and
      if
        local.get $out i32.const 60 i32.add i32.const 0 i32.store
      else
        local.get $out i32.const 60 i32.add i32.const 1 i32.store
      end
      local.get $out local.get $flags i32.const 8192 i32.or i32.store
      i32.const 1 return
    end
    local.get $n i32.const 10 i32.eq
    local.get $name_ptr local.get $end local.get $idx i32.const 118 call $svg_alpha_eq i32.and
    if
      local.get $value_ptr local.get $value_len call $svg_trim_start local.set $idx
      local.get $value_ptr local.get $idx local.get $value_len call $svg_trim_end local.set $end
      local.get $end local.get $idx i32.sub i32.const 6 i32.eq
      local.get $value_ptr local.get $end local.get $idx i32.const 104 call $svg_alpha_eq i32.and
      local.get $value_ptr local.get $end local.get $idx i32.const 1 i32.add i32.const 105 call $svg_alpha_eq i32.and
      local.get $value_ptr local.get $end local.get $idx i32.const 2 i32.add i32.const 100 call $svg_alpha_eq i32.and
      local.get $value_ptr local.get $end local.get $idx i32.const 3 i32.add i32.const 100 call $svg_alpha_eq i32.and
      local.get $value_ptr local.get $end local.get $idx i32.const 4 i32.add i32.const 101 call $svg_alpha_eq i32.and
      local.get $value_ptr local.get $end local.get $idx i32.const 5 i32.add i32.const 110 call $svg_alpha_eq i32.and
      if
        local.get $out i32.const 64 i32.add i32.const 0 i32.store
      else
        local.get $out i32.const 64 i32.add i32.const 1 i32.store
      end
      local.get $out local.get $flags i32.const 16384 i32.or i32.store
      i32.const 1 return
    end
    local.get $n i32.const 17 i32.eq
    local.get $name_ptr local.get $end local.get $idx i32.const 115 call $svg_alpha_eq i32.and
    local.get $name_ptr local.get $end local.get $idx i32.const 11 i32.add i32.const 111 call $svg_alpha_eq i32.and
    if
      local.get $value_ptr local.get $value_len call $svg_style_parse_number_attr local.tee $fv
      f32.const -340282346638528859811704183484516925440 f32.eq if i32.const -1 return end
      local.get $out i32.const 52 i32.add local.get $fv f32.store
      local.get $out local.get $flags i32.const 2048 i32.or i32.store
      i32.const 1 return
    end
    i32.const 0)

  (func (export "er_ui_svg_style_attr_apply_normalized") (param $name_ptr i32) (param $name_len i32) (param $value_ptr i32) (param $value_len i32) (param $out i32) (param $scale f32) (result i32)
    (local $idx i32) (local $end i32) (local $n i32) (local $flags i32) (local $fv f32) (local $slot i32)
    local.get $name_ptr i32.eqz
    local.get $value_ptr i32.eqz i32.or
    local.get $out i32.eqz i32.or
    local.get $scale f32.const 0 f32.le i32.or
    if i32.const -1 return end
    local.get $out i32.load local.set $flags
    local.get $name_ptr local.get $name_len call $svg_trim_start local.set $idx
    local.get $name_ptr local.get $idx local.get $name_len call $svg_trim_end local.set $end
    local.get $end local.get $idx i32.sub local.set $n
    local.get $n i32.const 12 i32.eq
    local.get $name_ptr local.get $end local.get $idx i32.const 115 call $svg_alpha_eq i32.and
    local.get $name_ptr local.get $end local.get $idx i32.const 7 i32.add i32.const 119 call $svg_alpha_eq i32.and
    if
      local.get $value_ptr local.get $value_len local.get $scale call $er_ui_svg_length_parse_normalized local.tee $fv
      f32.const -340282346638528859811704183484516925440 f32.eq if i32.const -1 return end
      local.get $out i32.const 12 i32.add local.get $fv f32.store
      local.get $out local.get $flags i32.const 4 i32.or i32.store
      i32.const 1 return
    end
    local.get $n i32.const 16 i32.eq
    local.get $name_ptr local.get $end local.get $idx i32.const 115 call $svg_alpha_eq i32.and
    local.get $name_ptr local.get $end local.get $idx i32.const 7 i32.add i32.const 100 call $svg_alpha_eq i32.and
    if
      i32.const 65492 local.set $slot
      local.get $value_ptr local.get $value_len call $svg_trim_start local.set $idx
      local.get $value_ptr local.get $idx local.get $value_len call $svg_trim_end local.set $end
      local.get $end local.get $idx i32.sub i32.const 4 i32.eq
      local.get $value_ptr local.get $end local.get $idx i32.const 110 call $svg_alpha_eq i32.and
      local.get $value_ptr local.get $end local.get $idx i32.const 1 i32.add i32.const 111 call $svg_alpha_eq i32.and
      local.get $value_ptr local.get $end local.get $idx i32.const 2 i32.add i32.const 110 call $svg_alpha_eq i32.and
      local.get $value_ptr local.get $end local.get $idx i32.const 3 i32.add i32.const 101 call $svg_alpha_eq i32.and
      if
        local.get $out i32.const 44 i32.add f32.const 0 f32.store
        local.get $out i32.const 48 i32.add f32.const 0 f32.store
        local.get $out local.get $flags i32.const -1025 i32.and i32.store
        i32.const 1 return
      end
      local.get $slot local.get $idx i32.store
      local.get $value_ptr local.get $end local.get $slot call $svg_parse_number local.set $fv
      local.get $slot i32.load i32.const -1 i32.eq if i32.const -1 return end
      local.get $out i32.const 44 i32.add local.get $fv local.get $scale f32.div f32.store
      local.get $value_ptr local.get $end local.get $slot i32.load call $svg_skip_separators local.set $idx
      local.get $idx local.get $end i32.ge_u
      if
        local.get $out i32.const 48 i32.add local.get $fv local.get $scale f32.div f32.store
        local.get $out local.get $flags i32.const 1024 i32.or i32.store
        i32.const 1 return
      end
      local.get $value_ptr local.get $end local.get $slot call $svg_parse_number local.set $fv
      local.get $slot i32.load i32.const -1 i32.eq if i32.const -1 return end
      local.get $out i32.const 48 i32.add local.get $fv local.get $scale f32.div f32.store
      local.get $value_ptr local.get $end local.get $slot i32.load call $svg_skip_separators local.get $end i32.ne
      if i32.const -1 return end
      local.get $out local.get $flags i32.const 1024 i32.or i32.store
      i32.const 1 return
    end
    local.get $n i32.const 17 i32.eq
    local.get $name_ptr local.get $end local.get $idx i32.const 115 call $svg_alpha_eq i32.and
    local.get $name_ptr local.get $end local.get $idx i32.const 11 i32.add i32.const 111 call $svg_alpha_eq i32.and
    if
      local.get $value_ptr local.get $value_len local.get $scale call $er_ui_svg_length_parse_normalized local.tee $fv
      f32.const -340282346638528859811704183484516925440 f32.eq if i32.const -1 return end
      local.get $out i32.const 52 i32.add local.get $fv f32.store
      local.get $out local.get $flags i32.const 2048 i32.or i32.store
      i32.const 1 return
    end
    local.get $name_ptr local.get $name_len local.get $value_ptr local.get $value_len local.get $out call $er_ui_svg_style_attr_apply)

  (func (export "er_ui_svg_style_declaration_list_apply") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $idx i32) (local $name_start i32) (local $name_end i32) (local $value_start i32) (local $value_end i32) (local $ch i32) (local $applied i32) (local $r i32)
    local.get $ptr i32.eqz
    local.get $out i32.eqz i32.or
    if i32.const -1 return end
    block $done
      loop $loop
        block $skip_done
          loop $skip_loop
            local.get $idx local.get $len i32.ge_u br_if $done
            local.get $ptr local.get $idx i32.add i32.load8_u local.tee $ch
            call $svg_is_ws
            local.get $ch i32.const 59 i32.eq i32.or
            i32.eqz br_if $skip_done
            local.get $idx i32.const 1 i32.add local.set $idx
            br $skip_loop
          end
        end
        local.get $idx local.set $name_start
        block $name_done
          loop $name_loop
            local.get $idx local.get $len i32.ge_u br_if $name_done
            local.get $ptr local.get $idx i32.add i32.load8_u local.tee $ch
            i32.const 58 i32.eq
            local.get $ch i32.const 59 i32.eq i32.or
            br_if $name_done
            local.get $idx i32.const 1 i32.add local.set $idx
            br $name_loop
          end
        end
        local.get $idx local.set $name_end
        local.get $idx local.get $len i32.ge_u if i32.const -1 return end
        local.get $ptr local.get $idx i32.add i32.load8_u i32.const 58 i32.ne if i32.const -1 return end
        local.get $idx i32.const 1 i32.add local.set $idx
        local.get $idx local.set $value_start
        block $value_done
          loop $value_loop
            local.get $idx local.get $len i32.ge_u br_if $value_done
            local.get $ptr local.get $idx i32.add i32.load8_u i32.const 59 i32.eq br_if $value_done
            local.get $idx i32.const 1 i32.add local.set $idx
            br $value_loop
          end
        end
        local.get $idx local.set $value_end
        local.get $name_end local.get $name_start i32.gt_u
        if
          local.get $ptr
          local.get $name_start
          i32.add
          local.get $name_end local.get $name_start i32.sub
          local.get $ptr
          local.get $value_start
          i32.add
          local.get $value_end local.get $value_start i32.sub
          local.get $out
          call $er_ui_svg_style_attr_apply
          local.tee $r
          i32.const -1
          i32.eq
          if i32.const -1 return end
          local.get $applied local.get $r i32.or local.set $applied
        end
        local.get $idx local.get $len i32.lt_u
        if
          local.get $idx i32.const 1 i32.add local.set $idx
        end
        br $loop
      end
    end
    local.get $applied)

  (func $svg_transform_apply (param $out i32) (param $na f32) (param $nb f32) (param $nc f32) (param $nd f32) (param $ntx f32) (param $nty f32)
    (local $a f32) (local $b f32) (local $c f32) (local $d f32) (local $tx f32) (local $ty f32)
    (local $oa f32) (local $ob f32) (local $oc f32) (local $od f32) (local $otx f32) (local $oty f32)
    local.get $out f32.load local.set $a
    local.get $out i32.const 4 i32.add f32.load local.set $b
    local.get $out i32.const 8 i32.add f32.load local.set $c
    local.get $out i32.const 12 i32.add f32.load local.set $d
    local.get $out i32.const 16 i32.add f32.load local.set $tx
    local.get $out i32.const 20 i32.add f32.load local.set $ty
    local.get $na local.get $a f32.mul local.get $nc local.get $b f32.mul f32.add local.set $oa
    local.get $nb local.get $a f32.mul local.get $nd local.get $b f32.mul f32.add local.set $ob
    local.get $na local.get $c f32.mul local.get $nc local.get $d f32.mul f32.add local.set $oc
    local.get $nb local.get $c f32.mul local.get $nd local.get $d f32.mul f32.add local.set $od
    local.get $na local.get $tx f32.mul local.get $nc local.get $ty f32.mul f32.add local.get $ntx f32.add local.set $otx
    local.get $nb local.get $tx f32.mul local.get $nd local.get $ty f32.mul f32.add local.get $nty f32.add local.set $oty
    local.get $out local.get $oa f32.store
    local.get $out i32.const 4 i32.add local.get $ob f32.store
    local.get $out i32.const 8 i32.add local.get $oc f32.store
    local.get $out i32.const 12 i32.add local.get $od f32.store
    local.get $out i32.const 16 i32.add local.get $otx f32.store
    local.get $out i32.const 20 i32.add local.get $oty f32.store)

  (func (export "er_ui_svg_matrix_invert") (param $matrix i32) (param $out i32) (result i32)
    (local $a f32) (local $b f32) (local $c f32) (local $d f32) (local $tx f32) (local $ty f32) (local $det f32)
    local.get $matrix i32.eqz
    local.get $out i32.eqz i32.or
    if i32.const -1 return end
    local.get $matrix f32.load local.set $a
    local.get $matrix i32.const 4 i32.add f32.load local.set $b
    local.get $matrix i32.const 8 i32.add f32.load local.set $c
    local.get $matrix i32.const 12 i32.add f32.load local.set $d
    local.get $matrix i32.const 16 i32.add f32.load local.set $tx
    local.get $matrix i32.const 20 i32.add f32.load local.set $ty
    local.get $a local.get $d f32.mul
    local.get $b local.get $c f32.mul
    f32.sub
    local.set $det
    local.get $det f32.abs f32.const 0.000001 f32.lt
    if i32.const -1 return end
    local.get $out local.get $d local.get $det f32.div f32.store
    local.get $out i32.const 4 i32.add local.get $b f32.neg local.get $det f32.div f32.store
    local.get $out i32.const 8 i32.add local.get $c f32.neg local.get $det f32.div f32.store
    local.get $out i32.const 12 i32.add local.get $a local.get $det f32.div f32.store
    local.get $out i32.const 16 i32.add
      local.get $c local.get $ty f32.mul
      local.get $d local.get $tx f32.mul
      f32.sub
      local.get $det f32.div
      f32.store
    local.get $out i32.const 20 i32.add
      local.get $b local.get $tx f32.mul
      local.get $a local.get $ty f32.mul
      f32.sub
      local.get $det f32.div
      f32.store
    i32.const 1)

  (func (export "er_ui_svg_transform_parse_to_matrix") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $idx i32) (local $idx_slot i32)
    (local $a f32) (local $b f32) (local $c f32) (local $d f32) (local $tx f32) (local $ty f32)
    (local $angle f32) (local $cx f32) (local $cy f32) (local $rad f32) (local $cos f32) (local $sin f32)
    local.get $ptr i32.eqz
    local.get $out i32.eqz i32.or
    if i32.const -1 return end
    local.get $out f32.const 1 f32.store
    local.get $out i32.const 4 i32.add f32.const 0 f32.store
    local.get $out i32.const 8 i32.add f32.const 0 f32.store
    local.get $out i32.const 12 i32.add f32.const 1 f32.store
    local.get $out i32.const 16 i32.add f32.const 0 f32.store
    local.get $out i32.const 20 i32.add f32.const 0 f32.store
    i32.const 65516 local.set $idx_slot
    block $done
      loop $loop
        local.get $ptr local.get $len local.get $idx call $svg_skip_separators local.set $idx
        local.get $idx local.get $len i32.ge_u br_if $done
        local.get $idx_slot local.get $idx i32.store
        local.get $ptr local.get $len local.get $idx i32.const 109 call $svg_alpha_eq
        local.get $ptr local.get $len local.get $idx i32.const 1 i32.add i32.const 97 call $svg_alpha_eq i32.and
        local.get $ptr local.get $len local.get $idx i32.const 2 i32.add i32.const 116 call $svg_alpha_eq i32.and
        local.get $ptr local.get $len local.get $idx i32.const 3 i32.add i32.const 114 call $svg_alpha_eq i32.and
        local.get $ptr local.get $len local.get $idx i32.const 4 i32.add i32.const 105 call $svg_alpha_eq i32.and
        local.get $ptr local.get $len local.get $idx i32.const 5 i32.add i32.const 120 call $svg_alpha_eq i32.and
        if
          local.get $ptr local.get $len local.get $idx i32.const 6 i32.add i32.const 40 call $svg_byte_eq i32.eqz if i32.const -1 return end
          local.get $idx_slot local.get $idx i32.const 7 i32.add i32.store
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $a
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $b
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $c
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $d
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $tx
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $ty
          local.get $idx_slot i32.load i32.const -1 i32.eq if i32.const -1 return end
          local.get $ptr local.get $len local.get $idx_slot i32.load call $svg_skip_separators local.set $idx
          local.get $ptr local.get $len local.get $idx i32.const 41 call $svg_byte_eq i32.eqz if i32.const -1 return end
          local.get $out local.get $a local.get $b local.get $c local.get $d local.get $tx local.get $ty call $svg_transform_apply
          local.get $idx i32.const 1 i32.add local.set $idx
          br $loop
        end
        local.get $ptr local.get $len local.get $idx i32.const 116 call $svg_alpha_eq
        local.get $ptr local.get $len local.get $idx i32.const 1 i32.add i32.const 114 call $svg_alpha_eq i32.and
        local.get $ptr local.get $len local.get $idx i32.const 2 i32.add i32.const 97 call $svg_alpha_eq i32.and
        local.get $ptr local.get $len local.get $idx i32.const 3 i32.add i32.const 110 call $svg_alpha_eq i32.and
        local.get $ptr local.get $len local.get $idx i32.const 4 i32.add i32.const 115 call $svg_alpha_eq i32.and
        local.get $ptr local.get $len local.get $idx i32.const 5 i32.add i32.const 108 call $svg_alpha_eq i32.and
        local.get $ptr local.get $len local.get $idx i32.const 6 i32.add i32.const 97 call $svg_alpha_eq i32.and
        local.get $ptr local.get $len local.get $idx i32.const 7 i32.add i32.const 116 call $svg_alpha_eq i32.and
        local.get $ptr local.get $len local.get $idx i32.const 8 i32.add i32.const 101 call $svg_alpha_eq i32.and
        if
          local.get $ptr local.get $len local.get $idx i32.const 9 i32.add i32.const 40 call $svg_byte_eq i32.eqz if i32.const -1 return end
          local.get $idx_slot local.get $idx i32.const 10 i32.add i32.store
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $tx
          local.get $idx_slot i32.load i32.const -1 i32.eq if i32.const -1 return end
          local.get $ptr local.get $len local.get $idx_slot i32.load call $svg_skip_separators local.set $idx
          local.get $ptr local.get $len local.get $idx i32.const 41 call $svg_byte_eq
          if
            f32.const 0 local.set $ty
          else
            local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $ty
            local.get $idx_slot i32.load i32.const -1 i32.eq if i32.const -1 return end
            local.get $ptr local.get $len local.get $idx_slot i32.load call $svg_skip_separators local.set $idx
            local.get $ptr local.get $len local.get $idx i32.const 41 call $svg_byte_eq i32.eqz if i32.const -1 return end
          end
          local.get $out f32.const 1 f32.const 0 f32.const 0 f32.const 1 local.get $tx local.get $ty call $svg_transform_apply
          local.get $idx i32.const 1 i32.add local.set $idx
          br $loop
        end
        local.get $ptr local.get $len local.get $idx i32.const 115 call $svg_alpha_eq
        local.get $ptr local.get $len local.get $idx i32.const 1 i32.add i32.const 107 call $svg_alpha_eq i32.and
        local.get $ptr local.get $len local.get $idx i32.const 2 i32.add i32.const 101 call $svg_alpha_eq i32.and
        local.get $ptr local.get $len local.get $idx i32.const 3 i32.add i32.const 119 call $svg_alpha_eq i32.and
        local.get $ptr local.get $len local.get $idx i32.const 4 i32.add i32.const 88 call $svg_alpha_eq i32.and
        if
          local.get $ptr local.get $len local.get $idx i32.const 5 i32.add i32.const 40 call $svg_byte_eq i32.eqz if i32.const -1 return end
          local.get $idx_slot local.get $idx i32.const 6 i32.add i32.store
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $angle
          local.get $idx_slot i32.load i32.const -1 i32.eq if i32.const -1 return end
          local.get $ptr local.get $len local.get $idx_slot i32.load call $svg_skip_separators local.set $idx
          local.get $ptr local.get $len local.get $idx i32.const 41 call $svg_byte_eq i32.eqz if i32.const -1 return end
          local.get $angle f32.const 0.017453292 f32.mul local.set $rad
          local.get $rad call $sin_rad local.set $sin
          local.get $rad call $cos_rad local.tee $cos f32.abs f32.const 0.000001 f32.lt
          if
            f32.const 0.000001 local.set $cos
          end
          local.get $out f32.const 1 f32.const 0 local.get $sin local.get $cos f32.div f32.const 1 f32.const 0 f32.const 0 call $svg_transform_apply
          local.get $idx i32.const 1 i32.add local.set $idx
          br $loop
        end
        local.get $ptr local.get $len local.get $idx i32.const 115 call $svg_alpha_eq
        local.get $ptr local.get $len local.get $idx i32.const 1 i32.add i32.const 107 call $svg_alpha_eq i32.and
        local.get $ptr local.get $len local.get $idx i32.const 2 i32.add i32.const 101 call $svg_alpha_eq i32.and
        local.get $ptr local.get $len local.get $idx i32.const 3 i32.add i32.const 119 call $svg_alpha_eq i32.and
        local.get $ptr local.get $len local.get $idx i32.const 4 i32.add i32.const 89 call $svg_alpha_eq i32.and
        if
          local.get $ptr local.get $len local.get $idx i32.const 5 i32.add i32.const 40 call $svg_byte_eq i32.eqz if i32.const -1 return end
          local.get $idx_slot local.get $idx i32.const 6 i32.add i32.store
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $angle
          local.get $idx_slot i32.load i32.const -1 i32.eq if i32.const -1 return end
          local.get $ptr local.get $len local.get $idx_slot i32.load call $svg_skip_separators local.set $idx
          local.get $ptr local.get $len local.get $idx i32.const 41 call $svg_byte_eq i32.eqz if i32.const -1 return end
          local.get $angle f32.const 0.017453292 f32.mul local.set $rad
          local.get $rad call $sin_rad local.set $sin
          local.get $rad call $cos_rad local.tee $cos f32.abs f32.const 0.000001 f32.lt
          if
            f32.const 0.000001 local.set $cos
          end
          local.get $out f32.const 1 local.get $sin local.get $cos f32.div f32.const 0 f32.const 1 f32.const 0 f32.const 0 call $svg_transform_apply
          local.get $idx i32.const 1 i32.add local.set $idx
          br $loop
        end
        local.get $ptr local.get $len local.get $idx i32.const 115 call $svg_alpha_eq
        local.get $ptr local.get $len local.get $idx i32.const 1 i32.add i32.const 99 call $svg_alpha_eq i32.and
        local.get $ptr local.get $len local.get $idx i32.const 2 i32.add i32.const 97 call $svg_alpha_eq i32.and
        local.get $ptr local.get $len local.get $idx i32.const 3 i32.add i32.const 108 call $svg_alpha_eq i32.and
        local.get $ptr local.get $len local.get $idx i32.const 4 i32.add i32.const 101 call $svg_alpha_eq i32.and
        if
          local.get $ptr local.get $len local.get $idx i32.const 5 i32.add i32.const 40 call $svg_byte_eq i32.eqz if i32.const -1 return end
          local.get $idx_slot local.get $idx i32.const 6 i32.add i32.store
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $a
          local.get $idx_slot i32.load i32.const -1 i32.eq if i32.const -1 return end
          local.get $ptr local.get $len local.get $idx_slot i32.load call $svg_skip_separators local.set $idx
          local.get $ptr local.get $len local.get $idx i32.const 41 call $svg_byte_eq
          if
            local.get $a local.set $d
          else
            local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $d
            local.get $idx_slot i32.load i32.const -1 i32.eq if i32.const -1 return end
            local.get $ptr local.get $len local.get $idx_slot i32.load call $svg_skip_separators local.set $idx
            local.get $ptr local.get $len local.get $idx i32.const 41 call $svg_byte_eq i32.eqz if i32.const -1 return end
          end
          local.get $out local.get $a f32.const 0 f32.const 0 local.get $d f32.const 0 f32.const 0 call $svg_transform_apply
          local.get $idx i32.const 1 i32.add local.set $idx
          br $loop
        end
        local.get $ptr local.get $len local.get $idx i32.const 114 call $svg_alpha_eq
        local.get $ptr local.get $len local.get $idx i32.const 1 i32.add i32.const 111 call $svg_alpha_eq i32.and
        local.get $ptr local.get $len local.get $idx i32.const 2 i32.add i32.const 116 call $svg_alpha_eq i32.and
        local.get $ptr local.get $len local.get $idx i32.const 3 i32.add i32.const 97 call $svg_alpha_eq i32.and
        local.get $ptr local.get $len local.get $idx i32.const 4 i32.add i32.const 116 call $svg_alpha_eq i32.and
        local.get $ptr local.get $len local.get $idx i32.const 5 i32.add i32.const 101 call $svg_alpha_eq i32.and
        if
          local.get $ptr local.get $len local.get $idx i32.const 6 i32.add i32.const 40 call $svg_byte_eq i32.eqz if i32.const -1 return end
          local.get $idx_slot local.get $idx i32.const 7 i32.add i32.store
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $angle
          local.get $idx_slot i32.load i32.const -1 i32.eq if i32.const -1 return end
          f32.const 0 local.set $cx
          f32.const 0 local.set $cy
          local.get $ptr local.get $len local.get $idx_slot i32.load call $svg_skip_separators local.set $idx
          local.get $ptr local.get $len local.get $idx i32.const 41 call $svg_byte_eq
          if
          else
            local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $cx
            local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $cy
            local.get $idx_slot i32.load i32.const -1 i32.eq if i32.const -1 return end
            local.get $ptr local.get $len local.get $idx_slot i32.load call $svg_skip_separators local.set $idx
            local.get $ptr local.get $len local.get $idx i32.const 41 call $svg_byte_eq i32.eqz if i32.const -1 return end
          end
          local.get $angle f32.const 0.017453292 f32.mul local.set $rad
          local.get $rad call $cos_rad local.set $cos
          local.get $rad call $sin_rad local.set $sin
          local.get $cx
          local.get $cos local.get $cx f32.mul
          f32.sub
          local.get $sin local.get $cy f32.mul
          f32.add
          local.set $tx
          local.get $cy
          local.get $sin local.get $cx f32.mul
          f32.sub
          local.get $cos local.get $cy f32.mul
          f32.sub
          local.set $ty
          local.get $out local.get $cos local.get $sin local.get $sin f32.neg local.get $cos local.get $tx local.get $ty call $svg_transform_apply
          local.get $idx i32.const 1 i32.add local.set $idx
          br $loop
        end
        i32.const -1
        return
      end
    end
    i32.const 1)
