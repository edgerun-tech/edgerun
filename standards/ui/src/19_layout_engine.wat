  (func $er_ui_layout_linear_child  (param $out i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $children i32) (param $index i32) (param $axis i32) (param $gap f32) (param $padding f32) (result i32)
    (local $content_x f32)
    (local $content_y f32)
    (local $content_w f32)
    (local $content_h f32)
    (local $step f32)
    local.get $children
    i32.eqz
    local.get $index
    local.get $children
    i32.ge_u
    i32.or
    if
      i32.const 0
      return
    end
    local.get $x
    local.get $padding
    f32.add
    local.set $content_x
    local.get $y
    local.get $padding
    f32.add
    local.set $content_y
    local.get $w
    local.get $padding
    f32.const 2
    f32.mul
    f32.sub
    local.set $content_w
    local.get $h
    local.get $padding
    f32.const 2
    f32.mul
    f32.sub
    local.set $content_h
    local.get $axis
    i32.const 1
    i32.gt_u
    local.get $content_w
    f32.const 0
    f32.le
    i32.or
    local.get $content_h
    f32.const 0
    f32.le
    i32.or
    if
      i32.const 0
      return
    end
    local.get $axis
    i32.eqz
    if
      local.get $content_w
      local.get $gap
      local.get $children
      i32.const 1
      i32.sub
      f32.convert_i32_u
      f32.mul
      f32.sub
      local.get $children
      f32.convert_i32_u
      f32.div
      local.set $step
      local.get $step
      f32.const 0
      f32.le
      if
        i32.const 0
        return
      end
      local.get $out
      local.get $content_x
      local.get $step
      local.get $gap
      f32.add
      local.get $index
      f32.convert_i32_u
      f32.mul
      f32.add
      f32.store
      local.get $out
      i32.const 4
      i32.add
      local.get $content_y
      f32.store
      local.get $out
      i32.const 8
      i32.add
      local.get $step
      f32.store
      local.get $out
      i32.const 12
      i32.add
      local.get $content_h
      f32.store
    else
      local.get $content_h
      local.get $gap
      local.get $children
      i32.const 1
      i32.sub
      f32.convert_i32_u
      f32.mul
      f32.sub
      local.get $children
      f32.convert_i32_u
      f32.div
      local.set $step
      local.get $step
      f32.const 0
      f32.le
      if
        i32.const 0
        return
      end
      local.get $out
      local.get $content_x
      f32.store
      local.get $out
      i32.const 4
      i32.add
      local.get $content_y
      local.get $step
      local.get $gap
      f32.add
      local.get $index
      f32.convert_i32_u
      f32.mul
      f32.add
      f32.store
      local.get $out
      i32.const 8
      i32.add
      local.get $content_w
      f32.store
      local.get $out
      i32.const 12
      i32.add
      local.get $step
      f32.store
    end
    i32.const 1)

  (func $er_ui_layout_scrolled_child  (param $out i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $children i32) (param $index i32) (param $gap f32) (param $padding f32) (param $scroll_y f32) (result i32)
    local.get $out
    local.get $x
    local.get $y
    local.get $scroll_y
    f32.sub
    local.get $w
    local.get $h
    local.get $children
    local.get $index
    i32.const 1
    local.get $gap
    local.get $padding
    call $er_ui_layout_linear_child)

  (func $er_ui_layout_scratch_base  (result i32)
    i32.const 120000)

  (func $er_ui_layout_scratch_size  (result i32)
    i32.const 1024)

  (func $er_ui_layout_grid_child  (param $out i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $children i32) (param $index i32) (param $columns i32) (param $gap f32) (param $padding f32) (result i32)
    (local $rows i32)
    (local $col i32)
    (local $row i32)
    (local $cw f32)
    (local $ch f32)
    local.get $children
    i32.eqz
    local.get $index
    local.get $children
    i32.ge_u
    i32.or
    if
      i32.const 0
      return
    end
    local.get $columns
    i32.eqz
    if
      i32.const 1
      local.set $columns
    end
    local.get $columns
    local.get $children
    i32.gt_u
    if
      local.get $children
      local.set $columns
    end
    local.get $children
    local.get $columns
    i32.add
    i32.const 1
    i32.sub
    local.get $columns
    i32.div_u
    local.set $rows
    local.get $index
    local.get $columns
    i32.rem_u
    local.set $col
    local.get $index
    local.get $columns
    i32.div_u
    local.set $row
    local.get $w
    local.get $padding
    f32.const 2
    f32.mul
    f32.sub
    local.get $gap
    local.get $columns
    i32.const 1
    i32.sub
    f32.convert_i32_u
    f32.mul
    f32.sub
    local.get $columns
    f32.convert_i32_u
    f32.div
    local.set $cw
    local.get $cw
    f32.const 0
    f32.le
    if
      i32.const 0
      return
    end
    local.get $h
    local.get $padding
    f32.const 2
    f32.mul
    f32.sub
    local.get $gap
    local.get $rows
    i32.const 1
    i32.sub
    f32.convert_i32_u
    f32.mul
    f32.sub
    local.get $rows
    f32.convert_i32_u
    f32.div
    local.set $ch
    local.get $ch
    f32.const 0
    f32.le
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $x
    local.get $padding
    f32.add
    local.get $cw
    local.get $gap
    f32.add
    local.get $col
    f32.convert_i32_u
    f32.mul
    f32.add
    f32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $y
    local.get $padding
    f32.add
    local.get $ch
    local.get $gap
    f32.add
    local.get $row
    f32.convert_i32_u
    f32.mul
    f32.add
    f32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $cw
    f32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $ch
    f32.store
    i32.const 1)
  (func $er_ui_layout_masonry_child  (param $out i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $children i32) (param $index i32) (param $columns i32) (param $gap f32) (param $padding f32) (param $heights_ptr i32) (result i32)
    (local $i i32)
    (local $col i32)
    (local $best_col i32)
    (local $best_y f32)
    (local $cw f32)
    (local $child_h f32)
    (local $assigned_y f32)
    local.get $children
    i32.eqz
    local.get $index
    local.get $children
    i32.ge_u
    i32.or
    if
      i32.const 0
      return
    end
    local.get $columns
    i32.eqz
    if
      i32.const 1
      local.set $columns
    end
    local.get $columns
    i32.const 32
    i32.gt_u
    if
      i32.const 32
      local.set $columns
    end
    local.get $columns
    local.get $children
    i32.gt_u
    if
      local.get $children
      local.set $columns
    end
    local.get $w
    local.get $padding
    f32.const 2
    f32.mul
    f32.sub
    local.get $gap
    local.get $columns
    i32.const 1
    i32.sub
    f32.convert_i32_u
    f32.mul
    f32.sub
    local.get $columns
    f32.convert_i32_u
    f32.div
    local.set $cw
    local.get $cw
    f32.const 0
    f32.le
    local.get $h
    local.get $padding
    f32.const 2
    f32.mul
    f32.sub
    f32.const 0
    f32.le
    i32.or
    if
      i32.const 0
      return
    end
    i32.const 120000
    i32.const 0
    local.get $columns
    i32.const 4
    i32.mul
    memory.fill
    block $done
      loop $items
        local.get $i
        local.get $index
        i32.gt_u
        br_if $done
        i32.const 0
        local.set $best_col
        i32.const 120000
        f32.load
        local.set $best_y
        i32.const 1
        local.set $col
        block $cols_done
          loop $cols
            local.get $col
            local.get $columns
            i32.ge_u
            br_if $cols_done
            i32.const 120000
            local.get $col
            i32.const 4
            i32.mul
            i32.add
            f32.load
            local.get $best_y
            f32.lt
            if
              i32.const 120000
              local.get $col
              i32.const 4
              i32.mul
              i32.add
              f32.load
              local.set $best_y
              local.get $col
              local.set $best_col
            end
            local.get $col
            i32.const 1
            i32.add
            local.set $col
            br $cols
          end
        end
        local.get $heights_ptr
        i32.eqz
        if
          f32.const 0
          local.set $child_h
        else
          local.get $heights_ptr
          local.get $i
          i32.const 4
          i32.mul
          i32.add
          f32.load
          local.set $child_h
        end
        local.get $child_h
        f32.const 0
        f32.le
        if
          local.get $cw
          f32.const 0.72
          local.get $i
          i32.const 4
          i32.rem_u
          f32.convert_i32_u
          f32.const 0.16
          f32.mul
          f32.add
          f32.mul
          local.set $child_h
        end
        local.get $best_y
        local.set $assigned_y
        local.get $i
        local.get $index
        i32.eq
        if
          local.get $out
          local.get $x
          local.get $padding
          f32.add
          local.get $cw
          local.get $gap
          f32.add
          local.get $best_col
          f32.convert_i32_u
          f32.mul
          f32.add
          f32.store
          local.get $out
          i32.const 4
          i32.add
          local.get $y
          local.get $padding
          f32.add
          local.get $assigned_y
          f32.add
          f32.store
          local.get $out
          i32.const 8
          i32.add
          local.get $cw
          f32.store
          local.get $out
          i32.const 12
          i32.add
          local.get $child_h
          f32.store
          i32.const 1
          return
        end
        i32.const 120000
        local.get $best_col
        i32.const 4
        i32.mul
        i32.add
        local.get $best_y
        local.get $child_h
        f32.add
        local.get $gap
        f32.add
        f32.store
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $items
      end
    end
    i32.const 0)

  (func $er_ui_layout_bento_child  (param $out i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $children i32) (param $index i32) (param $columns i32) (param $gap f32) (param $padding f32) (param $col_spans_ptr i32) (param $row_spans_ptr i32) (result i32)
    (local $rows i32)
    (local $i i32)
    (local $row i32)
    (local $col i32)
    (local $r i32)
    (local $c i32)
    (local $cs i32)
    (local $rs i32)
    (local $fits i32)
    (local $found i32)
    (local $cell_w f32)
    (local $cell_h f32)
    local.get $children
    i32.eqz
    local.get $index
    local.get $children
    i32.ge_u
    i32.or
    if
      i32.const 0
      return
    end
    local.get $columns
    i32.eqz
    if
      i32.const 1
      local.set $columns
    end
    local.get $columns
    i32.const 32
    i32.gt_u
    if
      i32.const 32
      local.set $columns
    end
    i32.const 16
    local.set $rows
    local.get $w
    local.get $padding
    f32.const 2
    f32.mul
    f32.sub
    local.get $gap
    local.get $columns
    i32.const 1
    i32.sub
    f32.convert_i32_u
    f32.mul
    f32.sub
    local.get $columns
    f32.convert_i32_u
    f32.div
    local.set $cell_w
    local.get $cell_w
    f32.const 0.62
    f32.mul
    local.set $cell_h
    local.get $cell_w
    f32.const 0
    f32.le
    local.get $cell_h
    f32.const 0
    f32.le
    i32.or
    local.get $cell_h
    local.get $h
    local.get $padding
    f32.const 2
    f32.mul
    f32.sub
    f32.gt
    i32.or
    if
      i32.const 0
      return
    end
    i32.const 120512
    i32.const 0
    i32.const 512
    memory.fill
    block $items_done
      loop $items
        local.get $i
        local.get $index
        i32.gt_u
        br_if $items_done
        i32.const 1
        local.set $cs
        local.get $col_spans_ptr
        i32.eqz
        i32.eqz
        if
          local.get $col_spans_ptr
          local.get $i
          i32.add
          i32.load8_u
          local.set $cs
        end
        local.get $cs
        i32.eqz
        if
          i32.const 1
          local.set $cs
        end
        local.get $cs
        local.get $columns
        i32.gt_u
        if
          local.get $columns
          local.set $cs
        end
        i32.const 1
        local.set $rs
        local.get $row_spans_ptr
        i32.eqz
        i32.eqz
        if
          local.get $row_spans_ptr
          local.get $i
          i32.add
          i32.load8_u
          local.set $rs
        end
        local.get $rs
        i32.eqz
        if
          i32.const 1
          local.set $rs
        end
        local.get $rs
        local.get $rows
        i32.gt_u
        if
          local.get $rows
          local.set $rs
        end
        i32.const 0
        local.set $found
        i32.const 0
        local.set $row
        block $rows_done
          loop $rows_loop
            local.get $row
            local.get $rs
            i32.add
            local.get $rows
            i32.gt_u
            br_if $rows_done
            i32.const 0
            local.set $col
            block $cols_done
              loop $cols_loop
                local.get $col
                local.get $cs
                i32.add
                local.get $columns
                i32.gt_u
                br_if $cols_done
                i32.const 1
                local.set $fits
                i32.const 0
                local.set $r
                block $span_rows_done
                  loop $span_rows
                    local.get $r
                    local.get $rs
                    i32.ge_u
                    br_if $span_rows_done
                    i32.const 0
                    local.set $c
                    block $span_cols_done
                      loop $span_cols
                        local.get $c
                        local.get $cs
                        i32.ge_u
                        br_if $span_cols_done
                        i32.const 120512
                        local.get $row
                        local.get $r
                        i32.add
                        i32.const 32
                        i32.mul
                        i32.add
                        local.get $col
                        i32.add
                        local.get $c
                        i32.add
                        i32.load8_u
                        if
                          i32.const 0
                          local.set $fits
                        end
                        local.get $c
                        i32.const 1
                        i32.add
                        local.set $c
                        br $span_cols
                      end
                    end
                    local.get $r
                    i32.const 1
                    i32.add
                    local.set $r
                    br $span_rows
                  end
                end
                local.get $fits
                if
                  i32.const 1
                  local.set $found
                  br $cols_done
                end
                local.get $col
                i32.const 1
                i32.add
                local.set $col
                br $cols_loop
              end
            end
            local.get $found
            if
              br $rows_done
            end
            local.get $row
            i32.const 1
            i32.add
            local.set $row
            br $rows_loop
          end
        end
        local.get $found
        i32.eqz
        if
          i32.const 0
          return
        end
        local.get $i
        local.get $index
        i32.eq
        if
          local.get $out
          local.get $x
          local.get $padding
          f32.add
          local.get $cell_w
          local.get $gap
          f32.add
          local.get $col
          f32.convert_i32_u
          f32.mul
          f32.add
          f32.store
          local.get $out
          i32.const 4
          i32.add
          local.get $y
          local.get $padding
          f32.add
          local.get $cell_h
          local.get $gap
          f32.add
          local.get $row
          f32.convert_i32_u
          f32.mul
          f32.add
          f32.store
          local.get $out
          i32.const 8
          i32.add
          local.get $cell_w
          local.get $cs
          f32.convert_i32_u
          f32.mul
          local.get $gap
          local.get $cs
          i32.const 1
          i32.sub
          f32.convert_i32_u
          f32.mul
          f32.add
          f32.store
          local.get $out
          i32.const 12
          i32.add
          local.get $cell_h
          local.get $rs
          f32.convert_i32_u
          f32.mul
          local.get $gap
          local.get $rs
          i32.const 1
          i32.sub
          f32.convert_i32_u
          f32.mul
          f32.add
          f32.store
          i32.const 1
          return
        end
        i32.const 0
        local.set $r
        block $mark_rows_done
          loop $mark_rows
            local.get $r
            local.get $rs
            i32.ge_u
            br_if $mark_rows_done
            i32.const 0
            local.set $c
            block $mark_cols_done
              loop $mark_cols
                local.get $c
                local.get $cs
                i32.ge_u
                br_if $mark_cols_done
                i32.const 120512
                local.get $row
                local.get $r
                i32.add
                i32.const 32
                i32.mul
                i32.add
                local.get $col
                i32.add
                local.get $c
                i32.add
                i32.const 1
                i32.store8
                local.get $c
                i32.const 1
                i32.add
                local.set $c
                br $mark_cols
              end
            end
            local.get $r
            i32.const 1
            i32.add
            local.set $r
            br $mark_rows
          end
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $items
      end
    end
    i32.const 0)
  (func $er_ui_list_equal_segment_bounds_gap  (param $bounds i32) (param $index i32) (param $item_count i32) (param $gap f32) (param $out i32) (result i32)
    (local $count f32)
    (local $total_gap f32)
    (local $segment_w f32)
    local.get $bounds
    i32.eqz
    local.get $out
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $item_count
    f32.convert_i32_u
    f32.const 1
    call $max_f32
    local.set $count
    local.get $gap
    local.get $count
    f32.const 1
    f32.sub
    f32.const 0
    call $max_f32
    f32.mul
    local.set $total_gap
    local.get $bounds
    i32.const 8
    i32.add
    f32.load
    local.get $total_gap
    f32.sub
    local.get $count
    f32.div
    f32.const 1
    call $max_f32
    local.set $segment_w
    local.get $out
    local.get $bounds
    f32.load
    local.get $index
    f32.convert_i32_u
    local.get $segment_w
    local.get $gap
    f32.add
    f32.mul
    f32.add
    local.get $bounds
    i32.const 4
    i32.add
    f32.load
    local.get $segment_w
    local.get $bounds
    i32.const 12
    i32.add
    f32.load
    call $rect_store)

  (func $er_ui_list_equal_segment_bounds  (param $bounds i32) (param $index i32) (param $item_count i32) (param $out i32) (result i32)
    local.get $bounds
    local.get $index
    local.get $item_count
    f32.const 0
    local.get $out
    call $er_ui_list_equal_segment_bounds_gap)

  (func $er_ui_list_padded_equal_segment_bounds  (param $bounds i32) (param $index i32) (param $item_count i32) (param $padding f32) (param $out i32) (result i32)
    (local $content_x f32)
    (local $content_y f32)
    (local $content_w f32)
    (local $content_h f32)
    (local $count f32)
    (local $segment_w f32)
    local.get $bounds
    i32.eqz
    local.get $out
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $bounds
    f32.load
    local.get $padding
    f32.add
    local.set $content_x
    local.get $bounds
    i32.const 4
    i32.add
    f32.load
    local.get $padding
    f32.add
    local.set $content_y
    local.get $bounds
    i32.const 8
    i32.add
    f32.load
    local.get $padding
    f32.const 2
    f32.mul
    f32.sub
    f32.const 0
    call $max_f32
    local.set $content_w
    local.get $bounds
    i32.const 12
    i32.add
    f32.load
    local.get $padding
    f32.const 2
    f32.mul
    f32.sub
    f32.const 0
    call $max_f32
    local.set $content_h
    local.get $item_count
    f32.convert_i32_u
    f32.const 1
    call $max_f32
    local.set $count
    local.get $content_w
    local.get $count
    f32.div
    f32.const 1
    call $max_f32
    local.set $segment_w
    local.get $out
    local.get $content_x
    local.get $index
    f32.convert_i32_u
    local.get $segment_w
    f32.mul
    f32.add
    local.get $content_y
    local.get $segment_w
    local.get $content_h
    call $rect_store)

  (func $er_ui_list_item_strip_bounds  (param $bounds i32) (param $index i32) (param $widths_ptr i32) (param $width_count i32) (param $padding f32) (param $gap f32) (param $item_h f32) (param $out i32) (result i32)
    (local $i i32)
    (local $limit i32)
    (local $x f32)
    (local $resolved_w f32)
    (local $resolved_h f32)
    local.get $bounds
    i32.eqz
    local.get $out
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $bounds
    f32.load
    local.get $padding
    f32.add
    local.set $x
    local.get $index
    local.get $width_count
    i32.lt_u
    if
      local.get $index
      local.set $limit
    else
      local.get $width_count
      local.set $limit
    end
    block $done
      loop $widths
        local.get $i
        local.get $limit
        i32.ge_u
        br_if $done
        local.get $widths_ptr
        i32.eqz
        if
          br $done
        end
        local.get $x
        local.get $widths_ptr
        local.get $i
        i32.const 4
        i32.mul
        i32.add
        f32.load
        local.get $gap
        f32.add
        f32.add
        local.set $x
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $widths
      end
    end
    local.get $index
    local.get $width_count
    i32.lt_u
    local.get $widths_ptr
    i32.const 0
    i32.ne
    i32.and
    if
      local.get $widths_ptr
      local.get $index
      i32.const 4
      i32.mul
      i32.add
      f32.load
      local.set $resolved_w
    else
      f32.const 1
      local.set $resolved_w
    end
    local.get $bounds
    i32.const 12
    i32.add
    f32.load
    local.get $padding
    f32.const 2
    f32.mul
    f32.sub
    f32.const 1
    call $max_f32
    local.get $item_h
    call $min_f32
    local.set $resolved_h
    local.get $out
    local.get $x
    local.get $bounds
    i32.const 4
    i32.add
    f32.load
    local.get $padding
    f32.add
    local.get $resolved_w
    local.get $resolved_h
    call $rect_store)

  (func $er_ui_list_clamped_index  (param $value i32) (param $item_count i32) (result i32)
    local.get $item_count
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $value
    local.get $item_count
    i32.const 1
    i32.sub
    i32.gt_u
    if (result i32)
      local.get $item_count
      i32.const 1
      i32.sub
    else
      local.get $value
    end)

  (func $er_ui_list_resolve_index  (param $controlled i32) (param $default_value i32) (param $item_count i32) (result i32)
    local.get $controlled
    i32.const -1
    i32.eq
    if (result i32)
      local.get $default_value
    else
      local.get $controlled
    end
    local.get $item_count
    call $er_ui_list_clamped_index)

  (func $er_ui_list_encoded_indexed_id  (param $id i32) (param $active i32) (param $item_count i32) (result i32)
    local.get $id
    local.get $item_count
    i32.mul
    local.get $active
    local.get $item_count
    call $er_ui_list_clamped_index
    i32.add)
  (func $er_ui_view_stack_cursor_size  (result i32)
    i32.const 24)

  (func $er_ui_view_row_cursor_size  (result i32)
    i32.const 24)

  (func $er_ui_view_split_size  (result i32)
    i32.const 32)

  (func $er_ui_view_stack_cursor_init  (param $cursor i32) (param $bounds i32) (param $gap f32) (result i32)
    local.get $cursor
    i32.eqz
    local.get $bounds
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $cursor
    local.get $bounds
    f32.load
    local.get $bounds
    i32.const 4
    i32.add
    f32.load
    local.get $bounds
    i32.const 8
    i32.add
    f32.load
    local.get $bounds
    i32.const 12
    i32.add
    f32.load
    call $rect_store
    drop
    local.get $cursor
    i32.const 16
    i32.add
    local.get $gap
    f32.store
    local.get $cursor
    i32.const 20
    i32.add
    local.get $bounds
    i32.const 4
    i32.add
    f32.load
    f32.store
    i32.const 1)

  (func $er_ui_view_stack_cursor_take  (param $cursor i32) (param $height f32) (param $out i32) (result i32)
    (local $resolved_h f32)
    (local $cursor_y f32)
    local.get $cursor
    i32.eqz
    local.get $out
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $height
    f32.const 1
    call $max_f32
    local.set $resolved_h
    local.get $cursor
    i32.const 20
    i32.add
    f32.load
    local.set $cursor_y
    local.get $out
    local.get $cursor
    f32.load
    local.get $cursor_y
    local.get $cursor
    i32.const 8
    i32.add
    f32.load
    local.get $resolved_h
    call $rect_store
    drop
    local.get $cursor
    i32.const 20
    i32.add
    local.get $cursor_y
    local.get $resolved_h
    f32.add
    local.get $cursor
    i32.const 16
    i32.add
    f32.load
    f32.add
    f32.store
    i32.const 1)

  (func $er_ui_view_stack_cursor_take_if_fits  (param $cursor i32) (param $height f32) (param $out i32) (result i32)
    local.get $cursor
    i32.eqz
    local.get $out
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $cursor
    i32.const 20
    i32.add
    f32.load
    local.get $height
    f32.add
    local.get $cursor
    i32.const 4
    i32.add
    f32.load
    local.get $cursor
    i32.const 12
    i32.add
    f32.load
    f32.add
    f32.gt
    if
      i32.const 0
      return
    end
    local.get $cursor
    local.get $height
    local.get $out
    call $er_ui_view_stack_cursor_take)

  (func $er_ui_view_stack_cursor_skip  (param $cursor i32) (param $amount f32) (result i32)
    local.get $cursor
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $cursor
    i32.const 20
    i32.add
    local.get $cursor
    i32.const 20
    i32.add
    f32.load
    local.get $amount
    f32.add
    f32.store
    i32.const 1)

  (func $er_ui_view_stack_cursor_remaining  (param $cursor i32) (param $out i32) (result i32)
    local.get $cursor
    i32.eqz
    local.get $out
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $cursor
    f32.load
    local.get $cursor
    i32.const 20
    i32.add
    f32.load
    local.get $cursor
    i32.const 8
    i32.add
    f32.load
    local.get $cursor
    i32.const 4
    i32.add
    f32.load
    local.get $cursor
    i32.const 12
    i32.add
    f32.load
    f32.add
    local.get $cursor
    i32.const 20
    i32.add
    f32.load
    f32.sub
    f32.const 1
    call $max_f32
    call $rect_store)

  (func $er_ui_view_row_cursor_init  (param $cursor i32) (param $bounds i32) (param $gap f32) (result i32)
    local.get $cursor
    i32.eqz
    local.get $bounds
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $cursor
    local.get $bounds
    f32.load
    local.get $bounds
    i32.const 4
    i32.add
    f32.load
    local.get $bounds
    i32.const 8
    i32.add
    f32.load
    local.get $bounds
    i32.const 12
    i32.add
    f32.load
    call $rect_store
    drop
    local.get $cursor
    i32.const 16
    i32.add
    local.get $gap
    f32.store
    local.get $cursor
    i32.const 20
    i32.add
    local.get $bounds
    f32.load
    f32.store
    i32.const 1)

  (func $er_ui_view_row_cursor_take  (param $cursor i32) (param $width f32) (param $out i32) (result i32)
    (local $resolved_w f32)
    (local $cursor_x f32)
    local.get $cursor
    i32.eqz
    local.get $out
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $width
    f32.const 1
    call $max_f32
    local.set $resolved_w
    local.get $cursor
    i32.const 20
    i32.add
    f32.load
    local.set $cursor_x
    local.get $out
    local.get $cursor_x
    local.get $cursor
    i32.const 4
    i32.add
    f32.load
    local.get $resolved_w
    local.get $cursor
    i32.const 12
    i32.add
    f32.load
    call $rect_store
    drop
    local.get $cursor
    i32.const 20
    i32.add
    local.get $cursor_x
    local.get $resolved_w
    f32.add
    local.get $cursor
    i32.const 16
    i32.add
    f32.load
    f32.add
    f32.store
    i32.const 1)

  (func $er_ui_view_row_cursor_remaining  (param $cursor i32) (param $out i32) (result i32)
    local.get $cursor
    i32.eqz
    local.get $out
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $cursor
    i32.const 20
    i32.add
    f32.load
    local.get $cursor
    i32.const 4
    i32.add
    f32.load
    local.get $cursor
    f32.load
    local.get $cursor
    i32.const 8
    i32.add
    f32.load
    f32.add
    local.get $cursor
    i32.const 20
    i32.add
    f32.load
    f32.sub
    f32.const 1
    call $max_f32
    local.get $cursor
    i32.const 12
    i32.add
    f32.load
    call $rect_store)

  (func $er_ui_view_grid_item  (param $bounds i32) (param $columns i32) (param $gap f32) (param $item_h f32) (param $index i32) (param $out i32) (result i32)
    (local $columns_value i32)
    (local $col i32)
    (local $row i32)
    (local $item_w f32)
    local.get $bounds
    i32.eqz
    local.get $out
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $columns
    i32.eqz
    if
      i32.const 1
      local.set $columns_value
    else
      local.get $columns
      local.set $columns_value
    end
    local.get $index
    local.get $columns_value
    i32.rem_u
    local.set $col
    local.get $index
    local.get $columns_value
    i32.div_u
    local.set $row
    local.get $bounds
    i32.const 8
    i32.add
    f32.load
    local.get $gap
    local.get $columns_value
    i32.const 1
    i32.sub
    f32.convert_i32_u
    f32.mul
    f32.sub
    local.get $columns_value
    f32.convert_i32_u
    f32.div
    f32.const 1
    call $max_f32
    local.set $item_w
    local.get $out
    local.get $bounds
    f32.load
    local.get $col
    f32.convert_i32_u
    local.get $item_w
    local.get $gap
    f32.add
    f32.mul
    f32.add
    local.get $bounds
    i32.const 4
    i32.add
    f32.load
    local.get $row
    f32.convert_i32_u
    local.get $item_h
    local.get $gap
    f32.add
    f32.mul
    f32.add
    local.get $item_w
    local.get $item_h
    call $rect_store)

  (func $er_ui_view_grid_height  (param $columns i32) (param $gap f32) (param $item_h f32) (param $item_count i32) (result f32)
    (local $columns_value i32)
    (local $rows i32)
    local.get $item_count
    i32.eqz
    if
      f32.const 0
      return
    end
    local.get $columns
    i32.eqz
    if
      i32.const 1
      local.set $columns_value
    else
      local.get $columns
      local.set $columns_value
    end
    local.get $item_count
    local.get $columns_value
    i32.add
    i32.const 1
    i32.sub
    local.get $columns_value
    i32.div_u
    local.set $rows
    local.get $rows
    f32.convert_i32_u
    local.get $item_h
    f32.mul
    local.get $rows
    i32.const 1
    i32.sub
    f32.convert_i32_u
    local.get $gap
    f32.mul
    f32.add)

  (func $er_ui_view_split_left  (param $bounds i32) (param $width f32) (param $gap f32) (param $out i32) (result i32)
    (local $first_w f32)
    (local $rest_x f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $width f32.const 1 call $max_f32 local.get $bounds i32.const 8 i32.add f32.load call $min_f32 local.set $first_w
    local.get $bounds f32.load local.get $first_w f32.add local.get $gap f32.add local.set $rest_x
    local.get $out local.get $bounds f32.load local.get $bounds i32.const 4 i32.add f32.load local.get $first_w local.get $bounds i32.const 12 i32.add f32.load call $rect_store drop
    local.get $out i32.const 16 i32.add local.get $rest_x local.get $bounds i32.const 4 i32.add f32.load local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add local.get $rest_x f32.sub f32.const 1 call $max_f32 local.get $bounds i32.const 12 i32.add f32.load call $rect_store)

  (func $er_ui_view_split_right  (param $bounds i32) (param $width f32) (param $gap f32) (param $out i32) (result i32)
    (local $second_w f32)
    (local $second_x f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $width f32.const 1 call $max_f32 local.get $bounds i32.const 8 i32.add f32.load call $min_f32 local.set $second_w
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load local.get $second_w f32.sub f32.const 0 call $max_f32 f32.add local.set $second_x
    local.get $out local.get $bounds f32.load local.get $bounds i32.const 4 i32.add f32.load local.get $second_x local.get $gap f32.sub local.get $bounds f32.load f32.sub f32.const 1 call $max_f32 local.get $bounds i32.const 12 i32.add f32.load call $rect_store drop
    local.get $out i32.const 16 i32.add local.get $second_x local.get $bounds i32.const 4 i32.add f32.load local.get $second_w local.get $bounds i32.const 12 i32.add f32.load call $rect_store)

  (func $er_ui_view_split_top  (param $bounds i32) (param $height f32) (param $gap f32) (param $out i32) (result i32)
    (local $first_h f32)
    (local $rest_y f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $height f32.const 1 call $max_f32 local.get $bounds i32.const 12 i32.add f32.load call $min_f32 local.set $first_h
    local.get $bounds i32.const 4 i32.add f32.load local.get $first_h f32.add local.get $gap f32.add local.set $rest_y
    local.get $out local.get $bounds f32.load local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 8 i32.add f32.load local.get $first_h call $rect_store drop
    local.get $out i32.const 16 i32.add local.get $bounds f32.load local.get $rest_y local.get $bounds i32.const 8 i32.add f32.load local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.add local.get $rest_y f32.sub f32.const 1 call $max_f32 call $rect_store)

  (func $er_ui_view_split_bottom  (param $bounds i32) (param $height f32) (param $gap f32) (param $out i32) (result i32)
    (local $second_h f32)
    (local $second_y f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $height f32.const 1 call $max_f32 local.get $bounds i32.const 12 i32.add f32.load call $min_f32 local.set $second_h
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load local.get $second_h f32.sub f32.const 0 call $max_f32 f32.add local.set $second_y
    local.get $out local.get $bounds f32.load local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 8 i32.add f32.load local.get $second_y local.get $gap f32.sub local.get $bounds i32.const 4 i32.add f32.load f32.sub f32.const 1 call $max_f32 call $rect_store drop
    local.get $out i32.const 16 i32.add local.get $bounds f32.load local.get $second_y local.get $bounds i32.const 8 i32.add f32.load local.get $second_h call $rect_store)
  (func $er_ui_layout_axis_horizontal  (result i32) i32.const 0)
  (func $er_ui_layout_axis_vertical  (result i32) i32.const 1)
  (func $er_ui_layout_align_start  (result i32) i32.const 0)
  (func $er_ui_layout_align_stretch  (result i32) i32.const 1)
  (func $er_ui_layout_constraint_unconstrained  (result i32) i32.const 0)
  (func $er_ui_layout_constraint_at_most  (result i32) i32.const 1)
  (func $er_ui_layout_constraint_exact  (result i32) i32.const 2)
  (func $er_ui_layout_wrap_auto  (result i32) i32.const 0)
  (func $er_ui_layout_wrap_wrap  (result i32) i32.const 1)
  (func $er_ui_layout_wrap_nowrap  (result i32) i32.const 2)
  (func $er_ui_layout_wrap_truncate  (result i32) i32.const 3)
  (func $er_ui_layout_axis_constraint_size  (result i32) i32.const 8)
  (func $er_ui_layout_insets_size  (result i32) i32.const 16)
  (func $er_ui_layout_constraints_size  (result i32) i32.const 20)
  (func $er_ui_layout_measurement_size  (result i32) i32.const 24)
  (func $er_ui_layout_text_metrics_size  (result i32) i32.const 12)
  (func $er_ui_flex_options_size  (result i32) i32.const 28)
  (func $er_ui_flex_cursor_size  (result i32) i32.const 60)

  (func $layout_sanitize_size (param $value f32) (result f32)
    local.get $value
    call $finite_f32
    i32.eqz
    local.get $value
    f32.const 0
    f32.le
    i32.or
    if
      f32.const 0
      return
    end
    local.get $value)

  (func $layout_sanitize_positive (param $value f32) (param $fallback f32) (result f32)
    local.get $value
    call $finite_f32
    i32.eqz
    local.get $value
    f32.const 0
    f32.le
    i32.or
    if
      local.get $fallback
      return
    end
    local.get $value)

  (func $layout_min_i32_u (param $a i32) (param $b i32) (result i32)
    local.get $a local.get $b call $min_i32_u)

  (func $layout_max_i32_u (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.gt_u
    if (result i32)
      local.get $a
    else
      local.get $b
    end)

  (func $layout_store_size (param $out i32) (param $w f32) (param $h f32) (result i32)
    local.get $out
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $w
    call $layout_sanitize_size
    f32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $h
    call $layout_sanitize_size
    f32.store
    i32.const 1)

  (func $layout_store_constraint (param $out i32) (param $tag i32) (param $value f32) (result i32)
    local.get $out
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $tag
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $value
    call $layout_sanitize_size
    f32.store
    i32.const 1)

  (func $er_ui_layout_axis_constraint_init  (param $out i32) (param $tag i32) (param $value f32) (result i32)
    local.get $out
    local.get $tag
    local.get $value
    call $layout_store_constraint)

  (func $er_ui_layout_axis_constraint_limit  (param $constraint i32) (param $fallback f32) (result f32)
    local.get $constraint
    i32.eqz
    if
      local.get $fallback
      return
    end
    local.get $constraint
    i32.load
    i32.eqz
    if
      local.get $fallback
      return
    end
    local.get $constraint
    i32.const 4
    i32.add
    f32.load
    call $layout_sanitize_size)

  (func $er_ui_layout_axis_constraint_exact_value  (param $constraint i32) (param $out i32) (result i32)
    local.get $constraint
    i32.eqz
    local.get $out
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $constraint
    i32.load
    i32.const 2
    i32.ne
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $constraint
    i32.const 4
    i32.add
    f32.load
    call $layout_sanitize_size
    f32.store
    i32.const 1)

  (func $er_ui_layout_insets_uniform  (param $value f32) (param $out i32) (result i32)
    (local $safe f32)
    local.get $out
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $value
    call $layout_sanitize_size
    local.set $safe
    local.get $out local.get $safe f32.store
    local.get $out i32.const 4 i32.add local.get $safe f32.store
    local.get $out i32.const 8 i32.add local.get $safe f32.store
    local.get $out i32.const 12 i32.add local.get $safe f32.store
    i32.const 1)

  (func $er_ui_layout_insets_horizontal  (param $insets i32) (result f32)
    local.get $insets
    i32.eqz
    if
      f32.const 0
      return
    end
    local.get $insets
    i32.const 12
    i32.add
    f32.load
    local.get $insets
    i32.const 4
    i32.add
    f32.load
    f32.add)

  (func $er_ui_layout_insets_vertical  (param $insets i32) (result f32)
    local.get $insets
    i32.eqz
    if
      f32.const 0
      return
    end
    local.get $insets
    f32.load
    local.get $insets
    i32.const 8
    i32.add
    f32.load
    f32.add)

  (func $layout_shrink_constraint (param $constraint i32) (param $amount f32) (param $out i32) (result i32)
    (local $safe_amount f32)
    (local $tag i32)
    local.get $constraint i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $amount call $layout_sanitize_size local.set $safe_amount
    local.get $constraint i32.load local.set $tag
    local.get $tag i32.eqz
    if
      local.get $out i32.const 0 f32.const 0 call $layout_store_constraint return
    end
    local.get $out
    local.get $tag
    local.get $constraint i32.const 4 i32.add f32.load call $layout_sanitize_size
    local.get $safe_amount
    f32.sub
    f32.const 0
    call $max_f32
    call $layout_store_constraint)

  (func $er_ui_layout_constraints_init  (param $out i32) (param $width_tag i32) (param $width_value f32) (param $height_tag i32) (param $height_value f32) (param $text_wrap i32) (result i32)
    local.get $out
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $out local.get $width_tag local.get $width_value call $layout_store_constraint drop
    local.get $out i32.const 8 i32.add local.get $height_tag local.get $height_value call $layout_store_constraint drop
    local.get $out i32.const 16 i32.add local.get $text_wrap i32.store
    i32.const 1)

  (func $er_ui_layout_constraints_inner  (param $constraints i32) (param $insets i32) (param $out i32) (result i32)
    local.get $constraints i32.eqz local.get $insets i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $constraints local.get $insets call $er_ui_layout_insets_horizontal local.get $out call $layout_shrink_constraint drop
    local.get $constraints i32.const 8 i32.add local.get $insets call $er_ui_layout_insets_vertical local.get $out i32.const 8 i32.add call $layout_shrink_constraint drop
    local.get $out i32.const 16 i32.add local.get $constraints i32.const 16 i32.add i32.load i32.store
    i32.const 1)

  (func $layout_measurement_store (param $out i32) (param $min_w f32) (param $min_h f32) (param $pref_w f32) (param $pref_h f32) (param $max_w f32) (param $max_h f32) (result i32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $out local.get $min_w local.get $min_h call $layout_store_size drop
    local.get $out i32.const 8 i32.add local.get $pref_w local.get $pref_h call $layout_store_size drop
    local.get $out i32.const 16 i32.add local.get $max_w local.get $max_h call $layout_store_size drop
    i32.const 1)

  (func $er_ui_layout_measurement_fixed  (param $w f32) (param $h f32) (param $out i32) (result i32)
    (local $sw f32) (local $sh f32)
    local.get $w call $layout_sanitize_size local.set $sw
    local.get $h call $layout_sanitize_size local.set $sh
    local.get $out local.get $sw local.get $sh local.get $sw local.get $sh local.get $sw local.get $sh call $layout_measurement_store)

  (func $er_ui_layout_measurement_flexible  (param $min_w f32) (param $min_h f32) (param $pref_w f32) (param $pref_h f32) (param $max_w f32) (param $max_h f32) (param $out i32) (result i32)
    (local $mnw f32) (local $mnh f32) (local $pfw f32) (local $pfh f32) (local $mxw f32) (local $mxh f32)
    local.get $min_w call $layout_sanitize_size local.set $mnw
    local.get $min_h call $layout_sanitize_size local.set $mnh
    local.get $pref_w call $layout_sanitize_size local.get $mnw call $max_f32 local.set $pfw
    local.get $pref_h call $layout_sanitize_size local.get $mnh call $max_f32 local.set $pfh
    local.get $max_w call $layout_sanitize_size local.get $pfw call $max_f32 local.set $mxw
    local.get $max_h call $layout_sanitize_size local.get $pfh call $max_f32 local.set $mxh
    local.get $out local.get $mnw local.get $mnh local.get $pfw local.get $pfh local.get $mxw local.get $mxh call $layout_measurement_store)

  (func $er_ui_layout_measurement_with_insets  (param $measurement i32) (param $insets i32) (param $out i32) (result i32)
    (local $h f32) (local $v f32)
    local.get $measurement i32.eqz local.get $insets i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $insets call $er_ui_layout_insets_horizontal local.set $h
    local.get $insets call $er_ui_layout_insets_vertical local.set $v
    local.get $out
    local.get $measurement f32.load local.get $h f32.add
    local.get $measurement i32.const 4 i32.add f32.load local.get $v f32.add
    local.get $measurement i32.const 8 i32.add f32.load local.get $h f32.add
    local.get $measurement i32.const 12 i32.add f32.load local.get $v f32.add
    local.get $measurement i32.const 16 i32.add f32.load local.get $h f32.add
    local.get $measurement i32.const 20 i32.add f32.load local.get $v f32.add
    call $layout_measurement_store)

  (func $er_ui_layout_measurement_apply_exact  (param $measurement i32) (param $constraints i32) (param $out i32) (result i32)
    (local $pref_w f32) (local $pref_h f32)
    local.get $measurement i32.eqz local.get $constraints i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $measurement i32.const 8 i32.add f32.load local.set $pref_w
    local.get $measurement i32.const 12 i32.add f32.load local.set $pref_h
    local.get $constraints i32.load i32.const 2 i32.eq
    if local.get $constraints i32.const 4 i32.add f32.load call $layout_sanitize_size local.set $pref_w end
    local.get $constraints i32.const 8 i32.add i32.load i32.const 2 i32.eq
    if local.get $constraints i32.const 12 i32.add f32.load call $layout_sanitize_size local.set $pref_h end
    local.get $out
    local.get $measurement f32.load local.get $pref_w call $min_f32
    local.get $measurement i32.const 4 i32.add f32.load local.get $pref_h call $min_f32
    local.get $pref_w
    local.get $pref_h
    local.get $measurement i32.const 16 i32.add f32.load local.get $pref_w call $max_f32
    local.get $measurement i32.const 20 i32.add f32.load local.get $pref_h call $max_f32
    call $layout_measurement_store)

  (func $er_ui_layout_to_logical  (param $axis i32) (param $w f32) (param $h f32) (param $out i32) (result i32)
    local.get $axis i32.const 1 i32.eq
    if
      local.get $out local.get $h local.get $w call $layout_store_size
      return
    end
    local.get $out local.get $w local.get $h call $layout_store_size)

  (func $er_ui_layout_from_logical  (param $axis i32) (param $w f32) (param $h f32) (param $out i32) (result i32)
    local.get $axis local.get $w local.get $h local.get $out call $er_ui_layout_to_logical)

  (func $er_ui_layout_text_metrics_init  (param $out i32) (param $line_height f32) (param $average_char_width f32) (param $max_lines i32) (result i32)
    local.get $out
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $out local.get $line_height f32.store
    local.get $out i32.const 4 i32.add local.get $average_char_width f32.store
    local.get $out i32.const 8 i32.add local.get $max_lines i32.store
    i32.const 1)

  (func $er_ui_layout_measure_text  (param $text_ptr i32) (param $text_len i32) (param $constraints i32) (param $metrics i32) (param $out i32) (result i32)
    (local $line_height f32)
    (local $avg f32)
    (local $char_count i32)
    (local $longest i32)
    (local $natural_width f32)
    (local $min_width f32)
    (local $wrap_width f32)
    (local $measured_width f32)
    (local $line_count i32)
    (local $preferred_height f32)
    (local $pref_w f32)
    (local $pref_h f32)
    (local $max_width f32)
    (local $should_wrap i32)
    local.get $constraints i32.eqz local.get $metrics i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $text_len i32.eqz
    local.get $metrics i32.const 8 i32.add i32.load i32.eqz
    i32.or
    if
      f32.const 0 f32.const 0 local.get $out call $er_ui_layout_measurement_fixed
      return
    end
    local.get $metrics f32.load f32.const 16 call $layout_sanitize_positive local.set $line_height
    local.get $metrics i32.const 4 i32.add f32.load f32.const 8 call $layout_sanitize_positive local.set $avg
    local.get $text_ptr local.get $text_len call $er_ui_utf8_codepoint_count local.set $char_count
    local.get $text_ptr local.get $text_len call $er_ui_text_longest_utf8_run local.set $longest
    local.get $char_count f32.convert_i32_u local.get $avg f32.mul local.set $natural_width
    local.get $avg local.get $longest f32.convert_i32_u local.get $avg f32.mul call $max_f32 local.set $min_width
    local.get $constraints local.get $natural_width call $er_ui_layout_axis_constraint_limit local.set $wrap_width
    local.get $constraints i32.const 16 i32.add i32.load i32.const 1 i32.eq
    local.get $constraints i32.const 16 i32.add i32.load i32.eqz
    local.get $constraints i32.load i32.const 0 i32.ne
    i32.and
    i32.or
    local.set $should_wrap
    local.get $should_wrap
    if
      local.get $natural_width
      local.get $avg
      local.get $wrap_width
      call $max_f32
      call $min_f32
      local.set $measured_width
      local.get $text_ptr local.get $text_len local.get $measured_width local.get $avg local.get $metrics i32.const 8 i32.add i32.load call $er_ui_text_wrapped_line_count local.set $line_count
    else
      local.get $natural_width
      local.set $measured_width
      i32.const 1
      local.set $line_count
    end
    local.get $line_count
    i32.const 1
    call $layout_max_i32_u
    f32.convert_i32_u
    local.get $line_height
    f32.mul
    local.set $preferred_height
    local.get $measured_width local.set $pref_w
    local.get $preferred_height local.set $pref_h
    local.get $constraints i32.load i32.const 2 i32.eq
    if local.get $constraints i32.const 4 i32.add f32.load call $layout_sanitize_size local.set $pref_w end
    local.get $constraints i32.const 8 i32.add i32.load i32.const 2 i32.eq
    if local.get $constraints i32.const 12 i32.add f32.load call $layout_sanitize_size local.set $pref_h end
    local.get $constraints i32.load i32.eqz
    if
      local.get $natural_width local.set $max_width
    else
      local.get $constraints i32.const 4 i32.add f32.load call $layout_sanitize_size local.set $max_width
    end
    local.get $min_width local.get $pref_w call $min_f32
    local.get $line_height local.get $pref_h call $min_f32
    local.get $pref_w
    local.get $pref_h
    local.get $pref_w local.get $max_width call $max_f32
    local.get $pref_h local.get $preferred_height call $max_f32
    local.get $out
    call $er_ui_layout_measurement_flexible)

  (func $er_ui_flex_measure  (param $children i32) (param $child_count i32) (param $constraints i32) (param $axis i32) (param $gap f32) (param $insets i32) (param $out i32) (result i32)
    (local $i i32) (local $child i32) (local $gap_i f32)
    (local $min_main f32) (local $min_cross f32) (local $pref_main f32) (local $pref_cross f32) (local $max_main f32) (local $max_cross f32)
    local.get $children i32.eqz local.get $constraints i32.eqz i32.or local.get $insets i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    block $done
      loop $items
        local.get $i local.get $child_count i32.ge_u br_if $done
        local.get $children local.get $i i32.const 24 i32.mul i32.add local.set $child
        local.get $i i32.eqz if f32.const 0 local.set $gap_i else local.get $gap local.set $gap_i end
        local.get $min_main local.get $child i32.const 0 local.get $axis call $layout_logical_main local.get $gap_i f32.add f32.add local.set $min_main
        local.get $pref_main local.get $child i32.const 8 local.get $axis call $layout_logical_main local.get $gap_i f32.add f32.add local.set $pref_main
        local.get $max_main local.get $child i32.const 16 local.get $axis call $layout_logical_main local.get $gap_i f32.add f32.add local.set $max_main
        local.get $min_cross local.get $child i32.const 0 local.get $axis call $layout_logical_cross call $max_f32 local.set $min_cross
        local.get $pref_cross local.get $child i32.const 8 local.get $axis call $layout_logical_cross call $max_f32 local.set $pref_cross
        local.get $max_cross local.get $child i32.const 16 local.get $axis call $layout_logical_cross call $max_f32 local.set $max_cross
        local.get $i i32.const 1 i32.add local.set $i
        br $items
      end
    end
    local.get $axis i32.const 1 i32.eq
    if
      local.get $min_main local.get $min_cross local.set $min_main local.set $min_cross
      local.get $pref_main local.get $pref_cross local.set $pref_main local.set $pref_cross
      local.get $max_main local.get $max_cross local.set $max_main local.set $max_cross
    end
    local.get $min_main local.get $min_cross local.get $pref_main local.get $pref_cross local.get $max_main local.get $max_cross i32.const 120128 call $er_ui_layout_measurement_flexible drop
    i32.const 120128 local.get $insets i32.const 120160 call $er_ui_layout_measurement_with_insets drop
    i32.const 120160 local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func $flex_inner_rect (param $bounds i32) (param $insets i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $insets i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds
    local.get $out
    local.get $insets i32.const 12 i32.add f32.load
    local.get $insets f32.load
    local.get $insets i32.const 4 i32.add f32.load
    local.get $insets i32.const 8 i32.add f32.load
    call $er_ui_rect_inset_ltrb)

  (func $flex_inner_main (param $inner i32) (param $axis i32) (result f32)
    local.get $axis i32.const 1 i32.eq
    if (result f32)
      local.get $inner i32.const 12 i32.add f32.load
    else
      local.get $inner i32.const 8 i32.add f32.load
    end)

  (func $flex_inner_cross (param $inner i32) (param $axis i32) (result f32)
    local.get $axis i32.const 1 i32.eq
    if (result f32)
      local.get $inner i32.const 8 i32.add f32.load
    else
      local.get $inner i32.const 12 i32.add f32.load
    end)

  (func $er_ui_flex_resolve_main_sizes  (param $bounds i32) (param $children i32) (param $child_count i32) (param $axis i32) (param $gap f32) (param $insets i32) (param $out_sizes i32) (param $out_cap i32) (result i32)
    (local $count i32) (local $i i32) (local $child i32)
    (local $available f32) (local $available_children f32) (local $preferred_total f32) (local $min_total f32) (local $scale f32) (local $overflow f32) (local $shrink_capacity f32) (local $child_pref f32) (local $child_min f32) (local $child_shrink f32)
    local.get $bounds i32.eqz local.get $children i32.eqz i32.or local.get $insets i32.eqz i32.or local.get $out_sizes i32.eqz i32.or
    if i32.const 0 return end
    local.get $child_count local.get $out_cap i32.lt_u
    if local.get $child_count local.set $count else local.get $out_cap local.set $count end
    local.get $count i32.eqz if i32.const 0 return end
    local.get $bounds local.get $insets i32.const 120192 call $flex_inner_rect drop
    i32.const 120192 local.get $axis call $flex_inner_main local.set $available
    local.get $available local.get $gap local.get $count i32.const 1 i32.sub f32.convert_i32_u f32.mul f32.sub f32.const 0 call $max_f32 local.set $available_children
    block $scan_done
      loop $scan
        local.get $i local.get $count i32.ge_u br_if $scan_done
        local.get $children local.get $i i32.const 24 i32.mul i32.add local.set $child
        local.get $child i32.const 8 local.get $axis call $layout_logical_main local.set $child_pref
        local.get $child i32.const 0 local.get $axis call $layout_logical_main local.set $child_min
        local.get $out_sizes local.get $i i32.const 4 i32.mul i32.add local.get $child_pref f32.store
        local.get $preferred_total local.get $child_pref f32.add local.set $preferred_total
        local.get $min_total local.get $child_min f32.add local.set $min_total
        local.get $i i32.const 1 i32.add local.set $i
        br $scan
      end
    end
    local.get $preferred_total local.get $available_children f32.le
    if local.get $count return end
    local.get $min_total local.get $available_children f32.ge
    if
      local.get $min_total f32.const 0 f32.gt
      if local.get $available_children local.get $min_total f32.div local.set $scale else f32.const 0 local.set $scale end
      i32.const 0 local.set $i
      block $scale_done
        loop $scale_loop
          local.get $i local.get $count i32.ge_u br_if $scale_done
          local.get $children local.get $i i32.const 24 i32.mul i32.add i32.const 0 local.get $axis call $layout_logical_main local.get $scale f32.mul f32.const 1 call $max_f32 local.set $child_min
          local.get $out_sizes local.get $i i32.const 4 i32.mul i32.add local.get $child_min f32.store
          local.get $i i32.const 1 i32.add local.set $i
          br $scale_loop
        end
      end
      local.get $count
      return
    end
    local.get $preferred_total local.get $available_children f32.sub local.set $overflow
    local.get $preferred_total local.get $min_total f32.sub local.set $shrink_capacity
    i32.const 0 local.set $i
    block $shrink_done
      loop $shrink_loop
        local.get $i local.get $count i32.ge_u br_if $shrink_done
        local.get $children local.get $i i32.const 24 i32.mul i32.add local.set $child
        local.get $child i32.const 8 local.get $axis call $layout_logical_main local.set $child_pref
        local.get $child i32.const 0 local.get $axis call $layout_logical_main local.set $child_min
        local.get $child_pref local.get $child_min f32.sub local.set $child_shrink
        local.get $out_sizes local.get $i i32.const 4 i32.mul i32.add
        local.get $child_pref
        local.get $overflow
        local.get $child_shrink
        local.get $shrink_capacity
        f32.div
        f32.mul
        f32.sub
        f32.const 1
        call $max_f32
        f32.store
        local.get $i i32.const 1 i32.add local.set $i
        br $shrink_loop
      end
    end
    local.get $count)

  (func $er_ui_flex_place  (param $bounds i32) (param $children i32) (param $child_count i32) (param $axis i32) (param $gap f32) (param $insets i32) (param $cross_align i32) (param $out_rects i32) (param $out_cap i32) (param $scratch_sizes i32) (result i32)
    (local $count i32) (local $i i32) (local $child i32) (local $main_offset f32) (local $main_size f32) (local $cross_size f32) (local $inner_cross f32)
    local.get $bounds i32.eqz local.get $children i32.eqz i32.or local.get $insets i32.eqz i32.or local.get $out_rects i32.eqz i32.or local.get $scratch_sizes i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $children local.get $child_count local.get $axis local.get $gap local.get $insets local.get $scratch_sizes local.get $out_cap call $er_ui_flex_resolve_main_sizes local.set $count
    local.get $bounds local.get $insets i32.const 120192 call $flex_inner_rect drop
    i32.const 120192 local.get $axis call $flex_inner_cross local.set $inner_cross
    block $done
      loop $items
        local.get $i local.get $count i32.ge_u br_if $done
        local.get $i i32.eqz i32.eqz
        if local.get $main_offset local.get $gap f32.add local.set $main_offset end
        local.get $children local.get $i i32.const 24 i32.mul i32.add local.set $child
        local.get $scratch_sizes local.get $i i32.const 4 i32.mul i32.add f32.load local.set $main_size
        local.get $cross_align i32.eqz
        if
          local.get $child i32.const 8 local.get $axis call $layout_logical_cross local.get $inner_cross call $min_f32 local.set $cross_size
        else
          local.get $inner_cross local.set $cross_size
        end
        local.get $axis i32.const 1 i32.eq
        if
          local.get $out_rects local.get $i i32.const 16 i32.mul i32.add
          i32.const 120192 f32.load
          i32.const 120192 i32.const 4 i32.add f32.load local.get $main_offset f32.add
          local.get $cross_size
          local.get $main_size
          call $rect_store drop
        else
          local.get $out_rects local.get $i i32.const 16 i32.mul i32.add
          i32.const 120192 f32.load local.get $main_offset f32.add
          i32.const 120192 i32.const 4 i32.add f32.load
          local.get $main_size
          local.get $cross_size
          call $rect_store drop
        end
        local.get $main_offset local.get $main_size f32.add local.set $main_offset
        local.get $i i32.const 1 i32.add local.set $i
        br $items
      end
    end
    local.get $count)

  (func $er_ui_flex_options_init  (param $out i32) (param $axis i32) (param $gap f32) (param $insets i32) (param $cross_align i32) (result i32)
    local.get $out i32.eqz local.get $insets i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $axis i32.store
    local.get $out i32.const 4 i32.add local.get $gap f32.store
    local.get $out i32.const 8 i32.add local.get $insets f32.load f32.store
    local.get $out i32.const 12 i32.add local.get $insets i32.const 4 i32.add f32.load f32.store
    local.get $out i32.const 16 i32.add local.get $insets i32.const 8 i32.add f32.load f32.store
    local.get $out i32.const 20 i32.add local.get $insets i32.const 12 i32.add f32.load f32.store
    local.get $out i32.const 24 i32.add local.get $cross_align i32.store
    i32.const 1)

  (func $er_ui_flex_cursor_init  (param $cursor i32) (param $bounds i32) (param $options i32) (param $sizes_ptr i32) (param $sizes_len i32) (result i32)
    local.get $cursor i32.eqz local.get $bounds i32.eqz i32.or local.get $options i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $options i32.const 8 i32.add local.get $cursor call $flex_inner_rect drop
    local.get $cursor i32.const 16 i32.add local.get $options i32.load i32.store
    local.get $cursor i32.const 20 i32.add local.get $options i32.const 4 i32.add f32.load f32.store
    local.get $cursor i32.const 24 i32.add local.get $options i32.const 8 i32.add f32.load f32.store
    local.get $cursor i32.const 28 i32.add local.get $options i32.const 12 i32.add f32.load f32.store
    local.get $cursor i32.const 32 i32.add local.get $options i32.const 16 i32.add f32.load f32.store
    local.get $cursor i32.const 36 i32.add local.get $options i32.const 20 i32.add f32.load f32.store
    local.get $cursor i32.const 40 i32.add local.get $options i32.const 24 i32.add i32.load i32.store
    local.get $cursor i32.const 44 i32.add local.get $sizes_ptr i32.store
    local.get $cursor i32.const 48 i32.add local.get $sizes_len i32.store
    local.get $cursor i32.const 52 i32.add f32.const 0 f32.store
    local.get $cursor i32.const 56 i32.add i32.const 0 i32.store
    i32.const 1)

  (func $flex_cursor_axis (param $cursor i32) (result i32)
    local.get $cursor i32.const 16 i32.add i32.load)

  (func $flex_cursor_gap (param $cursor i32) (result f32)
    local.get $cursor i32.const 20 i32.add f32.load)

  (func $flex_cursor_cross_align (param $cursor i32) (result i32)
    local.get $cursor i32.const 40 i32.add i32.load)

  (func $flex_cursor_next_main_offset (param $cursor i32) (result f32)
    local.get $cursor i32.const 52 i32.add f32.load
    local.get $cursor i32.const 56 i32.add i32.load
    i32.eqz
    if (result f32)
      f32.const 0
    else
      local.get $cursor call $flex_cursor_gap
    end
    f32.add)

  (func $flex_cursor_child_main (param $cursor i32) (param $child i32) (result f32)
    (local $index i32)
    local.get $cursor i32.const 56 i32.add i32.load local.set $index
    local.get $cursor i32.const 44 i32.add i32.load
    i32.const 0
    i32.ne
    local.get $index
    local.get $cursor i32.const 48 i32.add i32.load
    i32.lt_u
    i32.and
    if (result f32)
      local.get $cursor i32.const 44 i32.add i32.load
      local.get $index i32.const 4 i32.mul i32.add
      f32.load
    else
      local.get $child i32.const 8 local.get $cursor call $flex_cursor_axis call $layout_logical_main
    end)

  (func $flex_cursor_child_cross (param $cursor i32) (param $child i32) (result f32)
    local.get $cursor call $flex_cursor_cross_align
    i32.eqz
    if (result f32)
      local.get $child i32.const 8 local.get $cursor call $flex_cursor_axis call $layout_logical_cross
      local.get $cursor local.get $cursor call $flex_cursor_axis call $flex_inner_cross
      call $min_f32
    else
      local.get $cursor local.get $cursor call $flex_cursor_axis call $flex_inner_cross
    end)

  (func $flex_cursor_rect_at (param $cursor i32) (param $main_offset f32) (param $child i32) (param $out i32) (result i32)
    (local $main_size f32) (local $cross_size f32)
    local.get $cursor local.get $child call $flex_cursor_child_main local.set $main_size
    local.get $cursor local.get $child call $flex_cursor_child_cross local.set $cross_size
    local.get $cursor call $flex_cursor_axis i32.const 1 i32.eq
    if
      local.get $out
      local.get $cursor f32.load
      local.get $cursor i32.const 4 i32.add f32.load local.get $main_offset f32.add
      local.get $cross_size
      local.get $main_size
      call $rect_store
      return
    end
    local.get $out
    local.get $cursor f32.load local.get $main_offset f32.add
    local.get $cursor i32.const 4 i32.add f32.load
    local.get $main_size
    local.get $cross_size
    call $rect_store)

  (func $flex_cursor_claim (param $cursor i32) (param $main_offset f32) (param $child i32)
    local.get $cursor i32.const 52 i32.add
    local.get $main_offset
    local.get $cursor local.get $child call $flex_cursor_child_main
    f32.add
    f32.store
    local.get $cursor i32.const 56 i32.add
    local.get $cursor i32.const 56 i32.add i32.load
    i32.const 1
    i32.add
    i32.store)

  (func $er_ui_flex_cursor_next  (param $cursor i32) (param $child i32) (param $out i32) (result i32)
    (local $main_offset f32)
    local.get $cursor i32.eqz local.get $child i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $cursor call $flex_cursor_next_main_offset local.set $main_offset
    local.get $cursor local.get $main_offset local.get $child local.get $out call $flex_cursor_rect_at drop
    local.get $cursor local.get $main_offset local.get $child call $flex_cursor_claim
    i32.const 1)

  (func $er_ui_flex_cursor_next_within_bounds  (param $cursor i32) (param $child i32) (param $out i32) (result i32)
    (local $main_offset f32)
    local.get $cursor i32.eqz local.get $child i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $cursor call $flex_cursor_next_main_offset local.set $main_offset
    local.get $main_offset
    local.get $cursor local.get $child call $flex_cursor_child_main
    f32.add
    local.get $cursor local.get $cursor call $flex_cursor_axis call $flex_inner_main
    f32.gt
    if
      i32.const 0
      return
    end
    local.get $cursor local.get $main_offset local.get $child local.get $out call $flex_cursor_rect_at drop
    local.get $cursor local.get $main_offset local.get $child call $flex_cursor_claim
    i32.const 1)
