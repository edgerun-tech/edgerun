  (func (export "er_ui_render") (param $base i32) (param $len i32) (param $out i32) (param $cap i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (result i32)
    (local $node_count i32)
    (local $axis i32)
    (local $gap f32)
    (local $padding f32)
    (local $i i32)
    (local $record i32)
    (local $kind i32)
    (local $id i32)
    (local $first_ref i32)
    (local $second_ref i32)
    (local $first_len i32)
    (local $second_len i32)
    (local $first_ptr i32)
    (local $second_ptr i32)
    (local $cw f32)
    (local $ch f32)
    (local $cx f32)
    (local $cy f32)
    (local $count i32)
    (local $prev_count i32)
    (local $child_i i32)
    (local $root_cw f32)
    (local $root_ch f32)
    (local $fill_w f32)
    (local $ancestor_ref i32)
    (local $ancestor_index i32)
    (local $is_descendant i32)
    (local $desc_depth i32)
    (local $state_value i32)
    local.get $base
    local.get $len
    call $er_ui_validate_deep
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $base
    i32.load
    i32.const 0x49755245
    i32.ne
    if
      i32.const -1
      return
    end
    local.get $base
    i32.const 16
    i32.add
    call $load16
    local.set $node_count
    local.get $node_count
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $base
    i32.const 10
    i32.add
    call $load16
    local.set $axis
    local.get $base
    i32.const 12
    i32.add
    call $load16
    f32.convert_i32_u
    local.set $gap
    local.get $base
    i32.const 14
    i32.add
    call $load16
    f32.convert_i32_u
    local.set $padding
    local.get $x
    local.get $padding
    f32.add
    local.set $cx
    local.get $y
    local.get $padding
    f32.add
    local.set $cy
    loop $loop
      local.get $i
      local.get $node_count
      i32.lt_u
      if
        local.get $base
        i32.const 20
        i32.add
        local.get $i
        i32.const 16
        i32.mul
        i32.add
        local.set $record
        local.get $record
        i32.const 2
        i32.add
        call $load16
        i32.eqz
        i32.eqz
        if
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $loop
        end
        local.get $record
        call $load16
        local.set $kind
        local.get $record
        i32.const 4
        i32.add
        i32.load
        local.set $id
        local.get $record
        i32.const 8
        i32.add
        call $load16
        local.get $record
        i32.const 10
        i32.add
        call $load16
        call $string_ref
        local.set $first_ref
        local.get $record
        i32.const 12
        i32.add
        call $load16
        local.get $record
        i32.const 14
        i32.add
        call $load16
        call $string_ref
        local.set $second_ref
        local.get $first_ref
        call $string_len
        local.set $first_len
        local.get $second_ref
        call $string_len
        local.set $second_len
        local.get $base
        local.get $node_count
        local.get $first_ref
        call $string_ptr
        local.set $first_ptr
        local.get $kind
        call $second_ref_is_packed
        i32.eqz
        local.get $second_len
        i32.eqz
        i32.eqz
        i32.and
        if
          local.get $base
          local.get $node_count
          local.get $second_ref
          call $string_ptr
          local.set $second_ptr
        end
        local.get $kind
        local.get $first_len
        call $preferred_w
        local.set $cw
        local.get $kind
        call $preferred_h
        local.set $ch
        local.get $axis
        i32.const 0
        i32.eq
        if
          local.get $w
          local.get $padding
          f32.const 2
          f32.mul
          f32.sub
          local.set $cw
        else
          local.get $h
          local.get $padding
          f32.const 2
          f32.mul
          f32.sub
          local.set $ch
        end
        local.get $cw
        local.get $ch
        call $valid_rect
        if
          local.get $cw
          local.set $root_cw
          local.get $ch
          local.set $root_ch
          local.get $kind
          i32.const 0
          i32.eq
          if
            local.get $count
            local.set $prev_count
            local.get $out
            local.get $cap
            local.get $count
            local.get $cx
            local.get $cy
            local.get $cw
            local.get $ch
            i32.const 0
            call $emit_rect
            local.set $count
            local.get $count
            local.get $prev_count
            i32.gt_s
            if
              local.get $out
              local.get $prev_count
              local.get $id
              local.get $kind
              i32.const 1
              call $command_set_owner_at
            end
          else
            local.get $kind
            i32.const 1
            i32.eq
            if
              local.get $count
              local.set $prev_count
              local.get $out
              local.get $cap
              local.get $count
              local.get $cx
              local.get $cy
              local.get $cw
              local.get $ch
              local.get $first_ptr
              local.get $first_len
              i32.const 0xfff7eee8
              call $emit_text
              local.set $count
              local.get $count
              local.get $prev_count
              i32.gt_s
              if
                local.get $out
                local.get $prev_count
                local.get $id
                local.get $kind
                i32.const 2
                call $command_set_owner_at
              end
            else
              local.get $count
              local.set $prev_count
              local.get $out
              local.get $cap
              local.get $count
              local.get $cx
              local.get $cy
              local.get $cw
              local.get $ch
              i32.const 0xff2a1f18
              call $emit_rect
              local.set $count
              local.get $count
              local.get $prev_count
              i32.gt_s
              if
                local.get $out
                local.get $prev_count
                local.get $id
                local.get $kind
                i32.const 1
                call $command_set_owner_at
              end
              local.get $count
              i32.const -1
              i32.ne
              if
                local.get $kind
                i32.const 46
                i32.eq
                local.get $kind
                i32.const 47
                i32.eq
                i32.or
                local.get $kind
                i32.const 52
                i32.eq
                i32.or
                local.get $kind
                i32.const 53
                i32.eq
                i32.or
                if
                  local.get $count
                  local.set $prev_count
                  local.get $out
                  local.get $cap
                  local.get $count
                  local.get $cx
                  f32.const 4
                  f32.add
                  local.get $cy
                  f32.const 4
                  f32.add
                  local.get $cw
                  f32.const 8
                  f32.sub
                  local.get $ch
                  f32.const 8
                  f32.sub
                  i32.const 0xaa000000
                  call $emit_rect
                  local.set $count
                  local.get $count
                  local.get $prev_count
                  i32.gt_s
                  if
                    local.get $out
                    local.get $prev_count
                    local.get $id
                    local.get $kind
                    i32.const 5
                    call $command_set_owner_at
                  end
                end
              end
              local.get $count
              i32.const -1
              i32.ne
              if
                local.get $kind
                i32.const 22
                i32.eq
                local.get $kind
                i32.const 23
                i32.eq
                i32.or
                local.get $kind
                i32.const 56
                i32.eq
                i32.or
                if
                  local.get $count
                  local.set $prev_count
                  local.get $out
                  local.get $cap
                  local.get $count
                  local.get $cx
                  f32.const 4
                  f32.add
                  local.get $cy
                  f32.const 4
                  f32.add
                  f32.const 16
                  local.get $ch
                  f32.const 8
                  f32.sub
                  local.get $second_ref
                  i32.const 65535
                  i32.and
                  if (result i32)
                    i32.const 0xff20c878
                  else
                    i32.const 0xff6d625a
                  end
                  call $emit_rect
                  local.set $count
                  local.get $count
                  local.get $prev_count
                  i32.gt_s
                  if
                    local.get $out
                    local.get $prev_count
                    local.get $id
                    local.get $kind
                    i32.const 3
                    call $command_set_owner_at
                    local.get $out
                    local.get $prev_count
                    local.get $second_ref
                    i32.const 65535
                    i32.and
                    call $command_set_meta_at
                  end
                else
                  local.get $kind
                  i32.const 24
                  i32.eq
                  local.get $kind
                  i32.const 37
                  i32.eq
                  i32.or
                  local.get $kind
                  i32.const 57
                  i32.eq
                  i32.or
                  if
                    local.get $second_ref
                    i32.const 65535
                    i32.and
                    f32.convert_i32_u
                    f32.const 65535
                    f32.div
                    local.get $cw
                    f32.const 8
                    f32.sub
                    f32.mul
                    local.set $fill_w
                    local.get $fill_w
                    f32.const 0
                    f32.gt
                    if
                      local.get $count
                      local.set $prev_count
                      local.get $out
                      local.get $cap
                      local.get $count
                      local.get $cx
                      f32.const 4
                      f32.add
                      local.get $cy
                      f32.const 4
                      f32.add
                      local.get $fill_w
                      local.get $ch
                      f32.const 8
                      f32.sub
                      i32.const 0xff20a4f3
                      call $emit_rect
                      local.set $count
                      local.get $count
                      local.get $prev_count
                      i32.gt_s
                      if
                        local.get $out
                        local.get $prev_count
                        local.get $id
                        local.get $kind
                        i32.const 3
                        call $command_set_owner_at
                        local.get $out
                        local.get $prev_count
                        local.get $second_ref
                        i32.const 65535
                        i32.and
                        call $command_set_meta_at
                  end
                end
                local.get $kind
                call $second_ref_is_packed
                i32.eqz
                local.get $second_len
                i32.eqz
                i32.eqz
                i32.and
                local.get $count
                i32.const -1
                i32.ne
                i32.and
                if
                  local.get $count
                  local.set $prev_count
                  local.get $out
                  local.get $cap
                  local.get $count
                  local.get $cx
                  f32.const 12
                  f32.add
                  local.get $cy
                  local.get $ch
                  f32.const 0.5
                  f32.mul
                  f32.add
                  local.get $cw
                  f32.const 24
                  f32.sub
                  local.get $ch
                  f32.const 16
                  f32.sub
                  local.get $second_ptr
                  local.get $second_len
                  i32.const 0xffc9b8ad
                  call $emit_text
                  local.set $count
                  local.get $count
                  local.get $prev_count
                  i32.gt_s
                  if
                    local.get $out
                    local.get $prev_count
                    local.get $id
                    local.get $kind
                    i32.const 4
                    call $command_set_owner_at
                  end
                end
              end
            end
          end
              local.get $count
              i32.const -1
              i32.ne
                if
                  local.get $kind
                  i32.const 16
                i32.eq
                local.get $kind
                i32.const 20
                i32.eq
                i32.or
                local.get $kind
                i32.const 44
                i32.eq
                i32.or
                local.get $kind
                i32.const 55
                i32.eq
                i32.or
                if
                  local.get $count
                  local.set $prev_count
                  local.get $out
                  local.get $cap
                  local.get $count
                  local.get $cx
                  local.get $cw
                  f32.add
                  f32.const 20
                  f32.sub
                  local.get $cy
                  f32.const 6
                  f32.add
                  f32.const 14
                  f32.const 14
                  i32.const 0xffd6a84f
                  call $emit_rect
                  local.set $count
                  local.get $count
                  local.get $prev_count
                  i32.gt_s
                  if
                    local.get $out
                    local.get $prev_count
                    local.get $id
                    local.get $kind
                    i32.const 6
                    call $command_set_owner_at
                  end
                end
                local.get $kind
                i32.const 12
                i32.eq
                local.get $kind
                i32.const 16
                i32.eq
                i32.or
                local.get $kind
                i32.const 20
                i32.eq
                i32.or
                local.get $kind
                i32.const 44
                i32.eq
                i32.or
                local.get $kind
                i32.const 55
                i32.eq
                i32.or
                local.get $second_ref
                i32.const 65535
                i32.and
                i32.const 0
                i32.ne
                i32.and
                if
                  local.get $count
                  local.set $prev_count
                  local.get $out
                  local.get $cap
                  local.get $count
                  local.get $cx
                  f32.const 4
                  f32.add
                  local.get $cy
                  f32.const 6
                  f32.add
                  f32.const 12
                  f32.const 12
                  local.get $second_ref
                  i32.const 65535
                  i32.and
                  i32.const 0xff9fb6ff
                  call $emit_icon_or_rect
                  local.set $count
                  local.get $count
                  local.get $prev_count
                  i32.gt_s
                  if
                    local.get $out
                    local.get $prev_count
                    local.get $id
                    local.get $kind
                    i32.const 9
                    call $command_set_owner_at
                    local.get $out
                    local.get $prev_count
                    local.get $second_ref
                    i32.const 65535
                    i32.and
                    call $command_set_meta_at
                  end
                end
                local.get $kind
                local.get $id
                call $record_encoded_state
                local.tee $state_value
                i32.const 0
                i32.gt_s
                if
                  local.get $count
                  local.set $prev_count
                  local.get $out
                  local.get $cap
                  local.get $count
                  local.get $cx
                  f32.const 12
                  f32.add
                  local.get $state_value
                  f32.convert_i32_u
                  f32.const 20
                  f32.mul
                  f32.add
                  local.get $cy
                  local.get $ch
                  f32.add
                  f32.const 8
                  f32.sub
                  f32.const 16
                  f32.const 4
                  i32.const 0xffffcc55
                  call $emit_rect
                  local.set $count
                  local.get $count
                  local.get $prev_count
                  i32.gt_s
                  if
                    local.get $out
                    local.get $prev_count
                    local.get $id
                    local.get $kind
                    i32.const 10
                    call $command_set_owner_at
                    local.get $out
                    local.get $prev_count
                    local.get $state_value
                    call $command_set_meta_at
                  end
                end
                local.get $kind
                i32.const 32
                i32.eq
                if
                  local.get $count
                  local.set $prev_count
                  local.get $out
                  local.get $cap
                  local.get $count
                  local.get $cx
                  f32.const 8
                  f32.add
                  local.get $cy
                  f32.const 6
                  f32.add
                  local.get $cw
                  f32.const 16
                  f32.sub
                  f32.const 12
                  i32.const 0xff4d3f36
                  call $emit_rect
                  local.set $count
                  local.get $count
                  local.get $prev_count
                  i32.gt_s
                  if
                    local.get $out
                    local.get $prev_count
                    local.get $id
                    local.get $kind
                    i32.const 7
                    call $command_set_owner_at
                  end
                end
                local.get $kind
                i32.const 4
                i32.eq
                local.get $kind
                i32.const 11
                i32.eq
                i32.or
                local.get $kind
                i32.const 26
                i32.eq
                i32.or
                local.get $kind
                i32.const 27
                i32.eq
                i32.or
                if
                  local.get $count
                  local.set $prev_count
                  local.get $out
                  local.get $cap
                  local.get $count
                  local.get $cx
                  f32.const 4
                  f32.add
                  local.get $cy
                  f32.const 6
                  f32.add
                  f32.const 12
                  f32.const 12
                  i32.const 0xff9fb6ff
                  call $emit_rect
                  local.set $count
                  local.get $count
                  local.get $prev_count
                  i32.gt_s
                  if
                    local.get $out
                    local.get $prev_count
                    local.get $id
                    local.get $kind
                    i32.const 9
                    call $command_set_owner_at
                    local.get $out
                    local.get $prev_count
                    local.get $kind
                    i32.const 26
                    i32.eq
                    if (result i32)
                      local.get $id
                      i32.const 0x3fff
                      i32.and
                    else
                      local.get $kind
                      i32.const 4
                      i32.eq
                      local.get $kind
                      i32.const 11
                      i32.eq
                      i32.or
                      if (result i32)
                        local.get $id
                        i32.const 65535
                        i32.and
                      else
                        local.get $second_ref
                        i32.const 65535
                        i32.and
                      end
                    end
                    call $command_set_meta_at
                  end
                end
                local.get $kind
                i32.const 39
                i32.eq
                local.get $kind
                i32.const 40
                i32.eq
                i32.or
                local.get $kind
                i32.const 54
                i32.eq
                i32.or
                if
                  local.get $count
                  local.set $prev_count
                  local.get $out
                  local.get $cap
                  local.get $count
                  local.get $cx
                  f32.const 8
                  f32.add
                  local.get $cy
                  f32.const 6
                  f32.add
                  f32.const 20
                  local.get $ch
                  f32.const 12
                  f32.sub
                  i32.const 0xff5a463b
                  call $emit_rect
                  local.set $count
                  local.get $count
                  local.get $prev_count
                  i32.gt_s
                  if
                    local.get $out
                    local.get $prev_count
                    local.get $id
                    local.get $kind
                    i32.const 8
                    call $command_set_owner_at
                  end
                end
              end
              local.get $count
              i32.const -1
              i32.ne
              if
                local.get $count
                local.set $prev_count
                local.get $out
                local.get $cap
                local.get $count
                local.get $cx
                f32.const 12
                f32.add
                local.get $cy
                f32.const 8
                f32.add
                local.get $cw
                f32.const 24
                f32.sub
                local.get $ch
                f32.const 16
                f32.sub
                local.get $first_ptr
                local.get $first_len
                i32.const 0xfff7eee8
                call $emit_text
                local.set $count
                local.get $count
                local.get $prev_count
                i32.gt_s
                if
                  local.get $out
                  local.get $prev_count
                  local.get $id
                  local.get $kind
                  i32.const 2
                  call $command_set_owner_at
                end
                local.get $kind
                call $second_ref_is_packed
                i32.eqz
                local.get $second_len
                i32.eqz
                i32.eqz
                i32.and
                local.get $count
                i32.const -1
                i32.ne
                i32.and
                if
                  local.get $count
                  local.set $prev_count
                  local.get $out
                  local.get $cap
                  local.get $count
                  local.get $cx
                  f32.const 12
                  f32.add
                  local.get $cy
                  local.get $ch
                  f32.const 0.5
                  f32.mul
                  f32.add
                  local.get $cw
                  f32.const 24
                  f32.sub
                  local.get $ch
                  f32.const 16
                  f32.sub
                  local.get $second_ptr
                  local.get $second_len
                  i32.const 0xffc9b8ad
                  call $emit_text
                  local.set $count
                  local.get $count
                  local.get $prev_count
                  i32.gt_s
                  if
                    local.get $out
                    local.get $prev_count
                    local.get $id
                    local.get $kind
                    i32.const 4
                    call $command_set_owner_at
                  end
                end
              end
            end
          end
        end
        local.get $count
        i32.const -1
        i32.eq
        if
          i32.const -1
          return
        end
        i32.const 0
        local.set $child_i
        block $children_done
          loop $children
            local.get $child_i
            local.get $node_count
            i32.ge_u
            br_if $children_done
            local.get $base
            i32.const 20
            i32.add
            local.get $child_i
            i32.const 16
            i32.mul
            i32.add
            local.set $record
            i32.const 0
            local.set $is_descendant
            i32.const 0
            local.set $desc_depth
            local.get $record
            i32.const 2
            i32.add
            call $load16
            local.tee $ancestor_ref
            i32.eqz
            i32.eqz
            if
              block $ancestor_done
                loop $ancestor_loop
                  local.get $ancestor_ref
                  i32.const 1
                  i32.sub
                  local.tee $ancestor_index
                  local.get $i
                  i32.eq
                  if
                    i32.const 1
                    local.set $is_descendant
                    br $ancestor_done
                  end
                  local.get $desc_depth
                  i32.const 1
                  i32.add
                  local.set $desc_depth
                  local.get $base
                  local.get $ancestor_index
                  call $record_at
                  i32.const 2
                  i32.add
                  call $load16
                  local.tee $ancestor_ref
                  i32.eqz
                  br_if $ancestor_done
                  br $ancestor_loop
                end
              end
            end
            local.get $is_descendant
            if
              local.get $record
              call $load16
              local.set $kind
              local.get $record
              i32.const 4
              i32.add
              i32.load
              local.set $id
              local.get $record
              i32.const 8
              i32.add
              call $load16
              local.get $record
              i32.const 10
              i32.add
              call $load16
              call $string_ref
              local.set $first_ref
              local.get $first_ref
              call $string_len
              local.set $first_len
              local.get $base
              local.get $node_count
              local.get $first_ref
              call $string_ptr
              local.set $first_ptr
              local.get $record
              i32.const 12
              i32.add
              call $load16
              local.get $record
              i32.const 14
              i32.add
              call $load16
              call $string_ref
              local.set $second_ref
              local.get $second_ref
              call $string_len
              local.set $second_len
              local.get $kind
              call $second_ref_is_packed
              i32.eqz
              local.get $second_len
              i32.eqz
              i32.eqz
              i32.and
              if
                local.get $base
                local.get $node_count
                local.get $second_ref
                call $string_ptr
                local.set $second_ptr
              end
              local.get $root_cw
              f32.const 24
              local.get $desc_depth
              f32.convert_i32_u
              f32.const 16
              f32.mul
              f32.add
              f32.sub
              local.set $cw
              local.get $root_ch
              f32.const 16
              local.get $desc_depth
              f32.convert_i32_u
              f32.const 8
              f32.mul
              f32.add
              f32.sub
              local.set $ch
              local.get $cw
              local.get $ch
              call $valid_rect
              if
                local.get $kind
                i32.const 1
                i32.eq
                if
                  local.get $count
                  local.set $prev_count
                  local.get $out
                  local.get $cap
                  local.get $count
                  local.get $cx
                  f32.const 12
                  local.get $desc_depth
                  f32.convert_i32_u
                  f32.const 8
                  f32.mul
                  f32.add
                  f32.add
                  local.get $cy
                  f32.const 8
                  local.get $desc_depth
                  f32.convert_i32_u
                  f32.const 4
                  f32.mul
                  f32.add
                  f32.add
                  local.get $cw
                  local.get $ch
                  local.get $first_ptr
                  local.get $first_len
                  i32.const 0xfff7eee8
                  call $emit_text
                  local.set $count
                  local.get $count
                  local.get $prev_count
                  i32.gt_s
                  if
                    local.get $out
                    local.get $prev_count
                    local.get $id
                    local.get $kind
                    i32.const 2
                    call $command_set_owner_at
                  end
                else
                  local.get $count
                  local.set $prev_count
                  local.get $out
                  local.get $cap
                  local.get $count
                  local.get $cx
                  f32.const 12
                  local.get $desc_depth
                  f32.convert_i32_u
                  f32.const 8
                  f32.mul
                  f32.add
                  f32.add
                  local.get $cy
                  f32.const 8
                  local.get $desc_depth
                  f32.convert_i32_u
                  f32.const 4
                  f32.mul
                  f32.add
                  f32.add
                  local.get $cw
                  local.get $ch
                  i32.const 0xff3a2d25
                  call $emit_rect
                  local.set $count
                  local.get $count
                  local.get $prev_count
                  i32.gt_s
                  if
                    local.get $out
                    local.get $prev_count
                    local.get $id
                    local.get $kind
                    i32.const 1
                    call $command_set_owner_at
                  end
                  local.get $count
                  i32.const -1
                  i32.ne
                  if
                    local.get $kind
                    i32.const 22
                    i32.eq
                    local.get $kind
                    i32.const 23
                    i32.eq
                    i32.or
                    local.get $kind
                    i32.const 56
                    i32.eq
                    i32.or
                    if
                      local.get $count
                      local.set $prev_count
                      local.get $out
                      local.get $cap
                      local.get $count
                      local.get $cx
                      f32.const 16
                      local.get $desc_depth
                      f32.convert_i32_u
                      f32.const 8
                      f32.mul
                      f32.add
                      f32.add
                      local.get $cy
                      f32.const 12
                      local.get $desc_depth
                      f32.convert_i32_u
                      f32.const 4
                      f32.mul
                      f32.add
                      f32.add
                      f32.const 16
                      local.get $ch
                      f32.const 16
                      f32.sub
                      local.get $record
                      i32.const 12
                      i32.add
                      call $load16
                      if (result i32)
                        i32.const 0xff20c878
                      else
                        i32.const 0xff6d625a
                      end
                      call $emit_rect
                      local.set $count
                      local.get $count
                      local.get $prev_count
                      i32.gt_s
                      if
                        local.get $out
                        local.get $prev_count
                        local.get $id
                        local.get $kind
                        i32.const 3
                        call $command_set_owner_at
                        local.get $out
                        local.get $prev_count
                        local.get $record
                        i32.const 12
                        i32.add
                        call $load16
                        call $command_set_meta_at
                      end
                    else
                      local.get $kind
                      i32.const 24
                      i32.eq
                      local.get $kind
                      i32.const 37
                      i32.eq
                      i32.or
                      local.get $kind
                      i32.const 57
                      i32.eq
                      i32.or
                      if
                        local.get $record
                        i32.const 12
                        i32.add
                        call $load16
                        f32.convert_i32_u
                        f32.const 65535
                        f32.div
                        local.get $cw
                        f32.const 16
                        f32.sub
                        f32.mul
                        local.set $fill_w
                        local.get $fill_w
                        f32.const 0
                        f32.gt
                        if
                          local.get $count
                          local.set $prev_count
                          local.get $out
                          local.get $cap
                          local.get $count
                          local.get $cx
                          f32.const 16
                          local.get $desc_depth
                          f32.convert_i32_u
                          f32.const 8
                          f32.mul
                          f32.add
                          f32.add
                          local.get $cy
                          f32.const 12
                          local.get $desc_depth
                          f32.convert_i32_u
                          f32.const 4
                          f32.mul
                          f32.add
                          f32.add
                          local.get $fill_w
                          local.get $ch
                          f32.const 16
                          f32.sub
                          i32.const 0xff20a4f3
                          call $emit_rect
                          local.set $count
                          local.get $count
                          local.get $prev_count
                          i32.gt_s
                          if
                            local.get $out
                            local.get $prev_count
                            local.get $id
                            local.get $kind
                            i32.const 3
                            call $command_set_owner_at
                            local.get $out
                            local.get $prev_count
                            local.get $record
                            i32.const 12
                            i32.add
                            call $load16
                            call $command_set_meta_at
                          end
                        end
                      end
                      local.get $kind
                      i32.const 16
                      i32.eq
                      local.get $kind
                      i32.const 20
                      i32.eq
                      i32.or
                      local.get $kind
                      i32.const 44
                      i32.eq
                      i32.or
                      local.get $kind
                      i32.const 55
                      i32.eq
                      i32.or
                      if
                        local.get $count
                        local.set $prev_count
                        local.get $out
                        local.get $cap
                        local.get $count
                        local.get $cx
                        local.get $cw
                        f32.add
                        f32.const 24
                        f32.sub
                        local.get $cy
                        f32.const 12
                        local.get $desc_depth
                        f32.convert_i32_u
                        f32.const 4
                        f32.mul
                        f32.add
                        f32.add
                        f32.const 14
                        f32.const 14
                        i32.const 0xffd6a84f
                        call $emit_rect
                        local.set $count
                        local.get $count
                        local.get $prev_count
                        i32.gt_s
                        if
                          local.get $out
                          local.get $prev_count
                          local.get $id
                          local.get $kind
                          i32.const 6
                          call $command_set_owner_at
                        end
                      end
                    end
                  end
                  local.get $kind
                  i32.const 12
                  i32.eq
                  local.get $kind
                  i32.const 16
                  i32.eq
                  i32.or
                  local.get $kind
                  i32.const 20
                  i32.eq
                  i32.or
                  local.get $kind
                  i32.const 44
                  i32.eq
                  i32.or
                  local.get $kind
                  i32.const 55
                  i32.eq
                  i32.or
                  local.get $second_ref
                  i32.const 65535
                  i32.and
                  i32.const 0
                  i32.ne
                  i32.and
                  local.get $count
                  i32.const -1
                  i32.ne
                  i32.and
                  if
                    local.get $count
                    local.set $prev_count
                    local.get $out
                    local.get $cap
