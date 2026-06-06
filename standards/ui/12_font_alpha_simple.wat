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

  (func (export "er_ui_font_text_render_alpha") (param $ptr i32) (param $len i32) (param $alpha i32) (param $width i32) (param $height i32) (param $x f32) (param $baseline_y f32) (param $size f32) (result i32)
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

  (func (export "er_ui_font_text_render_alpha_aa") (param $ptr i32) (param $len i32) (param $alpha i32) (param $width i32) (param $height i32) (param $x f32) (param $baseline_y f32) (param $size f32) (result i32)
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
