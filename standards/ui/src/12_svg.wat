  (func $svg_is_ws (param $ch i32) (result i32)
    local.get $ch
    i32.const 32
    i32.eq
    local.get $ch
    i32.const 10
    i32.eq
    i32.or
    local.get $ch
    i32.const 13
    i32.eq
    i32.or
    local.get $ch
    i32.const 9
    i32.eq
    i32.or)

  (func $svg_is_digit (param $ch i32) (result i32)
    local.get $ch
    i32.const 48
    i32.ge_u
    local.get $ch
    i32.const 57
    i32.le_u
    i32.and)

  (func $svg_is_number_start (param $ch i32) (result i32)
    local.get $ch
    call $svg_is_digit
    local.get $ch
    i32.const 46
    i32.eq
    i32.or
    local.get $ch
    i32.const 45
    i32.eq
    i32.or
    local.get $ch
    i32.const 43
    i32.eq
    i32.or)

  (func $svg_is_command (param $ch i32) (result i32)
    local.get $ch i32.const 77 i32.eq
    local.get $ch i32.const 109 i32.eq i32.or
    local.get $ch i32.const 76 i32.eq i32.or
    local.get $ch i32.const 108 i32.eq i32.or
    local.get $ch i32.const 72 i32.eq i32.or
    local.get $ch i32.const 104 i32.eq i32.or
    local.get $ch i32.const 86 i32.eq i32.or
    local.get $ch i32.const 118 i32.eq i32.or
    local.get $ch i32.const 67 i32.eq i32.or
    local.get $ch i32.const 99 i32.eq i32.or
    local.get $ch i32.const 83 i32.eq i32.or
    local.get $ch i32.const 115 i32.eq i32.or
    local.get $ch i32.const 81 i32.eq i32.or
    local.get $ch i32.const 113 i32.eq i32.or
    local.get $ch i32.const 84 i32.eq i32.or
    local.get $ch i32.const 116 i32.eq i32.or
    local.get $ch i32.const 65 i32.eq i32.or
    local.get $ch i32.const 97 i32.eq i32.or
    local.get $ch i32.const 90 i32.eq i32.or
    local.get $ch i32.const 122 i32.eq i32.or)

  (func $svg_skip_separators (param $ptr i32) (param $len i32) (param $idx i32) (result i32)
    (local $i i32)
    local.get $idx
    local.set $i
    block $done
      loop $loop
        local.get $i
        local.get $len
        i32.ge_u
        br_if $done
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        call $svg_is_ws
        if
          local.get $i i32.const 1 i32.add local.set $i
          br $loop
        end
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.const 44
        i32.eq
        if
          local.get $i i32.const 1 i32.add local.set $i
          block $ws_done
            loop $ws_loop
              local.get $i
              local.get $len
              i32.ge_u
              br_if $ws_done
              local.get $ptr local.get $i i32.add i32.load8_u
              call $svg_is_ws
              i32.eqz
              br_if $ws_done
              local.get $i i32.const 1 i32.add local.set $i
              br $ws_loop
            end
          end
        end
        br $done
      end
    end
    local.get $i)

  (func $svg_parse_number (param $ptr i32) (param $len i32) (param $idx_ptr i32) (result f32)
    (local $i i32)
    (local $ch i32)
    (local $sign f32)
    (local $value f32)
    (local $scale f32)
    (local $has_digit i32)
    (local $exp_sign i32)
    (local $exp_value i32)
    (local $has_exp_digit i32)
    local.get $ptr
    local.get $len
    local.get $idx_ptr
    i32.load
    call $svg_skip_separators
    local.set $i
    local.get $i
    local.get $len
    i32.ge_u
    if
      local.get $idx_ptr i32.const -1 i32.store
      f32.const 0
      return
    end
    f32.const 1
    local.set $sign
    local.get $ptr local.get $i i32.add i32.load8_u
    local.tee $ch
    i32.const 45
    i32.eq
    if
      f32.const -1
      local.set $sign
      local.get $i i32.const 1 i32.add local.set $i
    else
      local.get $ch
      i32.const 43
      i32.eq
      if
        local.get $i i32.const 1 i32.add local.set $i
      end
    end
    block $int_done
      loop $int_loop
        local.get $i
        local.get $len
        i32.ge_u
        br_if $int_done
        local.get $ptr local.get $i i32.add i32.load8_u
        local.tee $ch
        call $svg_is_digit
        i32.eqz
        br_if $int_done
        local.get $value
        f32.const 10
        f32.mul
        local.get $ch
        i32.const 48
        i32.sub
        f32.convert_i32_u
        f32.add
        local.set $value
        i32.const 1 local.set $has_digit
        local.get $i i32.const 1 i32.add local.set $i
        br $int_loop
      end
    end
    local.get $i
    local.get $len
    i32.lt_u
    if
      local.get $ptr local.get $i i32.add i32.load8_u
      i32.const 46
      i32.eq
      if
        local.get $i i32.const 1 i32.add local.set $i
        f32.const 0.1
        local.set $scale
        block $frac_done
          loop $frac_loop
            local.get $i
            local.get $len
            i32.ge_u
            br_if $frac_done
            local.get $ptr local.get $i i32.add i32.load8_u
            local.tee $ch
            call $svg_is_digit
            i32.eqz
            br_if $frac_done
            local.get $value
            local.get $ch
            i32.const 48
            i32.sub
            f32.convert_i32_u
            local.get $scale
            f32.mul
            f32.add
            local.set $value
            local.get $scale
            f32.const 0.1
            f32.mul
            local.set $scale
            i32.const 1 local.set $has_digit
            local.get $i i32.const 1 i32.add local.set $i
            br $frac_loop
          end
        end
      end
    end
    local.get $has_digit
    i32.eqz
    if
      local.get $idx_ptr i32.const -1 i32.store
      f32.const 0
      return
    end
    local.get $i
    local.get $len
    i32.lt_u
    if
      local.get $ptr local.get $i i32.add i32.load8_u
      local.tee $ch
      i32.const 101
      i32.eq
      local.get $ch
      i32.const 69
      i32.eq
      i32.or
      if
        i32.const 1
        local.set $exp_sign
        i32.const 0
        local.set $exp_value
        i32.const 0
        local.set $has_exp_digit
        local.get $i i32.const 1 i32.add local.set $i
        local.get $i
        local.get $len
        i32.lt_u
        if
          local.get $ptr local.get $i i32.add i32.load8_u
          local.tee $ch
          i32.const 45
          i32.eq
          if
            i32.const -1
            local.set $exp_sign
            local.get $i i32.const 1 i32.add local.set $i
          else
            local.get $ch
            i32.const 43
            i32.eq
            if
              local.get $i i32.const 1 i32.add local.set $i
            end
          end
        end
        block $exp_done
          loop $exp_loop
            local.get $i
            local.get $len
            i32.ge_u
            br_if $exp_done
            local.get $ptr local.get $i i32.add i32.load8_u
            local.tee $ch
            call $svg_is_digit
            i32.eqz
            br_if $exp_done
            local.get $exp_value
            i32.const 10
            i32.mul
            local.get $ch
            i32.const 48
            i32.sub
            i32.add
            local.set $exp_value
            i32.const 1
            local.set $has_exp_digit
            local.get $i i32.const 1 i32.add local.set $i
            br $exp_loop
          end
        end
        local.get $has_exp_digit
        i32.eqz
        if
          local.get $idx_ptr i32.const -1 i32.store
          f32.const 0
          return
        end
        block $scale_done
          loop $scale_loop
            local.get $exp_value
            i32.eqz
            br_if $scale_done
            local.get $exp_sign
            i32.const 0
            i32.gt_s
            if
              local.get $value
              f32.const 10
              f32.mul
              local.set $value
            else
              local.get $value
              f32.const 10
              f32.div
              local.set $value
            end
            local.get $exp_value i32.const 1 i32.sub local.set $exp_value
            br $scale_loop
          end
        end
      end
    end
    local.get $idx_ptr
    local.get $i
    i32.store
    local.get $value
    local.get $sign
    f32.mul)

  (func $svg_parse_flag (param $ptr i32) (param $len i32) (param $idx_ptr i32) (result f32)
    (local $i i32)
    (local $ch i32)
    local.get $ptr
    local.get $len
    local.get $idx_ptr
    i32.load
    call $svg_skip_separators
    local.set $i
    local.get $i
    local.get $len
    i32.ge_u
    if
      local.get $idx_ptr i32.const -1 i32.store
      f32.const 0
      return
    end
    local.get $ptr local.get $i i32.add i32.load8_u local.set $ch
    local.get $ch i32.const 48 i32.eq
    if
      local.get $idx_ptr local.get $i i32.const 1 i32.add i32.store
      f32.const 0
      return
    end
    local.get $ch i32.const 49 i32.eq
    if
      local.get $idx_ptr local.get $i i32.const 1 i32.add i32.store
      f32.const 1
      return
    end
    local.get $idx_ptr i32.const -1 i32.store
    f32.const 0)

  (func $svg_byte_eq (param $ptr i32) (param $len i32) (param $idx i32) (param $ch i32) (result i32)
    local.get $idx
    local.get $len
    i32.lt_u
    if (result i32)
      local.get $ptr local.get $idx i32.add i32.load8_u
      local.get $ch
      i32.eq
    else
      i32.const 0
    end)

  (func $svg_ascii_lower (param $ch i32) (result i32)
    local.get $ch i32.const 65 i32.ge_u
    local.get $ch i32.const 90 i32.le_u
    i32.and
    if (result i32)
      local.get $ch i32.const 32 i32.add
    else
      local.get $ch
    end)

  (func $svg_alpha_eq (param $ptr i32) (param $len i32) (param $idx i32) (param $ch i32) (result i32)
    local.get $idx
    local.get $len
    i32.lt_u
    if (result i32)
      local.get $ptr local.get $idx i32.add i32.load8_u call $svg_ascii_lower
      local.get $ch call $svg_ascii_lower
      i32.eq
    else
      i32.const 0
    end)

  (func $svg_hex_value (param $ch i32) (result i32)
    local.get $ch
    call $svg_ascii_lower
    local.tee $ch
    i32.const 48
    i32.ge_u
    local.get $ch
    i32.const 57
    i32.le_u
    i32.and
    if (result i32)
      local.get $ch i32.const 48 i32.sub
    else
      local.get $ch
      i32.const 97
      i32.ge_u
      local.get $ch
      i32.const 102
      i32.le_u
      i32.and
      if (result i32)
        local.get $ch i32.const 87 i32.sub
      else
        i32.const -1
      end
    end)

  (func $er_ui_svg_color_parse_rgba  (param $ptr i32) (param $len i32) (result i32)
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
      local.get $r local.get $g local.get $b local.get $a call $er_ui_color_pack return
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
      local.get $r local.get $g local.get $b local.get $a call $er_ui_color_pack return
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
      local.get $r local.get $g local.get $b local.get $a call $er_ui_color_pack return
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
      local.get $r local.get $g local.get $b local.get $a call $er_ui_color_pack return
    end
    i32.const -1)

  (func $er_ui_svg_opacity_parse_alpha  (param $ptr i32) (param $len i32) (result i32)
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

  (func $er_ui_svg_color_apply_alpha  (param $rgba i32) (param $alpha i32) (result i32)
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

  (func $er_ui_svg_stroke_cap_parse  (param $ptr i32) (param $len i32) (result i32)
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

  (func $er_ui_svg_stroke_join_parse  (param $ptr i32) (param $len i32) (result i32)
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

  (func $er_ui_svg_fill_rule_parse  (param $ptr i32) (param $len i32) (result i32)
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

  (func $er_ui_svg_style_record_init  (param $out i32) (result i32)
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

  (func $er_ui_svg_style_record_inherit  (param $parent i32) (param $out i32) (result i32)
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

  (func $er_ui_svg_style_resolve_paint  (param $style i32) (param $which i32) (result i32)
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

  (func $er_ui_svg_length_parse_normalized  (param $ptr i32) (param $len i32) (param $scale f32) (result f32)
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

  (func $er_ui_svg_style_attr_apply  (param $name_ptr i32) (param $name_len i32) (param $value_ptr i32) (param $value_len i32) (param $out i32) (result i32)
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

  (func $er_ui_svg_style_attr_apply_normalized  (param $name_ptr i32) (param $name_len i32) (param $value_ptr i32) (param $value_len i32) (param $out i32) (param $scale f32) (result i32)
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

  (func $er_ui_svg_style_declaration_list_apply  (param $ptr i32) (param $len i32) (param $out i32) (result i32)
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

  (func $er_ui_svg_matrix_invert  (param $matrix i32) (param $out i32) (result i32)
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

  (func $er_ui_svg_transform_parse_to_matrix  (param $ptr i32) (param $len i32) (param $out i32) (result i32)
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
  (func $svg_write_op1 (param $out i32) (param $cap i32) (param $count i32) (param $op f32) (result i32)
    local.get $count i32.const 1 i32.add i32.const 4 i32.mul local.get $cap i32.gt_u
    if i32.const -1 return end
    local.get $out local.get $count i32.const 4 i32.mul i32.add local.get $op f32.store
    local.get $count i32.const 1 i32.add)

  (func $svg_write_op2 (param $out i32) (param $cap i32) (param $count i32) (param $op f32) (param $a f32) (result i32)
    (local $p i32)
    local.get $count i32.const 2 i32.add i32.const 4 i32.mul local.get $cap i32.gt_u
    if i32.const -1 return end
    local.get $out local.get $count i32.const 4 i32.mul i32.add local.set $p
    local.get $p local.get $op f32.store
    local.get $p i32.const 4 i32.add local.get $a f32.store
    local.get $count i32.const 2 i32.add)

  (func $svg_write_op4 (param $out i32) (param $cap i32) (param $count i32) (param $op f32) (param $a f32) (param $b f32) (param $c f32) (result i32)
    (local $p i32)
    local.get $count i32.const 4 i32.add i32.const 4 i32.mul local.get $cap i32.gt_u
    if i32.const -1 return end
    local.get $out local.get $count i32.const 4 i32.mul i32.add local.set $p
    local.get $p local.get $op f32.store
    local.get $p i32.const 4 i32.add local.get $a f32.store
    local.get $p i32.const 8 i32.add local.get $b f32.store
    local.get $p i32.const 12 i32.add local.get $c f32.store
    local.get $count i32.const 4 i32.add)

  (func $svg_write_op3 (param $out i32) (param $cap i32) (param $count i32) (param $op f32) (param $x f32) (param $y f32) (result i32)
    (local $p i32)
    local.get $count i32.const 3 i32.add i32.const 4 i32.mul local.get $cap i32.gt_u
    if i32.const -1 return end
    local.get $out local.get $count i32.const 4 i32.mul i32.add local.set $p
    local.get $p local.get $op f32.store
    local.get $p i32.const 4 i32.add local.get $x f32.store
    local.get $p i32.const 8 i32.add local.get $y f32.store
    local.get $count i32.const 3 i32.add)

  (func $svg_write_op5 (param $out i32) (param $cap i32) (param $count i32) (param $op f32) (param $x1 f32) (param $y1 f32) (param $x2 f32) (param $y2 f32) (result i32)
    (local $p i32)
    local.get $count i32.const 5 i32.add i32.const 4 i32.mul local.get $cap i32.gt_u
    if i32.const -1 return end
    local.get $out local.get $count i32.const 4 i32.mul i32.add local.set $p
    local.get $p local.get $op f32.store
    local.get $p i32.const 4 i32.add local.get $x1 f32.store
    local.get $p i32.const 8 i32.add local.get $y1 f32.store
    local.get $p i32.const 12 i32.add local.get $x2 f32.store
    local.get $p i32.const 16 i32.add local.get $y2 f32.store
    local.get $count i32.const 5 i32.add)

  (func $svg_write_op6 (param $out i32) (param $cap i32) (param $count i32) (param $op f32) (param $a f32) (param $b f32) (param $c f32) (param $d f32) (param $e f32) (result i32)
    (local $p i32)
    local.get $count i32.const 6 i32.add i32.const 4 i32.mul local.get $cap i32.gt_u
    if i32.const -1 return end
    local.get $out local.get $count i32.const 4 i32.mul i32.add local.set $p
    local.get $p local.get $op f32.store
    local.get $p i32.const 4 i32.add local.get $a f32.store
    local.get $p i32.const 8 i32.add local.get $b f32.store
    local.get $p i32.const 12 i32.add local.get $c f32.store
    local.get $p i32.const 16 i32.add local.get $d f32.store
    local.get $p i32.const 20 i32.add local.get $e f32.store
    local.get $count i32.const 6 i32.add)

  (func $svg_write_op7 (param $out i32) (param $cap i32) (param $count i32) (param $op f32) (param $x1 f32) (param $y1 f32) (param $x2 f32) (param $y2 f32) (param $x3 f32) (param $y3 f32) (result i32)
    (local $p i32)
    local.get $count i32.const 7 i32.add i32.const 4 i32.mul local.get $cap i32.gt_u
    if i32.const -1 return end
    local.get $out local.get $count i32.const 4 i32.mul i32.add local.set $p
    local.get $p local.get $op f32.store
    local.get $p i32.const 4 i32.add local.get $x1 f32.store
    local.get $p i32.const 8 i32.add local.get $y1 f32.store
    local.get $p i32.const 12 i32.add local.get $x2 f32.store
    local.get $p i32.const 16 i32.add local.get $y2 f32.store
    local.get $p i32.const 20 i32.add local.get $x3 f32.store
    local.get $p i32.const 24 i32.add local.get $y3 f32.store
    local.get $count i32.const 7 i32.add)

  (func $svg_write_op8 (param $out i32) (param $cap i32) (param $count i32) (param $op f32) (param $a f32) (param $b f32) (param $c f32) (param $d f32) (param $e f32) (param $f f32) (param $g f32) (result i32)
    (local $p i32)
    local.get $count i32.const 8 i32.add i32.const 4 i32.mul local.get $cap i32.gt_u
    if i32.const -1 return end
    local.get $out local.get $count i32.const 4 i32.mul i32.add local.set $p
    local.get $p local.get $op f32.store
    local.get $p i32.const 4 i32.add local.get $a f32.store
    local.get $p i32.const 8 i32.add local.get $b f32.store
    local.get $p i32.const 12 i32.add local.get $c f32.store
    local.get $p i32.const 16 i32.add local.get $d f32.store
    local.get $p i32.const 20 i32.add local.get $e f32.store
    local.get $p i32.const 24 i32.add local.get $f f32.store
    local.get $p i32.const 28 i32.add local.get $g f32.store
    local.get $count i32.const 8 i32.add)

  (func $svg_style_emit_paint (param $style i32) (param $which i32) (param $out i32) (param $cap i32) (param $count i32) (result i32)
    (local $paint i32)
    local.get $style local.get $which call $er_ui_svg_style_resolve_paint local.tee $paint
    i32.const -2
    i32.eq
    if
      local.get $count
      return
    end
    local.get $paint i32.const -1 i32.eq
    if
      i32.const -1
      return
    end
    local.get $out local.get $cap local.get $count f32.const 17
      local.get $paint i32.const 255 i32.and f32.convert_i32_u
      local.get $paint i32.const 8 i32.shr_u i32.const 255 i32.and f32.convert_i32_u
      local.get $paint i32.const 16 i32.shr_u i32.const 255 i32.and f32.convert_i32_u
      local.get $paint i32.const 24 i32.shr_u i32.const 255 i32.and f32.convert_i32_u
      call $svg_write_op5)

  (func $er_ui_svg_path_style_parse_to_ir_transform_impl  (param $ptr i32) (param $len i32) (param $out i32) (param $cap i32) (param $min_x f32) (param $min_y f32) (param $vw f32) (param $vh f32) (param $style i32) (param $a f32) (param $b f32) (param $c f32) (param $d f32) (param $tx f32) (param $ty f32) (result i32)
    (local $count i32) (local $body i32) (local $paint i32)
    local.get $ptr i32.eqz
    local.get $out i32.eqz i32.or
    local.get $style i32.eqz i32.or
    if i32.const -1 return end
    local.get $style i32.const 60 i32.add i32.load i32.eqz
    local.get $style i32.const 64 i32.add i32.load i32.eqz i32.or
    if i32.const 0 return end
    local.get $style i32.const 0 call $er_ui_svg_style_resolve_paint local.set $paint
    local.get $paint i32.const -1 i32.eq if i32.const -1 return end
    local.get $paint i32.const -2 i32.ne
    if
      local.get $style i32.const 0 local.get $out local.get $cap local.get $count call $svg_style_emit_paint local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $out local.get $cap local.get $count
      local.get $style i32.const 36 i32.add i32.load
      if (result f32)
        f32.const 16
      else
        f32.const 14
      end
      call $svg_write_op1 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $ptr local.get $len
      local.get $out local.get $count i32.const 4 i32.mul i32.add
      local.get $cap local.get $count i32.const 4 i32.mul i32.sub
      local.get $min_x local.get $min_y local.get $vw local.get $vh
      local.get $a local.get $b local.get $c local.get $d local.get $tx local.get $ty
      call $er_ui_svg_path_parse_to_ir_transform_impl local.tee $body i32.const -1 i32.eq if i32.const -1 return end
      local.get $count local.get $body i32.add local.set $count
      local.get $out local.get $cap local.get $count f32.const 15 call $svg_write_op1 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    end
    local.get $style i32.const 1 call $er_ui_svg_style_resolve_paint local.set $paint
    local.get $paint i32.const -1 i32.eq if i32.const -1 return end
    local.get $paint i32.const -2 i32.ne
    if
      local.get $style i32.const 1 local.get $out local.get $cap local.get $count call $svg_style_emit_paint local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $out local.get $cap local.get $count f32.const 22 local.get $style i32.const 12 i32.add f32.load call $svg_write_op2 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $out local.get $cap local.get $count f32.const 23 local.get $style i32.const 28 i32.add i32.load f32.convert_i32_s call $svg_write_op2 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $out local.get $cap local.get $count f32.const 24 local.get $style i32.const 32 i32.add i32.load f32.convert_i32_s call $svg_write_op2 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $out local.get $cap local.get $count f32.const 25 local.get $style i32.const 40 i32.add f32.load call $svg_write_op2 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $style i32.load i32.const 1024 i32.and
      if
        local.get $out local.get $cap local.get $count f32.const 30
          local.get $style i32.const 44 i32.add f32.load
          local.get $style i32.const 48 i32.add f32.load
          local.get $style i32.const 52 i32.add f32.load
          call $svg_write_op4 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      end
      local.get $ptr local.get $len
      local.get $out local.get $count i32.const 4 i32.mul i32.add
      local.get $cap local.get $count i32.const 4 i32.mul i32.sub
      local.get $min_x local.get $min_y local.get $vw local.get $vh
      local.get $a local.get $b local.get $c local.get $d local.get $tx local.get $ty
      call $er_ui_svg_path_parse_to_ir_transform_impl local.tee $body i32.const -1 i32.eq if i32.const -1 return end
      local.get $count local.get $body i32.add local.set $count
    end
    local.get $count)

  (func $er_ui_svg_path_style_parse_to_ir  (param $ptr i32) (param $len i32) (param $out i32) (param $cap i32) (param $min_x f32) (param $min_y f32) (param $vw f32) (param $vh f32) (param $style i32) (result i32)
    local.get $ptr
    local.get $len
    local.get $out
    local.get $cap
    local.get $min_x
    local.get $min_y
    local.get $vw
    local.get $vh
    local.get $style
    f32.const 1
    f32.const 0
    f32.const 0
    f32.const 1
    f32.const 0
    f32.const 0
    call $er_ui_svg_path_style_parse_to_ir_transform_impl)

  (func $svg_style_emit_stroke_state (param $style i32) (param $out i32) (param $cap i32) (param $count i32) (result i32)
    local.get $out local.get $cap local.get $count f32.const 22 local.get $style i32.const 12 i32.add f32.load call $svg_write_op2 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    local.get $out local.get $cap local.get $count f32.const 23 local.get $style i32.const 28 i32.add i32.load f32.convert_i32_s call $svg_write_op2 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    local.get $out local.get $cap local.get $count f32.const 24 local.get $style i32.const 32 i32.add i32.load f32.convert_i32_s call $svg_write_op2 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    local.get $out local.get $cap local.get $count f32.const 25 local.get $style i32.const 40 i32.add f32.load call $svg_write_op2)

  (func $er_ui_svg_circle_style_emit_ir  (param $out i32) (param $cap i32) (param $cx f32) (param $cy f32) (param $r f32) (param $style i32) (result i32)
    (local $count i32) (local $paint i32)
    local.get $out i32.eqz
    local.get $style i32.eqz i32.or
    local.get $r f32.const 0 f32.le i32.or
    if i32.const -1 return end
    local.get $style i32.const 60 i32.add i32.load i32.eqz
    local.get $style i32.const 64 i32.add i32.load i32.eqz i32.or
    if i32.const 0 return end
    local.get $style i32.const 0 call $er_ui_svg_style_resolve_paint local.set $paint
    local.get $paint i32.const -1 i32.eq if i32.const -1 return end
    local.get $paint i32.const -2 i32.ne
    if
      local.get $style i32.const 0 local.get $out local.get $cap local.get $count call $svg_style_emit_paint local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $out local.get $cap local.get $count f32.const 5 local.get $cx local.get $cy local.get $r call $svg_write_op4 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    end
    local.get $style i32.const 1 call $er_ui_svg_style_resolve_paint local.set $paint
    local.get $paint i32.const -1 i32.eq if i32.const -1 return end
    local.get $paint i32.const -2 i32.ne
    if
      local.get $style i32.const 1 local.get $out local.get $cap local.get $count call $svg_style_emit_paint local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $style local.get $out local.get $cap local.get $count call $svg_style_emit_stroke_state local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $out local.get $cap local.get $count f32.const 2 local.get $cx local.get $cy local.get $r call $svg_write_op4 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    end
    local.get $count)

  (func $er_ui_svg_ellipse_style_emit_ir  (param $out i32) (param $cap i32) (param $cx f32) (param $cy f32) (param $rx f32) (param $ry f32) (param $style i32) (result i32)
    (local $count i32) (local $paint i32)
    local.get $out i32.eqz
    local.get $style i32.eqz i32.or
    local.get $rx f32.const 0 f32.le i32.or
    local.get $ry f32.const 0 f32.le i32.or
    if i32.const -1 return end
    local.get $style i32.const 60 i32.add i32.load i32.eqz
    local.get $style i32.const 64 i32.add i32.load i32.eqz i32.or
    if i32.const 0 return end
    local.get $style i32.const 0 call $er_ui_svg_style_resolve_paint local.set $paint
    local.get $paint i32.const -1 i32.eq if i32.const -1 return end
    local.get $paint i32.const -2 i32.ne
    if
      local.get $style i32.const 0 local.get $out local.get $cap local.get $count call $svg_style_emit_paint local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $out local.get $cap local.get $count f32.const 12 local.get $cx local.get $cy local.get $rx local.get $ry call $svg_write_op5 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    end
    local.get $style i32.const 1 call $er_ui_svg_style_resolve_paint local.set $paint
    local.get $paint i32.const -1 i32.eq if i32.const -1 return end
    local.get $paint i32.const -2 i32.ne
    if
      local.get $style i32.const 1 local.get $out local.get $cap local.get $count call $svg_style_emit_paint local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $style local.get $out local.get $cap local.get $count call $svg_style_emit_stroke_state local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $out local.get $cap local.get $count f32.const 3 local.get $cx local.get $cy local.get $rx local.get $ry call $svg_write_op5 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    end
    local.get $count)

  (func $er_ui_svg_rect_style_emit_ir  (param $out i32) (param $cap i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $r f32) (param $style i32) (result i32)
    (local $count i32) (local $paint i32)
    local.get $out i32.eqz
    local.get $style i32.eqz i32.or
    local.get $w f32.const 0 f32.le i32.or
    local.get $h f32.const 0 f32.le i32.or
    if i32.const -1 return end
    local.get $style i32.const 60 i32.add i32.load i32.eqz
    local.get $style i32.const 64 i32.add i32.load i32.eqz i32.or
    if i32.const 0 return end
    local.get $style i32.const 0 call $er_ui_svg_style_resolve_paint local.set $paint
    local.get $paint i32.const -1 i32.eq if i32.const -1 return end
    local.get $paint i32.const -2 i32.ne
    if
      local.get $style i32.const 0 local.get $out local.get $cap local.get $count call $svg_style_emit_paint local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $out local.get $cap local.get $count f32.const 13 local.get $x local.get $y local.get $w local.get $h local.get $r call $svg_write_op6 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    end
    local.get $style i32.const 1 call $er_ui_svg_style_resolve_paint local.set $paint
    local.get $paint i32.const -1 i32.eq if i32.const -1 return end
    local.get $paint i32.const -2 i32.ne
    if
      local.get $style i32.const 1 local.get $out local.get $cap local.get $count call $svg_style_emit_paint local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $style local.get $out local.get $cap local.get $count call $svg_style_emit_stroke_state local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $out local.get $cap local.get $count f32.const 4 local.get $x local.get $y local.get $w local.get $h local.get $r call $svg_write_op6 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    end
    local.get $count)

  (func $er_ui_svg_line_style_emit_ir  (param $out i32) (param $cap i32) (param $x1 f32) (param $y1 f32) (param $x2 f32) (param $y2 f32) (param $style i32) (result i32)
    (local $count i32) (local $paint i32)
    local.get $out i32.eqz
    local.get $style i32.eqz i32.or
    if i32.const -1 return end
    local.get $style i32.const 60 i32.add i32.load i32.eqz
    local.get $style i32.const 64 i32.add i32.load i32.eqz i32.or
    if i32.const 0 return end
    local.get $style i32.const 1 call $er_ui_svg_style_resolve_paint local.set $paint
    local.get $paint i32.const -1 i32.eq if i32.const -1 return end
    local.get $paint i32.const -2 i32.eq if i32.const 0 return end
    local.get $style i32.const 1 local.get $out local.get $cap local.get $count call $svg_style_emit_paint local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    local.get $style local.get $out local.get $cap local.get $count call $svg_style_emit_stroke_state local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    local.get $style i32.load i32.const 1024 i32.and
    if
      local.get $out local.get $cap local.get $count f32.const 30
        local.get $style i32.const 44 i32.add f32.load
        local.get $style i32.const 48 i32.add f32.load
        local.get $style i32.const 52 i32.add f32.load
        call $svg_write_op4 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    end
    local.get $out local.get $cap local.get $count f32.const 6 local.get $x1 local.get $y1 call $svg_write_op3 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    local.get $out local.get $cap local.get $count f32.const 7 local.get $x2 local.get $y2 call $svg_write_op3 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    local.get $count)

  (func $er_ui_svg_polyline_style_emit_ir  (param $points i32) (param $point_count i32) (param $out i32) (param $cap i32) (param $style i32) (result i32)
    (local $count i32) (local $paint i32) (local $i i32) (local $p i32) (local $needed i32)
    local.get $points i32.eqz
    local.get $out i32.eqz i32.or
    local.get $style i32.eqz i32.or
    local.get $point_count i32.const 2 i32.lt_u i32.or
    if i32.const -1 return end
    local.get $style i32.const 60 i32.add i32.load i32.eqz
    local.get $style i32.const 64 i32.add i32.load i32.eqz i32.or
    if i32.const 0 return end
    local.get $style i32.const 1 call $er_ui_svg_style_resolve_paint local.set $paint
    local.get $paint i32.const -1 i32.eq if i32.const -1 return end
    local.get $paint i32.const -2 i32.eq if i32.const 0 return end
    local.get $style i32.const 1 local.get $out local.get $cap local.get $count call $svg_style_emit_paint local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    local.get $style local.get $out local.get $cap local.get $count call $svg_style_emit_stroke_state local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    local.get $style i32.load i32.const 1024 i32.and
    if
      local.get $out local.get $cap local.get $count f32.const 30
        local.get $style i32.const 44 i32.add f32.load
        local.get $style i32.const 48 i32.add f32.load
        local.get $style i32.const 52 i32.add f32.load
        call $svg_write_op4 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    end
    local.get $point_count i32.const 2 i32.mul i32.const 2 i32.add local.set $needed
    local.get $count local.get $needed i32.add i32.const 4 i32.mul local.get $cap i32.gt_u
    if i32.const -1 return end
    local.get $out local.get $count i32.const 4 i32.mul i32.add local.set $p
    local.get $p f32.const 1 f32.store
    local.get $p i32.const 4 i32.add local.get $point_count f32.convert_i32_u f32.store
    i32.const 0 local.set $i
    block $done
      loop $loop
        local.get $i local.get $point_count i32.ge_u br_if $done
        local.get $p i32.const 8 i32.add local.get $i i32.const 8 i32.mul i32.add
          local.get $points local.get $i i32.const 8 i32.mul i32.add f32.load
          f32.store
        local.get $p i32.const 12 i32.add local.get $i i32.const 8 i32.mul i32.add
          local.get $points local.get $i i32.const 8 i32.mul i32.add i32.const 4 i32.add f32.load
          f32.store
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      end
    end
    local.get $count local.get $needed i32.add)

  (func $er_ui_svg_polygon_style_emit_ir  (param $points i32) (param $point_count i32) (param $out i32) (param $cap i32) (param $style i32) (result i32)
    (local $count i32) (local $paint i32) (local $i i32)
    local.get $points i32.eqz
    local.get $out i32.eqz i32.or
    local.get $style i32.eqz i32.or
    local.get $point_count i32.const 3 i32.lt_u i32.or
    if i32.const -1 return end
    local.get $style i32.const 60 i32.add i32.load i32.eqz
    local.get $style i32.const 64 i32.add i32.load i32.eqz i32.or
    if i32.const 0 return end

    local.get $style i32.const 0 call $er_ui_svg_style_resolve_paint local.set $paint
    local.get $paint i32.const -1 i32.eq if i32.const -1 return end
    local.get $paint i32.const -2 i32.ne
    if
      local.get $style i32.const 0 local.get $out local.get $cap local.get $count call $svg_style_emit_paint local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $out local.get $cap local.get $count
      local.get $style i32.const 36 i32.add i32.load
      if (result f32)
        f32.const 16
      else
        f32.const 14
      end
      call $svg_write_op1 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $out local.get $cap local.get $count f32.const 6
        local.get $points f32.load
        local.get $points i32.const 4 i32.add f32.load
        call $svg_write_op3 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      i32.const 1 local.set $i
      block $fill_done
        loop $fill_loop
          local.get $i local.get $point_count i32.ge_u br_if $fill_done
          local.get $out local.get $cap local.get $count f32.const 7
            local.get $points local.get $i i32.const 8 i32.mul i32.add f32.load
            local.get $points local.get $i i32.const 8 i32.mul i32.add i32.const 4 i32.add f32.load
            call $svg_write_op3 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
          local.get $i i32.const 1 i32.add local.set $i
          br $fill_loop
        end
      end
      local.get $out local.get $cap local.get $count f32.const 11 call $svg_write_op1 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $out local.get $cap local.get $count f32.const 15 call $svg_write_op1 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    end

    local.get $style i32.const 1 call $er_ui_svg_style_resolve_paint local.set $paint
    local.get $paint i32.const -1 i32.eq if i32.const -1 return end
    local.get $paint i32.const -2 i32.ne
    if
      local.get $style i32.const 1 local.get $out local.get $cap local.get $count call $svg_style_emit_paint local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $style local.get $out local.get $cap local.get $count call $svg_style_emit_stroke_state local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $style i32.load i32.const 1024 i32.and
      if
        local.get $out local.get $cap local.get $count f32.const 30
          local.get $style i32.const 44 i32.add f32.load
          local.get $style i32.const 48 i32.add f32.load
          local.get $style i32.const 52 i32.add f32.load
          call $svg_write_op4 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      end
      local.get $out local.get $cap local.get $count f32.const 6
        local.get $points f32.load
        local.get $points i32.const 4 i32.add f32.load
        call $svg_write_op3 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      i32.const 1 local.set $i
      block $stroke_done
        loop $stroke_loop
          local.get $i local.get $point_count i32.ge_u br_if $stroke_done
          local.get $out local.get $cap local.get $count f32.const 7
            local.get $points local.get $i i32.const 8 i32.mul i32.add f32.load
            local.get $points local.get $i i32.const 8 i32.mul i32.add i32.const 4 i32.add f32.load
            call $svg_write_op3 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
          local.get $i i32.const 1 i32.add local.set $i
          br $stroke_loop
        end
      end
      local.get $out local.get $cap local.get $count f32.const 11 call $svg_write_op1 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    end
    local.get $count)
  (func $svg_norm_x (param $x f32) (param $min_x f32) (param $vw f32) (result f32)
    local.get $x local.get $min_x f32.sub local.get $vw f32.div)
  (func $svg_norm_y (param $y f32) (param $min_y f32) (param $vh f32) (result f32)
    local.get $y local.get $min_y f32.sub local.get $vh f32.div)
  (func $svg_tx_norm_x (param $x f32) (param $y f32) (param $min_x f32) (param $vw f32) (param $a f32) (param $c f32) (param $tx f32) (result f32)
    local.get $a local.get $x f32.mul
    local.get $c local.get $y f32.mul
    f32.add
    local.get $tx
    f32.add
    local.get $min_x
    f32.sub
    local.get $vw
    f32.div)
  (func $svg_tx_norm_y (param $x f32) (param $y f32) (param $min_y f32) (param $vh f32) (param $b f32) (param $d f32) (param $ty f32) (result f32)
    local.get $b local.get $x f32.mul
    local.get $d local.get $y f32.mul
    f32.add
    local.get $ty
    f32.add
    local.get $min_y
    f32.sub
    local.get $vh
    f32.div)

  (func $er_ui_svg_path_parse_to_ir_transform_impl  (param $ptr i32) (param $len i32) (param $out i32) (param $cap i32) (param $min_x f32) (param $min_y f32) (param $vw f32) (param $vh f32) (param $a f32) (param $b f32) (param $c f32) (param $d f32) (param $tx f32) (param $ty f32) (result i32)
    (local $idx i32) (local $idx_slot i32) (local $cmd i32) (local $ch i32) (local $count i32) (local $started i32)
    (local $has_prev_c i32) (local $has_prev_q i32)
    (local $cx f32) (local $cy f32) (local $sx f32) (local $sy f32) (local $x f32) (local $y f32)
    (local $c0x f32) (local $c0y f32) (local $c1x f32) (local $c1y f32) (local $prev_cx f32) (local $prev_cy f32) (local $prev_qx f32) (local $prev_qy f32)
    (local $rx f32) (local $ry f32) (local $rot f32) (local $large f32) (local $sweep f32)
    (local $arc_rx f32) (local $arc_ry f32) (local $arc_sweep f32)
    local.get $ptr i32.eqz
    local.get $out i32.eqz i32.or
    local.get $vw f32.const 0 f32.le i32.or
    local.get $vh f32.const 0 f32.le i32.or
    if i32.const -1 return end
    i32.const 65520
    local.set $idx_slot
    block $done
      loop $loop
        local.get $ptr local.get $len local.get $idx call $svg_skip_separators local.set $idx
        local.get $idx local.get $len i32.ge_u br_if $done
        local.get $ptr local.get $idx i32.add i32.load8_u local.tee $ch call $svg_is_command
        if
          local.get $ch local.set $cmd
          local.get $idx i32.const 1 i32.add local.set $idx
        end
        local.get $cmd i32.eqz
        if i32.const -1 return end
        local.get $started
        i32.eqz
        local.get $cmd
        i32.const 77
        i32.ne
        local.get $cmd
        i32.const 109
        i32.ne
        i32.and
        i32.and
        if i32.const -1 return end
        local.get $idx_slot local.get $idx i32.store
        local.get $cmd i32.const 77 i32.eq
        local.get $cmd i32.const 109 i32.eq i32.or
        if
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $x
          local.get $idx_slot i32.load i32.const -1 i32.eq if i32.const -1 return end
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $y
          local.get $idx_slot i32.load i32.const -1 i32.eq if i32.const -1 return end
          local.get $idx_slot i32.load local.set $idx
          i32.const 1 local.set $started
          local.get $cmd i32.const 109 i32.eq
          if
            local.get $cx local.get $x f32.add local.set $x
            local.get $cy local.get $y f32.add local.set $y
          end
          local.get $x local.set $cx local.get $y local.set $cy local.get $x local.set $sx local.get $y local.set $sy
          i32.const 0 local.set $has_prev_c
          i32.const 0 local.set $has_prev_q
          local.get $out local.get $cap local.get $count f32.const 6
            local.get $x local.get $y local.get $min_x local.get $vw local.get $a local.get $c local.get $tx call $svg_tx_norm_x
            local.get $x local.get $y local.get $min_y local.get $vh local.get $b local.get $d local.get $ty call $svg_tx_norm_y
            call $svg_write_op3 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
          local.get $cmd i32.const 109 i32.eq if i32.const 108 local.set $cmd else i32.const 76 local.set $cmd end
          br $loop
        end
        local.get $cmd i32.const 76 i32.eq
        local.get $cmd i32.const 108 i32.eq i32.or
        if
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $x
          local.get $idx_slot i32.load i32.const -1 i32.eq if i32.const -1 return end
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $y
          local.get $idx_slot i32.load i32.const -1 i32.eq if i32.const -1 return end
          local.get $idx_slot i32.load local.set $idx
          local.get $cmd i32.const 108 i32.eq if local.get $cx local.get $x f32.add local.set $x local.get $cy local.get $y f32.add local.set $y end
          local.get $x local.set $cx local.get $y local.set $cy
          i32.const 0 local.set $has_prev_c
          i32.const 0 local.set $has_prev_q
          local.get $out local.get $cap local.get $count f32.const 7
            local.get $x local.get $y local.get $min_x local.get $vw local.get $a local.get $c local.get $tx call $svg_tx_norm_x
            local.get $x local.get $y local.get $min_y local.get $vh local.get $b local.get $d local.get $ty call $svg_tx_norm_y
            call $svg_write_op3 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
          br $loop
        end
        local.get $cmd i32.const 72 i32.eq
        local.get $cmd i32.const 104 i32.eq i32.or
        if
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $x
          local.get $idx_slot i32.load i32.const -1 i32.eq if i32.const -1 return end
          local.get $idx_slot i32.load local.set $idx
          local.get $cmd i32.const 104 i32.eq if local.get $cx local.get $x f32.add local.set $x end
          local.get $x local.set $cx
          i32.const 0 local.set $has_prev_c
          i32.const 0 local.set $has_prev_q
          local.get $out local.get $cap local.get $count f32.const 7
            local.get $cx local.get $cy local.get $min_x local.get $vw local.get $a local.get $c local.get $tx call $svg_tx_norm_x
            local.get $cx local.get $cy local.get $min_y local.get $vh local.get $b local.get $d local.get $ty call $svg_tx_norm_y
            call $svg_write_op3 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
          br $loop
        end
        local.get $cmd i32.const 86 i32.eq
        local.get $cmd i32.const 118 i32.eq i32.or
        if
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $y
          local.get $idx_slot i32.load i32.const -1 i32.eq if i32.const -1 return end
          local.get $idx_slot i32.load local.set $idx
          local.get $cmd i32.const 118 i32.eq if local.get $cy local.get $y f32.add local.set $y end
          local.get $y local.set $cy
          i32.const 0 local.set $has_prev_c
          i32.const 0 local.set $has_prev_q
          local.get $out local.get $cap local.get $count f32.const 7
            local.get $cx local.get $cy local.get $min_x local.get $vw local.get $a local.get $c local.get $tx call $svg_tx_norm_x
            local.get $cx local.get $cy local.get $min_y local.get $vh local.get $b local.get $d local.get $ty call $svg_tx_norm_y
            call $svg_write_op3 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
          br $loop
        end
        local.get $cmd i32.const 67 i32.eq
        local.get $cmd i32.const 99 i32.eq i32.or
        if
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $c0x
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $c0y
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $c1x
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $c1y
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $x
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $y
          local.get $idx_slot i32.load i32.const -1 i32.eq if i32.const -1 return end
          local.get $idx_slot i32.load local.set $idx
          local.get $cmd i32.const 99 i32.eq
          if
            local.get $cx local.get $c0x f32.add local.set $c0x
            local.get $cy local.get $c0y f32.add local.set $c0y
            local.get $cx local.get $c1x f32.add local.set $c1x
            local.get $cy local.get $c1y f32.add local.set $c1y
            local.get $cx local.get $x f32.add local.set $x
            local.get $cy local.get $y f32.add local.set $y
          end
          local.get $c1x local.set $prev_cx
          local.get $c1y local.set $prev_cy
          i32.const 1 local.set $has_prev_c
          i32.const 0 local.set $has_prev_q
          local.get $x local.set $cx local.get $y local.set $cy
          local.get $out local.get $cap local.get $count f32.const 9
            local.get $c0x local.get $c0y local.get $min_x local.get $vw local.get $a local.get $c local.get $tx call $svg_tx_norm_x
            local.get $c0x local.get $c0y local.get $min_y local.get $vh local.get $b local.get $d local.get $ty call $svg_tx_norm_y
            local.get $c1x local.get $c1y local.get $min_x local.get $vw local.get $a local.get $c local.get $tx call $svg_tx_norm_x
            local.get $c1x local.get $c1y local.get $min_y local.get $vh local.get $b local.get $d local.get $ty call $svg_tx_norm_y
            local.get $x local.get $y local.get $min_x local.get $vw local.get $a local.get $c local.get $tx call $svg_tx_norm_x
            local.get $x local.get $y local.get $min_y local.get $vh local.get $b local.get $d local.get $ty call $svg_tx_norm_y
            call $svg_write_op7 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
          br $loop
        end
        local.get $cmd i32.const 83 i32.eq
        local.get $cmd i32.const 115 i32.eq i32.or
        if
          local.get $has_prev_c
          if
            local.get $cx f32.const 2 f32.mul local.get $prev_cx f32.sub local.set $c0x
            local.get $cy f32.const 2 f32.mul local.get $prev_cy f32.sub local.set $c0y
          else
            local.get $cx local.set $c0x
            local.get $cy local.set $c0y
          end
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $c1x
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $c1y
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $x
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $y
          local.get $idx_slot i32.load i32.const -1 i32.eq if i32.const -1 return end
          local.get $idx_slot i32.load local.set $idx
          local.get $cmd i32.const 115 i32.eq
          if
            local.get $cx local.get $c1x f32.add local.set $c1x
            local.get $cy local.get $c1y f32.add local.set $c1y
            local.get $cx local.get $x f32.add local.set $x
            local.get $cy local.get $y f32.add local.set $y
          end
          local.get $c1x local.set $prev_cx
          local.get $c1y local.set $prev_cy
          i32.const 1 local.set $has_prev_c
          i32.const 0 local.set $has_prev_q
          local.get $x local.set $cx local.get $y local.set $cy
          local.get $out local.get $cap local.get $count f32.const 9
            local.get $c0x local.get $c0y local.get $min_x local.get $vw local.get $a local.get $c local.get $tx call $svg_tx_norm_x
            local.get $c0x local.get $c0y local.get $min_y local.get $vh local.get $b local.get $d local.get $ty call $svg_tx_norm_y
            local.get $c1x local.get $c1y local.get $min_x local.get $vw local.get $a local.get $c local.get $tx call $svg_tx_norm_x
            local.get $c1x local.get $c1y local.get $min_y local.get $vh local.get $b local.get $d local.get $ty call $svg_tx_norm_y
            local.get $x local.get $y local.get $min_x local.get $vw local.get $a local.get $c local.get $tx call $svg_tx_norm_x
            local.get $x local.get $y local.get $min_y local.get $vh local.get $b local.get $d local.get $ty call $svg_tx_norm_y
            call $svg_write_op7 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
          br $loop
        end
        local.get $cmd i32.const 81 i32.eq
        local.get $cmd i32.const 113 i32.eq i32.or
        if
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $c0x
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $c0y
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $x
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $y
          local.get $idx_slot i32.load i32.const -1 i32.eq if i32.const -1 return end
          local.get $idx_slot i32.load local.set $idx
          local.get $cmd i32.const 113 i32.eq
          if
            local.get $cx local.get $c0x f32.add local.set $c0x
            local.get $cy local.get $c0y f32.add local.set $c0y
            local.get $cx local.get $x f32.add local.set $x
            local.get $cy local.get $y f32.add local.set $y
          end
          local.get $c0x local.set $prev_qx
          local.get $c0y local.set $prev_qy
          i32.const 0 local.set $has_prev_c
          i32.const 1 local.set $has_prev_q
          local.get $x local.set $cx local.get $y local.set $cy
          local.get $out local.get $cap local.get $count f32.const 8
            local.get $c0x local.get $c0y local.get $min_x local.get $vw local.get $a local.get $c local.get $tx call $svg_tx_norm_x
            local.get $c0x local.get $c0y local.get $min_y local.get $vh local.get $b local.get $d local.get $ty call $svg_tx_norm_y
            local.get $x local.get $y local.get $min_x local.get $vw local.get $a local.get $c local.get $tx call $svg_tx_norm_x
            local.get $x local.get $y local.get $min_y local.get $vh local.get $b local.get $d local.get $ty call $svg_tx_norm_y
            call $svg_write_op5 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
          br $loop
        end
        local.get $cmd i32.const 84 i32.eq
        local.get $cmd i32.const 116 i32.eq i32.or
        if
          local.get $has_prev_q
          if
            local.get $cx f32.const 2 f32.mul local.get $prev_qx f32.sub local.set $c0x
            local.get $cy f32.const 2 f32.mul local.get $prev_qy f32.sub local.set $c0y
          else
            local.get $cx local.set $c0x
            local.get $cy local.set $c0y
          end
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $x
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $y
          local.get $idx_slot i32.load i32.const -1 i32.eq if i32.const -1 return end
          local.get $idx_slot i32.load local.set $idx
          local.get $cmd i32.const 116 i32.eq
          if
            local.get $cx local.get $x f32.add local.set $x
            local.get $cy local.get $y f32.add local.set $y
          end
          local.get $c0x local.set $prev_qx
          local.get $c0y local.set $prev_qy
          i32.const 0 local.set $has_prev_c
          i32.const 1 local.set $has_prev_q
          local.get $x local.set $cx local.get $y local.set $cy
          local.get $out local.get $cap local.get $count f32.const 8
            local.get $c0x local.get $c0y local.get $min_x local.get $vw local.get $a local.get $c local.get $tx call $svg_tx_norm_x
            local.get $c0x local.get $c0y local.get $min_y local.get $vh local.get $b local.get $d local.get $ty call $svg_tx_norm_y
            local.get $x local.get $y local.get $min_x local.get $vw local.get $a local.get $c local.get $tx call $svg_tx_norm_x
            local.get $x local.get $y local.get $min_y local.get $vh local.get $b local.get $d local.get $ty call $svg_tx_norm_y
            call $svg_write_op5 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
          br $loop
        end
        local.get $cmd i32.const 65 i32.eq
        local.get $cmd i32.const 97 i32.eq i32.or
        if
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number f32.abs local.set $rx
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number f32.abs local.set $ry
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $rot
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_flag local.set $large
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_flag local.set $sweep
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $x
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $y
          local.get $idx_slot i32.load i32.const -1 i32.eq if i32.const -1 return end
          local.get $idx_slot i32.load local.set $idx
          local.get $cmd i32.const 97 i32.eq
          if
            local.get $cx local.get $x f32.add local.set $x
            local.get $cy local.get $y f32.add local.set $y
          end
          i32.const 0 local.set $has_prev_c
          i32.const 0 local.set $has_prev_q
          local.get $rx f32.const 0.00001 f32.le
          local.get $ry f32.const 0.00001 f32.le
          i32.or
          if
            local.get $x local.set $cx local.get $y local.set $cy
            local.get $out local.get $cap local.get $count f32.const 7
              local.get $x local.get $y local.get $min_x local.get $vw local.get $a local.get $c local.get $tx call $svg_tx_norm_x
              local.get $x local.get $y local.get $min_y local.get $vh local.get $b local.get $d local.get $ty call $svg_tx_norm_y
              call $svg_write_op3 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
            br $loop
          end
          local.get $x local.set $cx local.get $y local.set $cy
          local.get $rx
          local.get $a local.get $a f32.mul
          local.get $b local.get $b f32.mul
          f32.add
          f32.sqrt
          f32.mul
          local.set $arc_rx
          local.get $ry
          local.get $c local.get $c f32.mul
          local.get $d local.get $d f32.mul
          f32.add
          f32.sqrt
          f32.mul
          local.set $arc_ry
          local.get $sweep local.set $arc_sweep
          local.get $a local.get $d f32.mul
          local.get $b local.get $c f32.mul
          f32.sub
          f32.const 0
          f32.lt
          if
            f32.const 1
            local.get $sweep
            f32.sub
            local.set $arc_sweep
          end
          local.get $out local.get $cap local.get $count f32.const 10
            local.get $arc_rx local.get $vw f32.div
            local.get $arc_ry local.get $vh f32.div
            local.get $rot
            local.get $large
            local.get $arc_sweep
            local.get $x local.get $y local.get $min_x local.get $vw local.get $a local.get $c local.get $tx call $svg_tx_norm_x
            local.get $x local.get $y local.get $min_y local.get $vh local.get $b local.get $d local.get $ty call $svg_tx_norm_y
            call $svg_write_op8 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
          br $loop
        end
        local.get $cmd i32.const 90 i32.eq
        local.get $cmd i32.const 122 i32.eq i32.or
        if
          local.get $sx local.set $cx local.get $sy local.set $cy
          i32.const 0 local.set $has_prev_c
          i32.const 0 local.set $has_prev_q
          local.get $out local.get $cap local.get $count f32.const 11 call $svg_write_op1 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
          i32.const 0 local.set $cmd
          br $loop
        end
        i32.const -1
        return
      end
    end
    local.get $count)

  (func $er_ui_svg_path_parse_to_ir  (param $ptr i32) (param $len i32) (param $out i32) (param $cap i32) (param $min_x f32) (param $min_y f32) (param $vw f32) (param $vh f32) (result i32)
    local.get $ptr
    local.get $len
    local.get $out
    local.get $cap
    local.get $min_x
    local.get $min_y
    local.get $vw
    local.get $vh
    f32.const 1
    f32.const 0
    f32.const 0
    f32.const 1
    f32.const 0
    f32.const 0
    call $er_ui_svg_path_parse_to_ir_transform_impl)
