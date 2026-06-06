

  (func (export "proto_standard_id") (result i32)
    i32.const 300072)


  (func $store_record
    (param $base i32) (param $idx i32)
    (param $event_ptr i32) (param $event_len i32)
    (param $data_ptr i32) (param $data_len i32)
    (param $id_ptr i32) (param $id_len i32)
    (param $retry_lo i32) (param $retry_hi i32)
    (param $flags i32) (param $end_off i32)
    (local $p i32)
    local.get $base
    local.get $idx
    i32.const 40
    i32.mul
    i32.add
    local.tee $p
    local.get $event_ptr
    i32.store
    local.get $p
    i32.const 4
    i32.add
    local.get $event_len
    i32.store
    local.get $p
    i32.const 8
    i32.add
    local.get $data_ptr
    i32.store
    local.get $p
    i32.const 12
    i32.add
    local.get $data_len
    i32.store
    local.get $p
    i32.const 16
    i32.add
    local.get $id_ptr
    i32.store
    local.get $p
    i32.const 20
    i32.add
    local.get $id_len
    i32.store
    local.get $p
    i32.const 24
    i32.add
    local.get $retry_lo
    i32.store
    local.get $p
    i32.const 28
    i32.add
    local.get $retry_hi
    i32.store
    local.get $p
    i32.const 32
    i32.add
    local.get $flags
    i32.store
    local.get $p
    i32.const 36
    i32.add
    local.get $end_off
    i32.store)

  (func $m168copy (param $src i32) (param $len i32) (param $dst i32)
    (local $i i32)
    loop $loop
      local.get $i
      local.get $len
      i32.lt_u
      if
        local.get $dst
        local.get $i
        i32.add
        local.get $src
        local.get $i
        i32.add
        i32.load8_u
        i32.store8
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end)

  (func $m168copy_message (param $dst i32)
    local.get $dst
    i32.const 109
    i32.store8
    local.get $dst
    i32.const 1
    i32.add
    i32.const 101
    i32.store8
    local.get $dst
    i32.const 2
    i32.add
    i32.const 115
    i32.store8
    local.get $dst
    i32.const 3
    i32.add
    i32.const 115
    i32.store8
    local.get $dst
    i32.const 4
    i32.add
    i32.const 97
    i32.store8
    local.get $dst
    i32.const 5
    i32.add
    i32.const 103
    i32.store8
    local.get $dst
    i32.const 6
    i32.add
    i32.const 101
    i32.store8)

  (func $valid_utf8 (param $ptr i32) (param $len i32) (result i32)
    (local $i i32) (local $b0 i32) (local $b1 i32) (local $b2 i32) (local $b3 i32)
    loop $loop
      local.get $i
      local.get $len
      i32.lt_u
      if
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        local.tee $b0
        i32.const 128
        i32.lt_u
        if
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $loop
        end
        local.get $b0
        i32.const 194
        i32.ge_u
        local.get $b0
        i32.const 223
        i32.le_u
        i32.and
        if
          local.get $i
          i32.const 1
          i32.add
          local.get $len
          i32.ge_u
          if
            i32.const 0
            return
          end
          local.get $ptr
          local.get $i
          i32.add
          i32.const 1
          i32.add
          i32.load8_u
          local.tee $b1
          i32.const 128
          i32.ge_u
          local.get $b1
          i32.const 191
          i32.le_u
          i32.and
          i32.eqz
          if
            i32.const 0
            return
          end
          local.get $i
          i32.const 2
          i32.add
          local.set $i
          br $loop
        end
        local.get $b0
        i32.const 224
        i32.ge_u
        local.get $b0
        i32.const 239
        i32.le_u
        i32.and
        if
          local.get $i
          i32.const 2
          i32.add
          local.get $len
          i32.ge_u
          if
            i32.const 0
            return
          end
          local.get $ptr
          local.get $i
          i32.add
          i32.const 1
          i32.add
          i32.load8_u
          local.set $b1
          local.get $ptr
          local.get $i
          i32.add
          i32.const 2
          i32.add
          i32.load8_u
          local.set $b2
          local.get $b2
          i32.const 128
          i32.ge_u
          local.get $b2
          i32.const 191
          i32.le_u
          i32.and
          i32.eqz
          if
            i32.const 0
            return
          end
          local.get $b0
          i32.const 224
          i32.eq
          if
            local.get $b1
            i32.const 160
            i32.ge_u
            local.get $b1
            i32.const 191
            i32.le_u
            i32.and
            i32.eqz
            if
              i32.const 0
              return
            end
          else
            local.get $b0
            i32.const 237
            i32.eq
            if
              local.get $b1
              i32.const 128
              i32.ge_u
              local.get $b1
              i32.const 159
              i32.le_u
              i32.and
              i32.eqz
              if
                i32.const 0
                return
              end
            else
              local.get $b1
              i32.const 128
              i32.ge_u
              local.get $b1
              i32.const 191
              i32.le_u
              i32.and
              i32.eqz
              if
                i32.const 0
                return
              end
            end
          end
          local.get $i
          i32.const 3
          i32.add
          local.set $i
          br $loop
        end
        local.get $b0
        i32.const 240
        i32.ge_u
        local.get $b0
        i32.const 244
        i32.le_u
        i32.and
        if
          local.get $i
          i32.const 3
          i32.add
          local.get $len
          i32.ge_u
          if
            i32.const 0
            return
          end
          local.get $ptr
          local.get $i
          i32.add
          i32.const 1
          i32.add
          i32.load8_u
          local.set $b1
          local.get $ptr
          local.get $i
          i32.add
          i32.const 2
          i32.add
          i32.load8_u
          local.set $b2
          local.get $ptr
          local.get $i
          i32.add
          i32.const 3
          i32.add
          i32.load8_u
          local.set $b3
          local.get $b2
          i32.const 128
          i32.ge_u
          local.get $b2
          i32.const 191
          i32.le_u
          i32.and
          local.get $b3
          i32.const 128
          i32.ge_u
          local.get $b3
          i32.const 191
          i32.le_u
          i32.and
          i32.and
          i32.eqz
          if
            i32.const 0
            return
          end
          local.get $b0
          i32.const 240
          i32.eq
          if
            local.get $b1
            i32.const 144
            i32.ge_u
            local.get $b1
            i32.const 191
            i32.le_u
            i32.and
            i32.eqz
            if
              i32.const 0
              return
            end
          else
            local.get $b0
            i32.const 244
            i32.eq
            if
              local.get $b1
              i32.const 128
              i32.ge_u
              local.get $b1
              i32.const 143
              i32.le_u
              i32.and
              i32.eqz
              if
                i32.const 0
                return
              end
            else
              local.get $b1
              i32.const 128
              i32.ge_u
              local.get $b1
              i32.const 191
              i32.le_u
              i32.and
              i32.eqz
              if
                i32.const 0
                return
              end
            end
          end
          local.get $i
          i32.const 4
          i32.add
          local.set $i
          br $loop
        end
        i32.const 0
        return
      end
    end
    i32.const 1)

  (func $field_is_data (param $ptr i32) (param $len i32) (result i32)
    local.get $len
    i32.const 4
    i32.eq
    local.get $ptr
    i32.load8_u
    i32.const 100
    i32.eq
    i32.and
    local.get $ptr
    i32.const 1
    i32.add
    i32.load8_u
    i32.const 97
    i32.eq
    i32.and
    local.get $ptr
    i32.const 2
    i32.add
    i32.load8_u
    i32.const 116
    i32.eq
    i32.and
    local.get $ptr
    i32.const 3
    i32.add
    i32.load8_u
    i32.const 97
    i32.eq
    i32.and)

  (func $field_is_event (param $ptr i32) (param $len i32) (result i32)
    local.get $len
    i32.const 5
    i32.eq
    local.get $ptr
    i32.load8_u
    i32.const 101
    i32.eq
    i32.and
    local.get $ptr
    i32.const 1
    i32.add
    i32.load8_u
    i32.const 118
    i32.eq
    i32.and
    local.get $ptr
    i32.const 2
    i32.add
    i32.load8_u
    i32.const 101
    i32.eq
    i32.and
    local.get $ptr
    i32.const 3
    i32.add
    i32.load8_u
    i32.const 110
    i32.eq
    i32.and
    local.get $ptr
    i32.const 4
    i32.add
    i32.load8_u
    i32.const 116
    i32.eq
    i32.and)

  (func $field_is_id (param $ptr i32) (param $len i32) (result i32)
    local.get $len
    i32.const 2
    i32.eq
    local.get $ptr
    i32.load8_u
    i32.const 105
    i32.eq
    i32.and
    local.get $ptr
    i32.const 1
    i32.add
    i32.load8_u
    i32.const 100
    i32.eq
    i32.and)

  (func $field_is_retry (param $ptr i32) (param $len i32) (result i32)
    local.get $len
    i32.const 5
    i32.eq
    local.get $ptr
    i32.load8_u
    i32.const 114
    i32.eq
    i32.and
    local.get $ptr
    i32.const 1
    i32.add
    i32.load8_u
    i32.const 101
    i32.eq
    i32.and
    local.get $ptr
    i32.const 2
    i32.add
    i32.load8_u
    i32.const 116
    i32.eq
    i32.and
    local.get $ptr
    i32.const 3
    i32.add
    i32.load8_u
    i32.const 114
    i32.eq
    i32.and
    local.get $ptr
    i32.const 4
    i32.add
    i32.load8_u
    i32.const 121
    i32.eq
    i32.and)

  (func $contains_nul (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    loop $loop
      local.get $i
      local.get $len
      i32.lt_u
      if
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.eqz
        if
          i32.const 1
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

  (func (export "sse_parse_events")
    (param $ptr i32) (param $len i32)
    (param $last_ptr i32) (param $last_len i32)
    (param $rec_ptr i32) (param $rec_cap i32)
    (param $text_ptr i32) (param $text_cap i32)
    (result i64)
    (local $i i32) (local $line_start i32) (local $line_end i32)
    (local $line_len i32) (local $field_len i32) (local $value_start i32)
    (local $value_len i32) (local $scan i32) (local $colon i32)
    (local $text_used i32) (local $count i32) (local $flags i32)
    (local $event_src i32) (local $event_len i32)
    (local $data_off i32) (local $data_len i32) (local $saw_data i32)
    (local $id_src i32) (local $id_len i32)
    (local $retry_lo i32) (local $retry_hi i32) (local $retry_seen i32)
    (local $digit i32) (local $any_digit i32)
    (local $event_out i32) (local $id_out i32)

    local.get $ptr
    local.get $len
    call $valid_utf8
    i32.eqz
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end

    local.get $last_ptr
    local.set $id_src
    local.get $last_len
    local.set $id_len

    block $done
      loop $loop
        local.get $i
        local.get $len
        i32.le_u
        if
          local.get $i
          local.get $len
          i32.eq
          local.get $ptr
          local.get $i
          i32.add
          i32.load8_u
          i32.const 10
          i32.eq
          i32.or
          if
            local.get $i
            local.set $line_end
            local.get $line_end
            local.get $line_start
            i32.gt_u
            if
              local.get $ptr
              local.get $line_end
              i32.add
              i32.const 1
              i32.sub
              i32.load8_u
              i32.const 13
              i32.eq
              if
                local.get $line_end
                i32.const 1
                i32.sub
                local.set $line_end
              end
            end
            local.get $line_end
            local.get $line_start
            i32.sub
            local.tee $line_len
            i32.eqz
            if
              local.get $saw_data
              if
                local.get $count
                local.get $rec_cap
                i32.ge_u
                if
                  i32.const 2
                  local.get $count
                  call $pack
                  return
                end
                local.get $event_len
                i32.eqz
                if
                  local.get $text_used
                  i32.const 7
                  i32.add
                  local.get $text_cap
                  i32.gt_u
                  if
                    i32.const 2
                    local.get $count
                    call $pack
                    return
                  end
                  local.get $text_ptr
                  local.get $text_used
                  i32.add
                  local.tee $event_out
                  call $m168copy_message
                  local.get $text_used
                  i32.const 7
                  i32.add
                  local.set $text_used
                  i32.const 7
                  local.set $event_len
                else
                  local.get $text_used
                  local.get $event_len
                  i32.add
                  local.get $text_cap
                  i32.gt_u
                  if
                    i32.const 2
                    local.get $count
                    call $pack
                    return
                  end
                  local.get $text_ptr
                  local.get $text_used
                  i32.add
                  local.set $event_out
                  local.get $event_src
                  local.get $event_len
                  local.get $text_ptr
                  local.get $text_used
                  i32.add
                  call $m168copy
                  local.get $text_used
                  local.get $event_len
                  i32.add
                  local.set $text_used
                end
                local.get $text_used
                local.get $id_len
                i32.add
                local.get $text_cap
                i32.gt_u
                if
                  i32.const 2
                  local.get $count
                  call $pack
                  return
                end
                local.get $text_ptr
                local.get $text_used
                i32.add
                local.set $id_out
                local.get $id_src
                local.get $id_len
                local.get $text_ptr
                local.get $text_used
                i32.add
                call $m168copy
                local.get $text_used
                local.get $id_len
                i32.add
                local.set $text_used
                local.get $retry_seen
                local.set $flags
                local.get $rec_ptr
                local.get $count
                local.get $event_out
                local.get $event_len
                local.get $data_off
                local.get $data_len
                local.get $id_out
                local.get $id_len
                local.get $retry_lo
                local.get $retry_hi
                local.get $flags
                local.get $i
                call $store_record
                local.get $count
                i32.const 1
                i32.add
                local.set $count
              end
              i32.const 0
              local.set $event_src
              i32.const 0
              local.set $event_len
              i32.const 0
              local.set $data_off
              i32.const 0
              local.set $data_len
              i32.const 0
              local.set $saw_data
              i32.const 0
              local.set $retry_lo
              i32.const 0
              local.set $retry_hi
              i32.const 0
              local.set $retry_seen
            else
              local.get $ptr
              local.get $line_start
              i32.add
              i32.load8_u
              i32.const 58
              i32.ne
              if
                local.get $line_start
                local.set $scan
                local.get $line_len
                local.set $field_len
                i32.const -1
                local.set $colon
                loop $field_loop
                  local.get $scan
                  local.get $line_end
                  i32.lt_u
                  if
                    local.get $ptr
                    local.get $scan
                    i32.add
                    i32.load8_u
                    i32.const 58
                    i32.eq
                    if
                      local.get $scan
                      local.set $colon
                      local.get $scan
                      local.get $line_start
                      i32.sub
                      local.set $field_len
                      local.get $line_end
                      local.set $scan
                      br $field_loop
                    end
                    local.get $scan
                    i32.const 1
                    i32.add
                    local.set $scan
                    br $field_loop
                  end
                end
                local.get $colon
                i32.const -1
                i32.eq
                if
                  local.get $line_end
                  local.set $value_start
                  i32.const 0
                  local.set $value_len
                else
                  local.get $colon
                  i32.const 1
                  i32.add
                  local.set $value_start
                  local.get $line_end
                  local.get $value_start
                  i32.sub
                  local.set $value_len
                  local.get $value_len
                  if
                    local.get $ptr
                    local.get $value_start
                    i32.add
                    i32.load8_u
                    i32.const 32
                    i32.eq
                    if
                      local.get $value_start
                      i32.const 1
                      i32.add
                      local.set $value_start
                      local.get $value_len
                      i32.const 1
                      i32.sub
                      local.set $value_len
                    end
                  end
                end
                local.get $ptr
                local.get $line_start
                i32.add
                local.get $field_len
                call $field_is_data
                if
                  local.get $saw_data
                  if
                    local.get $text_used
                    i32.const 1
                    i32.add
                    local.get $text_cap
                    i32.gt_u
                    if
                      i32.const 2
                      local.get $count
                      call $pack
                      return
                    end
                    local.get $text_ptr
                    local.get $text_used
                    i32.add
                    i32.const 10
                    i32.store8
                    local.get $text_used
                    i32.const 1
                    i32.add
                    local.set $text_used
                    local.get $data_len
                    i32.const 1
                    i32.add
                    local.set $data_len
                  else
                    local.get $text_ptr
                    local.get $text_used
                    i32.add
                    local.set $data_off
                    i32.const 1
                    local.set $saw_data
                  end
                  local.get $text_used
                  local.get $value_len
                  i32.add
                  local.get $text_cap
                  i32.gt_u
                  if
                    i32.const 2
                    local.get $count
                    call $pack
                    return
                  end
                  local.get $ptr
                  local.get $value_start
                  i32.add
                  local.get $value_len
                  local.get $text_ptr
                  local.get $text_used
                  i32.add
                  call $m168copy
                  local.get $text_used
                  local.get $value_len
                  i32.add
                  local.set $text_used
                  local.get $data_len
                  local.get $value_len
                  i32.add
                  local.set $data_len
                else
                  local.get $ptr
                  local.get $line_start
                  i32.add
                  local.get $field_len
                  call $field_is_event
                  if
                    local.get $ptr
                    local.get $value_start
                    i32.add
                    local.set $event_src
                    local.get $value_len
                    local.set $event_len
                  else
                    local.get $ptr
                    local.get $line_start
                    i32.add
                    local.get $field_len
                    call $field_is_id
                    if
                      local.get $ptr
                      local.get $value_start
                      i32.add
                      local.get $value_len
                      call $contains_nul
                      i32.eqz
                      if
                        local.get $ptr
                        local.get $value_start
                        i32.add
                        local.set $id_src
                        local.get $value_len
                        local.set $id_len
                      end
                    else
                      local.get $ptr
                      local.get $line_start
                      i32.add
                      local.get $field_len
                      call $field_is_retry
                      if
                        i32.const 0
                        local.set $retry_lo
                        i32.const 0
                        local.set $retry_hi
                        i32.const 0
                        local.set $any_digit
                        i32.const 0
                        local.set $scan
                        loop $digit_loop
                          local.get $scan
                          local.get $value_len
                          i32.lt_u
                          if
                            local.get $ptr
                            local.get $value_start
                            i32.add
                            local.get $scan
                            i32.add
                            i32.load8_u
                            local.tee $digit
                            i32.const 48
                            i32.lt_u
                            local.get $digit
                            i32.const 57
                            i32.gt_u
                            i32.or
                            if
                              i32.const 0
                              local.set $any_digit
                              local.get $value_len
                              local.set $scan
                              br $digit_loop
                            end
                            local.get $retry_lo
                            i32.const 10
                            i32.mul
                            local.get $digit
                            i32.const 48
                            i32.sub
                            i32.add
                            local.set $retry_lo
                            i32.const 1
                            local.set $any_digit
                            local.get $scan
                            i32.const 1
                            i32.add
                            local.set $scan
                            br $digit_loop
                          end
                        end
                        local.get $any_digit
                        if
                          i32.const 1
                          local.set $retry_seen
                        end
                      end
                    end
                  end
                end
              end
            end
            local.get $i
            i32.const 1
            i32.add
            local.set $line_start
          end
          local.get $i
          local.get $len
          i32.eq
          if
            br $done
          end
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $loop
        end
      end
    end
    local.get $saw_data
    if
      local.get $count
      local.get $rec_cap
      i32.ge_u
      if
        i32.const 2
        local.get $count
        call $pack
        return
      end
      local.get $event_len
      i32.eqz
      if
        local.get $text_used
        i32.const 7
        i32.add
        local.get $text_cap
        i32.gt_u
        if
          i32.const 2
          local.get $count
          call $pack
          return
        end
        local.get $text_ptr
        local.get $text_used
        i32.add
        local.tee $event_out
        call $m168copy_message
        local.get $text_used
        i32.const 7
        i32.add
        local.set $text_used
        i32.const 7
        local.set $event_len
      else
        local.get $text_used
        local.get $event_len
        i32.add
        local.get $text_cap
        i32.gt_u
        if
          i32.const 2
          local.get $count
          call $pack
          return
        end
        local.get $text_ptr
        local.get $text_used
        i32.add
        local.set $event_out
        local.get $event_src
        local.get $event_len
        local.get $text_ptr
        local.get $text_used
        i32.add
        call $m168copy
        local.get $text_used
        local.get $event_len
        i32.add
        local.set $text_used
      end
      local.get $text_used
      local.get $id_len
      i32.add
      local.get $text_cap
      i32.gt_u
      if
        i32.const 2
        local.get $count
        call $pack
        return
      end
      local.get $text_ptr
      local.get $text_used
      i32.add
      local.set $id_out
      local.get $id_src
      local.get $id_len
      local.get $text_ptr
      local.get $text_used
      i32.add
      call $m168copy
      local.get $text_used
      local.get $id_len
      i32.add
      local.set $text_used
      local.get $rec_ptr
      local.get $count
      local.get $event_out
      local.get $event_len
      local.get $data_off
      local.get $data_len
      local.get $id_out
      local.get $id_len
      local.get $retry_lo
      local.get $retry_hi
      local.get $retry_seen
      local.get $len
      call $store_record
      local.get $count
      i32.const 1
      i32.add
      local.set $count
    end
    i32.const 0
    local.get $count
    call $pack)
