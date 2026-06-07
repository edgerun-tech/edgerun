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
