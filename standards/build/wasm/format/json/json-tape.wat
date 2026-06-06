(module
  (import "edgerun-core" "memory" (memory 1))
  (import "edgerun-core" "is_digit" (func $is_digit (param i32) (result i32)))
  (import "edgerun-core" "pack" (func $pack (param i32 i32) (result i64)))
  (import "edgerun-core" "is_hex" (func $is_hex (param i32) (result i32)))

  (global $in_ptr (mut i32) (i32.const 0))
  (global $in_len (mut i32) (i32.const 0))
  (global $tok_ptr (mut i32) (i32.const 0))
  (global $tok_cap (mut i32) (i32.const 0))
  (global $tok_len (mut i32) (i32.const 0))


  (func (export "proto_standard_id") (result i32)
    i32.const 300007)


  (func $lo (param $packed i64) (result i32)
    local.get $packed
    i32.wrap_i64)

  (func $hi (param $packed i64) (result i32)
    local.get $packed
    i64.const 32
    i64.shr_u
    i32.wrap_i64)

  (func $m125byte_at (param $off i32) (result i32)
    global.get $in_ptr
    local.get $off
    i32.add
    i32.load8_u)

  (func $m125is_ws (param $c i32) (result i32)
    local.get $c
    i32.const 32
    i32.eq
    local.get $c
    i32.const 10
    i32.eq
    i32.or
    local.get $c
    i32.const 13
    i32.eq
    i32.or
    local.get $c
    i32.const 9
    i32.eq
    i32.or)

  (func $m125skip_ws (param $off i32) (result i32)
    (local $p i32)
    local.get $off
    local.set $p
    (block $done
      (loop $again
        local.get $p
        global.get $in_len
        i32.ge_u
        br_if $done
        local.get $p
        call $m125byte_at
        call $m125is_ws
        i32.eqz
        br_if $done
        local.get $p
        i32.const 1
        i32.add
        local.set $p
        br $again))
    local.get $p)

  (func $m125is_hex (param $c i32) (result i32)
    (call $is_hex (local.get $c)))


  (func $m125is_digit_1_9 (param $c i32) (result i32)
    local.get $c
    i32.const 49
    i32.ge_u
    local.get $c
    i32.const 57
    i32.le_u
    i32.and)

  (func $is_delim (param $c i32) (result i32)
    local.get $c
    call $m125is_ws
    local.get $c
    i32.const 44
    i32.eq
    i32.or
    local.get $c
    i32.const 93
    i32.eq
    i32.or
    local.get $c
    i32.const 125
    i32.eq
    i32.or)

  (func $json_scan_string (export "json_scan_string") (param $input_ptr i32) (param $input_len i32) (param $offset i32) (result i64)
    (local $p i32)
    (local $c i32)
    (local $n i32)
    (local $i i32)
    local.get $input_ptr
    global.set $in_ptr
    local.get $input_len
    global.set $in_len
    local.get $offset
    global.get $in_len
    i32.ge_u
    if (result i64)
      i32.const 1
      local.get $offset
      call $pack
    else
      local.get $offset
      call $m125byte_at
      i32.const 34
      i32.ne
      if (result i64)
        i32.const 3
        local.get $offset
        call $pack
      else
        local.get $offset
        i32.const 1
        i32.add
        local.set $p
        (block $done
          (loop $again
            local.get $p
            global.get $in_len
            i32.ge_u
            if
              i32.const 5
              global.get $in_len
              call $pack
              return
            end
            local.get $p
            call $m125byte_at
            local.set $c
            local.get $c
            i32.const 34
            i32.eq
            if
              i32.const 0
              local.get $p
              i32.const 1
              i32.add
              call $pack
              return
            end
            local.get $c
            i32.const 32
            i32.lt_u
            if
              i32.const 3
              local.get $p
              call $pack
              return
            end
            local.get $c
            i32.const 92
            i32.eq
            if
              local.get $p
              i32.const 1
              i32.add
              local.set $n
              local.get $n
              global.get $in_len
              i32.ge_u
              if
                i32.const 5
                global.get $in_len
                call $pack
                return
              end
              local.get $n
              call $m125byte_at
              local.set $c
              local.get $c
              i32.const 34
              i32.eq
              local.get $c
              i32.const 92
              i32.eq
              i32.or
              local.get $c
              i32.const 47
              i32.eq
              i32.or
              local.get $c
              i32.const 98
              i32.eq
              i32.or
              local.get $c
              i32.const 102
              i32.eq
              i32.or
              local.get $c
              i32.const 110
              i32.eq
              i32.or
              local.get $c
              i32.const 114
              i32.eq
              i32.or
              local.get $c
              i32.const 116
              i32.eq
              i32.or
              if
                local.get $p
                i32.const 2
                i32.add
                local.set $p
                br $again
              end
              local.get $c
              i32.const 117
              i32.ne
              if
                i32.const 3
                local.get $n
                call $pack
                return
              end
              local.get $n
              i32.const 5
              i32.add
              global.get $in_len
              i32.ge_u
              if
                i32.const 5
                global.get $in_len
                call $pack
                return
              end
              i32.const 0
              local.set $i
              (block $hex_done
                (loop $hex
                  local.get $i
                  i32.const 4
                  i32.ge_u
                  br_if $hex_done
                  local.get $n
                  i32.const 1
                  i32.add
                  local.get $i
                  i32.add
                  call $m125byte_at
                  call $m125is_hex
                  i32.eqz
                  if
                    i32.const 3
                    local.get $n
                    i32.const 1
                    i32.add
                    local.get $i
                    i32.add
                    call $pack
                    return
                  end
                  local.get $i
                  i32.const 1
                  i32.add
                  local.set $i
                  br $hex))
              local.get $p
              i32.const 6
              i32.add
              local.set $p
              br $again
            end
            local.get $p
            i32.const 1
            i32.add
            local.set $p
            br $again))
        i32.const 5
        global.get $in_len
        call $pack
      end
    end)

  (func $json_scan_number (export "json_scan_number") (param $input_ptr i32) (param $input_len i32) (param $offset i32) (result i64)
    (local $p i32)
    (local $c i32)
    (local $class i32)
    local.get $input_ptr
    global.set $in_ptr
    local.get $input_len
    global.set $in_len
    local.get $offset
    global.get $in_len
    i32.ge_u
    if
      i32.const 1
      local.get $offset
      call $pack
      return
    end
    local.get $offset
    local.set $p
    i32.const 1
    local.set $class
    local.get $p
    call $m125byte_at
    i32.const 45
    i32.eq
    if
      i32.const 2
      local.set $class
      local.get $p
      i32.const 1
      i32.add
      local.set $p
      local.get $p
      global.get $in_len
      i32.ge_u
      if
        i32.const 5
        local.get $p
        call $pack
        return
      end
    end
    local.get $p
    call $m125byte_at
    local.set $c
    local.get $c
    i32.const 48
    i32.eq
    if
      local.get $p
      i32.const 1
      i32.add
      local.set $p
      local.get $p
      global.get $in_len
      i32.lt_u
      if
        local.get $p
        call $m125byte_at
        call $is_digit
        if
          i32.const 3
          local.get $p
          call $pack
          return
        end
      end
    else
      local.get $c
      call $m125is_digit_1_9
      i32.eqz
      if
        i32.const 3
        local.get $p
        call $pack
        return
      end
      local.get $p
      i32.const 1
      i32.add
      local.set $p
      (block $digits_done
        (loop $digits
          local.get $p
          global.get $in_len
          i32.ge_u
          br_if $digits_done
          local.get $p
          call $m125byte_at
          call $is_digit
          i32.eqz
          br_if $digits_done
          local.get $p
          i32.const 1
          i32.add
          local.set $p
          br $digits))
    end
    local.get $p
    global.get $in_len
    i32.lt_u
    if
      local.get $p
      call $m125byte_at
      i32.const 46
      i32.eq
      if
        i32.const 3
        local.set $class
        local.get $p
        i32.const 1
        i32.add
        local.set $p
        local.get $p
        global.get $in_len
        i32.ge_u
        if
          i32.const 5
          local.get $p
          call $pack
          return
        end
        local.get $p
        call $m125byte_at
        call $is_digit
        i32.eqz
        if
          i32.const 3
          local.get $p
          call $pack
          return
        end
        (block $frac_done
          (loop $frac
            local.get $p
            global.get $in_len
            i32.ge_u
            br_if $frac_done
            local.get $p
            call $m125byte_at
            call $is_digit
            i32.eqz
            br_if $frac_done
            local.get $p
            i32.const 1
            i32.add
            local.set $p
            br $frac))
      end
    end
    local.get $p
    global.get $in_len
    i32.lt_u
    if
      local.get $p
      call $m125byte_at
      local.set $c
      local.get $c
      i32.const 101
      i32.eq
      local.get $c
      i32.const 69
      i32.eq
      i32.or
      if
        i32.const 3
        local.set $class
        local.get $p
        i32.const 1
        i32.add
        local.set $p
        local.get $p
        global.get $in_len
        i32.ge_u
        if
          i32.const 5
          local.get $p
          call $pack
          return
        end
        local.get $p
        call $m125byte_at
        local.set $c
        local.get $c
        i32.const 43
        i32.eq
        local.get $c
        i32.const 45
        i32.eq
        i32.or
        if
          local.get $p
          i32.const 1
          i32.add
          local.set $p
          local.get $p
          global.get $in_len
          i32.ge_u
          if
            i32.const 5
            local.get $p
            call $pack
            return
          end
        end
        local.get $p
        call $m125byte_at
        call $is_digit
        i32.eqz
        if
          i32.const 3
          local.get $p
          call $pack
          return
        end
        (block $exp_done
          (loop $exp
            local.get $p
            global.get $in_len
            i32.ge_u
            br_if $exp_done
            local.get $p
            call $m125byte_at
            call $is_digit
            i32.eqz
            br_if $exp_done
            local.get $p
            i32.const 1
            i32.add
            local.set $p
            br $exp))
      end
    end
    local.get $p
    global.get $in_len
    i32.lt_u
    if
      local.get $p
      call $m125byte_at
      call $is_delim
      i32.eqz
      if
        i32.const 3
        local.get $p
        call $pack
        return
      end
    end
    i32.const 4096
    local.get $class
    i32.store
    i32.const 0
    local.get $p
    call $pack)

  (func $emit_token (param $kind i32) (param $start i32) (param $end i32) (param $parent i32) (param $flags i32) (result i64)
    (local $idx i32)
    (local $addr i32)
    global.get $tok_len
    global.get $tok_cap
    i32.ge_u
    if
      i32.const 2
      global.get $tok_len
      call $pack
      return
    end
    global.get $tok_len
    local.set $idx
    global.get $tok_ptr
    local.get $idx
    i32.const 20
    i32.mul
    i32.add
    local.set $addr
    local.get $addr
    local.get $kind
    i32.store
    local.get $addr
    i32.const 4
    i32.add
    local.get $start
    i32.store
    local.get $addr
    i32.const 8
    i32.add
    local.get $end
    i32.store
    local.get $addr
    i32.const 12
    i32.add
    local.get $parent
    i32.store
    local.get $addr
    i32.const 16
    i32.add
    local.get $flags
    i32.store
    local.get $idx
    i32.const 1
    i32.add
    global.set $tok_len
    i32.const 0
    local.get $idx
    call $pack)

  (func $set_token_end (param $idx i32) (param $end i32)
    global.get $tok_ptr
    local.get $idx
    i32.const 20
    i32.mul
    i32.add
    i32.const 8
    i32.add
    local.get $end
    i32.store)

  (func $m125match_lit (param $off i32) (param $a i32) (param $b i32) (param $c i32) (param $d i32) (param $len i32) (result i32)
    local.get $off
    local.get $len
    i32.add
    global.get $in_len
    i32.gt_u
    if
      i32.const 0
      return
    end
    local.get $off
    call $m125byte_at
    local.get $a
    i32.ne
    if
      i32.const 0
      return
    end
    local.get $len
    i32.const 1
    i32.gt_u
    if
      local.get $off
      i32.const 1
      i32.add
      call $m125byte_at
      local.get $b
      i32.ne
      if
        i32.const 0
        return
      end
    end
    local.get $len
    i32.const 2
    i32.gt_u
    if
      local.get $off
      i32.const 2
      i32.add
      call $m125byte_at
      local.get $c
      i32.ne
      if
        i32.const 0
        return
      end
    end
    local.get $len
    i32.const 3
    i32.gt_u
    if
      local.get $off
      i32.const 3
      i32.add
      call $m125byte_at
      local.get $d
      i32.ne
      if
        i32.const 0
        return
      end
    end
    i32.const 1)

  (func $parse_value (param $off i32) (param $parent i32) (param $depth i32) (result i64)
    (local $p i32)
    (local $c i32)
    (local $packed i64)
    (local $status i32)
    (local $end i32)
    local.get $depth
    i32.const 64
    i32.gt_u
    if
      i32.const 6
      local.get $off
      call $pack
      return
    end
    local.get $off
    call $m125skip_ws
    local.set $p
    local.get $p
    global.get $in_len
    i32.ge_u
    if
      i32.const 1
      local.get $p
      call $pack
      return
    end
    local.get $p
    call $m125byte_at
    local.set $c
    local.get $c
    i32.const 123
    i32.eq
    if
      local.get $p
      local.get $parent
      local.get $depth
      call $m125parse_object
      return
    end
    local.get $c
    i32.const 91
    i32.eq
    if
      local.get $p
      local.get $parent
      local.get $depth
      call $parse_array
      return
    end
    local.get $c
    i32.const 34
    i32.eq
    if
      global.get $in_ptr
      global.get $in_len
      local.get $p
      call $json_scan_string
      local.set $packed
      local.get $packed
      call $lo
      local.set $status
      local.get $packed
      call $hi
      local.set $end
      local.get $status
      i32.const 0
      i32.ne
      if
        local.get $packed
        return
      end
      i32.const 4
      local.get $p
      local.get $end
      local.get $parent
      i32.const 0
      call $emit_token
      local.set $packed
      local.get $packed
      call $lo
      if
        local.get $packed
        return
      end
      i32.const 0
      local.get $end
      call $pack
      return
    end
    local.get $c
    i32.const 45
    i32.eq
    local.get $c
    call $is_digit
    i32.or
    if
      global.get $in_ptr
      global.get $in_len
      local.get $p
      call $json_scan_number
      local.set $packed
      local.get $packed
      call $lo
      local.set $status
      local.get $packed
      call $hi
      local.set $end
      local.get $status
      i32.const 0
      i32.ne
      if
        local.get $packed
        return
      end
      i32.const 5
      local.get $p
      local.get $end
      local.get $parent
      i32.const 4096
      i32.load
      call $emit_token
      local.set $packed
      local.get $packed
      call $lo
      if
        local.get $packed
        return
      end
      i32.const 0
      local.get $end
      call $pack
      return
    end
    local.get $p
    i32.const 116
    i32.const 114
    i32.const 117
    i32.const 101
    i32.const 4
    call $m125match_lit
    if
      i32.const 6
      local.get $p
      local.get $p
      i32.const 4
      i32.add
      local.get $parent
      i32.const 1
      call $emit_token
      local.set $packed
      local.get $packed
      call $lo
      if
        local.get $packed
        return
      end
      i32.const 0
      local.get $p
      i32.const 4
      i32.add
      call $pack
      return
    end
    local.get $p
    i32.const 102
    i32.const 97
    i32.const 108
    i32.const 115
    i32.const 4
    call $m125match_lit
    if
      local.get $p
      i32.const 5
      i32.add
      global.get $in_len
      i32.le_u
      if
        local.get $p
        i32.const 4
        i32.add
        call $m125byte_at
        i32.const 101
        i32.eq
        if
          i32.const 6
          local.get $p
          local.get $p
          i32.const 5
          i32.add
          local.get $parent
          i32.const 0
          call $emit_token
          local.set $packed
          local.get $packed
          call $lo
          if
            local.get $packed
            return
          end
          i32.const 0
          local.get $p
          i32.const 5
          i32.add
          call $pack
          return
        end
      end
    end
    local.get $p
    i32.const 110
    i32.const 117
    i32.const 108
    i32.const 108
    i32.const 4
    call $m125match_lit
    if
      i32.const 7
      local.get $p
      local.get $p
      i32.const 4
      i32.add
      local.get $parent
      i32.const 0
      call $emit_token
      local.set $packed
      local.get $packed
      call $lo
      if
        local.get $packed
        return
      end
      i32.const 0
      local.get $p
      i32.const 4
      i32.add
      call $pack
      return
    end
    i32.const 3
    local.get $p
    call $pack)

  (func $m125parse_object (param $off i32) (param $parent i32) (param $depth i32) (result i64)
    (local $idx i32)
    (local $p i32)
    (local $end i32)
    (local $packed i64)
    (local $key_idx i32)
    (local $status i32)
    i32.const 1
    local.get $off
    i32.const 0
    local.get $parent
    i32.const 0
    call $emit_token
    local.set $packed
    local.get $packed
    call $lo
    if
      local.get $packed
      return
    end
    local.get $packed
    call $hi
    local.set $idx
    local.get $off
    i32.const 1
    i32.add
    call $m125skip_ws
    local.set $p
    local.get $p
    global.get $in_len
    i32.ge_u
    if
      i32.const 5
      local.get $p
      call $pack
      return
    end
    local.get $p
    call $m125byte_at
    i32.const 125
    i32.eq
    if
      local.get $idx
      local.get $p
      i32.const 1
      i32.add
      call $set_token_end
      i32.const 0
      local.get $p
      i32.const 1
      i32.add
      call $pack
      return
    end
    (block $done
      (loop $members
        local.get $p
        global.get $in_len
        i32.ge_u
        if
          i32.const 5
          local.get $p
          call $pack
          return
        end
        local.get $p
        call $m125byte_at
        i32.const 34
        i32.ne
        if
          i32.const 3
          local.get $p
          call $pack
          return
        end
        global.get $in_ptr
        global.get $in_len
        local.get $p
        call $json_scan_string
        local.set $packed
        local.get $packed
        call $lo
        local.set $status
        local.get $packed
        call $hi
        local.set $end
        local.get $status
        i32.const 0
        i32.ne
        if
          local.get $packed
          return
        end
        i32.const 3
        local.get $p
        local.get $end
        local.get $idx
        i32.const 0
        call $emit_token
        local.set $packed
        local.get $packed
        call $lo
        if
          local.get $packed
          return
        end
        local.get $packed
        call $hi
        local.set $key_idx
        local.get $end
        call $m125skip_ws
        local.set $p
        local.get $p
        global.get $in_len
        i32.ge_u
        if
          i32.const 5
          local.get $p
          call $pack
          return
        end
        local.get $p
        call $m125byte_at
        i32.const 58
        i32.ne
        if
          i32.const 3
          local.get $p
          call $pack
          return
        end
        local.get $p
        i32.const 1
        i32.add
        local.get $key_idx
        local.get $depth
        i32.const 1
        i32.add
        call $parse_value
        local.set $packed
        local.get $packed
        call $lo
        if
          local.get $packed
          return
        end
        local.get $packed
        call $hi
        call $m125skip_ws
        local.set $p
        local.get $p
        global.get $in_len
        i32.ge_u
        if
          i32.const 5
          local.get $p
          call $pack
          return
        end
        local.get $p
        call $m125byte_at
        i32.const 44
        i32.eq
        if
          local.get $p
          i32.const 1
          i32.add
          call $m125skip_ws
          local.set $p
          br $members
        end
        local.get $p
        call $m125byte_at
        i32.const 125
        i32.eq
        if
          local.get $idx
          local.get $p
          i32.const 1
          i32.add
          call $set_token_end
          i32.const 0
          local.get $p
          i32.const 1
          i32.add
          call $pack
          return
        end
        i32.const 3
        local.get $p
        call $pack
        return))
    i32.const 3
    local.get $p
    call $pack)

  (func $parse_array (param $off i32) (param $parent i32) (param $depth i32) (result i64)
    (local $idx i32)
    (local $p i32)
    (local $packed i64)
    i32.const 2
    local.get $off
    i32.const 0
    local.get $parent
    i32.const 0
    call $emit_token
    local.set $packed
    local.get $packed
    call $lo
    if
      local.get $packed
      return
    end
    local.get $packed
    call $hi
    local.set $idx
    local.get $off
    i32.const 1
    i32.add
    call $m125skip_ws
    local.set $p
    local.get $p
    global.get $in_len
    i32.ge_u
    if
      i32.const 5
      local.get $p
      call $pack
      return
    end
    local.get $p
    call $m125byte_at
    i32.const 93
    i32.eq
    if
      local.get $idx
      local.get $p
      i32.const 1
      i32.add
      call $set_token_end
      i32.const 0
      local.get $p
      i32.const 1
      i32.add
      call $pack
      return
    end
    (block $done
      (loop $items
        local.get $p
        local.get $idx
        local.get $depth
        i32.const 1
        i32.add
        call $parse_value
        local.set $packed
        local.get $packed
        call $lo
        if
          local.get $packed
          return
        end
        local.get $packed
        call $hi
        call $m125skip_ws
        local.set $p
        local.get $p
        global.get $in_len
        i32.ge_u
        if
          i32.const 5
          local.get $p
          call $pack
          return
        end
        local.get $p
        call $m125byte_at
        i32.const 44
        i32.eq
        if
          local.get $p
          i32.const 1
          i32.add
          call $m125skip_ws
          local.set $p
          br $items
        end
        local.get $p
        call $m125byte_at
        i32.const 93
        i32.eq
        if
          local.get $idx
          local.get $p
          i32.const 1
          i32.add
          call $set_token_end
          i32.const 0
          local.get $p
          i32.const 1
          i32.add
          call $pack
          return
        end
        i32.const 3
        local.get $p
        call $pack
        return))
    i32.const 3
    local.get $p
    call $pack)

  (func (export "json_parse_tape") (param $input_ptr i32) (param $input_len i32) (param $token_ptr i32) (param $token_cap i32) (param $scratch_ptr i32) (param $scratch_len i32) (result i64)
    (local $packed i64)
    (local $end i32)
    local.get $input_ptr
    global.set $in_ptr
    local.get $input_len
    global.set $in_len
    local.get $token_ptr
    global.set $tok_ptr
    local.get $token_cap
    global.set $tok_cap
    i32.const 0
    global.set $tok_len
    i32.const 0
    i32.const -1
    i32.const 0
    call $parse_value
    local.set $packed
    local.get $packed
    call $lo
    if
      local.get $packed
      return
    end
    local.get $packed
    call $hi
    call $m125skip_ws
    local.set $end
    local.get $end
    global.get $in_len
    i32.ne
    if
      i32.const 3
      global.get $tok_len
      call $pack
      return
    end
    i32.const 0
    global.get $tok_len
    call $pack)

)