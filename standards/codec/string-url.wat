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


  ;; Jagex string escape/unescape.
  ;; Escape: < -> <lt>, > -> <gt>, \n -> <br>
  ;; Unescape: <lt> -> <, <gt> -> >, <br> -> \n
  ;;
  ;; Exports:
  ;;   escape_text(in_ptr, in_len, out_ptr) -> out_len
  ;;   unescape_text(in_ptr, in_len, out_ptr) -> out_len
  (func (export "escape_text") (param $in i32) (param $len i32) (param $out i32) (result i32)
    (local $i i32) (local $o i32) (local $c i32)
    i32.const 0 local.set $i
    i32.const 0 local.set $o
    block $done
    loop $loop
      local.get $i local.get $len i32.ge_u br_if $done
      local.get $in local.get $i i32.add i32.load8_u local.set $c
      local.get $c i32.const 60 i32.eq  ;; '<'
      if
        local.get $out local.get $o i32.add i32.const 60 i32.store8  ;; '<'
        local.get $out local.get $o i32.const 1 i32.add i32.add i32.const 108 i32.store8  ;; 'l'
        local.get $out local.get $o i32.const 2 i32.add i32.add i32.const 116 i32.store8  ;; 't'
        local.get $out local.get $o i32.const 3 i32.add i32.add i32.const 62 i32.store8  ;; '>'
        local.get $o i32.const 4 i32.add local.set $o
      else
        local.get $c i32.const 62 i32.eq  ;; '>'
        if
          local.get $out local.get $o i32.add i32.const 60 i32.store8
          local.get $out local.get $o i32.const 1 i32.add i32.add i32.const 103 i32.store8  ;; 'g'
          local.get $out local.get $o i32.const 2 i32.add i32.add i32.const 116 i32.store8  ;; 't'
          local.get $out local.get $o i32.const 3 i32.add i32.add i32.const 62 i32.store8  ;; '>'
          local.get $o i32.const 4 i32.add local.set $o
        else
          local.get $c i32.const 10 i32.eq  ;; '\n'
          if
            local.get $out local.get $o i32.add i32.const 60 i32.store8
            local.get $out local.get $o i32.const 1 i32.add i32.add i32.const 98 i32.store8  ;; 'b'
            local.get $out local.get $o i32.const 2 i32.add i32.add i32.const 114 i32.store8  ;; 'r'
            local.get $out local.get $o i32.const 3 i32.add i32.add i32.const 62 i32.store8  ;; '>'
            local.get $o i32.const 4 i32.add local.set $o
          else
            local.get $c i32.const 13 i32.ne  ;; skip '\r'
            if
              local.get $out local.get $o i32.add local.get $c i32.store8
              local.get $o i32.const 1 i32.add local.set $o
            end
          end
        end
      end
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end
    local.get $o
  )
  (func (export "unescape_text") (param $in i32) (param $len i32) (param $out i32) (result i32)
    (local $i i32) (local $o i32) (local $c i32)
    i32.const 0 local.set $i
    i32.const 0 local.set $o
    block $done
    loop $loop
      local.get $i local.get $len i32.ge_u br_if $done
      local.get $in local.get $i i32.add i32.load8_u local.set $c
      local.get $c i32.const 60 i32.eq  ;; '<'
      if
        local.get $i i32.const 4 i32.add local.get $len i32.le_u
        if
          local.get $in local.get $i i32.add i32.load
          local.tee $c
          i32.const 0x3E746C3C i32.eq  ;; "<lt>"
          if
            local.get $out local.get $o i32.add i32.const 60 i32.store8
            local.get $o i32.const 1 i32.add local.set $o
            local.get $i i32.const 4 i32.add local.set $i
            br $loop
          end
          local.get $c i32.const 0x3E74673C i32.eq  ;; "<gt>"
          if
            local.get $out local.get $o i32.add i32.const 62 i32.store8
            local.get $o i32.const 1 i32.add local.set $o
            local.get $i i32.const 4 i32.add local.set $i
            br $loop
          end
          local.get $c i32.const 0x3E72623C i32.eq  ;; "<br>"
          if
            local.get $out local.get $o i32.add i32.const 10 i32.store8
            local.get $o i32.const 1 i32.add local.set $o
            local.get $i i32.const 4 i32.add local.set $i
            br $loop
          end
        end
      end
      local.get $out local.get $o i32.add local.get $c i32.store8
      local.get $o i32.const 1 i32.add local.set $o
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end
    local.get $o
  )


  (func $m38write_record
    (param $out i32) (param $str_off i32) (param $str_len i32) (param $consumed i32)
    local.get $out
    local.get $str_off
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $str_len
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $consumed
    i32.store)

  ;; Return status 0 when a NUL terminator is found and status 4 when the input
  ;; ends before a terminator. Offsets are relative to ptr.
  (func (export "c_string_scan")
    (param $ptr i32) (param $len i32) (param $out i32)
    (result i32)
    (local $i i32)
    (local $end i32)
    (local $consumed i32)

    local.get $ptr
    local.get $len
    i32.add
    local.set $end
    local.get $ptr
    local.set $i

    (block $truncated
      (loop $scan
        local.get $i
        local.get $end
        i32.ge_u
        br_if $truncated

        local.get $i
        i32.load8_u
        i32.eqz
        if
          local.get $i
          local.get $ptr
          i32.sub
          i32.const 1
          i32.add
          local.set $consumed
          local.get $out
          i32.const 0
          local.get $consumed
          i32.const 1
          i32.sub
          local.get $consumed
          call $m38write_record
          i32.const 0
          return
        end

        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan))

    local.get $out
    i32.const 0
    local.get $len
    local.get $len
    call $m38write_record
    i32.const 4)

  ;; Scan the next entry in a NUL-separated multi-string. Empty entries terminate
  ;; the list with status 5 and a next offset after the empty terminator.
  (func (export "c_multi_string_next")
    (param $ptr i32) (param $len i32) (param $offset i32) (param $out i32)
    (result i64)
    (local $i i32)
    (local $start i32)
    (local $end i32)
    (local $consumed i32)
    (local $str_len i32)
    (local $next_offset i32)

    local.get $offset
    local.get $len
    i32.ge_u
    if
      local.get $out
      local.get $len
      i32.const 0
      i32.const 0
      call $m38write_record
      i32.const 5
      local.get $len
      call $pack
      return
    end

    local.get $ptr
    local.get $offset
    i32.add
    local.set $start
    local.get $start
    local.set $i
    local.get $ptr
    local.get $len
    i32.add
    local.set $end

    (block $truncated
      (loop $scan
        local.get $i
        local.get $end
        i32.ge_u
        br_if $truncated

        local.get $i
        i32.load8_u
        i32.eqz
        if
          local.get $i
          local.get $start
          i32.sub
          local.set $str_len
          local.get $str_len
          i32.const 1
          i32.add
          local.set $consumed
          local.get $offset
          local.get $consumed
          i32.add
          local.set $next_offset

          local.get $out
          local.get $offset
          local.get $str_len
          local.get $consumed
          call $m38write_record

          local.get $str_len
          i32.eqz
          if
            i32.const 5
            local.get $next_offset
            call $pack
            return
          end

          i32.const 0
          local.get $next_offset
          call $pack
          return
        end

        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan))

    local.get $len
    local.get $offset
    i32.sub
    local.set $str_len
    local.get $out
    local.get $offset
    local.get $str_len
    local.get $str_len
    call $m38write_record
    i32.const 4
    local.get $len
    call $pack)



  (func $m146is_upper (param $b i32) (result i32)
    local.get $b
    i32.const 65
    i32.ge_u
    local.get $b
    i32.const 90
    i32.le_u
    i32.and)

  (func $is_label_separator (param $b i32) (result i32)
    local.get $b
    i32.const 32
    i32.eq
    local.get $b
    i32.const 45
    i32.eq
    i32.or)

  (func $m146is_space (param $b i32) (result i32)
    local.get $b
    i32.const 32
    i32.eq
    local.get $b
    i32.const 9
    i32.eq
    i32.or
    local.get $b
    i32.const 13
    i32.eq
    i32.or
    local.get $b
    i32.const 10
    i32.eq
    i32.or)

  (func $is_b64 (param $b i32) (result i32)
    local.get $b
    call $m146is_upper
    local.get $b
    i32.const 97
    i32.ge_u
    local.get $b
    i32.const 122
    i32.le_u
    i32.and
    i32.or
    local.get $b
    call $is_digit
    i32.or
    local.get $b
    i32.const 43
    i32.eq
    i32.or
    local.get $b
    i32.const 47
    i32.eq
    i32.or)

  (func $match_begin (param $ptr i32) (result i32)
    local.get $ptr
    i32.load8_u
    i32.const 45
    i32.eq
    local.get $ptr
    i32.const 1
    i32.add
    i32.load8_u
    i32.const 45
    i32.eq
    i32.and
    local.get $ptr
    i32.const 2
    i32.add
    i32.load8_u
    i32.const 45
    i32.eq
    i32.and
    local.get $ptr
    i32.const 3
    i32.add
    i32.load8_u
    i32.const 45
    i32.eq
    i32.and
    local.get $ptr
    i32.const 4
    i32.add
    i32.load8_u
    i32.const 45
    i32.eq
    i32.and
    local.get $ptr
    i32.const 5
    i32.add
    i32.load8_u
    i32.const 66
    i32.eq
    i32.and
    local.get $ptr
    i32.const 6
    i32.add
    i32.load8_u
    i32.const 69
    i32.eq
    i32.and
    local.get $ptr
    i32.const 7
    i32.add
    i32.load8_u
    i32.const 71
    i32.eq
    i32.and
    local.get $ptr
    i32.const 8
    i32.add
    i32.load8_u
    i32.const 73
    i32.eq
    i32.and
    local.get $ptr
    i32.const 9
    i32.add
    i32.load8_u
    i32.const 78
    i32.eq
    i32.and
    local.get $ptr
    i32.const 10
    i32.add
    i32.load8_u
    i32.const 32
    i32.eq
    i32.and)

  (func $match_end_prefix (param $ptr i32) (result i32)
    local.get $ptr
    i32.load8_u
    i32.const 45
    i32.eq
    local.get $ptr
    i32.const 1
    i32.add
    i32.load8_u
    i32.const 45
    i32.eq
    i32.and
    local.get $ptr
    i32.const 2
    i32.add
    i32.load8_u
    i32.const 45
    i32.eq
    i32.and
    local.get $ptr
    i32.const 3
    i32.add
    i32.load8_u
    i32.const 45
    i32.eq
    i32.and
    local.get $ptr
    i32.const 4
    i32.add
    i32.load8_u
    i32.const 45
    i32.eq
    i32.and
    local.get $ptr
    i32.const 5
    i32.add
    i32.load8_u
    i32.const 69
    i32.eq
    i32.and
    local.get $ptr
    i32.const 6
    i32.add
    i32.load8_u
    i32.const 78
    i32.eq
    i32.and
    local.get $ptr
    i32.const 7
    i32.add
    i32.load8_u
    i32.const 68
    i32.eq
    i32.and
    local.get $ptr
    i32.const 8
    i32.add
    i32.load8_u
    i32.const 32
    i32.eq
    i32.and)

  (func $has_five_hyphens (param $ptr i32) (result i32)
    local.get $ptr
    i32.load8_u
    i32.const 45
    i32.eq
    local.get $ptr
    i32.const 1
    i32.add
    i32.load8_u
    i32.const 45
    i32.eq
    i32.and
    local.get $ptr
    i32.const 2
    i32.add
    i32.load8_u
    i32.const 45
    i32.eq
    i32.and
    local.get $ptr
    i32.const 3
    i32.add
    i32.load8_u
    i32.const 45
    i32.eq
    i32.and
    local.get $ptr
    i32.const 4
    i32.add
    i32.load8_u
    i32.const 45
    i32.eq
    i32.and)

  (func $labels_equal
    (param $ptr i32) (param $a i32) (param $b i32) (param $len i32)
    (result i32)
    (local $i i32)
    block $bad
      loop $loop
        local.get $i
        local.get $len
        i32.ge_u
        if
          i32.const 1
          return
        end
        local.get $ptr
        local.get $a
        i32.add
        local.get $i
        i32.add
        i32.load8_u
        local.get $ptr
        local.get $b
        i32.add
        local.get $i
        i32.add
        i32.load8_u
        i32.ne
        br_if $bad
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    i32.const 0)

  (func $pem_validate_label (export "pem_validate_label") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $b i32)
    (local $prev_sep i32)
    local.get $len
    i32.eqz
    if
      i32.const 3
      return
    end
    i32.const 1
    local.set $prev_sep
    loop $loop
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
        call $m146is_upper
        local.get $b
        call $is_digit
        i32.or
        if
          i32.const 0
          local.set $prev_sep
        else
          local.get $b
          call $is_label_separator
          if
            local.get $prev_sep
            if
              i32.const 3
              return
            end
            i32.const 1
            local.set $prev_sep
          else
            i32.const 3
            return
          end
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    local.get $prev_sep
    if
      i32.const 3
      return
    end
    i32.const 0)

  (func (export "pem_find_boundaries")
    (param $ptr i32) (param $len i32) (param $out_ptr i32)
    (result i32)
    (local $i i32)
    (local $begin i32)
    (local $label_start i32)
    (local $label_len i32)
    (local $begin_end i32)
    (local $body_start i32)
    (local $end i32)
    (local $end_label_start i32)
    (local $end_len i32)
    local.get $len
    i32.const 16
    i32.lt_u
    if
      i32.const 1
      return
    end
    i32.const -1
    local.set $begin
    block $found_begin
      loop $find_begin
        local.get $i
        i32.const 11
        i32.add
        local.get $len
        i32.le_u
        if
          local.get $ptr
          local.get $i
          i32.add
          i32.load8_u
          i32.eqz
          if
            i32.const 3
            return
          end
          local.get $ptr
          local.get $i
          i32.add
          call $match_begin
          if
            local.get $i
            local.set $begin
            local.get $i
            i32.const 11
            i32.add
            local.set $label_start
            local.get $label_start
            local.set $i
            br $found_begin
          end
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $find_begin
        end
      end
    end
    local.get $begin
    i32.const -1
    i32.eq
    if
      i32.const 1
      return
    end
    loop $find_begin_end
      local.get $i
      i32.const 5
      i32.add
      local.get $len
      i32.le_u
      if
        local.get $ptr
        local.get $i
        i32.add
        call $has_five_hyphens
        if
          local.get $i
          local.set $begin_end
        else
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $find_begin_end
        end
      else
        i32.const 5
        return
      end
    end
    local.get $begin_end
    local.get $label_start
    i32.sub
    local.set $label_len
    local.get $ptr
    local.get $label_start
    i32.add
    local.get $label_len
    call $pem_validate_label
    if
      i32.const 3
      return
    end
    local.get $begin_end
    i32.const 5
    i32.add
    local.set $body_start
    local.get $body_start
    local.set $i
    i32.const -1
    local.set $end
    loop $find_end
      local.get $i
      i32.const 9
      i32.add
      local.get $len
      i32.le_u
      if
        local.get $ptr
        local.get $i
        i32.add
        call $match_end_prefix
        if
          local.get $i
          i32.const 9
          i32.add
          local.set $end_label_start
          local.get $end_label_start
          local.get $label_len
          i32.add
          i32.const 5
          i32.add
          local.get $len
          i32.le_u
          if
            local.get $ptr
            local.get $label_start
            local.get $end_label_start
            local.get $label_len
            call $labels_equal
            local.get $ptr
            local.get $end_label_start
            local.get $label_len
            i32.add
            i32.add
            call $has_five_hyphens
            i32.and
            if
              local.get $i
              local.set $end
            else
              local.get $i
              i32.const 1
              i32.add
              local.set $i
              br $find_end
            end
          else
            i32.const 5
            return
          end
        else
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $find_end
        end
      else
        i32.const 5
        return
      end
    end
    local.get $end
    i32.const -1
    i32.eq
    if
      i32.const 5
      return
    end
    local.get $end_label_start
    local.get $label_len
    i32.add
    i32.const 5
    i32.add
    local.get $end
    i32.sub
    local.set $end_len
    local.get $out_ptr
    local.get $begin
    i32.store
    local.get $out_ptr
    i32.const 4
    i32.add
    local.get $body_start
    local.get $begin
    i32.sub
    i32.store
    local.get $out_ptr
    i32.const 8
    i32.add
    local.get $label_start
    i32.store
    local.get $out_ptr
    i32.const 12
    i32.add
    local.get $label_len
    i32.store
    local.get $out_ptr
    i32.const 16
    i32.add
    local.get $body_start
    i32.store
    local.get $out_ptr
    i32.const 20
    i32.add
    local.get $end
    local.get $body_start
    i32.sub
    i32.store
    local.get $out_ptr
    i32.const 24
    i32.add
    local.get $end
    i32.store
    local.get $out_ptr
    i32.const 28
    i32.add
    local.get $end_len
    i32.store
    local.get $out_ptr
    i32.const 32
    i32.add
    local.get $end_label_start
    i32.store
    local.get $out_ptr
    i32.const 36
    i32.add
    local.get $label_len
    i32.store
    i32.const 0)

  (func (export "pem_compact_base64")
    (param $ptr i32) (param $len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $i i32)
    (local $o i32)
    (local $b i32)
    (local $pad i32)
    loop $loop
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
        call $m146is_space
        if
        else
          local.get $b
          i32.const 58
          i32.eq
          if
            i32.const 3
            local.get $o
            call $pack
            return
          end
          local.get $b
          i32.const 61
          i32.eq
          if
            local.get $pad
            i32.const 2
            i32.ge_u
            if
              i32.const 3
              local.get $o
              call $pack
              return
            end
            local.get $pad
            i32.const 1
            i32.add
            local.set $pad
          else
            local.get $b
            call $is_b64
            i32.eqz
            if
              i32.const 3
              local.get $o
              call $pack
              return
            end
            local.get $pad
            if
              i32.const 3
              local.get $o
              call $pack
              return
            end
          end
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
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    local.get $o
    i32.const 4
    i32.rem_u
    i32.const 1
    i32.eq
    if
      i32.const 3
      local.get $o
      call $pack
      return
    end
    local.get $pad
    if
      local.get $o
      i32.const 4
      i32.rem_u
      if
        i32.const 3
        local.get $o
        call $pack
        return
      end
    end
    i32.const 0
    local.get $o
    call $pack)

  (func (export "pem_encoded_len")
    (param $der_len i32) (param $label_len i32)
    (result i64)
    (local $base64_len i32)
    (local $lines i32)
    (local $total i32)
    local.get $der_len
    i32.const 2147483645
    i32.gt_u
    if
      i32.const 4
      i32.const 0
      call $pack
      return
    end
    local.get $der_len
    i32.const 2
    i32.add
    i32.const 3
    i32.div_u
    i32.const 4
    i32.mul
    local.set $base64_len
    local.get $base64_len
    if
      local.get $base64_len
      i32.const 63
      i32.add
      i32.const 64
      i32.div_u
      local.set $lines
    end
    i32.const 11
    local.get $label_len
    i32.add
    i32.const 5
    i32.add
    i32.const 1
    i32.add
    local.get $base64_len
    i32.add
    local.get $lines
    i32.add
    i32.const 9
    i32.add
    local.get $label_len
    i32.add
    i32.const 5
    i32.add
    i32.const 1
    i32.add
    local.set $total
    local.get $total
    local.get $base64_len
    i32.lt_u
    if
      i32.const 4
      i32.const 0
      call $pack
      return
    end
    i32.const 0
    local.get $total
    call $pack)
