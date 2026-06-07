(func $font_write_command (param $out i32) (param $cap i32) (param $count i32) (param $src i32) (result i32)
    (local $dst i32)
    local.get $count
    i32.const 1
    i32.add
    i32.const 20
    i32.mul
    local.get $cap
    i32.gt_u
    if
      i32.const -1
      return
    end
    local.get $out
    local.get $count
    i32.const 20
    i32.mul
    i32.add
    local.set $dst
    local.get $dst
    local.get $src
    i32.const 20
    call $copy
    local.get $count
    i32.const 1
    i32.add)

  (func $font_write_command_scaled (param $out i32) (param $cap i32) (param $count i32) (param $src i32) (param $x f32) (param $baseline_y f32) (param $scale f32) (param $dpr f32) (result i32)
    (local $dst i32)
    (local $op i32)
    local.get $count
    i32.const 1
    i32.add
    i32.const 20
    i32.mul
    local.get $cap
    i32.gt_u
    if
      i32.const -1
      return
    end
    local.get $out
    local.get $count
    i32.const 20
    i32.mul
    i32.add
    local.set $dst
    local.get $dst
    local.get $src
    i32.load
    local.tee $op
    i32.store
    local.get $op
    i32.const 4
    i32.eq
    if
      local.get $dst
      i32.const 4
      i32.add
      i32.const 16
      call $zero
      local.get $count
      i32.const 1
      i32.add
      return
    end
    local.get $dst
    i32.const 4
    i32.add
    local.get $x
    local.get $src
    i32.const 4
    i32.add
    f32.load
    local.get $scale
    f32.mul
    f32.add
    local.get $dpr
    call $snap_pixel
    f32.store
    local.get $dst
    i32.const 8
    i32.add
    local.get $baseline_y
    local.get $src
    i32.const 8
    i32.add
    f32.load
    local.get $scale
    f32.mul
    f32.sub
    local.get $dpr
    call $snap_pixel
    f32.store
    local.get $op
    i32.const 3
    i32.eq
    if
      local.get $dst
      i32.const 12
      i32.add
      local.get $x
      local.get $src
      i32.const 12
      i32.add
      f32.load
      local.get $scale
      f32.mul
      f32.add
      local.get $dpr
      call $snap_pixel
      f32.store
      local.get $dst
      i32.const 16
      i32.add
      local.get $baseline_y
      local.get $src
      i32.const 16
      i32.add
      f32.load
      local.get $scale
      f32.mul
      f32.sub
      local.get $dpr
      call $snap_pixel
      f32.store
    else
      local.get $dst
      i32.const 12
      i32.add
      i32.const 8
      call $zero
    end
    local.get $count
    i32.const 1
    i32.add)

  (func (export "er_ui_font_glyph_commands_write") (param $codepoint i32) (param $out i32) (param $cap i32) (result i32)
    (local $glyph i32)
    (local $i i32)
    (local $start i32)
    (local $count i32)
    (local $written i32)
    local.get $codepoint
    call $font_glyph_record_ptr
    local.tee $glyph
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $glyph
    i32.const 8
    i32.add
    i32.load
    local.set $start
    local.get $glyph
    i32.const 12
    i32.add
    i32.load
    local.tee $count
    i32.const 20
    i32.mul
    local.get $cap
    i32.gt_u
    if
      i32.const -1
      return
    end
    block $done
      loop $loop
        local.get $i
        local.get $count
        i32.ge_u
        br_if $done
        local.get $out
        local.get $cap
        local.get $written
        local.get $start
        local.get $i
        i32.add
        call $font_command_ptr
        call $font_write_command
        local.set $written
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    local.get $written)

  (func (export "er_ui_font_glyph_commands_write_scaled") (param $codepoint i32) (param $out i32) (param $cap i32) (param $x f32) (param $baseline_y f32) (param $size f32) (param $dpr f32) (result i32)
    (local $glyph i32)
    (local $i i32)
    (local $start i32)
    (local $count i32)
    (local $written i32)
    (local $scale f32)
    local.get $codepoint
    call $font_glyph_record_ptr
    local.tee $glyph
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $glyph
    i32.const 8
    i32.add
    i32.load
    local.set $start
    local.get $glyph
    i32.const 12
    i32.add
    i32.load
    local.tee $count
    i32.const 20
    i32.mul
    local.get $cap
    i32.gt_u
    if
      i32.const -1
      return
    end
    local.get $size
    call $font_units_per_em
    f32.div
    local.set $scale
    block $done
      loop $loop
        local.get $i
        local.get $count
        i32.ge_u
        br_if $done
        local.get $out
        local.get $cap
        local.get $written
        local.get $start
        local.get $i
        i32.add
        call $font_command_ptr
        local.get $x
        local.get $baseline_y
        local.get $scale
        local.get $dpr
        call $font_write_command_scaled
        local.set $written
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    local.get $written)

  (func $er_ui_font_text_command_count (export "er_ui_font_text_command_count") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $glyph i32)
    (local $count i32)
    global.get $font_body_ptr
    global.get $font_body_len
    call $font_body_valid_at
    i32.eqz
    if
      i32.const -1
      return
    end
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
        call $font_glyph_record_ptr
        local.tee $glyph
        i32.const 0
        i32.ne
        if
          local.get $count
          local.get $glyph
          i32.const 12
          i32.add
          i32.load
          i32.add
          local.set $count
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    local.get $count)

  (func (export "er_ui_font_text_commands_write_scaled") (param $ptr i32) (param $len i32) (param $out i32) (param $cap i32) (param $x f32) (param $baseline_y f32) (param $size f32) (param $dpr f32) (result i32)
    (local $i i32)
    (local $glyph i32)
    (local $start i32)
    (local $cmd_count i32)
    (local $cmd_i i32)
    (local $written i32)
    (local $pen_x f32)
    (local $scale f32)
    global.get $font_body_ptr
    global.get $font_body_len
    call $font_body_valid_at
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $ptr
    local.get $len
    call $er_ui_font_text_command_count
    i32.const 20
    i32.mul
    local.get $cap
    i32.gt_u
    if
      i32.const -1
      return
    end
    local.get $size
    call $font_units_per_em
    f32.div
    local.set $scale
    local.get $x
    local.set $pen_x
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
        call $font_glyph_record_ptr
        local.set $glyph
        local.get $glyph
        i32.const 0
        i32.ne
        if
          local.get $glyph
          i32.const 8
          i32.add
          i32.load
          local.set $start
          local.get $glyph
          i32.const 12
          i32.add
          i32.load
          local.set $cmd_count
          i32.const 0
          local.set $cmd_i
          block $glyph_done
            loop $glyph_loop
              local.get $cmd_i
              local.get $cmd_count
              i32.ge_u
              br_if $glyph_done
              local.get $out
              local.get $cap
              local.get $written
              local.get $start
              local.get $cmd_i
              i32.add
              call $font_command_ptr
              local.get $pen_x
              local.get $baseline_y
              local.get $scale
              local.get $dpr
              call $font_write_command_scaled
              local.set $written
              local.get $cmd_i
              i32.const 1
              i32.add
              local.set $cmd_i
              br $glyph_loop
            end
          end
          local.get $pen_x
          local.get $glyph
          i32.const 16
          i32.add
          f32.load
          local.get $scale
          f32.mul
          f32.add
          local.set $pen_x
        end
        local.get $i
