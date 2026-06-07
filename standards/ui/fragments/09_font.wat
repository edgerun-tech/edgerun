(module
  ;; Imports from other UI fragments
  (import "ui" "clamp_f32" (func $clamp_f32 (param f32) (param f32) (param f32) (result f32)))
  (import "ui" "copy" (func $copy (param i32) (param i32) (param i32)))
  (import "ui" "max_f32" (func $max_f32 (param f32) (param f32) (result f32)))
  (import "ui" "min_f32" (func $min_f32 (param f32) (param f32) (result f32)))
  (import "ui" "min_i32_u" (func $min_i32_u (param i32) (param i32) (result i32)))
  (import "ui" "snap_pixel" (func $snap_pixel (param f32) (param f32) (result f32)))
  (import "ui" "zero" (func $zero (param i32) (param i32)))

  (func $er_ui_font_glyph_key  (param $codepoint i32) (param $weight i32) (result i32)
    local.get $codepoint
    i32.const 0x10ffff
    i32.gt_u
    if
      i32.const 0
      return
    end
    local.get $weight
    i32.const 2
    i32.gt_u
    if
      i32.const 0
      return
    end
    local.get $codepoint
    i32.const 4
    i32.mul
    local.get $weight
    i32.add)

  (func $er_ui_font_glyph_key_codepoint  (param $glyph_key i32) (result i32)
    local.get $glyph_key
    i32.const 2
    i32.shr_u)

  (func $er_ui_font_glyph_key_weight  (param $glyph_key i32) (result i32)
    local.get $glyph_key
    i32.const 3
    i32.and)

  (func $font_body_valid_at (param $ptr i32) (param $len i32) (result i32)
    (local $glyphs i32)
    (local $commands i32)
    (local $kerns i32)
    (local $total i32)
    local.get $len
    i32.const 48
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $ptr
    i32.load
    i32.const 0x4e465245
    i32.ne
    if
      i32.const 0
      return
    end
    local.get $ptr
    i32.const 4
    i32.add
    i32.load
    i32.const 0x0a335654
    i32.ne
    if
      i32.const 0
      return
    end
    local.get $ptr
    i32.const 8
    i32.add
    i32.load16_u
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $ptr
    i32.const 10
    i32.add
    i32.load16_u
    i32.const 0
    i32.ne
    if
      i32.const 0
      return
    end
    local.get $ptr
    i32.const 44
    i32.add
    i32.load
    i32.const 0
    i32.ne
    if
      i32.const 0
      return
    end
    local.get $ptr
    i32.const 12
    i32.add
    i32.load
    local.set $glyphs
    local.get $ptr
    i32.const 16
    i32.add
    i32.load
    local.set $commands
    local.get $ptr
    i32.const 20
    i32.add
    i32.load
    local.set $kerns
    i32.const 48
    local.get $glyphs
    i32.const 20
    i32.mul
    i32.add
    local.get $kerns
    i32.const 12
    i32.mul
    i32.add
    local.get $commands
    i32.const 20
    i32.mul
    i32.add
    local.set $total
    local.get $total
    local.get $len
    i32.eq)

  (func $er_ui_font_body_set  (param $ptr i32) (param $len i32) (result i32)
    local.get $ptr
    local.get $len
    call $font_body_valid_at
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $ptr
    global.set $font_body_ptr
    local.get $len
    global.set $font_body_len
    i32.const 0
    global.set $font_weight
    local.get $ptr
    global.set $font_regular_ptr
    local.get $len
    global.set $font_regular_len
    i32.const 1)

  (func $er_ui_font_body_set_weight  (param $weight i32) (param $ptr i32) (param $len i32) (result i32)
    local.get $ptr
    local.get $len
    call $font_body_valid_at
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $weight
    i32.const 0
    i32.eq
    if
      local.get $ptr
      global.set $font_regular_ptr
      local.get $len
      global.set $font_regular_len
      i32.const 1
      return
    end
    local.get $weight
    i32.const 1
    i32.eq
    if
      local.get $ptr
      global.set $font_semibold_ptr
      local.get $len
      global.set $font_semibold_len
      i32.const 1
      return
    end
    local.get $weight
    i32.const 2
    i32.eq
    if
      local.get $ptr
      global.set $font_bold_ptr
      local.get $len
      global.set $font_bold_len
      i32.const 1
      return
    end
    i32.const 0)

  (func $er_ui_font_select_weight  (param $weight i32) (result i32)
    local.get $weight
    i32.const 0
    i32.eq
    if
      global.get $font_regular_ptr
      global.get $font_regular_len
      call $font_body_valid_at
      i32.eqz
      if
        i32.const 0
        return
      end
      global.get $font_regular_ptr
      global.set $font_body_ptr
      global.get $font_regular_len
      global.set $font_body_len
      i32.const 0
      global.set $font_weight
      i32.const 1
      return
    end
    local.get $weight
    i32.const 1
    i32.eq
    if
      global.get $font_semibold_ptr
      global.get $font_semibold_len
      call $font_body_valid_at
      i32.eqz
      if
        i32.const 0
        return
      end
      global.get $font_semibold_ptr
      global.set $font_body_ptr
      global.get $font_semibold_len
      global.set $font_body_len
      i32.const 1
      global.set $font_weight
      i32.const 1
      return
    end
    local.get $weight
    i32.const 2
    i32.eq
    if
      global.get $font_bold_ptr
      global.get $font_bold_len
      call $font_body_valid_at
      i32.eqz
      if
        i32.const 0
        return
      end
      global.get $font_bold_ptr
      global.set $font_body_ptr
      global.get $font_bold_len
      global.set $font_body_len
      i32.const 2
      global.set $font_weight
      i32.const 1
      return
    end
    i32.const 0)

  (func $er_ui_font_selected_weight  (result i32)
    global.get $font_weight)

  (func $er_ui_font_weight_loaded  (param $weight i32) (result i32)
    local.get $weight
    i32.const 0
    i32.eq
    if
      global.get $font_regular_ptr
      global.get $font_regular_len
      call $font_body_valid_at
      return
    end
    local.get $weight
    i32.const 1
    i32.eq
    if
      global.get $font_semibold_ptr
      global.get $font_semibold_len
      call $font_body_valid_at
      return
    end
    local.get $weight
    i32.const 2
    i32.eq
    if
      global.get $font_bold_ptr
      global.get $font_bold_len
      call $font_body_valid_at
      return
    end
    i32.const 0)

  (func $er_ui_font_weight_body_ptr  (param $weight i32) (result i32)
    local.get $weight
    i32.const 0
    i32.eq
    if
      global.get $font_regular_ptr
      return
    end
    local.get $weight
    i32.const 1
    i32.eq
    if
      global.get $font_semibold_ptr
      return
    end
    local.get $weight
    i32.const 2
    i32.eq
    if
      global.get $font_bold_ptr
      return
    end
    i32.const 0)

  (func $er_ui_font_weight_body_len  (param $weight i32) (result i32)
    local.get $weight
    i32.const 0
    i32.eq
    if
      global.get $font_regular_len
      return
    end
    local.get $weight
    i32.const 1
    i32.eq
    if
      global.get $font_semibold_len
      return
    end
    local.get $weight
    i32.const 2
    i32.eq
    if
      global.get $font_bold_len
      return
    end
    i32.const 0)

  (func $font_glyph_count (result i32)
    global.get $font_body_ptr
    i32.const 12
    i32.add
    i32.load)

  (func $font_command_count (result i32)
    global.get $font_body_ptr
    i32.const 16
    i32.add
    i32.load)

  (func $font_kern_count (result i32)
    global.get $font_body_ptr
    i32.const 20
    i32.add
    i32.load)

  (func $font_units_per_em (result f32)
    global.get $font_body_ptr
    i32.const 8
    i32.add
    i32.load16_u
    f32.convert_i32_u)

  (func $font_glyph_record_ptr_by_index (param $index i32) (result i32)
    global.get $font_body_ptr
    i32.const 48
    i32.add
    local.get $index
    i32.const 20
    i32.mul
    i32.add)

  (func $font_glyph_record_ptr (param $codepoint i32) (result i32)
    (local $i i32)
    (local $count i32)
    global.get $font_body_ptr
    global.get $font_body_len
    call $font_body_valid_at
    i32.eqz
    if
      i32.const 0
      return
    end
    call $font_glyph_count
    local.set $count
    block $done
      loop $loop
        local.get $i
        local.get $count
        i32.ge_u
        br_if $done
        local.get $i
        call $font_glyph_record_ptr_by_index
        i32.load
        local.get $codepoint
        i32.eq
        if
          local.get $i
          call $font_glyph_record_ptr_by_index
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    i32.const 0)

  (func $er_ui_font_ref_glyph_index  (param $codepoint i32) (result i32)
    (local $i i32)
    (local $count i32)
    global.get $font_body_ptr
    global.get $font_body_len
    call $font_body_valid_at
    i32.eqz
    if
      i32.const -1
      return
    end
    call $font_glyph_count
    local.set $count
    block $done
      loop $loop
        local.get $i
        local.get $count
        i32.ge_u
        br_if $done
        local.get $i
        call $font_glyph_record_ptr_by_index
        i32.load
        local.get $codepoint
        i32.eq
        if
          local.get $i
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    i32.const -1)

  (func $er_ui_font_ref_glyph_id  (param $codepoint i32) (result i32)
    local.get $codepoint
    call $font_glyph_record_ptr
    local.tee $codepoint
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $codepoint
    i32.const 4
    i32.add
    i32.load16_u)

  (func $er_ui_font_ref_glyph_command_offset  (param $codepoint i32) (result i32)
    local.get $codepoint
    call $font_glyph_record_ptr
    local.tee $codepoint
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $codepoint
    i32.const 8
    i32.add
    i32.load)

  (func $er_ui_font_ref_glyph_command_count  (param $codepoint i32) (result i32)
    local.get $codepoint
    call $font_glyph_record_ptr
    local.tee $codepoint
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $codepoint
    i32.const 12
    i32.add
    i32.load)

  (func $er_ui_font_ref_glyph_advance_units  (param $codepoint i32) (result f32)
    local.get $codepoint
    call $font_glyph_record_ptr
    local.tee $codepoint
    i32.eqz
    if
      f32.const 0
      return
    end
    local.get $codepoint
    i32.const 16
    i32.add
    f32.load)

  (func $er_ui_font_ref_glyph_advance  (param $codepoint i32) (param $size f32) (result f32)
    local.get $codepoint
    call $font_glyph_record_ptr
    local.tee $codepoint
    i32.eqz
    if
      f32.const 0
      return
    end
    local.get $codepoint
    i32.const 16
    i32.add
    f32.load
    local.get $size
    f32.mul
    call $font_units_per_em
    f32.div)

  (func $font_command_base (result i32)
    global.get $font_body_ptr
    i32.const 48
    i32.add
    call $font_glyph_count
    i32.const 20
    i32.mul
    i32.add
    call $font_kern_count
    i32.const 12
    i32.mul
    i32.add)

  (func $font_command_ptr (param $command_index i32) (result i32)
    call $font_command_base
    local.get $command_index
    i32.const 20
    i32.mul
    i32.add)

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

  (func $er_ui_font_glyph_commands_write  (param $codepoint i32) (param $out i32) (param $cap i32) (result i32)
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

  (func $er_ui_font_glyph_commands_write_scaled  (param $codepoint i32) (param $out i32) (param $cap i32) (param $x f32) (param $baseline_y f32) (param $size f32) (param $dpr f32) (result i32)
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

  (func $er_ui_font_text_command_count  (param $ptr i32) (param $len i32) (result i32)
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

  (func $er_ui_font_text_commands_write_scaled  (param $ptr i32) (param $len i32) (param $out i32) (param $cap i32) (param $x f32) (param $baseline_y f32) (param $size f32) (param $dpr f32) (result i32)
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
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    local.get $written)

  (func $font_edge_crosses (param $px f32) (param $py f32) (param $x1 f32) (param $y1 f32) (param $x2 f32) (param $y2 f32) (result i32)
    local.get $y1
    local.get $py
    f32.gt
    local.get $y2
    local.get $py
    f32.gt
    i32.xor
    if (result i32)
      local.get $px
      local.get $x2
      local.get $x1
      f32.sub
      local.get $py
      local.get $y1
      f32.sub
      f32.mul
      local.get $y2
      local.get $y1
      f32.sub
      f32.div
      local.get $x1
      f32.add
      f32.lt
    else
      i32.const 0
    end)

  (func $font_point_x (param $pen_x f32) (param $scale f32) (param $src i32) (param $offset i32) (result f32)
    local.get $pen_x
    local.get $src
    local.get $offset
    i32.add
    f32.load
    local.get $scale
    f32.mul
    f32.add)

  (func $font_point_y (param $baseline_y f32) (param $scale f32) (param $src i32) (param $offset i32) (result f32)
    local.get $baseline_y
    local.get $src
    local.get $offset
    i32.add
    f32.load
    local.get $scale
    f32.mul
    f32.sub)

  (func $font_quad_coord (param $p0 f32) (param $c f32) (param $p1 f32) (param $t f32) (result f32)
    (local $mt f32)
    f32.const 1
    local.get $t
    f32.sub
    local.set $mt
    local.get $mt
    local.get $mt
    f32.mul
    local.get $p0
    f32.mul
    f32.const 2
    local.get $mt
    f32.mul
    local.get $t
    f32.mul
    local.get $c
    f32.mul
    f32.add
    local.get $t
    local.get $t
    f32.mul
    local.get $p1
    f32.mul
    f32.add)

  (func $font_glyph_point_inside (param $glyph i32) (param $pen_x f32) (param $baseline_y f32) (param $size f32) (param $px f32) (param $py f32) (result i32)
    (local $start i32)
    (local $cmd_count i32)
    (local $i i32)
    (local $src i32)
    (local $op i32)
    (local $scale f32)
    (local $current_x f32)
    (local $current_y f32)
    (local $start_x f32)
    (local $start_y f32)
    (local $next_x f32)
    (local $next_y f32)
    (local $ctrl_x f32)
    (local $ctrl_y f32)
    (local $prev_x f32)
    (local $prev_y f32)
    (local $seg i32)
    (local $t f32)
    (local $inside i32)
    local.get $size
    call $font_units_per_em
    f32.div
    local.set $scale
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
    block $done
      loop $loop
        local.get $i
        local.get $cmd_count
        i32.ge_u
        br_if $done
        local.get $start
        local.get $i
        i32.add
        call $font_command_ptr
        local.tee $src
        i32.load
        local.set $op
        local.get $op
        i32.const 1
        i32.eq
        if
          local.get $pen_x
          local.get $scale
          local.get $src
          i32.const 4
          call $font_point_x
          local.tee $current_x
          local.set $start_x
          local.get $baseline_y
          local.get $scale
          local.get $src
          i32.const 8
          call $font_point_y
          local.tee $current_y
          local.set $start_y
        else
          local.get $op
          i32.const 2
          i32.eq
          if
            local.get $pen_x
            local.get $scale
            local.get $src
            i32.const 4
            call $font_point_x
            local.set $next_x
            local.get $baseline_y
            local.get $scale
            local.get $src
            i32.const 8
            call $font_point_y
            local.set $next_y
            local.get $px
            local.get $py
            local.get $current_x
            local.get $current_y
            local.get $next_x
            local.get $next_y
            call $font_edge_crosses
            if
              local.get $inside
              i32.const 1
              i32.xor
              local.set $inside
            end
            local.get $next_x
            local.set $current_x
            local.get $next_y
            local.set $current_y
          else
            local.get $op
            i32.const 3
            i32.eq
            if
              local.get $pen_x
              local.get $scale
              local.get $src
              i32.const 4
              call $font_point_x
              local.set $next_x
              local.get $baseline_y
              local.get $scale
              local.get $src
              i32.const 8
              call $font_point_y
              local.set $next_y
              local.get $pen_x
              local.get $scale
              local.get $src
              i32.const 12
              call $font_point_x
              local.set $ctrl_x
              local.get $baseline_y
              local.get $scale
              local.get $src
              i32.const 16
              call $font_point_y
              local.set $ctrl_y
              local.get $current_x
              local.set $prev_x
              local.get $current_y
              local.set $prev_y
              i32.const 1
              local.set $seg
              block $quad_done
                loop $quad_loop
                  local.get $seg
                  i32.const 8
                  i32.gt_u
                  br_if $quad_done
                  local.get $seg
                  f32.convert_i32_u
                  f32.const 8
                  f32.div
                  local.set $t
                  local.get $current_x
                  local.get $ctrl_x
                  local.get $next_x
                  local.get $t
                  call $font_quad_coord
                  local.get $current_y
                  local.get $ctrl_y
                  local.get $next_y
                  local.get $t
                  call $font_quad_coord
                  local.set $start_y
                  local.set $start_x
                  local.get $px
                  local.get $py
                  local.get $prev_x
                  local.get $prev_y
                  local.get $start_x
                  local.get $start_y
                  call $font_edge_crosses
                  if
                    local.get $inside
                    i32.const 1
                    i32.xor
                    local.set $inside
                  end
                  local.get $start_x
                  local.set $prev_x
                  local.get $start_y
                  local.set $prev_y
                  local.get $seg
                  i32.const 1
                  i32.add
                  local.set $seg
                  br $quad_loop
                end
              end
              local.get $next_x
              local.set $current_x
              local.get $next_y
              local.set $current_y
            else
              local.get $op
              i32.const 4
              i32.eq
              if
                local.get $px
                local.get $py
                local.get $current_x
                local.get $current_y
                local.get $start_x
                local.get $start_y
                call $font_edge_crosses
                if
                  local.get $inside
                  i32.const 1
                  i32.xor
                  local.set $inside
                end
                local.get $start_x
                local.set $current_x
                local.get $start_y
                local.set $current_y
              end
            end
          end
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    local.get $inside)

  (func $er_ui_font_text_render_alpha  (param $ptr i32) (param $len i32) (param $alpha i32) (param $width i32) (param $height i32) (param $x f32) (param $baseline_y f32) (param $size f32) (result i32)
    (local $i i32)
    (local $px i32)
    (local $py i32)
    (local $glyph i32)
    (local $pen_x f32)
    (local $drawn i32)
    global.get $font_body_ptr
    global.get $font_body_len
    call $font_body_valid_at
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $width
    i32.eqz
    local.get $height
    i32.eqz
    i32.or
    if
      i32.const -1
      return
    end
    local.get $x
    local.set $pen_x
    block $text_done
      loop $text_loop
        local.get $i
        local.get $len
        i32.ge_u
        br_if $text_done
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
          i32.const 0
          local.set $py
          block $y_done
            loop $y_loop
              local.get $py
              local.get $height
              i32.ge_u
              br_if $y_done
              i32.const 0
              local.set $px
              block $x_done
                loop $x_loop
                  local.get $px
                  local.get $width
                  i32.ge_u
                  br_if $x_done
                  local.get $glyph
                  local.get $pen_x
                  local.get $baseline_y
                  local.get $size
                  local.get $px
                  f32.convert_i32_u
                  f32.const 0.5
                  f32.add
                  local.get $py
                  f32.convert_i32_u
                  f32.const 0.5
                  f32.add
                  call $font_glyph_point_inside
                  if
                    local.get $alpha
                    local.get $py
                    local.get $width
                    i32.mul
                    local.get $px
                    i32.add
                    i32.add
                    i32.const 255
                    i32.store8
                    local.get $drawn
                    i32.const 1
                    i32.add
                    local.set $drawn
                  end
                  local.get $px
                  i32.const 1
                  i32.add
                  local.set $px
                  br $x_loop
                end
              end
              local.get $py
              i32.const 1
              i32.add
              local.set $py
              br $y_loop
            end
          end
          local.get $pen_x
          local.get $glyph
          i32.const 16
          i32.add
          f32.load
          local.get $size
          f32.mul
          call $font_units_per_em
          f32.div
          f32.add
          local.set $pen_x
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $text_loop
      end
    end
    local.get $drawn)

  (func $font_pixel_coverage_2x2 (param $glyph i32) (param $pen_x f32) (param $baseline_y f32) (param $size f32) (param $px i32) (param $py i32) (result i32)
    (local $hits i32)
    local.get $glyph
    local.get $pen_x
    local.get $baseline_y
    local.get $size
    local.get $px
    f32.convert_i32_u
    f32.const 0.25
    f32.add
    local.get $py
    f32.convert_i32_u
    f32.const 0.25
    f32.add
    call $font_glyph_point_inside
    local.set $hits
    local.get $hits
    local.get $glyph
    local.get $pen_x
    local.get $baseline_y
    local.get $size
    local.get $px
    f32.convert_i32_u
    f32.const 0.75
    f32.add
    local.get $py
    f32.convert_i32_u
    f32.const 0.25
    f32.add
    call $font_glyph_point_inside
    i32.add
    local.set $hits
    local.get $hits
    local.get $glyph
    local.get $pen_x
    local.get $baseline_y
    local.get $size
    local.get $px
    f32.convert_i32_u
    f32.const 0.25
    f32.add
    local.get $py
    f32.convert_i32_u
    f32.const 0.75
    f32.add
    call $font_glyph_point_inside
    i32.add
    local.set $hits
    local.get $hits
    local.get $glyph
    local.get $pen_x
    local.get $baseline_y
    local.get $size
    local.get $px
    f32.convert_i32_u
    f32.const 0.75
    f32.add
    local.get $py
    f32.convert_i32_u
    f32.const 0.75
    f32.add
    call $font_glyph_point_inside
    i32.add)

  (func $er_ui_font_text_render_alpha_aa  (param $ptr i32) (param $len i32) (param $alpha i32) (param $width i32) (param $height i32) (param $x f32) (param $baseline_y f32) (param $size f32) (result i32)
    (local $i i32)
    (local $px i32)
    (local $py i32)
    (local $glyph i32)
    (local $pen_x f32)
    (local $drawn i32)
    (local $coverage i32)
    global.get $font_body_ptr
    global.get $font_body_len
    call $font_body_valid_at
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $width
    i32.eqz
    local.get $height
    i32.eqz
    i32.or
    if
      i32.const -1
      return
    end
    local.get $x
    local.set $pen_x
    block $text_done
      loop $text_loop
        local.get $i
        local.get $len
        i32.ge_u
        br_if $text_done
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
          i32.const 0
          local.set $py
          block $y_done
            loop $y_loop
              local.get $py
              local.get $height
              i32.ge_u
              br_if $y_done
              i32.const 0
              local.set $px
              block $x_done
                loop $x_loop
                  local.get $px
                  local.get $width
                  i32.ge_u
                  br_if $x_done
                  local.get $glyph
                  local.get $pen_x
                  local.get $baseline_y
                  local.get $size
                  local.get $px
                  local.get $py
                  call $font_pixel_coverage_2x2
                  local.tee $coverage
                  i32.const 0
                  i32.ne
                  if
                    local.get $alpha
                    local.get $py
                    local.get $width
                    i32.mul
                    local.get $px
                    i32.add
                    i32.add
                    local.get $coverage
                    i32.const 255
                    i32.mul
                    i32.const 4
                    i32.div_u
                    i32.store8
                    local.get $drawn
                    i32.const 1
                    i32.add
                    local.set $drawn
                  end
                  local.get $px
                  i32.const 1
                  i32.add
                  local.set $px
                  br $x_loop
                end
              end
              local.get $py
              i32.const 1
              i32.add
              local.set $py
              br $y_loop
            end
          end
          local.get $pen_x
          local.get $glyph
          i32.const 16
          i32.add
          f32.load
          local.get $size
          f32.mul
          call $font_units_per_em
          f32.div
          f32.add
          local.set $pen_x
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $text_loop
      end
    end
    local.get $drawn)
  (func $font_sample_offset (param $i i32) (result f32)
    local.get $i
    f32.convert_i32_u
    f32.const 0.5
    f32.add
    f32.const 8
    f32.div)

  (func $font_sharpen_sample (param $sample i32) (result i32)
    local.get $sample
    i32.eqz
    local.get $sample
    i32.const 255
    i32.eq
    i32.or
    if
      local.get $sample
      return
    end
    f32.const 128
    local.get $sample
    f32.convert_i32_u
    f32.const 128
    f32.sub
    f32.const 1.14
    f32.mul
    f32.add
    f32.const 6
    f32.add
    f32.const 0
    f32.const 255
    call $clamp_f32
    f32.nearest
    i32.trunc_f32_u)

  (func $font_edge_ptr (param $index i32) (result i32)
    i32.const 900000
    local.get $index
    i32.const 16
    i32.mul
    i32.add)

  (func $font_hit_ptr (param $index i32) (result i32)
    i32.const 934000
    local.get $index
    i32.const 8
    i32.mul
    i32.add)

  (func $font_edge_append (param $count i32) (param $x1 f32) (param $y1 f32) (param $x2 f32) (param $y2 f32) (result i32)
    (local $p i32)
    local.get $count
    i32.const 2048
    i32.ge_u
    if
      i32.const -1
      return
    end
    local.get $x1
    local.get $x2
    f32.eq
    local.get $y1
    local.get $y2
    f32.eq
    i32.and
    if
      local.get $count
      return
    end
    local.get $count
    call $font_edge_ptr
    local.set $p
    local.get $p
    local.get $x1
    f32.store
    local.get $p
    i32.const 4
    i32.add
    local.get $y1
    f32.store
    local.get $p
    i32.const 8
    i32.add
    local.get $x2
    f32.store
    local.get $p
    i32.const 12
    i32.add
    local.get $y2
    f32.store
    local.get $count
    i32.const 1
    i32.add)

  (func $font_transform_x (param $src i32) (param $offset i32) (param $scale f32) (result f32)
    local.get $src
    local.get $offset
    i32.add
    f32.load
    local.get $scale
    f32.mul)

  (func $font_transform_y (param $src i32) (param $offset i32) (param $scale f32) (result f32)
    f32.const -0
    local.get $src
    local.get $offset
    i32.add
    f32.load
    local.get $scale
    f32.mul
    f32.sub)

  (func $font_flatten_edges (param $glyph i32) (param $scale f32) (result i32)
    (local $start i32)
    (local $cmd_count i32)
    (local $i i32)
    (local $src i32)
    (local $op i32)
    (local $edge_count i32)
    (local $has_current i32)
    (local $current_x f32)
    (local $current_y f32)
    (local $contour_x f32)
    (local $contour_y f32)
    (local $next_x f32)
    (local $next_y f32)
    (local $ctrl_x f32)
    (local $ctrl_y f32)
    (local $prev_x f32)
    (local $prev_y f32)
    (local $step i32)
    (local $t f32)
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
    block $done
      loop $loop
        local.get $i
        local.get $cmd_count
        i32.ge_u
        br_if $done
        local.get $start
        local.get $i
        i32.add
        call $font_command_ptr
        local.tee $src
        i32.load
        local.set $op
        local.get $op
        i32.const 1
        i32.eq
        if
          local.get $src
          i32.const 4
          local.get $scale
          call $font_transform_x
          local.tee $current_x
          local.set $contour_x
          local.get $src
          i32.const 8
          local.get $scale
          call $font_transform_y
          local.tee $current_y
          local.set $contour_y
          i32.const 1
          local.set $has_current
        else
          local.get $op
          i32.const 2
          i32.eq
          if
            local.get $has_current
            i32.eqz
            if
              i32.const -1
              return
            end
            local.get $src
            i32.const 4
            local.get $scale
            call $font_transform_x
            local.set $next_x
            local.get $src
            i32.const 8
            local.get $scale
            call $font_transform_y
            local.set $next_y
            local.get $edge_count
            local.get $current_x
            local.get $current_y
            local.get $next_x
            local.get $next_y
            call $font_edge_append
            local.tee $edge_count
            i32.const -1
            i32.eq
            if
              i32.const -1
              return
            end
            local.get $next_x
            local.set $current_x
            local.get $next_y
            local.set $current_y
          else
            local.get $op
            i32.const 3
            i32.eq
            if
              local.get $has_current
              i32.eqz
              if
                i32.const -1
                return
              end
              local.get $src
              i32.const 12
              local.get $scale
              call $font_transform_x
              local.set $ctrl_x
              local.get $src
              i32.const 16
              local.get $scale
              call $font_transform_y
              local.set $ctrl_y
              local.get $src
              i32.const 4
              local.get $scale
              call $font_transform_x
              local.set $next_x
              local.get $src
              i32.const 8
              local.get $scale
              call $font_transform_y
              local.set $next_y
              local.get $current_x
              local.set $prev_x
              local.get $current_y
              local.set $prev_y
              i32.const 1
              local.set $step
              block $quad_done
                loop $quad_loop
                  local.get $step
                  i32.const 10
                  i32.gt_u
                  br_if $quad_done
                  local.get $step
                  f32.convert_i32_u
                  f32.const 10
                  f32.div
                  local.set $t
                  local.get $current_x
                  local.get $ctrl_x
                  local.get $next_x
                  local.get $t
                  call $font_quad_coord
                  local.get $current_y
                  local.get $ctrl_y
                  local.get $next_y
                  local.get $t
                  call $font_quad_coord
                  local.set $contour_y
                  local.set $contour_x
                  local.get $edge_count
                  local.get $prev_x
                  local.get $prev_y
                  local.get $contour_x
                  local.get $contour_y
                  call $font_edge_append
                  local.tee $edge_count
                  i32.const -1
                  i32.eq
                  if
                    i32.const -1
                    return
                  end
                  local.get $contour_x
                  local.set $prev_x
                  local.get $contour_y
                  local.set $prev_y
                  local.get $step
                  i32.const 1
                  i32.add
                  local.set $step
                  br $quad_loop
                end
              end
              local.get $next_x
              local.set $current_x
              local.get $next_y
              local.set $current_y
            else
              local.get $op
              i32.const 4
              i32.eq
              if
                local.get $has_current
                i32.eqz
                if
                  i32.const -1
                  return
                end
                local.get $edge_count
                local.get $current_x
                local.get $current_y
                local.get $contour_x
                local.get $contour_y
                call $font_edge_append
                local.tee $edge_count
                i32.const -1
                i32.eq
                if
                  i32.const -1
                  return
                end
                i32.const 0
                local.set $has_current
              end
            end
          end
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    local.get $has_current
    if
      local.get $edge_count
      local.get $current_x
      local.get $current_y
      local.get $contour_x
      local.get $contour_y
      call $font_edge_append
      local.set $edge_count
    end
    local.get $edge_count)

  (func $font_x_at_y (param $edge i32) (param $y f32) (result f32)
    local.get $edge
    f32.load
    local.get $y
    local.get $edge
    i32.const 4
    i32.add
    f32.load
    f32.sub
    local.get $edge
    i32.const 8
    i32.add
    f32.load
    local.get $edge
    f32.load
    f32.sub
    f32.mul
    local.get $edge
    i32.const 12
    i32.add
    f32.load
    local.get $edge
    i32.const 4
    i32.add
    f32.load
    f32.sub
    f32.div
    f32.add)

  (func $font_insert_hit (param $count i32) (param $x f32) (param $direction i32) (result i32)
    (local $i i32)
    local.get $count
    local.set $i
    block $done
      loop $loop
        local.get $i
        i32.eqz
        br_if $done
        local.get $i
        i32.const 1
        i32.sub
        call $font_hit_ptr
        f32.load
        local.get $x
        f32.le
        br_if $done
        local.get $i
        call $font_hit_ptr
        local.get $i
        i32.const 1
        i32.sub
        call $font_hit_ptr
        i32.const 8
        call $copy
        local.get $i
        i32.const 1
        i32.sub
        local.set $i
        br $loop
      end
    end
    local.get $i
    call $font_hit_ptr
    local.get $x
    f32.store
    local.get $i
    call $font_hit_ptr
    i32.const 4
    i32.add
    local.get $direction
    i32.store
    local.get $count
    i32.const 1
    i32.add)

  (func $font_sorted_hits (param $y f32) (param $edge_count i32) (result i32)
    (local $i i32)
    (local $edge i32)
    (local $count i32)
    block $done
      loop $loop
        local.get $i
        local.get $edge_count
        i32.ge_u
        br_if $done
        local.get $i
        call $font_edge_ptr
        local.set $edge
        local.get $edge
        i32.const 4
        i32.add
        f32.load
        local.get $y
        f32.le
        if
          local.get $edge
          i32.const 12
          i32.add
          f32.load
          local.get $y
          f32.gt
          if
            local.get $count
            local.get $edge
            local.get $y
            call $font_x_at_y
            i32.const 1
            call $font_insert_hit
            local.set $count
          end
        else
          local.get $edge
          i32.const 12
          i32.add
          f32.load
          local.get $y
          f32.le
          if
            local.get $count
            local.get $edge
            local.get $y
            call $font_x_at_y
            i32.const -1
            call $font_insert_hit
            local.set $count
          end
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    local.get $count)

  (func $font_fill_span (param $row i32) (param $w i32) (param $left_i i32) (param $right_limit f32) (param $span_a f32) (param $span_b f32)
    (local $left f32)
    (local $right f32)
    (local $origin f32)
    (local $x i32)
    (local $end i32)
    (local $sx i32)
    (local $sample_x f32)
    local.get $span_a
    local.get $left_i
    f32.convert_i32_s
    call $max_f32
    local.set $left
    local.get $span_b
    local.get $right_limit
    call $min_f32
    local.set $right
    local.get $right
    local.get $left
    f32.le
    if
      return
    end
    local.get $left_i
    f32.convert_i32_s
    local.set $origin
    local.get $left
    local.get $origin
    f32.sub
    f32.floor
    f32.const 0
    call $max_f32
    i32.trunc_f32_u
    local.set $x
    local.get $right
    local.get $origin
    f32.sub
    f32.ceil
    i32.trunc_f32_u
    local.get $w
    call $min_i32_u
    local.set $end
    block $done_x
      loop $loop_x
        local.get $x
        local.get $end
        i32.ge_u
        br_if $done_x
        i32.const 0
        local.set $sx
        block $done_sx
          loop $loop_sx
            local.get $sx
            i32.const 8
            i32.ge_u
            br_if $done_sx
            local.get $origin
            local.get $x
            f32.convert_i32_u
            f32.add
            local.get $sx
            call $font_sample_offset
            f32.add
            local.set $sample_x
            local.get $sample_x
            local.get $left
            f32.ge
            local.get $sample_x
            local.get $right
            f32.lt
            i32.and
            if
              local.get $row
              local.get $x
              i32.add
              local.get $row
              local.get $x
              i32.add
              i32.load8_u
              i32.const 1
              i32.add
              i32.store8
            end
            local.get $sx
            i32.const 1
            i32.add
            local.set $sx
            br $loop_sx
          end
        end
        local.get $x
        i32.const 1
        i32.add
        local.set $x
        br $loop_x
      end
    end)
  (func $er_ui_font_glyph_render_alpha_baked  (param $codepoint i32) (param $alpha i32) (param $stride i32) (param $cap_w i32) (param $cap_h i32) (param $size f32) (param $out_metrics i32) (result i32)
    (local $glyph i32)
    (local $x i32)
    (local $y i32)
    (local $sx i32)
    (local $sy i32)
    (local $hits i32)
    (local $sample i32)
    (local $drawn i32)
    (local $left i32)
    (local $top i32)
    (local $right i32)
    (local $bottom i32)
    (local $w i32)
    (local $h i32)
    local.get $codepoint
    call $font_glyph_record_ptr
    local.tee $glyph
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $out_metrics
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $glyph
    i32.const 12
    i32.add
    i32.load
    i32.eqz
    if
      local.get $out_metrics
      i32.const 0
      i32.store
      local.get $out_metrics
      i32.const 4
      i32.add
      i32.const 0
      i32.store
      local.get $out_metrics
      i32.const 8
      i32.add
      i32.const 0
      i32.store
      local.get $out_metrics
      i32.const 12
      i32.add
      i32.const 0
      i32.store
      i32.const 0
      return
    end
    i32.const 0
    local.set $y
    block $done_y
      loop $loop_y
        local.get $y
        local.get $cap_h
        i32.ge_u
        br_if $done_y
        i32.const 0
        local.set $x
        block $done_x
          loop $loop_x
            local.get $x
            local.get $cap_w
            i32.ge_u
            br_if $done_x
            i32.const 0
            local.set $hits
            i32.const 0
            local.set $sy
            block $done_sy
              loop $loop_sy
                local.get $sy
                i32.const 8
                i32.ge_u
                br_if $done_sy
                i32.const 0
                local.set $sx
                block $done_sx
                  loop $loop_sx
                    local.get $sx
                    i32.const 8
                    i32.ge_u
                    br_if $done_sx
                    local.get $glyph
                    f32.const 0
                    local.get $size
                    local.get $size
                    local.get $x
                    f32.convert_i32_u
                    local.get $sx
                    call $font_sample_offset
                    f32.add
                    local.get $y
                    f32.convert_i32_u
                    local.get $sy
                    call $font_sample_offset
                    f32.add
                    call $font_glyph_point_inside
                    if
                      local.get $hits
                      i32.const 1
                      i32.add
                      local.set $hits
                    end
                    local.get $sx
                    i32.const 1
                    i32.add
                    local.set $sx
                    br $loop_sx
                  end
                end
                local.get $sy
                i32.const 1
                i32.add
                local.set $sy
                br $loop_sy
              end
            end
            local.get $hits
            i32.const 255
            i32.mul
            i32.const 64
            i32.div_u
            call $font_sharpen_sample
            local.tee $sample
            if
              local.get $drawn
              i32.const 1
              i32.add
              local.set $drawn
            end
            local.get $alpha
            local.get $y
            local.get $stride
            i32.mul
            local.get $x
            i32.add
            i32.add
            local.get $sample
            i32.store8
            local.get $x
            i32.const 1
            i32.add
            local.set $x
            br $loop_x
          end
        end
        local.get $y
        i32.const 1
        i32.add
        local.set $y
        br $loop_y
      end
    end
    local.get $out_metrics
    local.get $cap_w
    i32.store
    local.get $out_metrics
    i32.const 4
    i32.add
    local.get $cap_h
    i32.store
    local.get $out_metrics
    i32.const 8
    i32.add
    i32.const 0
    i32.store
    local.get $out_metrics
    i32.const 12
    i32.add
    i32.const 0
    i32.store
    local.get $drawn)

  (func $er_ui_font_glyph_bake_alpha  (param $codepoint i32) (param $alpha i32) (param $stride i32) (param $cap_w i32) (param $cap_h i32) (param $size f32) (param $out_metrics i32) (result i32)
    (local $glyph i32)
    (local $start i32)
    (local $cmd_count i32)
    (local $i i32)
    (local $src i32)
    (local $op i32)
    (local $scale f32)
    (local $pxf f32)
    (local $pyf f32)
    (local $min_x f32)
    (local $min_y f32)
    (local $max_x f32)
    (local $max_y f32)
    (local $initialized i32)
    (local $left i32)
    (local $top i32)
    (local $right i32)
    (local $bottom i32)
    (local $w i32)
    (local $h i32)
    (local $x i32)
    (local $y i32)
    (local $sx i32)
    (local $sy i32)
    (local $hits i32)
    (local $sample i32)
    (local $drawn i32)
    local.get $codepoint
    call $font_glyph_record_ptr
    local.tee $glyph
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $out_metrics
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
    local.tee $cmd_count
    i32.eqz
    if
      local.get $out_metrics i32.const 0 i32.store
      local.get $out_metrics i32.const 4 i32.add i32.const 0 i32.store
      local.get $out_metrics i32.const 8 i32.add i32.const 0 i32.store
      local.get $out_metrics i32.const 12 i32.add i32.const 0 i32.store
      i32.const 0
      return
    end
    local.get $size
    call $font_units_per_em
    f32.div
    local.set $scale
    block $bounds_done
      loop $bounds_loop
        local.get $i
        local.get $cmd_count
        i32.ge_u
        br_if $bounds_done
        local.get $start
        local.get $i
        i32.add
        call $font_command_ptr
        local.tee $src
        i32.load
        local.set $op
        local.get $op
        i32.const 1
        i32.eq
        local.get $op
        i32.const 2
        i32.eq
        i32.or
        local.get $op
        i32.const 3
        i32.eq
        i32.or
        if
          f32.const 0
          local.get $scale
          local.get $src
          i32.const 4
          call $font_point_x
          local.set $pxf
          f32.const 0
          local.get $scale
          local.get $src
          i32.const 8
          call $font_point_y
          local.set $pyf
          local.get $initialized
          i32.eqz
          if
            local.get $pxf local.set $min_x
            local.get $pxf local.set $max_x
            local.get $pyf local.set $min_y
            local.get $pyf local.set $max_y
            i32.const 1 local.set $initialized
          else
            local.get $pxf local.get $min_x call $min_f32 local.set $min_x
            local.get $pxf local.get $max_x call $max_f32 local.set $max_x
            local.get $pyf local.get $min_y call $min_f32 local.set $min_y
            local.get $pyf local.get $max_y call $max_f32 local.set $max_y
          end
        end
        local.get $op
        i32.const 3
        i32.eq
        if
          f32.const 0
          local.get $scale
          local.get $src
          i32.const 12
          call $font_point_x
          local.set $pxf
          f32.const 0
          local.get $scale
          local.get $src
          i32.const 16
          call $font_point_y
          local.set $pyf
          local.get $pxf local.get $min_x call $min_f32 local.set $min_x
          local.get $pxf local.get $max_x call $max_f32 local.set $max_x
          local.get $pyf local.get $min_y call $min_f32 local.set $min_y
          local.get $pyf local.get $max_y call $max_f32 local.set $max_y
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $bounds_loop
      end
    end
    local.get $initialized
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $min_x f32.floor i32.trunc_f32_s i32.const 1 i32.sub local.set $left
    local.get $min_y f32.floor i32.trunc_f32_s i32.const 1 i32.sub local.set $top
    local.get $max_x f32.ceil i32.trunc_f32_s i32.const 1 i32.add local.set $right
    local.get $max_y f32.ceil i32.trunc_f32_s i32.const 1 i32.add local.set $bottom
    local.get $right local.get $left i32.sub local.set $w
    local.get $bottom local.get $top i32.sub local.set $h
    local.get $w local.get $cap_w i32.gt_u
    local.get $h local.get $cap_h i32.gt_u
    i32.or
    if
      i32.const -1
      return
    end
    i32.const 0 local.set $y
    block $done_y
      loop $loop_y
        local.get $y local.get $h i32.ge_u br_if $done_y
        i32.const 0 local.set $x
        block $done_x
          loop $loop_x
            local.get $x local.get $w i32.ge_u br_if $done_x
            i32.const 0 local.set $hits
            i32.const 0 local.set $sy
            block $done_sy
              loop $loop_sy
                local.get $sy i32.const 8 i32.ge_u br_if $done_sy
                i32.const 0 local.set $sx
                block $done_sx
                  loop $loop_sx
                    local.get $sx i32.const 8 i32.ge_u br_if $done_sx
                    local.get $glyph
                    f32.const 0
                    f32.const 0
                    local.get $size
                    local.get $left f32.convert_i32_s
                    local.get $x f32.convert_i32_u f32.add
                    local.get $sx call $font_sample_offset f32.add
                    local.get $top f32.convert_i32_s
                    local.get $y f32.convert_i32_u f32.add
                    local.get $sy call $font_sample_offset f32.add
                    call $font_glyph_point_inside
                    if
                      local.get $hits i32.const 1 i32.add local.set $hits
                    end
                    local.get $sx i32.const 1 i32.add local.set $sx
                    br $loop_sx
                  end
                end
                local.get $sy i32.const 1 i32.add local.set $sy
                br $loop_sy
              end
            end
            local.get $hits i32.const 255 i32.mul i32.const 64 i32.div_u call $font_sharpen_sample local.set $sample
            local.get $sample
            if
              local.get $drawn i32.const 1 i32.add local.set $drawn
            end
            local.get $alpha
            local.get $y local.get $stride i32.mul
            local.get $x i32.add
            i32.add
            local.get $sample
            i32.store8
            local.get $x i32.const 1 i32.add local.set $x
            br $loop_x
          end
        end
        local.get $y i32.const 1 i32.add local.set $y
        br $loop_y
      end
    end
    local.get $out_metrics local.get $w i32.store
    local.get $out_metrics i32.const 4 i32.add local.get $h i32.store
    local.get $out_metrics i32.const 8 i32.add local.get $left i32.store
    local.get $out_metrics i32.const 12 i32.add local.get $top i32.store
    local.get $drawn)

  (func $er_ui_font_glyph_bake_alpha_exact  (param $codepoint i32) (param $alpha i32) (param $stride i32) (param $cap_w i32) (param $cap_h i32) (param $size f32) (param $out_metrics i32) (result i32)
    (local $glyph i32)
    (local $edge_count i32)
    (local $edge i32)
    (local $i i32)
    (local $j i32)
    (local $x i32)
    (local $y i32)
    (local $sy i32)
    (local $hit_count i32)
    (local $winding i32)
    (local $row i32)
    (local $left i32)
    (local $top i32)
    (local $right i32)
    (local $bottom i32)
    (local $w i32)
    (local $h i32)
    (local $drawn i32)
    (local $sample i32)
    (local $right_f f32)
    (local $sample_y f32)
    (local $min_x f32)
    (local $min_y f32)
    (local $max_x f32)
    (local $max_y f32)
    (local $scale f32)
    local.get $codepoint
    call $font_glyph_record_ptr
    local.tee $glyph
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $out_metrics
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $glyph
    local.get $size
    call $font_units_per_em
    f32.div
    local.tee $scale
    call $font_flatten_edges
    local.set $edge_count
    local.get $edge_count
    i32.const -1
    i32.eq
    if
      i32.const -1
      return
    end
    local.get $edge_count
    i32.eqz
    if
      local.get $out_metrics i32.const 0 i32.store
      local.get $out_metrics i32.const 4 i32.add i32.const 0 i32.store
      local.get $out_metrics i32.const 8 i32.add i32.const 0 i32.store
      local.get $out_metrics i32.const 12 i32.add i32.const 0 i32.store
      i32.const 0
      return
    end
    i32.const 0
    call $font_edge_ptr
    local.tee $edge
    f32.load
    local.set $min_x
    local.get $edge
    f32.load
    local.set $max_x
    local.get $edge
    i32.const 4
    i32.add
    f32.load
    local.set $min_y
    local.get $edge
    i32.const 4
    i32.add
    f32.load
    local.set $max_y
    i32.const 0
    local.set $i
    block $bounds_done
      loop $bounds_loop
        local.get $i
        local.get $edge_count
        i32.ge_u
        br_if $bounds_done
        local.get $i
        call $font_edge_ptr
        local.set $edge
        local.get $edge f32.load local.get $min_x call $min_f32 local.set $min_x
        local.get $edge f32.load local.get $max_x call $max_f32 local.set $max_x
        local.get $edge i32.const 4 i32.add f32.load local.get $min_y call $min_f32 local.set $min_y
        local.get $edge i32.const 4 i32.add f32.load local.get $max_y call $max_f32 local.set $max_y
        local.get $edge i32.const 8 i32.add f32.load local.get $min_x call $min_f32 local.set $min_x
        local.get $edge i32.const 8 i32.add f32.load local.get $max_x call $max_f32 local.set $max_x
        local.get $edge i32.const 12 i32.add f32.load local.get $min_y call $min_f32 local.set $min_y
        local.get $edge i32.const 12 i32.add f32.load local.get $max_y call $max_f32 local.set $max_y
        local.get $i i32.const 1 i32.add local.set $i
        br $bounds_loop
      end
    end
    local.get $min_x f32.floor i32.trunc_f32_s i32.const 1 i32.sub local.set $left
    local.get $max_x f32.ceil i32.trunc_f32_s i32.const 1 i32.add local.set $right
    local.get $min_y f32.floor i32.trunc_f32_s i32.const 1 i32.sub local.set $top
    local.get $max_y f32.ceil i32.trunc_f32_s i32.const 1 i32.add local.set $bottom
    local.get $right local.get $left i32.sub local.set $w
    local.get $bottom local.get $top i32.sub local.set $h
    local.get $w local.get $cap_w i32.gt_u
    local.get $h local.get $cap_h i32.gt_u
    i32.or
    if
      i32.const -1
      return
    end
    local.get $left f32.convert_i32_s local.get $w f32.convert_i32_u f32.add local.set $right_f
    i32.const 0 local.set $y
    block $done_y
      loop $loop_y
        local.get $y local.get $h i32.ge_u br_if $done_y
        local.get $alpha local.get $y local.get $stride i32.mul i32.add local.set $row
        local.get $row local.get $w call $zero
        i32.const 0 local.set $sy
        block $done_sy
          loop $loop_sy
            local.get $sy i32.const 8 i32.ge_u br_if $done_sy
            local.get $top f32.convert_i32_s
            local.get $y f32.convert_i32_u f32.add
            local.get $sy call $font_sample_offset f32.add
            local.tee $sample_y
            local.get $edge_count
            call $font_sorted_hits
            local.set $hit_count
            i32.const 0 local.set $winding
            i32.const 0 local.set $j
            block $done_hits
              loop $loop_hits
                local.get $j local.get $hit_count i32.ge_u br_if $done_hits
                local.get $winding
                local.get $j call $font_hit_ptr i32.const 4 i32.add i32.load
                i32.add
                local.set $winding
                local.get $winding
                i32.const 0
                i32.ne
                local.get $j i32.const 1 i32.add local.get $hit_count i32.lt_u
                i32.and
                if
                  local.get $row
                  local.get $w
                  local.get $left
                  local.get $right_f
                  local.get $j call $font_hit_ptr f32.load
                  local.get $j i32.const 1 i32.add call $font_hit_ptr f32.load
                  call $font_fill_span
                end
                local.get $j i32.const 1 i32.add local.set $j
                br $loop_hits
              end
            end
            local.get $sy i32.const 1 i32.add local.set $sy
            br $loop_sy
          end
        end
        i32.const 0 local.set $x
        block $done_x
          loop $loop_x
            local.get $x local.get $w i32.ge_u br_if $done_x
            local.get $row local.get $x i32.add i32.load8_u
            i32.const 255
            i32.mul
            i32.const 64
            i32.div_u
            local.set $sample
            local.get $size
            i32.trunc_f32_u
            i32.const 16
            i32.le_u
            if
              local.get $sample
              call $font_sharpen_sample
              local.set $sample
            end
            local.get $row local.get $x i32.add local.get $sample i32.store8
            local.get $sample
            if
              local.get $drawn i32.const 1 i32.add local.set $drawn
            end
            local.get $x i32.const 1 i32.add local.set $x
            br $loop_x
          end
        end
        local.get $y i32.const 1 i32.add local.set $y
        br $loop_y
      end
    end
    local.get $out_metrics local.get $w i32.store
    local.get $out_metrics i32.const 4 i32.add local.get $h i32.store
    local.get $out_metrics i32.const 8 i32.add local.get $left i32.store
    local.get $out_metrics i32.const 12 i32.add local.get $top i32.store
    local.get $drawn)

  (func $er_ui_font_text_render_alpha_exact  (param $ptr i32) (param $len i32) (param $alpha i32) (param $width i32) (param $height i32) (param $x f32) (param $baseline_y f32) (param $size f32) (result i32)
    (local $i i32)
    (local $gx i32)
    (local $gy i32)
    (local $dst_x i32)
    (local $dst_y i32)
    (local $glyph_w i32)
    (local $glyph_h i32)
    (local $glyph_left i32)
    (local $glyph_top i32)
    (local $glyph i32)
    (local $codepoint i32)
    (local $coverage i32)
    (local $drawn i32)
    (local $pen_x f32)
    global.get $font_body_ptr
    global.get $font_body_len
    call $font_body_valid_at
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $width
    i32.eqz
    local.get $height
    i32.eqz
    i32.or
    if
      i32.const -1
      return
    end
    local.get $x
    local.set $pen_x
    block $text_done
      loop $text_loop
        local.get $i
        local.get $len
        i32.ge_u
        br_if $text_done
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        local.tee $codepoint
        call $font_glyph_record_ptr
        local.set $glyph
        local.get $glyph
        i32.const 0
        i32.ne
        if
          local.get $codepoint
          i32.const 960000
          i32.const 192
          i32.const 192
          i32.const 192
          local.get $size
          i32.const 955000
          call $er_ui_font_glyph_bake_alpha_exact
          i32.const -1
          i32.eq
          if
            i32.const -1
            return
          end
          i32.const 955000
          i32.load
          local.set $glyph_w
          i32.const 955004
          i32.load
          local.set $glyph_h
          i32.const 955008
          i32.load
          local.set $glyph_left
          i32.const 955012
          i32.load
          local.set $glyph_top
          i32.const 0
          local.set $gy
          block $done_gy
            loop $loop_gy
              local.get $gy
              local.get $glyph_h
              i32.ge_u
              br_if $done_gy
              i32.const 0
              local.set $gx
              block $done_gx
                loop $loop_gx
                  local.get $gx
                  local.get $glyph_w
                  i32.ge_u
                  br_if $done_gx
                  i32.const 960000
                  local.get $gy
                  i32.const 192
                  i32.mul
                  local.get $gx
                  i32.add
                  i32.add
                  i32.load8_u
                  local.tee $coverage
                  i32.const 0
                  i32.ne
                  if
                    local.get $pen_x
                    i32.trunc_f32_s
                    local.get $glyph_left
                    i32.add
                    local.get $gx
                    i32.add
                    local.set $dst_x
                    local.get $baseline_y
                    i32.trunc_f32_s
                    local.get $glyph_top
                    i32.add
                    local.get $gy
                    i32.add
                    local.set $dst_y
                    local.get $dst_x
                    i32.const 0
                    i32.ge_s
                    local.get $dst_y
                    i32.const 0
                    i32.ge_s
                    i32.and
                    local.get $dst_x
                    local.get $width
                    i32.lt_s
                    i32.and
                    local.get $dst_y
                    local.get $height
                    i32.lt_s
                    i32.and
                    if
                      local.get $alpha
                      local.get $dst_y
                      local.get $width
                      i32.mul
                      local.get $dst_x
                      i32.add
                      i32.add
                      local.get $coverage
                      i32.store8
                      local.get $drawn
                      i32.const 1
                      i32.add
                      local.set $drawn
                    end
                  end
                  local.get $gx
                  i32.const 1
                  i32.add
                  local.set $gx
                  br $loop_gx
                end
              end
              local.get $gy
              i32.const 1
              i32.add
              local.set $gy
              br $loop_gy
            end
          end
          local.get $pen_x
          local.get $glyph
          i32.const 16
          i32.add
          f32.load
          local.get $size
          f32.mul
          call $font_units_per_em
          f32.div
          f32.add
          local.set $pen_x
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $text_loop
      end
    end
    local.get $drawn)
  (func $er_ui_font_ascent  (param $size f32) (result f32)
    local.get $size
    f32.const 0.8
    f32.mul)

  (func $er_ui_font_descent  (param $size f32) (result f32)
    local.get $size
    f32.const 0.2
    f32.mul)

  (func $er_ui_font_line_height  (param $size f32) (result f32)
    local.get $size
    f32.const 1.25
    f32.mul)

  (func $er_ui_font_cap_height  (param $size f32) (result f32)
    local.get $size
    f32.const 0.72
    f32.mul)

  (func $er_ui_font_x_height  (param $size f32) (result f32)
    local.get $size
    f32.const 0.52
    f32.mul)

  (func $er_ui_font_baseline  (param $size f32) (result f32)
    local.get $size
    call $er_ui_font_line_gap
    f32.const 0.5
    f32.mul
    local.get $size
    call $er_ui_font_ascent
    f32.add)

  (func $er_ui_font_line_gap  (param $size f32) (result f32)
    local.get $size
    call $er_ui_font_line_height
    local.get $size
    call $er_ui_font_ascent
    f32.sub
    local.get $size
    call $er_ui_font_descent
    f32.sub)

  (func $er_ui_font_underline_position  (param $size f32) (result f32)
    local.get $size
    f32.const 0.08
    f32.mul)

  (func $er_ui_font_underline_thickness  (param $size f32) (result f32)
    local.get $size
    f32.const 0.07
    f32.mul
    f32.const 1
    call $max_f32)

  (func $er_ui_font_glyph_advance  (param $codepoint i32) (param $size f32) (result f32)
    (local $glyph i32)
    global.get $font_body_ptr
    global.get $font_body_len
    call $font_body_valid_at
    if
      local.get $codepoint
      call $font_glyph_record_ptr
      local.tee $glyph
      i32.const 0
      i32.ne
      if
        local.get $glyph
        i32.const 16
        i32.add
        f32.load
        local.get $size
        f32.mul
        call $font_units_per_em
        f32.div
        return
      end
    end
    local.get $codepoint
    i32.const 32
    i32.eq
    if (result f32)
      f32.const 0.33
    else
      local.get $codepoint
      i32.const 105
      i32.eq
      local.get $codepoint
      i32.const 108
      i32.eq
      i32.or
      local.get $codepoint
      i32.const 73
      i32.eq
      i32.or
      if (result f32)
        f32.const 0.32
      else
        local.get $codepoint
        i32.const 77
        i32.eq
        local.get $codepoint
        i32.const 87
        i32.eq
        i32.or
        local.get $codepoint
        i32.const 109
        i32.eq
        i32.or
        local.get $codepoint
        i32.const 119
        i32.eq
        i32.or
        if (result f32)
          f32.const 0.9
        else
          local.get $codepoint
          i32.const 48
          i32.ge_u
          local.get $codepoint
          i32.const 57
          i32.le_u
          i32.and
          if (result f32)
            f32.const 0.58
          else
            local.get $codepoint
            i32.const 65
            i32.ge_u
            local.get $codepoint
            i32.const 90
            i32.le_u
            i32.and
            if (result f32)
              f32.const 0.64
            else
              local.get $codepoint
              i32.const 44
              i32.eq
              local.get $codepoint
              i32.const 46
              i32.eq
              i32.or
              local.get $codepoint
              i32.const 58
              i32.eq
              i32.or
              local.get $codepoint
              i32.const 59
              i32.eq
              i32.or
              if (result f32)
                f32.const 0.28
              else
                f32.const 0.56
              end
            end
          end
        end
      end
    end
    local.get $size
    f32.mul)

  (func $er_ui_font_text_width (export "er_ui_font_text_width")  (param $ptr i32) (param $len i32) (param $size f32) (result f32)
    (local $i i32)
    (local $width f32)
    block $done
      loop $loop
        local.get $i
        local.get $len
        i32.ge_u
        br_if $done
        local.get $width
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        local.get $size
        call $er_ui_font_glyph_advance
        f32.add
        local.set $width
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    local.get $width)

  (func $er_ui_font_text_fit_len  (param $ptr i32) (param $len i32) (param $size f32) (param $max_w f32) (result i32)
    (local $i i32)
    (local $width f32)
    local.get $max_w
    f32.const 0
    f32.lt
    if
      i32.const 0
      return
    end
    block $done
      loop $loop
        local.get $i
        local.get $len
        i32.ge_u
        br_if $done
        local.get $width
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        local.get $size
        call $er_ui_font_glyph_advance
        f32.add
        local.tee $width
        local.get $max_w
        f32.gt
        if
          local.get $i
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    local.get $len)

  (func $er_ui_font_text_fits  (param $ptr i32) (param $len i32) (param $size f32) (param $max_w f32) (result i32)
    local.get $ptr
    local.get $len
    local.get $size
    call $er_ui_font_text_width
    local.get $max_w
    f32.le)

  (func $er_ui_font_text_truncate_len  (param $ptr i32) (param $len i32) (param $size f32) (param $max_w f32) (param $ellipsis_ptr i32) (param $ellipsis_len i32) (result i32)
    (local $ellipsis_w f32)
    local.get $ptr
    local.get $len
    local.get $size
    local.get $max_w
    call $er_ui_font_text_fits
    if
      local.get $len
      return
    end
    local.get $ellipsis_ptr
    local.get $ellipsis_len
    local.get $size
    call $er_ui_font_text_width
    local.tee $ellipsis_w
    local.get $max_w
    f32.gt
    if
      i32.const 0
      return
    end
    local.get $ptr
    local.get $len
    local.get $size
    local.get $max_w
    local.get $ellipsis_w
    f32.sub
    call $er_ui_font_text_fit_len)

  (func $er_ui_font_measure_text  (param $ptr i32) (param $len i32) (param $size f32) (param $out i32) (result i32)
    local.get $out
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $ptr
    local.get $len
    local.get $size
    call $er_ui_font_text_width
    f32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $size
    call $er_ui_font_ascent
    f32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $size
    call $er_ui_font_descent
    f32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $size
    call $er_ui_font_line_height
    f32.store
    i32.const 1)

  (func $er_ui_font_text_box  (param $ptr i32) (param $len i32) (param $size f32) (param $max_w f32) (param $out i32) (result i32)
    (local $fit_len i32)
    local.get $out
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $ptr
    local.get $len
    local.get $size
    local.get $max_w
    call $er_ui_font_text_fit_len
    local.set $fit_len
    local.get $out
    local.get $ptr
    local.get $fit_len
    local.get $size
    call $er_ui_font_text_width
    f32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $size
    call $er_ui_font_line_height
    f32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $fit_len
    i32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $fit_len
    local.get $len
    i32.eq
    i32.store
    i32.const 1)

)
