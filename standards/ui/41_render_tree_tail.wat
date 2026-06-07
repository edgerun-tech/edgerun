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
                    f32.const 20
                    local.get $desc_depth
                    f32.convert_i32_u
                    f32.const 8
                    f32.mul
                    f32.add
                    f32.add
                    local.get $state_value
                    f32.convert_i32_u
                    f32.const 20
                    f32.mul
                    f32.add
                    local.get $cy
                    f32.const 12
                    local.get $desc_depth
                    f32.convert_i32_u
                    f32.const 4
                    f32.mul
                    f32.add
                    f32.add
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
                      local.get $cw
                      f32.const 24
                      f32.sub
                      local.get $ch
                      f32.const 16
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
                      local.get $cw
                      f32.const 24
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
                      f32.const 20
                      local.get $ch
                      f32.const 16
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
                    f32.const 20
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
                    local.get $cw
                    f32.const 16
                    f32.sub
                    local.get $ch
                    f32.const 8
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
                    f32.const 20
                    local.get $desc_depth
                    f32.convert_i32_u
                    f32.const 8
                    f32.mul
                    f32.add
                    f32.add
                    local.get $cy
                    local.get $ch
                    f32.const 0.5
                    f32.mul
                    f32.add
                    local.get $desc_depth
                    f32.convert_i32_u
                    f32.const 4
                    f32.mul
                    f32.add
                    local.get $cw
                    f32.const 16
                    f32.sub
                    local.get $ch
                    f32.const 8
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
              local.get $count
              i32.const -1
              i32.eq
              if
                i32.const -1
                return
              end
            end
            local.get $child_i
            i32.const 1
            i32.add
            local.set $child_i
            br $children
          end
        end
        local.get $root_cw
        local.set $cw
        local.get $root_ch
        local.set $ch
        local.get $axis
        i32.const 0
        i32.eq
        if
          local.get $cy
          local.get $ch
          local.get $gap
          f32.add
          f32.add
          local.set $cy
        else
          local.get $cx
          local.get $cw
          local.get $gap
          f32.add
          f32.add
          local.set $cx
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    local.get $count)
