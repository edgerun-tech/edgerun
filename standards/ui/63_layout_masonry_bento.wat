(func (export "er_ui_layout_masonry_child") (param $out i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $children i32) (param $index i32) (param $columns i32) (param $gap f32) (param $padding f32) (param $heights_ptr i32) (result i32)
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

  (func (export "er_ui_layout_bento_child") (param $out i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $children i32) (param $index i32) (param $columns i32) (param $gap f32) (param $padding f32) (param $col_spans_ptr i32) (param $row_spans_ptr i32) (result i32)
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
