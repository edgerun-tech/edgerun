(func (export "er_ui_font_glyph_render_alpha_baked") (param $codepoint i32) (param $alpha i32) (param $stride i32) (param $cap_w i32) (param $cap_h i32) (param $size f32) (param $out_metrics i32) (result i32)
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

  (func (export "er_ui_font_glyph_bake_alpha") (param $codepoint i32) (param $alpha i32) (param $stride i32) (param $cap_w i32) (param $cap_h i32) (param $size f32) (param $out_metrics i32) (result i32)
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

  (func $er_ui_font_glyph_bake_alpha_exact (export "er_ui_font_glyph_bake_alpha_exact") (param $codepoint i32) (param $alpha i32) (param $stride i32) (param $cap_w i32) (param $cap_h i32) (param $size f32) (param $out_metrics i32) (result i32)
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

  (func (export "er_ui_font_text_render_alpha_exact") (param $ptr i32) (param $len i32) (param $alpha i32) (param $width i32) (param $height i32) (param $x f32) (param $baseline_y f32) (param $size f32) (result i32)
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
