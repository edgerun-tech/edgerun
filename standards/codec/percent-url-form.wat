
  (func $m147hex_nibble (param $c i32) (result i32)
    local.get $c
    i32.const 48
    i32.ge_u
    local.get $c
    i32.const 57
    i32.le_u
    i32.and
    if (result i32)
      local.get $c
      i32.const 48
      i32.sub
    else
      local.get $c
      i32.const 65
      i32.ge_u
      local.get $c
      i32.const 70
      i32.le_u
      i32.and
      if (result i32)
        local.get $c
        i32.const 55
        i32.sub
      else
        local.get $c
        i32.const 97
        i32.ge_u
        local.get $c
        i32.const 102
        i32.le_u
        i32.and
        if (result i32)
          local.get $c
          i32.const 87
          i32.sub
        else
          i32.const -1
        end
      end
    end)

  (func $m147hex_upper (param $n i32) (result i32)
    local.get $n
    i32.const 10
    i32.lt_u
    if (result i32)
      local.get $n
      i32.const 48
      i32.add
    else
      local.get $n
      i32.const 55
      i32.add
    end)

  (func $is_unreserved (param $b i32) (result i32)
    local.get $b
    i32.const 65
    i32.ge_u
    local.get $b
    i32.const 90
    i32.le_u
    i32.and
    local.get $b
    i32.const 97
    i32.ge_u
    local.get $b
    i32.const 122
    i32.le_u
    i32.and
    i32.or
    local.get $b
    i32.const 48
    i32.ge_u
    local.get $b
    i32.const 57
    i32.le_u
    i32.and
    i32.or
    local.get $b
    i32.const 45
    i32.eq
    i32.or
    local.get $b
    i32.const 46
    i32.eq
    i32.or
    local.get $b
    i32.const 95
    i32.eq
    i32.or
    local.get $b
    i32.const 126
    i32.eq
    i32.or)

  (func $preserve_byte (param $b i32) (param $mode i32) (result i32)
    local.get $b
    call $is_unreserved
    if (result i32)
      i32.const 1
    else
      local.get $mode
      i32.const 1
      i32.eq
      local.get $mode
      i32.const 2
      i32.eq
      i32.or
      local.get $b
      i32.const 47
      i32.eq
      i32.and
      if (result i32)
        i32.const 1
      else
        local.get $mode
        i32.const 2
        i32.eq
        if (result i32)
          local.get $b
          i32.const 63
          i32.eq
          local.get $b
          i32.const 38
          i32.eq
          i32.or
          local.get $b
          i32.const 61
          i32.eq
          i32.or
        else
          i32.const 0
        end
      end
    end)

  (func $preserve_baggage_byte (param $b i32) (result i32)
    local.get $b
    i32.const 128
    i32.ge_u
    if (result i32)
      i32.const 0
    else
      local.get $b
      i32.const 32
      i32.lt_u
      local.get $b
      i32.const 127
      i32.eq
      i32.or
      local.get $b
      i32.const 32
      i32.eq
      i32.or
      local.get $b
      i32.const 34
      i32.eq
      i32.or
      local.get $b
      i32.const 44
      i32.eq
      i32.or
      local.get $b
      i32.const 59
      i32.eq
      i32.or
      local.get $b
      i32.const 61
      i32.eq
      i32.or
      i32.eqz
    end)

  (func $validate_percent_span (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $c i32)
    loop $loop
      local.get $i
      local.get $len
      i32.lt_u
      if
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        local.tee $c
        i32.const 37
        i32.eq
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
          call $m147hex_nibble
          i32.const 0
          i32.lt_s
          if
            i32.const 0
            return
          end
          local.get $ptr
          local.get $i
          i32.add
          i32.const 2
          i32.add
          i32.load8_u
          call $m147hex_nibble
          i32.const 0
          i32.lt_s
          if
            i32.const 0
            return
          end
          local.get $i
          i32.const 3
          i32.add
          local.set $i
        else
          local.get $i
          i32.const 1
          i32.add
          local.set $i
        end
        br $loop
      end
    end
    i32.const 1)

  (func (export "percent_decode_strict")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $i i32)
    (local $o i32)
    (local $b i32)
    (local $hi i32)
    (local $lo i32)
    loop $loop
      local.get $i
      local.get $in_len
      i32.lt_u
      if
        local.get $o
        local.get $out_cap
        i32.ge_u
        if
          i32.const 2
          local.get $o
          call $pack
          return
        end
        local.get $in_ptr
        local.get $i
        i32.add
        i32.load8_u
        local.tee $b
        i32.const 37
        i32.eq
        if
          local.get $i
          i32.const 2
          i32.add
          local.get $in_len
          i32.ge_u
          if
            i32.const 3
            local.get $o
            call $pack
            return
          end
          local.get $in_ptr
          local.get $i
          i32.add
          i32.const 1
          i32.add
          i32.load8_u
          call $m147hex_nibble
          local.tee $hi
          i32.const 0
          i32.lt_s
          if
            i32.const 3
            local.get $o
            call $pack
            return
          end
          local.get $in_ptr
          local.get $i
          i32.add
          i32.const 2
          i32.add
          i32.load8_u
          call $m147hex_nibble
          local.tee $lo
          i32.const 0
          i32.lt_s
          if
            i32.const 3
            local.get $o
            call $pack
            return
          end
          local.get $out_ptr
          local.get $o
          i32.add
          local.get $hi
          i32.const 4
          i32.shl
          local.get $lo
          i32.or
          i32.store8
          local.get $i
          i32.const 3
          i32.add
          local.set $i
        else
          local.get $out_ptr
          local.get $o
          i32.add
          local.get $b
          i32.store8
          local.get $i
          i32.const 1
          i32.add
          local.set $i
        end
        local.get $o
        i32.const 1
        i32.add
        local.set $o
        br $loop
      end
    end
    i32.const 0
    local.get $o
    call $pack)

  (func (export "percent_encode_component")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32) (param $mode i32)
    (result i64)
    (local $i i32)
    (local $o i32)
    (local $b i32)
    loop $loop
      local.get $i
      local.get $in_len
      i32.lt_u
      if
        local.get $in_ptr
        local.get $i
        i32.add
        i32.load8_u
        local.tee $b
        local.get $mode
        call $preserve_byte
        if
          local.get $o
          local.get $out_cap
          i32.ge_u
          if
            i32.const 2
            local.get $o
            call $pack
            return
          end
          local.get $out_ptr
          local.get $o
          i32.add
          local.get $b
          i32.store8
          local.get $o
          i32.const 1
          i32.add
          local.set $o
        else
          local.get $o
          i32.const 3
          i32.add
          local.get $out_cap
          i32.gt_u
          if
            i32.const 2
            local.get $o
            call $pack
            return
          end
          local.get $out_ptr
          local.get $o
          i32.add
          i32.const 37
          i32.store8
          local.get $out_ptr
          local.get $o
          i32.add
          i32.const 1
          i32.add
          local.get $b
          i32.const 4
          i32.shr_u
          call $m147hex_upper
          i32.store8
          local.get $out_ptr
          local.get $o
          i32.add
          i32.const 2
          i32.add
          local.get $b
          i32.const 15
          i32.and
          call $m147hex_upper
          i32.store8
          local.get $o
          i32.const 3
          i32.add
          local.set $o
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    i32.const 0
    local.get $o
    call $pack)

  (func (export "percent_encode_baggage")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $i i32)
    (local $o i32)
    (local $b i32)
    loop $loop
      local.get $i
      local.get $in_len
      i32.lt_u
      if
        local.get $in_ptr
        local.get $i
        i32.add
        i32.load8_u
        local.tee $b
        call $preserve_baggage_byte
        if
          local.get $o
          local.get $out_cap
          i32.ge_u
          if
            i32.const 2
            local.get $o
            call $pack
            return
          end
          local.get $out_ptr
          local.get $o
          i32.add
          local.get $b
          i32.store8
          local.get $o
          i32.const 1
          i32.add
          local.set $o
        else
          local.get $o
          i32.const 3
          i32.add
          local.get $out_cap
          i32.gt_u
          if
            i32.const 2
            local.get $o
            call $pack
            return
          end
          local.get $out_ptr
          local.get $o
          i32.add
          i32.const 37
          i32.store8
          local.get $out_ptr
          local.get $o
          i32.add
          i32.const 1
          i32.add
          local.get $b
          i32.const 4
          i32.shr_u
          call $m147hex_upper
          i32.store8
          local.get $out_ptr
          local.get $o
          i32.add
          i32.const 2
          i32.add
          local.get $b
          i32.const 15
          i32.and
          call $m147hex_upper
          i32.store8
          local.get $o
          i32.const 3
          i32.add
          local.set $o
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    i32.const 0
    local.get $o
    call $pack)

  ;; Writes key_start, key_len, value_start, value_len, next_offset.
  (func (export "form_urlencoded_next_pair")
    (param $ptr i32) (param $len i32) (param $start i32) (param $out_ptr i32)
    (result i32)
    (local $i i32)
    (local $pair_start i32)
    (local $pair_end i32)
    (local $eq i32)
    local.get $start
    local.get $len
    i32.gt_u
    if
      i32.const 3
      return
    end
    local.get $start
    local.set $i
    block $found_nonempty
      loop $skip
        local.get $i
        local.get $len
        i32.ge_u
        if
          i32.const 1
          return
        end
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.const 38
        i32.ne
        if
          br $found_nonempty
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $skip
      end
    end
    local.get $i
    local.set $pair_start
    local.get $len
    local.set $pair_end
    local.get $len
    local.set $eq
    loop $scan
      local.get $i
      local.get $len
      i32.lt_u
      if
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.const 38
        i32.eq
        if
          local.get $i
          local.set $pair_end
          local.get $len
          local.set $i
        else
          local.get $eq
          local.get $len
          i32.eq
          local.get $ptr
          local.get $i
          i32.add
          i32.load8_u
          i32.const 61
          i32.eq
          i32.and
          if
            local.get $i
            local.set $eq
          end
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $scan
        end
      end
    end
    local.get $ptr
    local.get $pair_start
    i32.add
    local.get $pair_end
    local.get $pair_start
    i32.sub
    call $validate_percent_span
    i32.eqz
    if
      i32.const 3
      return
    end
    local.get $out_ptr
    local.get $pair_start
    i32.store
    local.get $out_ptr
    i32.const 4
    i32.add
    local.get $eq
    local.get $len
    i32.eq
    if (result i32)
      local.get $pair_end
      local.get $pair_start
      i32.sub
    else
      local.get $eq
      local.get $pair_start
      i32.sub
    end
    i32.store
    local.get $out_ptr
    i32.const 8
    i32.add
    local.get $eq
    local.get $len
    i32.eq
    if (result i32)
      local.get $pair_end
    else
      local.get $eq
      i32.const 1
      i32.add
    end
    i32.store
    local.get $out_ptr
    i32.const 12
    i32.add
    local.get $eq
    local.get $len
    i32.eq
    if (result i32)
      i32.const 0
    else
      local.get $pair_end
      local.get $eq
      i32.const 1
      i32.add
      i32.sub
    end
    i32.store
    local.get $out_ptr
    i32.const 16
    i32.add
    local.get $pair_end
    local.get $len
    i32.lt_u
    if (result i32)
      local.get $pair_end
      i32.const 1
      i32.add
    else
      local.get $pair_end
    end
    i32.store
    i32.const 0)

  ;; Writes path_start, path_len, query_start, query_len, fragment_start, fragment_len.
  ;; Absent query/fragment start is 0xffffffff.
  (func (export "uri_scan_path_query")
    (param $ptr i32) (param $len i32) (param $out_ptr i32)
    (result i32)
    (local $i i32)
    (local $path_end i32)
    (local $query_start i32)
    (local $query_end i32)
    (local $frag_start i32)
    (local $b i32)
    local.get $len
    local.set $path_end
    i32.const -1
    local.set $query_start
    local.get $len
    local.set $query_end
    i32.const -1
    local.set $frag_start
    loop $scan_path
      local.get $i
      local.get $len
      i32.lt_u
      if
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        local.tee $b
        i32.const 63
        i32.eq
        if
          local.get $i
          local.set $path_end
          local.get $i
          i32.const 1
          i32.add
          local.set $query_start
          local.get $i
          i32.const 1
          i32.add
          local.set $i
        end
        local.get $query_start
        i32.const -1
        i32.ne
        if
          local.get $len
          local.set $i
        else
          local.get $b
          i32.const 35
          i32.eq
          if
            local.get $i
            local.set $path_end
            local.get $i
            i32.const 1
            i32.add
            local.set $frag_start
            local.get $len
            local.set $i
          else
            local.get $i
            i32.const 1
            i32.add
            local.set $i
          end
        end
        br $scan_path
      end
    end
    local.get $query_start
    i32.const -1
    i32.ne
    if
      local.get $query_start
      local.set $i
      local.get $len
      local.set $query_end
      loop $scan_query
        local.get $i
        local.get $len
        i32.lt_u
        if
          local.get $ptr
          local.get $i
          i32.add
          i32.load8_u
          i32.const 35
          i32.eq
          if
            local.get $i
            local.set $query_end
            local.get $i
            i32.const 1
            i32.add
            local.set $frag_start
            local.get $len
            local.set $i
          else
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $scan_query
          end
        end
      end
    end
    local.get $out_ptr
    i32.const 0
    i32.store
    local.get $out_ptr
    i32.const 4
    i32.add
    local.get $path_end
    i32.store
    local.get $out_ptr
    i32.const 8
    i32.add
    local.get $query_start
    i32.store
    local.get $out_ptr
    i32.const 12
    i32.add
    local.get $query_start
    i32.const -1
    i32.eq
    if (result i32)
      i32.const 0
    else
      local.get $query_end
      local.get $query_start
      i32.sub
    end
    i32.store
    local.get $out_ptr
    i32.const 16
    i32.add
    local.get $frag_start
    i32.store
    local.get $out_ptr
    i32.const 20
    i32.add
    local.get $frag_start
    i32.const -1
    i32.eq
    if (result i32)
      i32.const 0
    else
      local.get $len
      local.get $frag_start
      i32.sub
    end
    i32.store
    i32.const 0)
