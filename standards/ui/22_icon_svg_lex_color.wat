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

  (func $svg_color_pack (param $r i32) (param $g i32) (param $b i32) (param $a i32) (result i32)
    local.get $r i32.const 255 i32.and
    local.get $g i32.const 255 i32.and i32.const 8 i32.shl i32.or
    local.get $b i32.const 255 i32.and i32.const 16 i32.shl i32.or
    local.get $a i32.const 255 i32.and i32.const 24 i32.shl i32.or)
