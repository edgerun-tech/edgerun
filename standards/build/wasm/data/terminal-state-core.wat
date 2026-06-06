  (func (export "proto_abi_version") (result i32)
    i32.const 2)

  (func (export "proto_standard_id") (result i32)
    i32.const 300098)


  ;; Mirrors terminal.rs Perform classification:
  ;; printable: 0x20..0x7e plus 0x80.., execute/control: C0 plus DEL.
  (func (export "terminal_classify_byte") (param $byte i32) (result i32)
    local.get $byte
    i32.const 255
    i32.and
    local.tee $byte
    i32.const 32
    i32.ge_u
    local.get $byte
    i32.const 126
    i32.le_u
    i32.and
    local.get $byte
    i32.const 128
    i32.ge_u
    i32.or
    if
      i32.const 1
      return
    end
    local.get $byte
    i32.const 32
    i32.lt_u
    local.get $byte
    i32.const 127
    i32.eq
    i32.or
    if
      i32.const 2
      return
    end
    i32.const 0)

  ;; Execute action ids: 1 LF, 2 CR, 3 BS non-destructive-left,
  ;; 4 DEL destructive-backspace, 5 TAB, 0 ignored control.
  (func (export "terminal_control_action") (param $byte i32) (result i32)
    local.get $byte
    i32.const 255
    i32.and
    local.set $byte
    local.get $byte
    i32.const 10
    i32.eq
    if
      i32.const 1
      return
    end
    local.get $byte
    i32.const 13
    i32.eq
    if
      i32.const 2
      return
    end
    local.get $byte
    i32.const 8
    i32.eq
    if
      i32.const 3
      return
    end
    local.get $byte
    i32.const 127
    i32.eq
    if
      i32.const 4
      return
    end
    local.get $byte
    i32.const 9
    i32.eq
    if
      i32.const 5
      return
    end
    i32.const 0)

  (func $emit_param
    (param $out i32) (param $cap i32) (param $count i32)
    (param $value i32) (param $flags i32)
    (result i32)
    (local $base i32)
    local.get $count
    local.get $cap
    i32.ge_u
    if
      i32.const -1
      return
    end
    local.get $out
    local.get $count
    i32.const 8
    i32.mul
    i32.add
    local.set $base
    local.get $base
    local.get $value
    i32.store
    local.get $base
    i32.const 4
    i32.add
    local.get $flags
    i32.store
    local.get $count
    i32.const 1
    i32.add)

  ;; Parse the bytes after CSI introducer and through the final byte.
  ;; Return pack(status,count). status: 0 ok, 1 incomplete, 2 output cap, 3 invalid.
  ;; Output records are 8 bytes: u32 value, u32 flags.
  ;; flags bit0=value was present, bit1=private '?' marker, bit2=colon subparam separator.
  (func (export "terminal_csi_parse_params")
    (param $ptr i32) (param $len i32) (param $out i32) (param $cap i32)
    (result i64)
    (local $i i32)
    (local $b i32)
    (local $value i32)
    (local $have i32)
    (local $count i32)
    (local $private i32)
    (local $need_emit i32)
    (local $sep_flags i32)
    (local $final_found i32)

    block $done
      loop $scan
        local.get $i
        local.get $len
        i32.lt_u
        if
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        local.set $b

        local.get $b
        i32.const 64
        i32.ge_u
        local.get $b
        i32.const 126
        i32.le_u
        i32.and
        if
            i32.const 1
            local.set $final_found
            br $done
          end

          local.get $b
          i32.const 48
          i32.ge_u
          local.get $b
          i32.const 57
          i32.le_u
          i32.and
          if
            local.get $value
            i32.const 10
            i32.mul
            local.get $b
            i32.const 48
            i32.sub
            i32.add
            local.set $value
            i32.const 1
            local.set $have
            i32.const 1
            local.set $need_emit
          else
            local.get $b
            i32.const 63
            i32.eq
            local.get $i
            i32.const 0
            i32.eq
            i32.and
            if
              i32.const 1
              local.set $private
            else
              local.get $b
              i32.const 59
              i32.eq
              local.get $b
              i32.const 58
              i32.eq
              i32.or
              if
                local.get $b
                i32.const 58
                i32.eq
                if
                  i32.const 4
                  local.set $sep_flags
                else
                  i32.const 0
                  local.set $sep_flags
                end
                local.get $out
                local.get $cap
                local.get $count
                local.get $value
                local.get $have
                local.get $private
                i32.const 1
                i32.shl
                i32.or
                local.get $sep_flags
                i32.or
                call $emit_param
                local.tee $count
                i32.const -1
                i32.eq
                if
                  i32.const 2
                  local.get $cap
                  call $pack
                  return
                end
                i32.const 0
                local.set $value
                i32.const 0
                local.set $have
                i32.const 1
                local.set $need_emit
              else
                local.get $b
                i32.const 32
                i32.ge_u
                local.get $b
                i32.const 47
                i32.le_u
                i32.and
                if
                  ;; Intermediate bytes are accepted but do not produce params.
                else
                  i32.const 3
                  local.get $count
                  call $pack
                  return
                end
              end
            end
          end

          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $scan
        end
      end
    end

    local.get $final_found
    i32.eqz
    if
      i32.const 1
      local.get $count
      call $pack
      return
    end

    local.get $need_emit
    if
      local.get $out
      local.get $cap
      local.get $count
      local.get $value
      local.get $have
      local.get $private
      i32.const 1
      i32.shl
      i32.or
      call $emit_param
      local.tee $count
      i32.const -1
      i32.eq
      if
        i32.const 2
        local.get $cap
        call $pack
        return
      end
    end
    i32.const 0
    local.get $count
    call $pack)

  (func (export "terminal_csi_final")
    (param $ptr i32) (param $len i32)
    (result i32)
    (local $i i32)
    (local $b i32)
    loop $scan
      local.get $i
      local.get $len
      i32.lt_u
      if
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        local.set $b
        local.get $b
        i32.const 64
        i32.ge_u
        local.get $b
        i32.const 126
        i32.le_u
        i32.and
        if
          local.get $b
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan
      end
    end
    i32.const 0)

  (func $clamp (param $value i32) (param $min i32) (param $max i32) (result i32)
    local.get $value
    local.get $min
    i32.lt_u
    if
      local.get $min
      return
    end
    local.get $value
    local.get $max
    i32.gt_u
    if
      local.get $max
      return
    end
    local.get $value)

  ;; Pack cursor as row in high 32 bits, col in low 32 bits.
  (func (export "terminal_cursor_set")
    (param $cols i32) (param $rows i32)
    (param $left i32) (param $right i32) (param $top i32) (param $bottom i32)
    (param $origin i32) (param $col i32) (param $row i32)
    (result i64)
    (local $last_col i32)
    (local $last_row i32)
    (local $min_col i32)
    (local $max_col i32)
    (local $dst_row i32)
    local.get $cols
    i32.const 1
    i32.sub
    local.set $last_col
    local.get $rows
    i32.const 1
    i32.sub
    local.set $last_row
    local.get $left
    i32.const 0
    local.get $last_col
    call $clamp
    local.set $min_col
    local.get $right
    i32.const 0
    local.get $last_col
    call $clamp
    local.set $max_col
    local.get $origin
    if
      local.get $top
      local.get $row
      i32.add
      local.get $top
      local.get $bottom
      call $clamp
      local.set $dst_row
    else
      local.get $row
      i32.const 0
      local.get $last_row
      call $clamp
      local.set $dst_row
    end
    local.get $dst_row
    i64.extend_i32_u
    i64.const 32
    i64.shl
    local.get $col
    local.get $min_col
    local.get $max_col
    call $clamp
    i64.extend_i32_u
    i64.or)

  ;; Direction ids match CSI A/B/C/D: 0 up, 1 down, 2 right, 3 left.
  (func (export "terminal_cursor_move")
    (param $cols i32) (param $rows i32)
    (param $left i32) (param $right i32) (param $top i32) (param $bottom i32)
    (param $origin i32) (param $col i32) (param $row i32)
    (param $direction i32) (param $count i32)
    (result i64)
    (local $new_col i32)
    (local $new_row i32)
    (local $last_col i32)
    (local $last_row i32)
    (local $min_col i32)
    (local $max_col i32)
    (local $min_row i32)
    (local $max_row i32)
    local.get $col
    local.set $new_col
    local.get $row
    local.set $new_row
    local.get $cols
    i32.const 1
    i32.sub
    local.set $last_col
    local.get $rows
    i32.const 1
    i32.sub
    local.set $last_row
    local.get $left i32.const 0 local.get $last_col call $clamp local.set $min_col
    local.get $right i32.const 0 local.get $last_col call $clamp local.set $max_col
    local.get $origin
    if
      local.get $top
      local.set $min_row
      local.get $bottom
      local.get $top
      local.get $last_row
      call $clamp
      local.set $max_row
    else
      i32.const 0
      local.set $min_row
      local.get $last_row
      local.set $max_row
    end
    local.get $direction
    i32.const 0
    i32.eq
    if
      local.get $row
      local.get $count
      i32.gt_u
      if (result i32)
        local.get $row
        local.get $count
        i32.sub
      else
        i32.const 0
      end
      local.get $min_row
      local.get $max_row
      call $clamp
      local.set $new_row
    end
    local.get $direction
    i32.const 1
    i32.eq
    if
      local.get $row
      local.get $count
      i32.add
      local.get $min_row
      local.get $max_row
      call $clamp
      local.set $new_row
    end
    local.get $direction
    i32.const 2
    i32.eq
    if
      local.get $col
      local.get $count
      i32.add
      local.get $min_col
      local.get $max_col
      call $clamp
      local.set $new_col
    end
    local.get $direction
    i32.const 3
    i32.eq
    if
      local.get $col
      local.get $count
      i32.gt_u
      if (result i32)
        local.get $col
        local.get $count
        i32.sub
      else
        i32.const 0
      end
      local.get $min_col
      local.get $max_col
      call $clamp
      local.set $new_col
    end
    local.get $new_row
    i64.extend_i32_u
    i64.const 32
    i64.shl
    local.get $new_col
    i64.extend_i32_u
    i64.or)

  ;; Bit layout: bold=1, faint=2, blink=4, italic=8, underline=16,
  ;; strike=32, inverse=64, overline=128, concealed=256.
  (func (export "terminal_sgr_apply")
    (param $attrs i32) (param $param i32)
    (result i32)
    local.get $param
    i32.const 0
    i32.eq
    if
      i32.const 0
      return
    end
    local.get $param i32.const 1 i32.eq
    if local.get $attrs i32.const 1 i32.or return end
    local.get $param i32.const 2 i32.eq
    if local.get $attrs i32.const 2 i32.or return end
    local.get $param i32.const 5 i32.eq
    if local.get $attrs i32.const 4 i32.or return end
    local.get $param i32.const 3 i32.eq
    if local.get $attrs i32.const 8 i32.or return end
    local.get $param i32.const 4 i32.eq
    if local.get $attrs i32.const 16 i32.or return end
    local.get $param i32.const 9 i32.eq
    if local.get $attrs i32.const 32 i32.or return end
    local.get $param i32.const 7 i32.eq
    if local.get $attrs i32.const 64 i32.or return end
    local.get $param i32.const 53 i32.eq
    if local.get $attrs i32.const 128 i32.or return end
    local.get $param i32.const 8 i32.eq
    if local.get $attrs i32.const 256 i32.or return end
    local.get $param i32.const 21 i32.eq
    local.get $param i32.const 22 i32.eq
    i32.or
    if local.get $attrs i32.const -4 i32.and return end
    local.get $param i32.const 25 i32.eq
    if local.get $attrs i32.const -5 i32.and return end
    local.get $param i32.const 23 i32.eq
    if local.get $attrs i32.const -9 i32.and return end
    local.get $param i32.const 24 i32.eq
    if local.get $attrs i32.const -17 i32.and return end
    local.get $param i32.const 29 i32.eq
    if local.get $attrs i32.const -33 i32.and return end
    local.get $param i32.const 27 i32.eq
    if local.get $attrs i32.const -65 i32.and return end
    local.get $param i32.const 55 i32.eq
    if local.get $attrs i32.const -129 i32.and return end
    local.get $param i32.const 28 i32.eq
    if local.get $attrs i32.const -257 i32.and return end
    local.get $attrs)

  (func (export "terminal_grid_index")
    (param $cols i32) (param $rows i32) (param $col i32) (param $row i32)
    (result i32)
    local.get $col
    local.get $cols
    i32.ge_u
    local.get $row
    local.get $rows
    i32.ge_u
    i32.or
    if
      i32.const -1
      return
    end
    local.get $row
    local.get $cols
    i32.mul
    local.get $col
    i32.add)

  ;; Pack base_y in high 32 bits and base_x in low 32 bits, matching render/grid.rs.
  (func (export "terminal_grid_pixel_origin")
    (param $origin_x i32) (param $origin_y i32)
    (param $cell_w i32) (param $cell_h i32)
    (param $col i32) (param $row i32)
    (result i64)
    local.get $origin_y
    local.get $row
    local.get $cell_h
    i32.mul
    i32.add
    i64.extend_i32_u
    i64.const 32
    i64.shl
    local.get $origin_x
    local.get $col
    local.get $cell_w
    i32.mul
    i32.add
    i64.extend_i32_u
    i64.or)