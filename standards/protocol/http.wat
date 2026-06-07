;; ═════════════════════════════════════════════════════════════════════
  ;; HTTP Core Primitives — shared across all HTTP parsers
  ;; ═════════════════════════════════════════════════════════════════════

;; ── Character classification ──────────────────────────────────────

  ;; RFC 7230 tchar: ALPHA / DIGIT / "!" / "#" / "$" / "%" / "&" / "'" /
  ;; "*" / "+" / "-" / "." / "^" / "_" / "`" / "|" / "~"
  (func $is_tchar (export "is_tchar") (param $b i32) (result i32)
    (i32.or
      (i32.or
        (i32.or
          (i32.and (i32.ge_u (local.get $b) (i32.const 65)) (i32.le_u (local.get $b) (i32.const 90)))
          (i32.and (i32.ge_u (local.get $b) (i32.const 97)) (i32.le_u (local.get $b) (i32.const 122))))
        (i32.and (i32.ge_u (local.get $b) (i32.const 48)) (i32.le_u (local.get $b) (i32.const 57))))
      (i32.or
        (i32.or
          (i32.or
            (i32.or (i32.eq (local.get $b) (i32.const 33)) (i32.eq (local.get $b) (i32.const 35)))
            (i32.or (i32.eq (local.get $b) (i32.const 36)) (i32.eq (local.get $b) (i32.const 37))))
          (i32.or
            (i32.or (i32.eq (local.get $b) (i32.const 38)) (i32.eq (local.get $b) (i32.const 39)))
            (i32.or (i32.eq (local.get $b) (i32.const 42)) (i32.eq (local.get $b) (i32.const 43)))))
        (i32.or
          (i32.or
            (i32.or (i32.eq (local.get $b) (i32.const 45)) (i32.eq (local.get $b) (i32.const 46)))
            (i32.or (i32.eq (local.get $b) (i32.const 94)) (i32.eq (local.get $b) (i32.const 95))))
          (i32.or
            (i32.or (i32.eq (local.get $b) (i32.const 96)) (i32.eq (local.get $b) (i32.const 124)))
            (i32.eq (local.get $b) (i32.const 126)))))))

  ;; HTTP whitespace: space (0x20) or horizontal tab (0x09).
  ;; Narrower than general is_ws (which also accepts CR, LF, etc.).
  (func $is_space (export "is_space") (param $b i32) (result i32)
    (i32.or (i32.eq (local.get $b) (i32.const 32)) (i32.eq (local.get $b) (i32.const 9))))

  ;; Valid header field value byte: tab (0x09) or visible ASCII [32,126].
  (func $is_header_value_byte (export "is_header_value_byte") (param $b i32) (result i32)
    (i32.or
      (i32.eq (local.get $b) (i32.const 9))
      (i32.and (i32.ge_u (local.get $b) (i32.const 32)) (i32.le_u (local.get $b) (i32.const 126)))))

  ;; ── Case-insensitive comparison ───────────────────────────────────

  ;; Compare two ASCII byte sequences case-insensitively.
  ;; Returns 1 if equal, 0 otherwise.
  (func $ascii_eq_ci (export "ascii_eq_ci") (param $aptr i32) (param $alen i32) (param $bptr i32) (param $blen i32) (result i32)
    (local $i i32)
    (if (i32.ne (local.get $alen) (local.get $blen)) (then (return (i32.const 0))))
    (loop $scan
      (if (i32.ge_u (local.get $i) (local.get $alen)) (then (return (i32.const 1))))
      (if (i32.ne
            (call $to_lower (i32.load8_u (i32.add (local.get $aptr) (local.get $i))))
            (call $to_lower (i32.load8_u (i32.add (local.get $bptr) (local.get $i)))))
        (then (return (i32.const 0))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $scan))
    i32.const 1)


  (func $m111parse_u64_decimal (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $start i32)
    (local $end i32)
    (local $digits i32)
    (local $i i32)
    (local $acc i64)
    (local $b i32)
    loop $trim_start
      local.get $start
      local.get $len
      i32.lt_u
      if
        local.get $ptr
        local.get $start
        i32.add
        i32.load8_u
        call $is_space
        if
          local.get $start
          i32.const 1
          i32.add
          local.set $start
          br $trim_start
        end
      end
    end
    local.get $len
    local.set $end
    loop $trim_end
      local.get $end
      local.get $start
      i32.gt_u
      if
        local.get $ptr
        local.get $end
        i32.const 1
        i32.sub
        i32.add
        i32.load8_u
        call $is_space
        if
          local.get $end
          i32.const 1
          i32.sub
          local.set $end
          br $trim_end
        end
      end
    end
    local.get $end
    local.get $start
    i32.le_u
    if
      i32.const 3
      return
    end
    local.get $end
    local.get $start
    i32.sub
    local.tee $digits
    i32.const 20
    i32.gt_u
    if
      i32.const 4
      return
    end
    local.get $digits
    i32.const 20
    i32.eq
    if
      i32.const 0
      local.set $i
      block $max_done
        loop $max_cmp
          local.get $i
          i32.const 20
          i32.ge_u
          if
            br $max_done
          end
          local.get $ptr
          local.get $start
          local.get $i
          i32.add
          i32.add
          i32.load8_u
          local.tee $b
          call $is_digit
          i32.eqz
          if
            i32.const 3
            return
          end
          local.get $b
          i32.const 65500
          local.get $i
          i32.add
          i32.load8_u
          i32.gt_u
          if
            i32.const 4
            return
          end
          local.get $b
          i32.const 65500
          local.get $i
          i32.add
          i32.load8_u
          i32.lt_u
          if
            br $max_done
          end
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $max_cmp
        end
      end
    end
    local.get $start
    local.set $i
    loop $digits
      local.get $i
      local.get $end
      i32.ge_u
      if
        local.get $out
        local.get $acc
        i64.store
        i32.const 0
        return
      end
      local.get $ptr
      local.get $i
      i32.add
      i32.load8_u
      local.tee $b
      call $is_digit
      i32.eqz
      if
        i32.const 3
        return
      end
      local.get $acc
      i64.const 10
      i64.mul
      local.get $b
      i32.const 48
      i32.sub
      i64.extend_i32_u
      i64.add
      local.set $acc
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      br $digits
    end
    i32.const 0)

  (func (export "http_parse_content_length") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    local.get $ptr
    local.get $len
    local.get $out
    call $m111parse_u64_decimal)

  (func $http_has_transfer_token (export "http_has_transfer_token") (param $vptr i32) (param $vlen i32) (param $tptr i32) (param $tlen i32) (result i32)
    (local $i i32)
    (local $start i32)
    (local $end i32)
    (local $b i32)
    local.get $tlen
    i32.eqz
    if
      i32.const 3
      return
    end
    i32.const 0
    local.set $i
    block $validate_done
      loop $validate
        local.get $i
        local.get $tlen
        i32.ge_u
        if
          br $validate_done
        end
        local.get $tptr
        local.get $i
        i32.add
        i32.load8_u
        call $is_tchar
        i32.eqz
        if
          i32.const 3
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $validate
      end
    end
    i32.const 0
    local.set $i
    loop $segments
      local.get $i
      local.get $vlen
      i32.ge_u
      if
        i32.const 3
        return
      end
      loop $trim_start
        local.get $i
        local.get $vlen
        i32.lt_u
        if
          local.get $vptr
          local.get $i
          i32.add
          i32.load8_u
          call $is_space
          if
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $trim_start
          end
        end
      end
      local.get $i
      local.set $start
      loop $find_comma
        local.get $i
        local.get $vlen
        i32.lt_u
        if
          local.get $vptr
          local.get $i
          i32.add
          i32.load8_u
          local.tee $b
          i32.const 44
          i32.ne
          if
            local.get $b
            call $is_header_value_byte
            i32.eqz
            if
              i32.const 3
              return
            end
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $find_comma
          end
        end
      end
      local.get $i
      local.set $end
      loop $trim_end
        local.get $end
        local.get $start
        i32.gt_u
        if
          local.get $vptr
          local.get $end
          i32.const 1
          i32.sub
          i32.add
          i32.load8_u
          call $is_space
          if
            local.get $end
            i32.const 1
            i32.sub
            local.set $end
            br $trim_end
          end
        end
      end
      local.get $vptr
      local.get $start
      i32.add
      local.get $end
      local.get $start
      i32.sub
      local.get $tptr
      local.get $tlen
      call $ascii_eq_ci
      if
        i32.const 0
        return
      end
      local.get $i
      local.get $vlen
      i32.ge_u
      if
        i32.const 3
        return
      end
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      br $segments
    end
    i32.const 3)

  (func $m111find_crlf (param $ptr i32) (param $len i32) (param $start i32) (result i32)
    (local $i i32)
    local.get $start
    local.get $len
    i32.gt_u
    if
      i32.const -1
      return
    end
    local.get $start
    local.set $i
    block $not_found
      loop $scan
        local.get $i
        i32.const 1
        i32.add
        local.get $len
        i32.ge_u
        br_if $not_found
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.const 13
        i32.eq
        local.get $ptr
        local.get $i
        i32.add
        i32.const 1
        i32.add
        i32.load8_u
        i32.const 10
        i32.eq
        i32.and
        if
          local.get $i
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan
      end
    end
    i32.const -1)

  (func $name_is (param $ptr i32) (param $len i32) (param $lit i32) (param $lit_len i32) (result i32)
    local.get $ptr
    local.get $len
    local.get $lit
    local.get $lit_len
    call $ascii_eq_ci)


  ;; out: kind, content_length_low, content_length_high, header_count
  ;; kind: 0 none, 1 content-length, 2 transfer-encoding chunked
  (func (export "http_classify_body_framing") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $pos i32)
    (local $line_end i32)
    (local $i i32)
    (local $colon i32)
    (local $value_start i32)
    (local $value_end i32)
    (local $kind i32)
    (local $count i32)
    (local $tmp i32)
    (local $have_cl i32)
    (local $cl i64)
    (local $parsed i64)
    (local $status i32)
    block $headers_done
      loop $headers
        local.get $pos
        local.get $len
        i32.ge_u
        if
          br $headers_done
        end
        local.get $ptr
        local.get $len
        local.get $pos
        call $m111find_crlf
        local.tee $line_end
        i32.const 0
        i32.lt_s
        if
          i32.const 1
          return
        end
        local.get $line_end
        local.get $pos
        i32.eq
        if
          local.get $line_end
          i32.const 2
          i32.add
          local.set $pos
          br $headers_done
        end
      local.get $pos
      local.set $i
      i32.const -1
      local.set $colon
      loop $name
        local.get $i
        local.get $line_end
        i32.ge_u
        if
          i32.const 3
          return
        end
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.const 58
        i32.eq
        if
          local.get $i
          local.set $colon
        else
          local.get $ptr
          local.get $i
          i32.add
          i32.load8_u
          call $is_tchar
          i32.eqz
          if
            i32.const 3
            return
          end
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $name
        end
      end
      local.get $colon
      local.get $pos
      i32.le_s
      if
        i32.const 3
        return
      end
      local.get $colon
      i32.const 1
      i32.add
      local.set $value_start
      loop $trim_start
        local.get $value_start
        local.get $line_end
        i32.lt_u
        if
          local.get $ptr
          local.get $value_start
          i32.add
          i32.load8_u
          call $is_space
          if
            local.get $value_start
            i32.const 1
            i32.add
            local.set $value_start
            br $trim_start
          end
        end
      end
      local.get $line_end
      local.set $value_end
      loop $trim_end
        local.get $value_end
        local.get $value_start
        i32.gt_u
        if
          local.get $ptr
          local.get $value_end
          i32.const 1
          i32.sub
          i32.add
          i32.load8_u
          call $is_space
          if
            local.get $value_end
            i32.const 1
            i32.sub
            local.set $value_end
            br $trim_end
          end
        end
      end
        local.get $value_start
        local.set $i
        block $value_done
          loop $value
            local.get $i
            local.get $value_end
            i32.ge_u
            if
              br $value_done
            end
            local.get $ptr
            local.get $i
            i32.add
            i32.load8_u
            call $is_header_value_byte
            i32.eqz
            if
              i32.const 3
              return
            end
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $value
          end
        end
      local.get $ptr
      local.get $pos
      i32.add
      local.get $colon
      local.get $pos
      i32.sub
      i32.const 65440
      i32.const 14
      call $name_is
      if
        local.get $ptr
        local.get $value_start
        i32.add
        local.get $value_end
        local.get $value_start
        i32.sub
        local.get $out
        i32.const 32
        i32.add
        call $m111parse_u64_decimal
        local.tee $status
        i32.const 0
        i32.ne
        if
          local.get $status
          return
        end
        local.get $out
        i32.const 32
        i32.add
        i64.load
        local.set $parsed
        local.get $have_cl
        if
          local.get $parsed
          local.get $cl
          i64.ne
          if
            i32.const 3
            return
          end
        else
          i32.const 1
          local.set $have_cl
          local.get $parsed
          local.set $cl
        end
      end
      local.get $ptr
      local.get $pos
      i32.add
      local.get $colon
      local.get $pos
      i32.sub
      i32.const 65456
      i32.const 17
      call $name_is
      if
        local.get $ptr
        local.get $value_start
        i32.add
        local.get $value_end
        local.get $value_start
        i32.sub
        i32.const 65480
        i32.const 7
        call $http_has_transfer_token
        i32.const 0
        i32.eq
        if
          i32.const 2
          local.set $kind
        end
      end
      local.get $count
      i32.const 1
      i32.add
      local.set $count
      local.get $line_end
      i32.const 2
      i32.add
      local.set $pos
      br $headers
    end
    end
    local.get $kind
    i32.eqz
    if
      local.get $have_cl
      if
        i32.const 1
        local.set $kind
      end
    end
    local.get $out
    local.get $kind
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $cl
    i32.wrap_i64
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $cl
    i64.const 32
    i64.shr_u
    i32.wrap_i64
    i32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $count
    i32.store
    i32.const 0)

(func $m112hex_value (param $b i32) (result i32)
    local.get $b
    i32.const 48
    i32.ge_u
    local.get $b
    i32.const 57
    i32.le_u
    i32.and
    if (result i32)
      local.get $b
      i32.const 48
      i32.sub
    else
      local.get $b
      i32.const 65
      i32.ge_u
      local.get $b
      i32.const 70
      i32.le_u
      i32.and
      if (result i32)
        local.get $b
        i32.const 55
        i32.sub
      else
        local.get $b
        i32.const 97
        i32.ge_u
        local.get $b
        i32.const 102
        i32.le_u
        i32.and
        if (result i32)
          local.get $b
          i32.const 87
          i32.sub
        else
          i32.const -1
        end
      end
    end)

  (func $is_line_byte (param $b i32) (result i32)
    local.get $b
    i32.const 9
    i32.eq
    local.get $b
    i32.const 32
    i32.ge_u
    local.get $b
    i32.const 126
    i32.le_u
    i32.and
    i32.or)

  (func $find_double_crlf (param $ptr i32) (param $len i32) (param $start i32) (result i32)
    (local $i i32)
    local.get $start
    local.set $i
    block $not_found
      loop $scan
        local.get $i
        i32.const 3
        i32.add
        local.get $len
        i32.ge_u
        br_if $not_found
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.const 13
        i32.eq
        local.get $ptr
        local.get $i
        i32.add
        i32.const 1
        i32.add
        i32.load8_u
        i32.const 10
        i32.eq
        i32.and
        local.get $ptr
        local.get $i
        i32.add
        i32.const 2
        i32.add
        i32.load8_u
        i32.const 13
        i32.eq
        i32.and
        local.get $ptr
        local.get $i
        i32.add
        i32.const 3
        i32.add
        i32.load8_u
        i32.const 10
        i32.eq
        i32.and
        if
          local.get $i
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan
      end
    end
    i32.const -1)

  (func $write_chunk_record
    (param $out i32)
    (param $size i32)
    (param $size_line_off i32)
    (param $size_line_len i32)
    (param $data_off i32)
    (param $data_len i32)
    (param $next_off i32)
    (param $is_final i32)
    (param $flags i32)
    local.get $out
    local.get $size
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $size_line_off
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $size_line_len
    i32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $data_off
    i32.store
    local.get $out
    i32.const 16
    i32.add
    local.get $data_len
    i32.store
    local.get $out
    i32.const 20
    i32.add
    local.get $next_off
    i32.store
    local.get $out
    i32.const 24
    i32.add
    local.get $is_final
    i32.store
    local.get $out
    i32.const 28
    i32.add
    local.get $flags
    i32.store)

  (func $parse_next (param $ptr i32) (param $len i32) (param $start i32) (param $out i32) (result i32)
    (local $i i32)
    (local $size i32)
    (local $digit i32)
    (local $digits i32)
    (local $data_off i32)
    (local $line_end i32)
    (local $next i32)
    (local $flags i32)
    (local $trail_end i32)
    local.get $start
    local.get $len
    i32.ge_u
    if
      i32.const 1
      return
    end
    local.get $start
    local.set $i
    block $hex_done
      loop $hex
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
        local.tee $digit
        i32.const 59
        i32.eq
        if
          local.get $flags
          i32.const 1
          i32.or
          local.set $flags
          br $hex_done
        end
        local.get $digit
        i32.const 13
        i32.eq
        if
          br $hex_done
        end
        local.get $digit
        call $m112hex_value
        local.tee $digit
        i32.const 0
        i32.lt_s
        if
          i32.const 3
          return
        end
        local.get $size
        i32.const 0x0fffffff
        i32.gt_u
        if
          i32.const 4
          return
        end
        local.get $size
        i32.const 4
        i32.shl
        local.get $digit
        i32.or
        local.set $size
        local.get $digits
        i32.const 1
        i32.add
        local.set $digits
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $hex
      end
    end
    local.get $digits
    i32.eqz
    if
      i32.const 3
      return
    end
    block $line_done
      loop $line
        local.get $i
        i32.const 1
        i32.add
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
        local.tee $digit
        i32.const 13
        i32.eq
        local.get $ptr
        local.get $i
        i32.add
        i32.const 1
        i32.add
        i32.load8_u
        i32.const 10
        i32.eq
        i32.and
        if
          local.get $i
          local.set $line_end
          local.get $i
          i32.const 2
          i32.add
          local.set $data_off
          br $line_done
        end
        local.get $digit
        call $is_line_byte
        i32.eqz
        if
          i32.const 3
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $line
      end
    end
    local.get $size
    i32.eqz
    if
      local.get $data_off
      i32.const 1
      i32.add
      local.get $len
      i32.ge_u
      if
        i32.const 1
        return
      end
      local.get $ptr
      local.get $data_off
      i32.add
      i32.load8_u
      i32.const 13
      i32.eq
      local.get $ptr
      local.get $data_off
      i32.add
      i32.const 1
      i32.add
      i32.load8_u
      i32.const 10
      i32.eq
      i32.and
      if
        local.get $data_off
        i32.const 2
        i32.add
        local.set $next
      else
        local.get $ptr
        local.get $len
        local.get $data_off
        call $find_double_crlf
        local.tee $trail_end
        i32.const 0
        i32.lt_s
        if
          i32.const 1
          return
        end
        local.get $trail_end
        i32.const 4
        i32.add
        local.set $next
        local.get $flags
        i32.const 2
        i32.or
        local.set $flags
      end
      local.get $out
      local.get $size
      local.get $start
      local.get $line_end
      local.get $start
      i32.sub
      local.get $data_off
      i32.const 0
      local.get $next
      i32.const 1
      local.get $flags
      call $write_chunk_record
      i32.const 0
      return
    end
    local.get $data_off
    local.get $size
    i32.add
    local.tee $next
    local.get $data_off
    i32.lt_u
    if
      i32.const 4
      return
    end
    local.get $next
    i32.const 1
    i32.add
    local.get $len
    i32.ge_u
    if
      i32.const 1
      return
    end
    local.get $ptr
    local.get $next
    i32.add
    i32.load8_u
    i32.const 13
    i32.eq
    local.get $ptr
    local.get $next
    i32.add
    i32.const 1
    i32.add
    i32.load8_u
    i32.const 10
    i32.eq
    i32.and
    i32.eqz
    if
      i32.const 3
      return
    end
    local.get $next
    i32.const 2
    i32.add
    local.set $next
    local.get $out
    local.get $size
    local.get $start
    local.get $line_end
    local.get $start
    i32.sub
    local.get $data_off
    local.get $size
    local.get $next
    i32.const 0
    local.get $flags
    call $write_chunk_record
    i32.const 0)

  (func (export "http1_chunk_next") (param $ptr i32) (param $len i32) (param $start i32) (param $out i32) (result i32)
    local.get $ptr
    local.get $len
    local.get $start
    local.get $out
    call $parse_next)

  (func (export "http1_chunk_scan_body") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $offset i32)
    (local $rec i32)
    (local $count i32)
    (local $decoded i64)
    (local $size i32)
    (local $next i32)
    (local $flags i32)
    (local $final_flags i32)
    local.get $out
    i32.const 32
    i32.add
    local.set $rec
    loop $chunks
      local.get $ptr
      local.get $len
      local.get $offset
      local.get $rec
      call $parse_next
      local.tee $flags
      i32.eqz
      i32.eqz
      if
        local.get $flags
        return
      end
      local.get $rec
      i32.load
      local.set $size
      local.get $rec
      i32.const 20
      i32.add
      i32.load
      local.set $next
      local.get $rec
      i32.const 24
      i32.add
      i32.load
      if
        local.get $rec
        i32.const 28
        i32.add
        i32.load
        local.set $final_flags
        local.get $out
        local.get $decoded
        i64.const 0xffffffff
        i64.and
        i32.wrap_i64
        i32.store
        local.get $out
        i32.const 4
        i32.add
        local.get $decoded
        i64.const 32
        i64.shr_u
        i32.wrap_i64
        i32.store
        local.get $out
        i32.const 8
        i32.add
        local.get $count
        i32.store
        local.get $out
        i32.const 12
        i32.add
        local.get $rec
        i32.const 12
        i32.add
        i32.load
        i32.store
        local.get $out
        i32.const 16
        i32.add
        local.get $final_flags
        i32.const 2
        i32.and
        if (result i32)
          local.get $next
          local.get $rec
          i32.const 12
          i32.add
          i32.load
          i32.sub
          i32.const 4
          i32.sub
        else
          i32.const 0
        end
        i32.store
        local.get $out
        i32.const 20
        i32.add
        local.get $next
        i32.store
        local.get $out
        i32.const 24
        i32.add
        local.get $final_flags
        i32.store
        i32.const 0
        return
      end
      local.get $decoded
      local.get $size
      i64.extend_i32_u
      i64.add
      local.set $decoded
      local.get $count
      i32.const 1
      i32.add
      local.set $count
      local.get $next
      local.set $offset
      local.get $rec
      i32.const 32
      i32.add
      local.set $rec
      br $chunks
    end
    i32.const 1)



  (func $m113find_crlf (param $ptr i32) (param $len i32) (param $start i32) (result i32)
    (local $i i32)
    local.get $start
    local.get $len
    i32.gt_u
    if
      i32.const -1
      return
    end
    local.get $start
    local.set $i
    block $not_found
      loop $scan
        local.get $i
        i32.const 1
        i32.add
        local.get $len
        i32.ge_u
        br_if $not_found
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.const 13
        i32.eq
        local.get $ptr
        local.get $i
        i32.add
        i32.const 1
        i32.add
        i32.load8_u
        i32.const 10
        i32.eq
        i32.and
        if
          local.get $i
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan
      end
    end
    i32.const -1)

  (func $m113parse_u64_decimal (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $start i32)
    (local $end i32)
    (local $digits i32)
    (local $i i32)
    (local $acc i64)
    (local $b i32)
    loop $trim_start
      local.get $start
      local.get $len
      i32.lt_u
      if
        local.get $ptr
        local.get $start
        i32.add
        i32.load8_u
        call $is_space
        if
          local.get $start
          i32.const 1
          i32.add
          local.set $start
          br $trim_start
        end
      end
    end
    local.get $len
    local.set $end
    loop $trim_end
      local.get $end
      local.get $start
      i32.gt_u
      if
        local.get $ptr
        local.get $end
        i32.const 1
        i32.sub
        i32.add
        i32.load8_u
        call $is_space
        if
          local.get $end
          i32.const 1
          i32.sub
          local.set $end
          br $trim_end
        end
      end
    end
    local.get $end
    local.get $start
    i32.le_u
    if
      i32.const 3
      return
    end
    local.get $end
    local.get $start
    i32.sub
    local.tee $digits
    i32.const 20
    i32.gt_u
    if
      i32.const 4
      return
    end
    local.get $digits
    i32.const 20
    i32.eq
    if
      i32.const 0
      local.set $i
      block $max_done
        loop $max_cmp
          local.get $i
          i32.const 20
          i32.ge_u
          if
            br $max_done
          end
          local.get $ptr
          local.get $start
          local.get $i
          i32.add
          i32.add
          i32.load8_u
          local.tee $b
          call $is_digit
          i32.eqz
          if
            i32.const 3
            return
          end
          local.get $b
          i32.const 65380
          local.get $i
          i32.add
          i32.load8_u
          i32.gt_u
          if
            i32.const 4
            return
          end
          local.get $b
          i32.const 65380
          local.get $i
          i32.add
          i32.load8_u
          i32.lt_u
          if
            br $max_done
          end
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $max_cmp
        end
      end
    end
    local.get $start
    local.set $i
    loop $digits_loop
      local.get $i
      local.get $end
      i32.ge_u
      if
        local.get $out
        local.get $acc
        i64.store
        i32.const 0
        return
      end
      local.get $ptr
      local.get $i
      i32.add
      i32.load8_u
      local.tee $b
      call $is_digit
      i32.eqz
      if
        i32.const 3
        return
      end
      local.get $acc
      i64.const 10
      i64.mul
      local.get $b
      i32.const 48
      i32.sub
      i64.extend_i32_u
      i64.add
      local.set $acc
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      br $digits_loop
    end
    i32.const 0)

  (func $has_transfer_token (param $vptr i32) (param $vlen i32) (param $tptr i32) (param $tlen i32) (result i32)
    (local $i i32)
    (local $start i32)
    (local $end i32)
    (local $b i32)
    local.get $tlen
    i32.eqz
    if
      i32.const 3
      return
    end
    loop $segments
      local.get $i
      local.get $vlen
      i32.ge_u
      if
        i32.const 3
        return
      end
      loop $trim_start
        local.get $i
        local.get $vlen
        i32.lt_u
        if
          local.get $vptr
          local.get $i
          i32.add
          i32.load8_u
          call $is_space
          if
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $trim_start
          end
        end
      end
      local.get $i
      local.set $start
      loop $find_comma
        local.get $i
        local.get $vlen
        i32.lt_u
        if
          local.get $vptr
          local.get $i
          i32.add
          i32.load8_u
          local.tee $b
          i32.const 44
          i32.ne
          if
            local.get $b
            call $is_header_value_byte
            i32.eqz
            if
              i32.const 3
              return
            end
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $find_comma
          end
        end
      end
      local.get $i
      local.set $end
      loop $trim_end
        local.get $end
        local.get $start
        i32.gt_u
        if
          local.get $vptr
          local.get $end
          i32.const 1
          i32.sub
          i32.add
          i32.load8_u
          call $is_space
          if
            local.get $end
            i32.const 1
            i32.sub
            local.set $end
            br $trim_end
          end
        end
      end
      local.get $vptr
      local.get $start
      i32.add
      local.get $end
      local.get $start
      i32.sub
      local.get $tptr
      local.get $tlen
      call $ascii_eq_ci
      if
        i32.const 0
        return
      end
      local.get $i
      local.get $vlen
      i32.ge_u
      if
        i32.const 3
        return
      end
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      br $segments
    end
    i32.const 3)

  ;; out:
  ;; 0 header_count
  ;; 4 bytes_consumed, including the terminating empty CRLF
  ;; 8 host_count
  ;; 12 content_length_count
  ;; 16 transfer_encoding_count
  ;; 20 flags: bit0 content-length, bit1 transfer-encoding, bit2 chunked, bit3 duplicate same CL
  ;; 24 content_length_low
  ;; 28 content_length_high
  (func (export "http1_validate_header_block") (param $ptr i32) (param $len i32) (param $max_headers i32) (param $max_bytes i32) (param $out i32) (result i32)
    (local $pos i32)
    (local $line_end i32)
    (local $i i32)
    (local $colon i32)
    (local $value_start i32)
    (local $value_end i32)
    (local $count i32)
    (local $host_count i32)
    (local $cl_count i32)
    (local $te_count i32)
    (local $flags i32)
    (local $cl i64)
    (local $parsed i64)
    (local $status i32)
    (local $b i32)
    loop $headers
      local.get $pos
      local.get $len
      i32.ge_u
      if
        i32.const 1
        return
      end
      local.get $pos
      local.get $max_bytes
      i32.gt_u
      if
        i32.const 6
        return
      end
      local.get $ptr
      local.get $len
      local.get $pos
      call $m113find_crlf
      local.tee $line_end
      i32.const 0
      i32.lt_s
      if
        i32.const 1
        return
      end
      local.get $line_end
      i32.const 2
      i32.add
      local.get $max_bytes
      i32.gt_u
      if
        i32.const 6
        return
      end
      local.get $line_end
      local.get $pos
      i32.eq
      if
        local.get $out
        local.get $count
        i32.store
        local.get $out
        i32.const 4
        i32.add
        local.get $line_end
        i32.const 2
        i32.add
        i32.store
        local.get $out
        i32.const 8
        i32.add
        local.get $host_count
        i32.store
        local.get $out
        i32.const 12
        i32.add
        local.get $cl_count
        i32.store
        local.get $out
        i32.const 16
        i32.add
        local.get $te_count
        i32.store
        local.get $out
        i32.const 20
        i32.add
        local.get $flags
        i32.store
        local.get $out
        i32.const 24
        i32.add
        local.get $cl
        i32.wrap_i64
        i32.store
        local.get $out
        i32.const 28
        i32.add
        local.get $cl
        i64.const 32
        i64.shr_u
        i32.wrap_i64
        i32.store
        i32.const 0
        return
      end
      local.get $count
      local.get $max_headers
      i32.ge_u
      if
        i32.const 6
        return
      end
      local.get $ptr
      local.get $pos
      i32.add
      i32.load8_u
      local.tee $b
      call $is_space
      if
        i32.const 3
        return
      end
      local.get $pos
      local.set $i
      i32.const -1
      local.set $colon
      loop $name
        local.get $i
        local.get $line_end
        i32.ge_u
        if
          i32.const 3
          return
        end
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        local.tee $b
        i32.const 58
        i32.eq
        if
          local.get $i
          local.set $colon
        else
          local.get $b
          call $is_tchar
          i32.eqz
          if
            i32.const 3
            return
          end
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $name
        end
      end
      local.get $colon
      local.get $pos
      i32.le_s
      if
        i32.const 3
        return
      end
      local.get $colon
      i32.const 1
      i32.add
      local.set $value_start
      loop $trim_start_value
        local.get $value_start
        local.get $line_end
        i32.lt_u
        if
          local.get $ptr
          local.get $value_start
          i32.add
          i32.load8_u
          call $is_space
          if
            local.get $value_start
            i32.const 1
            i32.add
            local.set $value_start
            br $trim_start_value
          end
        end
      end
      local.get $line_end
      local.set $value_end
      loop $trim_end_value
        local.get $value_end
        local.get $value_start
        i32.gt_u
        if
          local.get $ptr
          local.get $value_end
          i32.const 1
          i32.sub
          i32.add
          i32.load8_u
          call $is_space
          if
            local.get $value_end
            i32.const 1
            i32.sub
            local.set $value_end
            br $trim_end_value
          end
        end
      end
      local.get $value_start
      local.set $i
      block $value_done
        loop $value
          local.get $i
          local.get $value_end
          i32.ge_u
          if
            br $value_done
          end
          local.get $ptr
          local.get $i
          i32.add
          i32.load8_u
          call $is_header_value_byte
          i32.eqz
          if
            i32.const 3
            return
          end
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $value
        end
      end
      local.get $ptr
      local.get $pos
      i32.add
      local.get $colon
      local.get $pos
      i32.sub
      i32.const 65456
      i32.const 4
      call $ascii_eq_ci
      if
        local.get $host_count
        i32.const 1
        i32.add
        local.tee $host_count
        i32.const 1
        i32.gt_u
        if
          i32.const 3
          return
        end
      end
      local.get $ptr
      local.get $pos
      i32.add
      local.get $colon
      local.get $pos
      i32.sub
      i32.const 65408
      i32.const 14
      call $ascii_eq_ci
      if
        local.get $cl_count
        i32.const 1
        i32.add
        local.set $cl_count
        local.get $ptr
        local.get $value_start
        i32.add
        local.get $value_end
        local.get $value_start
        i32.sub
        local.get $out
        i32.const 40
        i32.add
        call $m113parse_u64_decimal
        local.tee $status
        i32.const 0
        i32.ne
        if
          local.get $status
          return
        end
        local.get $out
        i32.const 40
        i32.add
        i64.load
        local.set $parsed
        local.get $flags
        i32.const 1
        i32.and
        if
          local.get $parsed
          local.get $cl
          i64.ne
          if
            i32.const 3
            return
          end
          local.get $flags
          i32.const 8
          i32.or
          local.set $flags
        else
          local.get $flags
          i32.const 1
          i32.or
          local.set $flags
          local.get $parsed
          local.set $cl
        end
      end
      local.get $ptr
      local.get $pos
      i32.add
      local.get $colon
      local.get $pos
      i32.sub
      i32.const 65424
      i32.const 17
      call $ascii_eq_ci
      if
        local.get $te_count
        i32.const 1
        i32.add
        local.set $te_count
        local.get $flags
        i32.const 2
        i32.or
        local.set $flags
        local.get $ptr
        local.get $value_start
        i32.add
        local.get $value_end
        local.get $value_start
        i32.sub
        i32.const 65448
        i32.const 7
        call $has_transfer_token
        i32.const 0
        i32.eq
        if
          local.get $flags
          i32.const 4
          i32.or
          local.set $flags
        end
      end
      local.get $count
      i32.const 1
      i32.add
      local.set $count
      local.get $line_end
      i32.const 2
      i32.add
      local.set $pos
      br $headers
    end
    i32.const 0)


(func $m114find_crlf (param $ptr i32) (param $len i32) (param $start i32) (result i32)
    (local $i i32)
    local.get $start
    local.get $len
    i32.gt_u
    if
      i32.const -1
      return
    end
    local.get $start
    local.set $i
    block $not_found
      loop $scan
        local.get $i
        i32.const 1
        i32.add
        local.get $len
        i32.ge_u
        br_if $not_found
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.const 13
        i32.eq
        local.get $ptr
        local.get $i
        i32.add
        i32.const 1
        i32.add
        i32.load8_u
        i32.const 10
        i32.eq
        i32.and
        if
          local.get $i
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan
      end
    end
    i32.const -1)

  (func $starts_http (param $ptr i32) (param $off i32) (param $end i32) (result i32)
    local.get $off
    i32.const 8
    i32.add
    local.get $end
    i32.ne
    if
      i32.const 0
      return
    end
    local.get $ptr
    local.get $off
    i32.add
    i32.load8_u
    i32.const 72
    i32.eq
    local.get $ptr
    local.get $off
    i32.add
    i32.const 1
    i32.add
    i32.load8_u
    i32.const 84
    i32.eq
    i32.and
    local.get $ptr
    local.get $off
    i32.add
    i32.const 2
    i32.add
    i32.load8_u
    i32.const 84
    i32.eq
    i32.and
    local.get $ptr
    local.get $off
    i32.add
    i32.const 3
    i32.add
    i32.load8_u
    i32.const 80
    i32.eq
    i32.and
    local.get $ptr
    local.get $off
    i32.add
    i32.const 4
    i32.add
    i32.load8_u
    i32.const 47
    i32.eq
    i32.and
    local.get $ptr
    local.get $off
    i32.add
    i32.const 5
    i32.add
    i32.load8_u
    call $is_digit
    i32.and
    local.get $ptr
    local.get $off
    i32.add
    i32.const 6
    i32.add
    i32.load8_u
    i32.const 46
    i32.eq
    i32.and
    local.get $ptr
    local.get $off
    i32.add
    i32.const 7
    i32.add
    i32.load8_u
    call $is_digit
    i32.and)

  ;; out: method_off, method_len, target_off, target_len, major, minor, next_off
  (func (export "http_parse_request_line") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $line_end i32)
    (local $i i32)
    (local $method_end i32)
    (local $target_start i32)
    (local $target_end i32)
    (local $b i32)
    local.get $ptr
    local.get $len
    i32.const 0
    call $m114find_crlf
    local.tee $line_end
    i32.const 0
    i32.lt_s
    if
      i32.const 1
      return
    end
    local.get $line_end
    i32.eqz
    if
      i32.const 3
      return
    end
    loop $method
      local.get $i
      local.get $line_end
      i32.ge_u
      if
        i32.const 3
        return
      end
      local.get $ptr
      local.get $i
      i32.add
      i32.load8_u
      local.tee $b
      i32.const 32
      i32.eq
      if
        local.get $i
        local.set $method_end
      else
        local.get $b
        call $is_tchar
        i32.eqz
        if
          i32.const 3
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $method
      end
    end
    local.get $method_end
    i32.eqz
    if
      i32.const 3
      return
    end
    local.get $method_end
    i32.const 1
    i32.add
    local.set $target_start
    local.get $target_start
    local.set $i
    loop $target
      local.get $i
      local.get $line_end
      i32.ge_u
      if
        i32.const 3
        return
      end
      local.get $ptr
      local.get $i
      i32.add
      i32.load8_u
      i32.const 32
      i32.eq
      if
        local.get $i
        local.set $target_end
      else
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $target
      end
    end
    local.get $target_end
    local.get $target_start
    i32.le_u
    if
      i32.const 3
      return
    end
    local.get $ptr
    local.get $target_end
    i32.const 1
    i32.add
    local.get $line_end
    call $starts_http
    i32.eqz
    if
      i32.const 3
      return
    end
    local.get $out
    i32.const 0
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $method_end
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $target_start
    i32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $target_end
    local.get $target_start
    i32.sub
    i32.store
    local.get $out
    i32.const 16
    i32.add
    local.get $ptr
    local.get $target_end
    i32.add
    i32.const 6
    i32.add
    i32.load8_u
    i32.const 48
    i32.sub
    i32.store
    local.get $out
    i32.const 20
    i32.add
    local.get $ptr
    local.get $target_end
    i32.add
    i32.const 8
    i32.add
    i32.load8_u
    i32.const 48
    i32.sub
    i32.store
    local.get $out
    i32.const 24
    i32.add
    local.get $line_end
    i32.const 2
    i32.add
    i32.store
    i32.const 0)

  ;; out: major, minor, status, reason_off, reason_len, next_off
  (func (export "http_parse_status_line") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $line_end i32)
    (local $code i32)
    local.get $ptr
    local.get $len
    i32.const 0
    call $m114find_crlf
    local.tee $line_end
    i32.const 0
    i32.lt_s
    if
      i32.const 1
      return
    end
    local.get $ptr
    i32.const 0
    i32.const 8
    call $starts_http
    i32.eqz
    if
      i32.const 3
      return
    end
    local.get $line_end
    i32.const 12
    i32.lt_u
    if
      i32.const 3
      return
    end
    local.get $ptr
    i32.const 8
    i32.add
    i32.load8_u
    i32.const 32
    i32.ne
    if
      i32.const 3
      return
    end
    local.get $ptr
    i32.const 9
    i32.add
    i32.load8_u
    call $is_digit
    local.get $ptr
    i32.const 10
    i32.add
    i32.load8_u
    call $is_digit
    i32.and
    local.get $ptr
    i32.const 11
    i32.add
    i32.load8_u
    call $is_digit
    i32.and
    i32.eqz
    if
      i32.const 3
      return
    end
    local.get $ptr
    i32.const 9
    i32.add
    i32.load8_u
    i32.const 48
    i32.sub
    i32.const 100
    i32.mul
    local.get $ptr
    i32.const 10
    i32.add
    i32.load8_u
    i32.const 48
    i32.sub
    i32.const 10
    i32.mul
    i32.add
    local.get $ptr
    i32.const 11
    i32.add
    i32.load8_u
    i32.const 48
    i32.sub
    i32.add
    local.tee $code
    i32.const 100
    i32.lt_u
    local.get $code
    i32.const 600
    i32.ge_u
    i32.or
    if
      i32.const 3
      return
    end
    local.get $line_end
    i32.const 12
    i32.gt_u
    if
      local.get $ptr
      i32.const 12
      i32.add
      i32.load8_u
      i32.const 32
      i32.ne
      if
        i32.const 3
        return
      end
    end
    local.get $out
    local.get $ptr
    i32.const 5
    i32.add
    i32.load8_u
    i32.const 48
    i32.sub
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $ptr
    i32.const 7
    i32.add
    i32.load8_u
    i32.const 48
    i32.sub
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $code
    i32.store
    local.get $out
    i32.const 12
    i32.add
    i32.const 13
    i32.store
    local.get $out
    i32.const 16
    i32.add
    local.get $line_end
    i32.const 13
    i32.gt_u
    if (result i32)
      local.get $line_end
      i32.const 13
      i32.sub
    else
      i32.const 0
    end
    i32.store
    local.get $out
    i32.const 20
    i32.add
    local.get $line_end
    i32.const 2
    i32.add
    i32.store
    i32.const 0)

  ;; out: name_off, name_len, value_off, value_len, next_off, is_end
  (func (export "http_next_header") (param $ptr i32) (param $len i32) (param $start i32) (param $out i32) (result i32)
    (local $line_end i32)
    (local $i i32)
    (local $colon i32)
    (local $value_start i32)
    (local $value_end i32)
    (local $b i32)
    local.get $start
    local.get $len
    i32.gt_u
    if
      i32.const 3
      return
    end
    local.get $ptr
    local.get $len
    local.get $start
    call $m114find_crlf
    local.tee $line_end
    i32.const 0
    i32.lt_s
    if
      i32.const 1
      return
    end
    local.get $line_end
    local.get $start
    i32.eq
    if
      local.get $out
      i32.const 0
      i32.store
      local.get $out
      i32.const 4
      i32.add
      i32.const 0
      i32.store
      local.get $out
      i32.const 8
      i32.add
      i32.const 0
      i32.store
      local.get $out
      i32.const 12
      i32.add
      i32.const 0
      i32.store
      local.get $out
      i32.const 16
      i32.add
      local.get $line_end
      i32.const 2
      i32.add
      i32.store
      local.get $out
      i32.const 20
      i32.add
      i32.const 1
      i32.store
      i32.const 0
      return
    end
    local.get $start
    local.set $i
    i32.const -1
    local.set $colon
    loop $name
      local.get $i
      local.get $line_end
      i32.ge_u
      if
        i32.const 3
        return
      end
      local.get $ptr
      local.get $i
      i32.add
      i32.load8_u
      local.tee $b
      i32.const 58
      i32.eq
      if
        local.get $i
        local.set $colon
      else
        local.get $b
        call $is_tchar
        i32.eqz
        if
          i32.const 3
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $name
      end
    end
    local.get $colon
    local.get $start
    i32.le_s
    if
      i32.const 3
      return
    end
    local.get $colon
    i32.const 1
    i32.add
    local.set $value_start
    loop $trim_start
      local.get $value_start
      local.get $line_end
      i32.lt_u
      if
        local.get $ptr
        local.get $value_start
        i32.add
        i32.load8_u
        call $is_space
        if
          local.get $value_start
          i32.const 1
          i32.add
          local.set $value_start
          br $trim_start
        end
      end
    end
    local.get $line_end
    local.set $value_end
    loop $trim_end
      local.get $value_end
      local.get $value_start
      i32.gt_u
      if
        local.get $ptr
        local.get $value_end
        i32.const 1
        i32.sub
        i32.add
        i32.load8_u
        call $is_space
        if
          local.get $value_end
          i32.const 1
          i32.sub
          local.set $value_end
          br $trim_end
        end
      end
    end
    local.get $value_start
    local.set $i
    loop $value
      local.get $i
      local.get $value_end
      i32.ge_u
      if
        local.get $out
        local.get $start
        i32.store
        local.get $out
        i32.const 4
        i32.add
        local.get $colon
        local.get $start
        i32.sub
        i32.store
        local.get $out
        i32.const 8
        i32.add
        local.get $value_start
        i32.store
        local.get $out
        i32.const 12
        i32.add
        local.get $value_end
        local.get $value_start
        i32.sub
        i32.store
        local.get $out
        i32.const 16
        i32.add
        local.get $line_end
        i32.const 2
        i32.add
        i32.store
        local.get $out
        i32.const 20
        i32.add
        i32.const 0
        i32.store
        i32.const 0
        return
      end
      local.get $ptr
      local.get $i
      i32.add
      i32.load8_u
      call $is_header_value_byte
      i32.eqz
      if
        i32.const 3
        return
      end
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      br $value
    end
    i32.const 0)


  ;; Shared HTTP primitives from http-core.wat: $is_space
  ;; NOTE: this file uses an extended $m115is_tchar that includes { (123) and } (125).
  ;; The standard $is_tchar from http-core.wat omits those — keep the extended version here.

(func $m115is_tchar (param $b i32) (result i32)
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
    i32.const 33
    i32.eq
    i32.or
    local.get $b
    i32.const 35
    i32.eq
    i32.or
    local.get $b
    i32.const 36
    i32.eq
    i32.or
    local.get $b
    i32.const 37
    i32.eq
    i32.or
    local.get $b
    i32.const 38
    i32.eq
    i32.or
    local.get $b
    i32.const 39
    i32.eq
    i32.or
    local.get $b
    i32.const 42
    i32.eq
    i32.or
    local.get $b
    i32.const 43
    i32.eq
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
    i32.const 94
    i32.eq
    i32.or
    local.get $b
    i32.const 95
    i32.eq
    i32.or
    local.get $b
    i32.const 96
    i32.eq
    i32.or
    local.get $b
    i32.const 123
    i32.eq
    i32.or
    local.get $b
    i32.const 124
    i32.eq
    i32.or
    local.get $b
    i32.const 125
    i32.eq
    i32.or
    local.get $b
    i32.const 126
    i32.eq
    i32.or)

  (func $is_ascii_graphic (param $b i32) (result i32)
    local.get $b
    i32.const 33
    i32.ge_u
    local.get $b
    i32.const 126
    i32.le_u
    i32.and)

  (func $m115hex_value (param $b i32) (result i32)
    local.get $b
    i32.const 48
    i32.ge_u
    local.get $b
    i32.const 57
    i32.le_u
    i32.and
    if (result i32)
      local.get $b
      i32.const 48
      i32.sub
    else
      local.get $b
      i32.const 65
      i32.ge_u
      local.get $b
      i32.const 70
      i32.le_u
      i32.and
      if (result i32)
        local.get $b
        i32.const 55
        i32.sub
      else
        local.get $b
        i32.const 97
        i32.ge_u
        local.get $b
        i32.const 102
        i32.le_u
        i32.and
        if (result i32)
          local.get $b
          i32.const 87
          i32.sub
        else
          i32.const -1
        end
      end
    end)

  (func (export "http_find_crlf") (param $ptr i32) (param $len i32) (param $start i32) (result i64)
    (local $i i32)
    local.get $start
    local.get $len
    i32.gt_u
    if
      i32.const 3
      local.get $len
      call $pack
      return
    end
    local.get $start
    local.set $i
    block $not_found
      loop $scan
        local.get $i
        i32.const 1
        i32.add
        local.get $len
        i32.ge_u
        br_if $not_found
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.const 13
        i32.eq
        local.get $ptr
        local.get $i
        i32.add
        i32.const 1
        i32.add
        i32.load8_u
        i32.const 10
        i32.eq
        i32.and
        if
          i32.const 0
          local.get $i
          call $pack
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan
      end
    end
    i32.const 1
    local.get $len
    call $pack)

  (func (export "http_find_double_crlf") (param $ptr i32) (param $len i32) (param $start i32) (result i64)
    (local $i i32)
    local.get $start
    local.get $len
    i32.gt_u
    if
      i32.const 3
      local.get $len
      call $pack
      return
    end
    local.get $start
    local.set $i
    block $not_found
      loop $scan
        local.get $i
        i32.const 3
        i32.add
        local.get $len
        i32.ge_u
        br_if $not_found
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.const 13
        i32.eq
        local.get $ptr
        local.get $i
        i32.add
        i32.const 1
        i32.add
        i32.load8_u
        i32.const 10
        i32.eq
        i32.and
        local.get $ptr
        local.get $i
        i32.add
        i32.const 2
        i32.add
        i32.load8_u
        i32.const 13
        i32.eq
        i32.and
        local.get $ptr
        local.get $i
        i32.add
        i32.const 3
        i32.add
        i32.load8_u
        i32.const 10
        i32.eq
        i32.and
        if
          i32.const 0
          local.get $i
          call $pack
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan
      end
    end
    i32.const 1
    local.get $len
    call $pack)

  (func (export "http_validate_header_name") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    local.get $len
    i32.eqz
    if
      i32.const 3
      return
    end
    loop $scan
      local.get $i
      local.get $len
      i32.ge_u
      if
        i32.const 0
        return
      end
      local.get $ptr
      local.get $i
      i32.add
      i32.load8_u
      call $m115is_tchar
      i32.eqz
      if
        i32.const 3
        return
      end
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      br $scan
    end
    i32.const 0)

  (func (export "http_validate_header_value") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $b i32)
    loop $scan
      local.get $i
      local.get $len
      i32.ge_u
      if
        i32.const 0
        return
      end
      local.get $ptr
      local.get $i
      i32.add
      i32.load8_u
      local.set $b
      local.get $b
      i32.const 9
      i32.eq
      local.get $b
      i32.const 32
      i32.ge_u
      i32.or
      i32.eqz
      if
        i32.const 3
        return
      end
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      br $scan
    end
    i32.const 0)

  (func (export "http_validate_header_value_simd") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $v v128)
    (local $valid v128)
    (local $b i32)
    block $scalar
      loop $simd
        local.get $i
        i32.const 16
        i32.add
        local.get $len
        i32.gt_u
        br_if $scalar
        local.get $ptr
        local.get $i
        i32.add
        v128.load
        local.tee $v
        i32.const 32
        i8x16.splat
        i8x16.ge_u
        local.get $v
        i32.const 9
        i8x16.splat
        i8x16.eq
        v128.or
        local.set $valid
        local.get $valid
        i8x16.all_true
        i32.eqz
        if
          i32.const 3
          return
        end
        local.get $i
        i32.const 16
        i32.add
        local.set $i
        br $simd
      end
    end
    loop $scan
      local.get $i
      local.get $len
      i32.ge_u
      if
        i32.const 0
        return
      end
      local.get $ptr
      local.get $i
      i32.add
      i32.load8_u
      local.set $b
      local.get $b
      i32.const 9
      i32.eq
      local.get $b
      i32.const 32
      i32.ge_u
      i32.or
      i32.eqz
      if
        i32.const 3
        return
      end
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      br $scan
    end
    i32.const 0)

  (func (export "http_lowercase_header_name") (param $ptr i32) (param $len i32) (param $out i32) (param $cap i32) (result i64)
    (local $i i32)
    (local $b i32)
    local.get $len
    i32.eqz
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end
    local.get $len
    local.get $cap
    i32.gt_u
    if
      i32.const 2
      local.get $len
      call $pack
      return
    end
    loop $scan
      local.get $i
      local.get $len
      i32.ge_u
      if
        i32.const 0
        local.get $len
        call $pack
        return
      end
      local.get $ptr
      local.get $i
      i32.add
      i32.load8_u
      local.tee $b
      call $m115is_tchar
      i32.eqz
      if
        i32.const 3
        local.get $i
        call $pack
        return
      end
      local.get $out
      local.get $i
      i32.add
      local.get $b
      call $to_lower
      i32.store8
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      br $scan
    end
    i32.const 0
    local.get $len
    call $pack)

  (func $bytes_eq_3 (param $ptr i32) (param $a i32) (param $b i32) (param $c i32) (result i32)
    local.get $ptr
    i32.load8_u
    local.get $a
    i32.eq
    local.get $ptr
    i32.const 1
    i32.add
    i32.load8_u
    local.get $b
    i32.eq
    i32.and
    local.get $ptr
    i32.const 2
    i32.add
    i32.load8_u
    local.get $c
    i32.eq
    i32.and)

  (func $bytes_eq_4 (param $ptr i32) (param $a i32) (param $b i32) (param $c i32) (param $d i32) (result i32)
    local.get $ptr
    i32.load8_u
    local.get $a
    i32.eq
    local.get $ptr
    i32.const 1
    i32.add
    i32.load8_u
    local.get $b
    i32.eq
    i32.and
    local.get $ptr
    i32.const 2
    i32.add
    i32.load8_u
    local.get $c
    i32.eq
    i32.and
    local.get $ptr
    i32.const 3
    i32.add
    i32.load8_u
    local.get $d
    i32.eq
    i32.and)

  (func $bytes_eq_5 (param $ptr i32) (param $a i32) (param $b i32) (param $c i32) (param $d i32) (param $e i32) (result i32)
    local.get $ptr
    local.get $a
    local.get $b
    local.get $c
    local.get $d
    call $bytes_eq_4
    local.get $ptr
    i32.const 4
    i32.add
    i32.load8_u
    local.get $e
    i32.eq
    i32.and)

  (func $bytes_eq_6 (param $ptr i32) (param $a i32) (param $b i32) (param $c i32) (param $d i32) (param $e i32) (param $f i32) (result i32)
    local.get $ptr
    local.get $a
    local.get $b
    local.get $c
    local.get $d
    local.get $e
    call $bytes_eq_5
    local.get $ptr
    i32.const 5
    i32.add
    i32.load8_u
    local.get $f
    i32.eq
    i32.and)

  (func $bytes_eq_7 (param $ptr i32) (param $a i32) (param $b i32) (param $c i32) (param $d i32) (param $e i32) (param $f i32) (param $g i32) (result i32)
    local.get $ptr
    local.get $a
    local.get $b
    local.get $c
    local.get $d
    local.get $e
    local.get $f
    call $bytes_eq_6
    local.get $ptr
    i32.const 6
    i32.add
    i32.load8_u
    local.get $g
    i32.eq
    i32.and)

  (func (export "http_method_classify") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    local.get $len
    i32.eqz
    if
      i32.const 0
      return
    end
    block $valid_done
      loop $validate
        local.get $i
        local.get $len
        i32.ge_u
        if
          br $valid_done
        end
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        call $is_ascii_graphic
        i32.eqz
        if
          i32.const 0
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $validate
      end
    end
    local.get $len
    i32.const 3
    i32.eq
    if
      local.get $ptr
      i32.const 71
      i32.const 69
      i32.const 84
      call $bytes_eq_3
      if
        i32.const 2
        return
      end
      local.get $ptr
      i32.const 80
      i32.const 85
      i32.const 84
      call $bytes_eq_3
      if
        i32.const 4
        return
      end
    end
    local.get $len
    i32.const 4
    i32.eq
    if
      local.get $ptr
      i32.const 80
      i32.const 79
      i32.const 83
      i32.const 84
      call $bytes_eq_4
      if
        i32.const 3
        return
      end
      local.get $ptr
      i32.const 72
      i32.const 69
      i32.const 65
      i32.const 68
      call $bytes_eq_4
      if
        i32.const 7
        return
      end
    end
    local.get $len
    i32.const 5
    i32.eq
    if
      local.get $ptr
      i32.const 80
      i32.const 65
      i32.const 84
      i32.const 67
      i32.const 72
      call $bytes_eq_5
      if
        i32.const 5
        return
      end
      local.get $ptr
      i32.const 84
      i32.const 82
      i32.const 65
      i32.const 67
      i32.const 69
      call $bytes_eq_5
      if
        i32.const 10
        return
      end
    end
    local.get $len
    i32.const 6
    i32.eq
    if
      local.get $ptr
      i32.const 68
      i32.const 69
      i32.const 76
      i32.const 69
      i32.const 84
      i32.const 69
      call $bytes_eq_6
      if
        i32.const 6
        return
      end
    end
    local.get $len
    i32.const 7
    i32.eq
    if
      local.get $ptr
      i32.const 79
      i32.const 80
      i32.const 84
      i32.const 73
      i32.const 79
      i32.const 78
      i32.const 83
      call $bytes_eq_7
      if
        i32.const 8
        return
      end
      local.get $ptr
      i32.const 67
      i32.const 79
      i32.const 78
      i32.const 78
      i32.const 69
      i32.const 67
      i32.const 84
      call $bytes_eq_7
      if
        i32.const 9
        return
      end
    end
    i32.const 1)

  (func (export "http_status_code_flags") (param $code i32) (result i32)
    local.get $code
    i32.const 100
    i32.lt_u
    local.get $code
    i32.const 999
    i32.gt_u
    i32.or
    if
      i32.const 0
      return
    end
    i32.const 1
    local.get $code
    i32.const 200
    i32.ge_u
    local.get $code
    i32.const 300
    i32.lt_u
    i32.and
    if (result i32)
      i32.const 2
    else
      i32.const 0
    end
    i32.or
    local.get $code
    i32.const 400
    i32.ge_u
    local.get $code
    i32.const 500
    i32.lt_u
    i32.and
    if (result i32)
      i32.const 4
    else
      i32.const 0
    end
    i32.or
    local.get $code
    i32.const 500
    i32.ge_u
    local.get $code
    i32.const 600
    i32.lt_u
    i32.and
    if (result i32)
      i32.const 8
    else
      i32.const 0
    end
    i32.or)

  (func $token_match (param $vptr i32) (param $tptr i32) (param $len i32) (result i32)
    (local $i i32)
    loop $scan
      local.get $i
      local.get $len
      i32.ge_u
      if
        i32.const 1
        return
      end
      local.get $vptr
      local.get $i
      i32.add
      i32.load8_u
      call $to_lower
      local.get $tptr
      local.get $i
      i32.add
      i32.load8_u
      call $to_lower
      i32.ne
      if
        i32.const 0
        return
      end
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      br $scan
    end
    i32.const 1)

  (func $validate_token (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    local.get $len
    i32.eqz
    if
      i32.const 0
      return
    end
    loop $scan
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
      call $m115is_tchar
      i32.eqz
      if
        i32.const 0
        return
      end
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      br $scan
    end
    i32.const 1)

  (func (export "http_value_has_token") (param $vptr i32) (param $vlen i32) (param $tptr i32) (param $tlen i32) (result i32)
    (local $i i32)
    (local $start i32)
    (local $end i32)
    (local $b i32)
    local.get $tptr
    local.get $tlen
    call $validate_token
    i32.eqz
    if
      i32.const 3
      return
    end
    loop $segments
      local.get $i
      local.get $vlen
      i32.ge_u
      if
        i32.const 3
        return
      end
      loop $trim_start
        local.get $i
        local.get $vlen
        i32.lt_u
        if
          local.get $vptr
          local.get $i
          i32.add
          i32.load8_u
          call $is_space
          if
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $trim_start
          end
        end
      end
      local.get $i
      local.set $start
      loop $find_comma
        local.get $i
        local.get $vlen
        i32.lt_u
        if
          local.get $vptr
          local.get $i
          i32.add
          i32.load8_u
          local.tee $b
          i32.const 44
          i32.ne
          if
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $find_comma
          end
        end
      end
      local.get $i
      local.set $end
      loop $trim_end
        local.get $end
        local.get $start
        i32.gt_u
        if
          local.get $vptr
          local.get $end
          i32.add
          i32.const 1
          i32.sub
          i32.load8_u
          call $is_space
          if
            local.get $end
            i32.const 1
            i32.sub
            local.set $end
            br $trim_end
          end
        end
      end
      local.get $end
      local.get $start
      i32.sub
      local.get $tlen
      i32.eq
      if
        local.get $vptr
        local.get $start
        i32.add
        local.get $tptr
        local.get $tlen
        call $token_match
        if
          i32.const 0
          return
        end
      end
      local.get $i
      local.get $vlen
      i32.ge_u
      if
        i32.const 3
        return
      end
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      br $segments
    end
    i32.const 3
    return)

  (func (export "http_parse_chunk_step") (param $ptr i32) (param $len i32) (param $start i32) (param $out i32) (result i32)
    (local $i i32)
    (local $size i32)
    (local $digit i32)
    (local $digits i32)
    (local $data_off i32)
    (local $next i32)
    local.get $start
    local.get $len
    i32.ge_u
    if
      i32.const 1
      return
    end
    local.get $start
    local.set $i
    block $hex_done
      loop $hex
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
        local.tee $digit
        i32.const 59
        i32.eq
        local.get $digit
        i32.const 13
        i32.eq
        i32.or
        if
          br $hex_done
        end
        local.get $digit
        call $m115hex_value
        local.tee $digit
        i32.const 0
        i32.lt_s
        if
          i32.const 3
          return
        end
        local.get $size
        i32.const 0x0fffffff
        i32.gt_u
        if
          i32.const 4
          return
        end
        local.get $size
        i32.const 4
        i32.shl
        local.get $digit
        i32.or
        local.set $size
        local.get $digits
        i32.const 1
        i32.add
        local.set $digits
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $hex
      end
    end
    local.get $digits
    i32.eqz
    if
      i32.const 3
      return
    end
    block $line_done
      loop $line
        local.get $i
        i32.const 1
        i32.add
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
        i32.const 13
        i32.eq
        local.get $ptr
        local.get $i
        i32.add
        i32.const 1
        i32.add
        i32.load8_u
        i32.const 10
        i32.eq
        i32.and
        if
          local.get $i
          i32.const 2
          i32.add
          local.set $data_off
          br $line_done
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $line
      end
    end
    local.get $size
    i32.eqz
    if
      local.get $data_off
      i32.const 1
      i32.add
      local.get $len
      i32.ge_u
      if
        i32.const 1
        return
      end
      local.get $ptr
      local.get $data_off
      i32.add
      i32.load8_u
      i32.const 13
      i32.eq
      local.get $ptr
      local.get $data_off
      i32.add
      i32.const 1
      i32.add
      i32.load8_u
      i32.const 10
      i32.eq
      i32.and
      if
        local.get $data_off
        i32.const 2
        i32.add
        local.set $next
      else
        local.get $data_off
        local.set $i
        block $trail_done
          loop $trail
            local.get $i
            i32.const 3
            i32.add
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
            i32.const 13
            i32.eq
            local.get $ptr
            local.get $i
            i32.add
            i32.const 1
            i32.add
            i32.load8_u
            i32.const 10
            i32.eq
            i32.and
            local.get $ptr
            local.get $i
            i32.add
            i32.const 2
            i32.add
            i32.load8_u
            i32.const 13
            i32.eq
            i32.and
            local.get $ptr
            local.get $i
            i32.add
            i32.const 3
            i32.add
            i32.load8_u
            i32.const 10
            i32.eq
            i32.and
            if
              local.get $i
              i32.const 4
              i32.add
              local.set $next
              br $trail_done
            end
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $trail
          end
        end
      end
      local.get $out
      local.get $size
      i32.store
      local.get $out
      i32.const 4
      i32.add
      local.get $data_off
      i32.store
      local.get $out
      i32.const 8
      i32.add
      i32.const 0
      i32.store
      local.get $out
      i32.const 12
      i32.add
      local.get $next
      i32.store
      local.get $out
      i32.const 16
      i32.add
      i32.const 1
      i32.store
      i32.const 0
      return
    end
    local.get $data_off
    local.get $size
    i32.add
    i32.const 1
    i32.add
    local.get $len
    i32.ge_u
    if
      i32.const 1
      return
    end
    local.get $ptr
    local.get $data_off
    local.get $size
    i32.add
    i32.add
    i32.load8_u
    i32.const 13
    i32.eq
    local.get $ptr
    local.get $data_off
    local.get $size
    i32.add
    i32.add
    i32.const 1
    i32.add
    i32.load8_u
    i32.const 10
    i32.eq
    i32.and
    i32.eqz
    if
      i32.const 3
      return
    end
    local.get $data_off
    local.get $size
    i32.add
    i32.const 2
    i32.add
    local.set $next
    local.get $out
    local.get $size
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $data_off
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $size
    i32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $next
    i32.store
    local.get $out
    i32.const 16
    i32.add
    i32.const 0
    i32.store
    i32.const 0)


  (func (export "simd_capabilities") (result i32)
    i32.const 1)

  (func (export "http_find_crlf_simd") (param $ptr i32) (param $len i32) (param $start i32) (result i64)
    (local $i i32)
    (local $v v128)
    local.get $start
    local.get $len
    i32.gt_u
    if
      i32.const 3
      local.get $len
      call $pack
      return
    end
    local.get $start
    local.set $i
    block $scalar
      loop $simd
        local.get $i
        i32.const 16
        i32.add
        local.get $len
        i32.gt_u
        br_if $scalar
        local.get $ptr
        local.get $i
        i32.add
        v128.load
        local.tee $v
        i32.const 0x0D
        i8x16.splat
        i8x16.eq
        local.get $v
        i32.const 0x0A
        i8x16.splat
        i8x16.eq
        v128.or
        v128.any_true
        br_if $scalar
        local.get $i
        i32.const 16
        i32.add
        local.set $i
        br $simd
      end
    end
    block $not_found
      loop $scan
        local.get $i
        local.get $len
        i32.ge_u
        br_if $not_found
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.const 13
        i32.eq
        if
          local.get $i
          i32.const 1
          i32.add
          local.get $len
          i32.lt_u
          if
            local.get $ptr
            local.get $i
            i32.add
            i32.const 1
            i32.add
            i32.load8_u
            i32.const 10
            i32.eq
            if
              i32.const 0
              local.get $i
              call $pack
              return
            end
          end
        end
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.const 10
        i32.eq
        if
          i32.const 0
          local.get $i
          call $pack
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan
      end
    end
    i32.const 1
    local.get $len
    call $pack)

  (func (export "http_find_double_crlf_simd") (param $ptr i32) (param $len i32) (param $start i32) (result i64)
    (local $i i32)
    (local $v v128)
    local.get $start
    local.get $len
    i32.gt_u
    if
      i32.const 3
      local.get $len
      call $pack
      return
    end
    local.get $start
    local.set $i
    block $scalar
      loop $simd
        local.get $i
        i32.const 16
        i32.add
        local.get $len
        i32.gt_u
        br_if $scalar
        local.get $ptr
        local.get $i
        i32.add
        v128.load
        local.tee $v
        i32.const 0x0D
        i8x16.splat
        i8x16.eq
        local.get $v
        i32.const 0x0A
        i8x16.splat
        i8x16.eq
        v128.or
        v128.any_true
        br_if $scalar
        local.get $i
        i32.const 16
        i32.add
        local.set $i
        br $simd
      end
    end
    block $not_found
      loop $scan
        local.get $i
        i32.const 3
        i32.add
        local.get $len
        i32.ge_u
        br_if $not_found
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.const 13
        i32.eq
        local.get $ptr
        local.get $i
        i32.add
        i32.const 1
        i32.add
        i32.load8_u
        i32.const 10
        i32.eq
        i32.and
        local.get $ptr
        local.get $i
        i32.add
        i32.const 2
        i32.add
        i32.load8_u
        i32.const 13
        i32.eq
        i32.and
        local.get $ptr
        local.get $i
        i32.add
        i32.const 3
        i32.add
        i32.load8_u
        i32.const 10
        i32.eq
        i32.and
        if
          i32.const 0
          local.get $i
          call $pack
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan
      end
    end
    i32.const 1
    local.get $len
    call $pack)


;; Known HTTP/2 frame types return their type byte. Unknown extension frames
  ;; return 255, matching the local Rust FrameType::Extension classifier.
  (func $http2_frame_type_classify (export "http2_frame_type_classify") (param $frame_type i32) (result i32)
    (if (result i32)
      (i32.le_u (local.get $frame_type) (i32.const 9))
      (then (local.get $frame_type))
      (else (i32.const 255))))

  ;; Decode output record, little-endian:
  ;; 0:u32 payload_length
  ;; 4:u32 frame_type_class
  ;; 8:u32 flags
  ;; 12:u32 stream_id_reserved_bit
  ;; 16:u32 stream_id_cleared
  ;; 20:u32 header_len
  ;; 24:u32 total_len
  (func (export "http2_frame_header_decode")
    (param $in_ptr i32) (param $in_len i32) (param $max_frame_size i32) (param $out_ptr i32)
    (result i32)
    (local $payload_len i32)
    (local $frame_type i32)
    (local $flags i32)
    (local $stream_raw i32)
    (local $stream_reserved i32)
    (local $stream_id i32)
    (local $total_len i32)
    (if (i32.lt_u (local.get $in_len) (i32.const 9))
      (then (return (i32.const 1))))
    (local.set $payload_len
      (i32.or
        (i32.or
          (i32.shl (i32.load8_u (local.get $in_ptr)) (i32.const 16))
          (i32.shl (i32.load8_u (i32.add (local.get $in_ptr) (i32.const 1))) (i32.const 8)))
        (i32.load8_u (i32.add (local.get $in_ptr) (i32.const 2)))))
    (if (i32.gt_u (local.get $payload_len) (local.get $max_frame_size))
      (then (return (i32.const 3))))
    (local.set $total_len (i32.add (local.get $payload_len) (i32.const 9)))
    (if (i32.lt_u (local.get $in_len) (local.get $total_len))
      (then (return (i32.const 1))))
    (local.set $frame_type (i32.load8_u (i32.add (local.get $in_ptr) (i32.const 3))))
    (local.set $flags (i32.load8_u (i32.add (local.get $in_ptr) (i32.const 4))))
    (local.set $stream_raw
      (i32.or
        (i32.or
          (i32.shl (i32.load8_u (i32.add (local.get $in_ptr) (i32.const 5))) (i32.const 24))
          (i32.shl (i32.load8_u (i32.add (local.get $in_ptr) (i32.const 6))) (i32.const 16)))
        (i32.or
          (i32.shl (i32.load8_u (i32.add (local.get $in_ptr) (i32.const 7))) (i32.const 8))
          (i32.load8_u (i32.add (local.get $in_ptr) (i32.const 8))))))
    (local.set $stream_reserved
      (i32.shr_u (local.get $stream_raw) (i32.const 31)))
    (local.set $stream_id
      (i32.and (local.get $stream_raw) (i32.const 0x7fffffff)))
    (i32.store (local.get $out_ptr) (local.get $payload_len))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4))
      (if (result i32)
        (i32.le_u (local.get $frame_type) (i32.const 9))
        (then (local.get $frame_type))
        (else (i32.const 255))))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $flags))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 12)) (local.get $stream_reserved))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 16)) (local.get $stream_id))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 20)) (i32.const 9))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 24)) (local.get $total_len))
    (i32.const 0))

  ;; Return bits: low32=status, high32=written.
  (func (export "http2_frame_header_encode")
    (param $payload_len i32) (param $frame_type i32) (param $flags i32) (param $stream_id i32)
    (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $sid i32)
    (if (i32.lt_u (local.get $out_cap) (i32.const 9))
      (then
        (return
          (i64.or
            (i64.extend_i32_u (i32.const 2))
            (i64.shl (i64.extend_i32_u (i32.const 0)) (i64.const 32))))))
    (if
      (i32.or
        (i32.or
          (i32.gt_u (local.get $payload_len) (i32.const 0x00ffffff))
          (i32.gt_u (local.get $frame_type) (i32.const 255)))
        (i32.gt_u (local.get $flags) (i32.const 255)))
      (then
        (return
          (i64.or
            (i64.extend_i32_u (i32.const 4))
            (i64.shl (i64.extend_i32_u (i32.const 0)) (i64.const 32))))))
    (local.set $sid (i32.and (local.get $stream_id) (i32.const 0x7fffffff)))
    (i32.store8 (local.get $out_ptr) (i32.shr_u (local.get $payload_len) (i32.const 16)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 1))
      (i32.shr_u (local.get $payload_len) (i32.const 8)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 2)) (local.get $payload_len))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 3)) (local.get $frame_type))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $flags))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 5)) (i32.shr_u (local.get $sid) (i32.const 24)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 6)) (i32.shr_u (local.get $sid) (i32.const 16)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 7)) (i32.shr_u (local.get $sid) (i32.const 8)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $sid))
    (i64.or
      (i64.extend_i32_u (i32.const 0))
      (i64.shl (i64.extend_i32_u (i32.const 9)) (i64.const 32))))


;; Classifications:
  ;; 0 DATA, 1 HEADERS, 2 SETTINGS, 3 CANCEL_PUSH, 4 PUSH_PROMISE,
  ;; 5 MAX_PUSH_ID, 6 GOAWAY, 7 STREAMS_BLOCKED, 8 reserved/greasing, 9 unknown.
  (func $http3_frame_type_classify (export "http3_frame_type_classify") (param $frame_type i64) (result i32)
    (if (i64.eq (local.get $frame_type) (i64.const 0))
      (then (return (i32.const 0))))
    (if (i64.eq (local.get $frame_type) (i64.const 1))
      (then (return (i32.const 1))))
    (if (i64.eq (local.get $frame_type) (i64.const 3))
      (then (return (i32.const 3))))
    (if (i64.eq (local.get $frame_type) (i64.const 4))
      (then (return (i32.const 2))))
    (if (i64.eq (local.get $frame_type) (i64.const 5))
      (then (return (i32.const 4))))
    (if (i64.eq (local.get $frame_type) (i64.const 7))
      (then (return (i32.const 5))))
    (if (i64.eq (local.get $frame_type) (i64.const 8))
      (then (return (i32.const 6))))
    (if (i64.eq (local.get $frame_type) (i64.const 9))
      (then (return (i32.const 7))))
    (if
      (i32.or
        (i64.eq (i64.rem_u (local.get $frame_type) (i64.const 31)) (i64.const 2))
        (i64.eq (i64.rem_u (local.get $frame_type) (i64.const 31)) (i64.const 6)))
      (then (return (i32.const 8))))
    (i32.const 9))

  (func $quic_varint_encode
    (param $value i64) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (if (i64.gt_u (local.get $value) (i64.const 4611686018427387903))
      (then (return (call $pack (i32.const 4) (i32.const 0)))))
    (if (i64.le_u (local.get $value) (i64.const 63))
      (then
        (if (i32.lt_u (local.get $out_cap) (i32.const 1))
          (then (return (call $pack (i32.const 2) (i32.const 0)))))
        (i32.store8 (local.get $out_ptr) (i32.wrap_i64 (local.get $value)))
        (return (call $pack (i32.const 0) (i32.const 1)))))
    (if (i64.le_u (local.get $value) (i64.const 16383))
      (then
        (if (i32.lt_u (local.get $out_cap) (i32.const 2))
          (then (return (call $pack (i32.const 2) (i32.const 0)))))
        (i32.store8
          (local.get $out_ptr)
          (i32.or
            (i32.const 64)
            (i32.and
              (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 8)))
              (i32.const 63))))
        (i32.store8
          (i32.add (local.get $out_ptr) (i32.const 1))
          (i32.wrap_i64 (local.get $value)))
        (return (call $pack (i32.const 0) (i32.const 2)))))
    (if (i64.le_u (local.get $value) (i64.const 1073741823))
      (then
        (if (i32.lt_u (local.get $out_cap) (i32.const 4))
          (then (return (call $pack (i32.const 2) (i32.const 0)))))
        (i32.store8
          (local.get $out_ptr)
          (i32.or
            (i32.const 128)
            (i32.and
              (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 24)))
              (i32.const 63))))
        (i32.store8
          (i32.add (local.get $out_ptr) (i32.const 1))
          (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 16))))
        (i32.store8
          (i32.add (local.get $out_ptr) (i32.const 2))
          (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 8))))
        (i32.store8
          (i32.add (local.get $out_ptr) (i32.const 3))
          (i32.wrap_i64 (local.get $value))
        )
        (return (call $pack (i32.const 0) (i32.const 4)))))
    (if (i32.lt_u (local.get $out_cap) (i32.const 8))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))
    (i32.store8
      (local.get $out_ptr)
      (i32.or
        (i32.const 192)
        (i32.and
          (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 56)))
          (i32.const 63))))
    (i32.store8
      (i32.add (local.get $out_ptr) (i32.const 1))
      (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 48))))
    (i32.store8
      (i32.add (local.get $out_ptr) (i32.const 2))
      (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 40))))
    (i32.store8
      (i32.add (local.get $out_ptr) (i32.const 3))
      (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 32))))
    (i32.store8
      (i32.add (local.get $out_ptr) (i32.const 4))
      (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 24))))
    (i32.store8
      (i32.add (local.get $out_ptr) (i32.const 5))
      (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 16))))
    (i32.store8
      (i32.add (local.get $out_ptr) (i32.const 6))
      (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 8))))
    (i32.store8
      (i32.add (local.get $out_ptr) (i32.const 7))
      (i32.wrap_i64 (local.get $value)))
    (call $pack (i32.const 0) (i32.const 8)))

  ;; Decode an HTTP/3 frame header and verify the declared payload is present.
  ;; out record u32 fields:
  ;; 0 type_low, 4 type_high, 8 payload_len_low, 12 payload_len_high,
  ;; 16 header_len, 20 classification.
  (func (export "http3_frame_header_decode")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32)
    (result i32)
    (local $packed i64)
    (local $status i32)
    (local $type_len i32)
    (local $payload_len_bytes i32)
    (local $header_len i32)
    (local $frame_type i64)
    (local $payload_len i64)
    (local.set $packed
      (call $quic_varint_decode_at
        (local.get $in_ptr)
        (local.get $in_len)
        (i32.const 0)
        (local.get $out_ptr)))
    (local.set $status (i32.wrap_i64 (local.get $packed)))
    (if (local.get $status)
      (then (return (local.get $status))))
    (local.set $type_len
      (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
    (local.set $frame_type (i64.load (local.get $out_ptr)))
    (local.set $packed
      (call $quic_varint_decode_at
        (local.get $in_ptr)
        (local.get $in_len)
        (local.get $type_len)
        (i32.add (local.get $out_ptr) (i32.const 8))))
    (local.set $status (i32.wrap_i64 (local.get $packed)))
    (if (local.get $status)
      (then (return (local.get $status))))
    (local.set $payload_len_bytes
      (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
    (local.set $payload_len (i64.load (i32.add (local.get $out_ptr) (i32.const 8))))
    (local.set $header_len (i32.add (local.get $type_len) (local.get $payload_len_bytes)))
    (if
      (i64.gt_u
        (local.get $payload_len)
        (i64.extend_i32_u (i32.sub (local.get $in_len) (local.get $header_len))))
      (then (return (i32.const 5))))
    (i32.store (local.get $out_ptr) (i32.wrap_i64 (local.get $frame_type)))
    (i32.store
      (i32.add (local.get $out_ptr) (i32.const 4))
      (i32.wrap_i64 (i64.shr_u (local.get $frame_type) (i64.const 32))))
    (i32.store
      (i32.add (local.get $out_ptr) (i32.const 8))
      (i32.wrap_i64 (local.get $payload_len)))
    (i32.store
      (i32.add (local.get $out_ptr) (i32.const 12))
      (i32.wrap_i64 (i64.shr_u (local.get $payload_len) (i64.const 32))))
    (i32.store
      (i32.add (local.get $out_ptr) (i32.const 16))
      (local.get $header_len))
    (i32.store
      (i32.add (local.get $out_ptr) (i32.const 20))
      (call $http3_frame_type_classify (local.get $frame_type)))
    (i32.const 0))

  ;; Return bits: low32=status, high32=written.
  (func (export "http3_frame_header_encode")
    (param $frame_type i64) (param $payload_len i64) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $packed i64)
    (local $status i32)
    (local $type_written i32)
    (if
      (i32.or
        (i64.gt_u (local.get $frame_type) (i64.const 4611686018427387903))
        (i64.gt_u (local.get $payload_len) (i64.const 4611686018427387903)))
      (then (return (call $pack (i32.const 4) (i32.const 0)))))
    (local.set $packed
      (call $quic_varint_encode
        (local.get $frame_type)
        (local.get $out_ptr)
        (local.get $out_cap)))
    (local.set $status (i32.wrap_i64 (local.get $packed)))
    (if (local.get $status)
      (then (return (local.get $packed))))
    (local.set $type_written
      (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
    (local.set $packed
      (call $quic_varint_encode
        (local.get $payload_len)
        (i32.add (local.get $out_ptr) (local.get $type_written))
        (i32.sub (local.get $out_cap) (local.get $type_written))))
    (local.set $status (i32.wrap_i64 (local.get $packed)))
    (if (local.get $status)
      (then (return (local.get $packed))))
    (call $pack
      (i32.const 0)
      (i32.add
        (local.get $type_written)
        (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))))

;; EdgeRun HTTP client portable semantics.
  ;; Codes:
  ;; version: 1=h1 2=h2 3=h3 4=best 5=h2_or_h1
  ;; route: 1=h1 2=h2 3=h3 4=race_h3_tcp 5=h2_then_h1
  ;; pool action: 1=reuse 2=drop 3=create 4=evict_oldest
  ;; result: 0=pending 1=ok 2=err 3=retry 4=redirect 5=close

  (func (export "http_client_default_connect_timeout_secs") (result i32) (i32.const 10))
  (func (export "http_client_default_read_timeout_secs") (result i32) (i32.const 30))
  (func (export "http_client_default_redirect_limit") (result i32) (i32.const 10))
  (func (export "http1_pool_default_max_per_host") (result i32) (i32.const 6))
  (func (export "http1_pool_default_idle_timeout_secs") (result i32) (i32.const 30))
  (func (export "http1_pool_default_dns_timeout_secs") (result i32) (i32.const 5))
  (func (export "http2_max_body_size") (result i32) (i32.const 104857600))
  (func (export "http2_idle_ping_secs") (result i32) (i32.const 30))
  (func (export "http2_ping_timeout_secs") (result i32) (i32.const 10))
  (func (export "tls_session_cache_default_per_server") (result i32) (i32.const 4))

  (func (export "http_client_route")
    (param $version i32) (param $is_https i32) (param $h2_disabled i32) (param $h3_disabled i32)
    (result i32)
    (if (i32.eq (local.get $version) (i32.const 1)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $version) (i32.const 2)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $version) (i32.const 3)) (then (return (i32.const 3))))
    (if (i32.eq (local.get $version) (i32.const 5))
      (then
        (if (local.get $h2_disabled) (then (return (i32.const 1))))
        (return (i32.const 5))))
    (if (local.get $is_https)
      (then
        (if (local.get $h3_disabled)
          (then
            (if (local.get $h2_disabled) (then (return (i32.const 1))))
            (return (i32.const 5)))
          (else (return (i32.const 4)))))
      (else
        (if (local.get $h2_disabled) (then (return (i32.const 1))))
        (return (i32.const 5))))
    (i32.const 0))

  (func (export "http_client_best_winner")
    (param $h3_ok i32) (param $tcp_ok i32) (param $tcp_first i32) (result i32)
    (if (i32.and (local.get $tcp_ok) (local.get $tcp_first))
      (then (return (i32.const 2))))
    (if (local.get $h3_ok) (then (return (i32.const 3))))
    (if (local.get $tcp_ok) (then (return (i32.const 2))))
    (i32.const 2))

  (func (export "http_redirect_action")
    (param $follow i32) (param $remaining i32) (param $status i32) (param $has_location i32) (param $method i32)
    (result i32)
    (if (i32.and
          (i32.and (local.get $follow) (i32.gt_u (local.get $remaining) (i32.const 0)))
          (i32.and
            (i32.and (i32.ge_u (local.get $status) (i32.const 300)) (i32.lt_u (local.get $status) (i32.const 400)))
            (local.get $has_location)))
      (then
        ;; 303 rewrites non-GET/non-HEAD to GET; other redirects keep method.
        (if (i32.and (i32.eq (local.get $status) (i32.const 303))
                     (i32.and (i32.ne (local.get $method) (i32.const 1)) (i32.ne (local.get $method) (i32.const 8))))
          (then (return (i32.const 6))))
        (return (i32.const 4))))
    (i32.const 1))

  (func (export "http1_pool_action")
    (param $pooled_count i32) (param $idle_expired i32) (param $response_allows_reuse i32) (param $current_count i32) (param $max_per_host i32)
    (result i32)
    (if (local.get $idle_expired) (then (return (i32.const 2))))
    (if (i32.gt_u (local.get $pooled_count) (i32.const 0)) (then (return (i32.const 1))))
    (if (i32.and (local.get $response_allows_reuse) (i32.ge_u (local.get $current_count) (local.get $max_per_host)))
      (then (return (i32.const 4))))
    (i32.const 3))

  (func (export "http1_response_allows_reuse")
    (param $connection_close i32) (param $body_fully_read i32) (result i32)
    (i32.and (i32.eqz (local.get $connection_close)) (local.get $body_fully_read)))

  (func (export "http1_default_port")
    (param $is_https i32) (param $explicit_port i32) (result i32)
    (if (i32.gt_u (local.get $explicit_port) (i32.const 0))
      (then (return (local.get $explicit_port))))
    (if (local.get $is_https) (then (return (i32.const 443))))
    (i32.const 80))

  (func (export "http_request_builder_header_mask")
    (param $has_host i32) (param $has_connection i32) (param $has_user_agent i32) (param $has_body i32)
    (result i32)
    ;; bit0 add Host, bit1 add keep-alive, bit2 add user-agent, bit3 set content-length.
    (i32.or
      (i32.or
        (select (i32.const 0) (i32.const 1) (local.get $has_host))
        (select (i32.const 0) (i32.const 2) (local.get $has_connection)))
      (i32.or
        (select (i32.const 0) (i32.const 4) (local.get $has_user_agent))
        (select (i32.const 8) (i32.const 0) (local.get $has_body)))))

  (func (export "http_decompress_gate")
    (param $auto_decompress i32) (param $encoding i32) (param $is_head i32) (result i32)
    ;; encoding: 0=none/unknown 1=gzip 2=zlib/deflate 3=br-reserved
    (if (i32.or (i32.eqz (local.get $auto_decompress)) (local.get $is_head))
      (then (return (i32.const 0))))
    (if (i32.or (i32.eq (local.get $encoding) (i32.const 1)) (i32.eq (local.get $encoding) (i32.const 2)))
      (then (return (i32.const 1))))
    (i32.const 0))

  (func (export "http2_next_client_stream_id")
    (param $current i32) (result i32)
    (i32.add (local.get $current) (i32.const 2)))

  (func (export "http2_request_end_stream")
    (param $body_present i32) (param $body_len i32) (result i32)
    (i32.or (i32.eqz (local.get $body_present)) (i32.eqz (local.get $body_len))))

  (func (export "http2_data_frame_count")
    (param $body_len i32) (param $max_frame_size i32) (result i32)
    (if (i32.eqz (local.get $body_len)) (then (return (i32.const 0))))
    (i32.div_u
      (i32.add (local.get $body_len) (i32.sub (local.get $max_frame_size) (i32.const 1)))
      (local.get $max_frame_size)))

  (func (export "http2_keepalive_action")
    (param $idle_elapsed i32) (param $ping_pending i32) (param $ping_timed_out i32) (param $incoming_frame i32)
    (result i32)
    ;; 0=none 1=send_ping 2=clear_ping 5=close
    (if (i32.and (local.get $ping_pending) (local.get $ping_timed_out)) (then (return (i32.const 5))))
    (if (i32.and (local.get $ping_pending) (local.get $incoming_frame)) (then (return (i32.const 2))))
    (if (i32.and (i32.eqz (local.get $ping_pending)) (local.get $idle_elapsed)) (then (return (i32.const 1))))
    (i32.const 0))

  (func (export "http3_initial_bidi_stream")
    (param $is_server i32) (result i32)
    (if (local.get $is_server) (then (return (i32.const 1))))
    (i32.const 0))

  (func (export "http3_initial_uni_stream")
    (param $is_server i32) (result i32)
    (if (local.get $is_server) (then (return (i32.const 3))))
    (i32.const 2))

  (func (export "http3_preface_stream_count") (result i32) (i32.const 3))

  (func (export "http3_uni_stream_role")
    (param $stream_type i32) (result i32)
    ;; 0=control 2=qpack-encoder 3=qpack-decoder 1=push 9=unknown
    (if (i32.eq (local.get $stream_type) (i32.const 0)) (then (return (i32.const 0))))
    (if (i32.eq (local.get $stream_type) (i32.const 2)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $stream_type) (i32.const 3)) (then (return (i32.const 3))))
    (if (i32.eq (local.get $stream_type) (i32.const 1)) (then (return (i32.const 1))))
    (i32.const 9))

  (func (export "http3_control_frame_allowed")
    (param $frame_type i32) (result i32)
    ;; SETTINGS=4, GOAWAY=7, MAX_PUSH_ID=13 are legal control stream frames here.
    (i32.or
      (i32.or (i32.eq (local.get $frame_type) (i32.const 4)) (i32.eq (local.get $frame_type) (i32.const 7)))
      (i32.eq (local.get $frame_type) (i32.const 13))))

  (func (export "http3_goaway_accept_new_stream")
    (param $going_away_id i64) (param $new_stream_id i64) (result i32)
    (i64.le_u (local.get $new_stream_id) (local.get $going_away_id)))

  (func (export "middleware_chain_index")
    (param $middleware_count i32) (param $call_depth i32) (result i32)
    ;; Chain applies first registered middleware outermost.
    (if (i32.ge_u (local.get $call_depth) (local.get $middleware_count))
      (then (return (i32.const -1))))
    (local.get $call_depth))

  (func (export "tls_session_ticket_action")
    (param $ticket_empty i32) (param $is_duplicate i32) (param $valid_count i32) (param $max_per_server i32)
    (result i32)
    ;; 0=skip 1=store 4=evict_oldest_then_store
    (if (i32.or (local.get $ticket_empty) (local.get $is_duplicate)) (then (return (i32.const 0))))
    (if (i32.ge_u (local.get $valid_count) (local.get $max_per_server)) (then (return (i32.const 4))))
    (i32.const 1))

  (func (export "tls_obfuscated_ticket_age")
    (param $elapsed_ms i32) (param $age_add i32) (result i32)
    (i32.add (local.get $elapsed_ms) (local.get $age_add)))


;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid, 4 overflow.
  ;; Parse output record, little-endian u32 fields:
  ;; 0 unix_secs_lo, 4 unix_secs_hi, 8 year, 12 month, 16 day, 20 hour,
  ;; 24 minute, 28 second, 32 weekday (Mon=1..Sun=7), 36 format_kind
  ;; format_kind: 1 IMF-fixdate, 2 RFC850 obsolete date, 3 asctime obsolete date.

  (func $m108b (param $p i32) (result i32)
    (i32.load8_u (local.get $p)))

  (func $eq3 (param $p i32) (param $a i32) (param $m108b i32) (param $c i32) (result i32)
    (i32.and
      (i32.and
        (i32.eq (call $m108b (local.get $p)) (local.get $a))
        (i32.eq (call $m108b (i32.add (local.get $p) (i32.const 1))) (local.get $m108b)))
      (i32.eq (call $m108b (i32.add (local.get $p) (i32.const 2))) (local.get $c))))

  (func $m108digit (param $p i32) (result i32)
    (local $x i32)
    (local.set $x (i32.sub (call $m108b (local.get $p)) (i32.const 48)))
    (if (i32.gt_u (local.get $x) (i32.const 9))
      (then (return (i32.const -1))))
    local.get $x)

  (func $one_or_two_day (param $p i32) (result i32)
    (local $a i32)
    (local $m108b i32)
    (if (i32.eq (call $m108b (local.get $p)) (i32.const 32))
      (then
        (local.set $m108b (call $m108digit (i32.add (local.get $p) (i32.const 1))))
        (if (i32.lt_s (local.get $m108b) (i32.const 0)) (then (return (i32.const -1))))
        (return (local.get $m108b))))
    (local.set $a (call $m108digit (local.get $p)))
    (local.set $m108b (call $m108digit (i32.add (local.get $p) (i32.const 1))))
    (if (i32.or (i32.lt_s (local.get $a) (i32.const 0)) (i32.lt_s (local.get $m108b) (i32.const 0)))
      (then (return (i32.const -1))))
    (i32.add (i32.mul (local.get $a) (i32.const 10)) (local.get $m108b)))

  (func $m108two (param $p i32) (result i32)
    (local $a i32)
    (local $m108b i32)
    (local.set $a (call $m108digit (local.get $p)))
    (local.set $m108b (call $m108digit (i32.add (local.get $p) (i32.const 1))))
    (if (i32.or (i32.lt_s (local.get $a) (i32.const 0)) (i32.lt_s (local.get $m108b) (i32.const 0)))
      (then (return (i32.const -1))))
    (i32.add (i32.mul (local.get $a) (i32.const 10)) (local.get $m108b)))

  (func $m108four (param $p i32) (result i32)
    (local $a i32)
    (local $m108b i32)
    (local.set $a (call $m108two (local.get $p)))
    (local.set $m108b (call $m108two (i32.add (local.get $p) (i32.const 2))))
    (if (i32.or (i32.lt_s (local.get $a) (i32.const 0)) (i32.lt_s (local.get $m108b) (i32.const 0)))
      (then (return (i32.const -1))))
    (i32.add (i32.mul (local.get $a) (i32.const 100)) (local.get $m108b)))

  (func $m108is_leap (param $year i32) (result i32)
    (if (i32.ne (i32.rem_u (local.get $year) (i32.const 4)) (i32.const 0))
      (then (return (i32.const 0))))
    (if (i32.ne (i32.rem_u (local.get $year) (i32.const 100)) (i32.const 0))
      (then (return (i32.const 1))))
    (i32.eq (i32.rem_u (local.get $year) (i32.const 400)) (i32.const 0)))

  (func $m108month_days (param $year i32) (param $month i32) (result i32)
    (if (i32.eq (local.get $month) (i32.const 2))
      (then (return (select (i32.const 29) (i32.const 28) (call $m108is_leap (local.get $year))))))
    (if (i32.or
          (i32.or (i32.eq (local.get $month) (i32.const 4)) (i32.eq (local.get $month) (i32.const 6)))
          (i32.or (i32.eq (local.get $month) (i32.const 9)) (i32.eq (local.get $month) (i32.const 11))))
      (then (return (i32.const 30))))
    i32.const 31)

  (func $month3 (param $p i32) (result i32)
    (if (call $eq3 (local.get $p) (i32.const 74) (i32.const 97) (i32.const 110)) (then (return (i32.const 1))))
    (if (call $eq3 (local.get $p) (i32.const 70) (i32.const 101) (i32.const 98)) (then (return (i32.const 2))))
    (if (call $eq3 (local.get $p) (i32.const 77) (i32.const 97) (i32.const 114)) (then (return (i32.const 3))))
    (if (call $eq3 (local.get $p) (i32.const 65) (i32.const 112) (i32.const 114)) (then (return (i32.const 4))))
    (if (call $eq3 (local.get $p) (i32.const 77) (i32.const 97) (i32.const 121)) (then (return (i32.const 5))))
    (if (call $eq3 (local.get $p) (i32.const 74) (i32.const 117) (i32.const 110)) (then (return (i32.const 6))))
    (if (call $eq3 (local.get $p) (i32.const 74) (i32.const 117) (i32.const 108)) (then (return (i32.const 7))))
    (if (call $eq3 (local.get $p) (i32.const 65) (i32.const 117) (i32.const 103)) (then (return (i32.const 8))))
    (if (call $eq3 (local.get $p) (i32.const 83) (i32.const 101) (i32.const 112)) (then (return (i32.const 9))))
    (if (call $eq3 (local.get $p) (i32.const 79) (i32.const 99) (i32.const 116)) (then (return (i32.const 10))))
    (if (call $eq3 (local.get $p) (i32.const 78) (i32.const 111) (i32.const 118)) (then (return (i32.const 11))))
    (if (call $eq3 (local.get $p) (i32.const 68) (i32.const 101) (i32.const 99)) (then (return (i32.const 12))))
    i32.const 0)

  (func $wday3 (param $p i32) (result i32)
    (if (call $eq3 (local.get $p) (i32.const 77) (i32.const 111) (i32.const 110)) (then (return (i32.const 1))))
    (if (call $eq3 (local.get $p) (i32.const 84) (i32.const 117) (i32.const 101)) (then (return (i32.const 2))))
    (if (call $eq3 (local.get $p) (i32.const 87) (i32.const 101) (i32.const 100)) (then (return (i32.const 3))))
    (if (call $eq3 (local.get $p) (i32.const 84) (i32.const 104) (i32.const 117)) (then (return (i32.const 4))))
    (if (call $eq3 (local.get $p) (i32.const 70) (i32.const 114) (i32.const 105)) (then (return (i32.const 5))))
    (if (call $eq3 (local.get $p) (i32.const 83) (i32.const 97) (i32.const 116)) (then (return (i32.const 6))))
    (if (call $eq3 (local.get $p) (i32.const 83) (i32.const 117) (i32.const 110)) (then (return (i32.const 7))))
    i32.const 0)

  (func $match (param $p i32) (param $len i32) (param $off i32) (param $s i32) (param $slen i32) (result i32)
    (local $i i32)
    (if (i32.gt_u (i32.add (local.get $off) (local.get $slen)) (local.get $len))
      (then (return (i32.const 0))))
    (loop $loop
      (if (i32.eq (local.get $i) (local.get $slen)) (then (return (i32.const 1))))
      (if (i32.ne
            (i32.load8_u (i32.add (local.get $p) (i32.add (local.get $off) (local.get $i))))
            (i32.load8_u (i32.add (local.get $s) (local.get $i))))
        (then (return (i32.const 0))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      br $loop)
    i32.const 1)

  (func $wday_long (param $p i32) (param $len i32) (result i64)
    ;; packed as low32=wday, high32=prefix length.
    (if (call $match (local.get $p) (local.get $len) (i32.const 0) (i32.const 1200) (i32.const 8)) (then (return (i64.const 34359738369)))) ;; Monday,
    (if (call $match (local.get $p) (local.get $len) (i32.const 0) (i32.const 1208) (i32.const 9)) (then (return (i64.const 38654705666)))) ;; Tuesday,
    (if (call $match (local.get $p) (local.get $len) (i32.const 0) (i32.const 1217) (i32.const 11)) (then (return (i64.const 47244640259)))) ;; Wednesday,
    (if (call $match (local.get $p) (local.get $len) (i32.const 0) (i32.const 1228) (i32.const 10)) (then (return (i64.const 42949672964)))) ;; Thursday,
    (if (call $match (local.get $p) (local.get $len) (i32.const 0) (i32.const 1238) (i32.const 8)) (then (return (i64.const 34359738373)))) ;; Friday,
    (if (call $match (local.get $p) (local.get $len) (i32.const 0) (i32.const 1246) (i32.const 10)) (then (return (i64.const 42949672966)))) ;; Saturday,
    (if (call $match (local.get $p) (local.get $len) (i32.const 0) (i32.const 1256) (i32.const 8)) (then (return (i64.const 34359738375)))) ;; Sunday,
    i64.const 0)

  (func $m108days_before_year (param $year i32) (result i32)
    (local $y i32)
    (local $days i32)
    (local.set $y (i32.const 1970))
    (loop $loop
      (if (i32.ge_u (local.get $y) (local.get $year)) (then (return (local.get $days))))
      (local.set $days (i32.add (local.get $days) (select (i32.const 366) (i32.const 365) (call $m108is_leap (local.get $y)))))
      (local.set $y (i32.add (local.get $y) (i32.const 1)))
      br $loop)
    local.get $days)

  (func $days_before_month (param $year i32) (param $month i32) (result i32)
    (local $m i32)
    (local $days i32)
    (local.set $m (i32.const 1))
    (loop $loop
      (if (i32.ge_u (local.get $m) (local.get $month)) (then (return (local.get $days))))
      (local.set $days (i32.add (local.get $days) (call $m108month_days (local.get $year) (local.get $m))))
      (local.set $m (i32.add (local.get $m) (i32.const 1)))
      br $loop)
    local.get $days)

  (func $parts_to_unix (param $year i32) (param $month i32) (param $day i32) (param $hour i32) (param $minute i32) (param $second i32) (result i64)
    (local $days i32)
    (local.set $days
      (i32.add
        (i32.add (call $m108days_before_year (local.get $year)) (call $days_before_month (local.get $year) (local.get $month)))
        (i32.sub (local.get $day) (i32.const 1))))
    (i64.add
      (i64.mul (i64.extend_i32_u (local.get $days)) (i64.const 86400))
      (i64.extend_i32_u
        (i32.add
          (i32.add (i32.mul (local.get $hour) (i32.const 3600)) (i32.mul (local.get $minute) (i32.const 60)))
          (local.get $second)))))

  (func $http_date_validate_parts (export "http_date_validate_parts")
    (param $year i32) (param $month i32) (param $day i32)
    (param $hour i32) (param $minute i32) (param $second i32) (param $weekday i32)
    (result i32)
    (local $days i32)
    (if (i32.or (i32.lt_u (local.get $year) (i32.const 1970)) (i32.gt_u (local.get $year) (i32.const 9999))) (then (return (i32.const 3))))
    (if (i32.or (i32.lt_u (local.get $month) (i32.const 1)) (i32.gt_u (local.get $month) (i32.const 12))) (then (return (i32.const 3))))
    (if (i32.or (i32.lt_u (local.get $day) (i32.const 1)) (i32.gt_u (local.get $day) (call $m108month_days (local.get $year) (local.get $month)))) (then (return (i32.const 3))))
    (if (i32.or (i32.ge_u (local.get $hour) (i32.const 24)) (i32.or (i32.ge_u (local.get $minute) (i32.const 60)) (i32.ge_u (local.get $second) (i32.const 60)))) (then (return (i32.const 3))))
    (if (i32.or (i32.lt_u (local.get $weekday) (i32.const 1)) (i32.gt_u (local.get $weekday) (i32.const 7))) (then (return (i32.const 3))))
    (local.set $days
      (i32.add
        (i32.add (call $m108days_before_year (local.get $year)) (call $days_before_month (local.get $year) (local.get $month)))
        (i32.sub (local.get $day) (i32.const 1))))
    (if (i32.ne (local.get $weekday) (i32.add (i32.rem_u (i32.add (local.get $days) (i32.const 3)) (i32.const 7)) (i32.const 1)))
      (then (return (i32.const 3))))
    i32.const 0)

  (func $m108write_record (param $out i32) (param $unix i64)
    (param $year i32) (param $month i32) (param $day i32)
    (param $hour i32) (param $minute i32) (param $second i32)
    (param $weekday i32) (param $kind i32)
    (i32.store (local.get $out) (i32.wrap_i64 (local.get $unix)))
    (i32.store (i32.add (local.get $out) (i32.const 4)) (i32.wrap_i64 (i64.shr_u (local.get $unix) (i64.const 32))))
    (i32.store (i32.add (local.get $out) (i32.const 8)) (local.get $year))
    (i32.store (i32.add (local.get $out) (i32.const 12)) (local.get $month))
    (i32.store (i32.add (local.get $out) (i32.const 16)) (local.get $day))
    (i32.store (i32.add (local.get $out) (i32.const 20)) (local.get $hour))
    (i32.store (i32.add (local.get $out) (i32.const 24)) (local.get $minute))
    (i32.store (i32.add (local.get $out) (i32.const 28)) (local.get $second))
    (i32.store (i32.add (local.get $out) (i32.const 32)) (local.get $weekday))
    (i32.store (i32.add (local.get $out) (i32.const 36)) (local.get $kind)))

  (func $finish_parse (param $out i32)
    (param $year i32) (param $month i32) (param $day i32)
    (param $hour i32) (param $minute i32) (param $second i32)
    (param $weekday i32) (param $kind i32) (result i32)
    (local $unix i64)
    (if (call $http_date_validate_parts
          (local.get $year) (local.get $month) (local.get $day)
          (local.get $hour) (local.get $minute) (local.get $second) (local.get $weekday))
      (then (return (i32.const 3))))
    (local.set $unix (call $parts_to_unix (local.get $year) (local.get $month) (local.get $day) (local.get $hour) (local.get $minute) (local.get $second)))
    (call $m108write_record (local.get $out) (local.get $unix)
      (local.get $year) (local.get $month) (local.get $day)
      (local.get $hour) (local.get $minute) (local.get $second)
      (local.get $weekday) (local.get $kind))
    i32.const 0)

  (func (export "http_date_parse") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $year i32) (local $month i32) (local $day i32)
    (local $hour i32) (local $minute i32) (local $second i32)
    (local $weekday i32) (local $x i64) (local $off i32)
    (if (i32.lt_u (local.get $len) (i32.const 24)) (then (return (i32.const 1))))

    ;; IMF-fixdate: Sun, 06 Nov 1994 08:49:37 GMT
    (if (i32.eq (local.get $len) (i32.const 29))
      (then
        (if (i32.and
              (i32.and
                (i32.and (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.const 3))) (i32.const 44)) (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.const 4))) (i32.const 32)))
                (i32.and (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.const 7))) (i32.const 32)) (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.const 11))) (i32.const 32))))
              (i32.and
                (i32.and (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.const 16))) (i32.const 32)) (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.const 19))) (i32.const 58)))
                (i32.and (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.const 22))) (i32.const 58)) (call $match (local.get $ptr) (local.get $len) (i32.const 25) (i32.const 1264) (i32.const 4)))))
          (then
            (local.set $weekday (call $wday3 (local.get $ptr)))
            (local.set $day (call $m108two (i32.add (local.get $ptr) (i32.const 5))))
            (local.set $month (call $month3 (i32.add (local.get $ptr) (i32.const 8))))
            (local.set $year (call $m108four (i32.add (local.get $ptr) (i32.const 12))))
            (local.set $hour (call $m108two (i32.add (local.get $ptr) (i32.const 17))))
            (local.set $minute (call $m108two (i32.add (local.get $ptr) (i32.const 20))))
            (local.set $second (call $m108two (i32.add (local.get $ptr) (i32.const 23))))
            (if (i32.and
                  (i32.and (i32.gt_u (local.get $weekday) (i32.const 0)) (i32.gt_u (local.get $month) (i32.const 0)))
                  (i32.and
                    (i32.and (i32.ge_s (local.get $day) (i32.const 0)) (i32.ge_s (local.get $year) (i32.const 0)))
                    (i32.and (i32.ge_s (local.get $hour) (i32.const 0)) (i32.and (i32.ge_s (local.get $minute) (i32.const 0)) (i32.ge_s (local.get $second) (i32.const 0))))))
              (then
                (return (call $finish_parse (local.get $out)
                  (local.get $year) (local.get $month) (local.get $day)
                  (local.get $hour) (local.get $minute) (local.get $second)
                  (local.get $weekday) (i32.const 1)))))))))

    ;; RFC850 obsolete: Sunday, 06-Nov-94 08:49:37 GMT
    (local.set $x (call $wday_long (local.get $ptr) (local.get $len)))
    (if (i64.ne (local.get $x) (i64.const 0))
      (then
        (local.set $weekday (i32.wrap_i64 (local.get $x)))
        (local.set $off (i32.wrap_i64 (i64.shr_u (local.get $x) (i64.const 32))))
        (if (i32.eq (local.get $len) (i32.add (local.get $off) (i32.const 22)))
          (then
            (if (i32.and
                  (i32.and (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.add (local.get $off) (i32.const 2)))) (i32.const 45)) (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.add (local.get $off) (i32.const 6)))) (i32.const 45)))
                  (i32.and
                    (i32.and (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.add (local.get $off) (i32.const 9)))) (i32.const 32)) (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.add (local.get $off) (i32.const 12)))) (i32.const 58)))
                    (i32.and (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.add (local.get $off) (i32.const 15)))) (i32.const 58)) (call $match (local.get $ptr) (local.get $len) (i32.add (local.get $off) (i32.const 18)) (i32.const 1264) (i32.const 4)))))
              (then
                (local.set $day (call $m108two (i32.add (local.get $ptr) (local.get $off))))
                (local.set $month (call $month3 (i32.add (local.get $ptr) (i32.add (local.get $off) (i32.const 3)))))
                (local.set $year (call $m108two (i32.add (local.get $ptr) (i32.add (local.get $off) (i32.const 7)))))
                (if (i32.ge_s (local.get $year) (i32.const 0))
                  (then
                    (if (i32.lt_u (local.get $year) (i32.const 70))
                      (then (local.set $year (i32.add (local.get $year) (i32.const 2000))))
                      (else (local.set $year (i32.add (local.get $year) (i32.const 1900)))))))
                (local.set $hour (call $m108two (i32.add (local.get $ptr) (i32.add (local.get $off) (i32.const 10)))))
                (local.set $minute (call $m108two (i32.add (local.get $ptr) (i32.add (local.get $off) (i32.const 13)))))
                (local.set $second (call $m108two (i32.add (local.get $ptr) (i32.add (local.get $off) (i32.const 16)))))
                (if (i32.and
                      (i32.gt_u (local.get $month) (i32.const 0))
                      (i32.and
                        (i32.and (i32.ge_s (local.get $day) (i32.const 0)) (i32.ge_s (local.get $year) (i32.const 0)))
                        (i32.and (i32.ge_s (local.get $hour) (i32.const 0)) (i32.and (i32.ge_s (local.get $minute) (i32.const 0)) (i32.ge_s (local.get $second) (i32.const 0))))))
                  (then
                    (return (call $finish_parse (local.get $out)
                      (local.get $year) (local.get $month) (local.get $day)
                      (local.get $hour) (local.get $minute) (local.get $second)
                      (local.get $weekday) (i32.const 2)))))))))))

    ;; asctime obsolete: Sun Nov  6 08:49:37 1994
    (if (i32.eq (local.get $len) (i32.const 24))
      (then
        (if (i32.and
              (i32.and (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.const 3))) (i32.const 32)) (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.const 7))) (i32.const 32)))
              (i32.and
                (i32.and (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.const 10))) (i32.const 32)) (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.const 13))) (i32.const 58)))
                (i32.and (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.const 16))) (i32.const 58)) (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.const 19))) (i32.const 32)))))
          (then
            (local.set $weekday (call $wday3 (local.get $ptr)))
            (local.set $month (call $month3 (i32.add (local.get $ptr) (i32.const 4))))
            (local.set $day (call $one_or_two_day (i32.add (local.get $ptr) (i32.const 8))))
            (local.set $hour (call $m108two (i32.add (local.get $ptr) (i32.const 11))))
            (local.set $minute (call $m108two (i32.add (local.get $ptr) (i32.const 14))))
            (local.set $second (call $m108two (i32.add (local.get $ptr) (i32.const 17))))
            (local.set $year (call $m108four (i32.add (local.get $ptr) (i32.const 20))))
            (if (i32.and
                  (i32.and (i32.gt_u (local.get $weekday) (i32.const 0)) (i32.gt_u (local.get $month) (i32.const 0)))
                  (i32.and
                    (i32.and (i32.ge_s (local.get $day) (i32.const 0)) (i32.ge_s (local.get $year) (i32.const 0)))
                    (i32.and (i32.ge_s (local.get $hour) (i32.const 0)) (i32.and (i32.ge_s (local.get $minute) (i32.const 0)) (i32.ge_s (local.get $second) (i32.const 0))))))
              (then
                (return (call $finish_parse (local.get $out)
                  (local.get $year) (local.get $month) (local.get $day)
                  (local.get $hour) (local.get $minute) (local.get $second)
                  (local.get $weekday) (i32.const 3)))))))))

    i32.const 3)

  (func $m108put2 (param $p i32) (param $v i32)
    (i32.store8 (local.get $p) (i32.add (i32.const 48) (i32.div_u (local.get $v) (i32.const 10))))
    (i32.store8 (i32.add (local.get $p) (i32.const 1)) (i32.add (i32.const 48) (i32.rem_u (local.get $v) (i32.const 10)))))

  (func $m108put4 (param $p i32) (param $v i32)
    (i32.store8 (local.get $p) (i32.add (i32.const 48) (i32.div_u (local.get $v) (i32.const 1000))))
    (i32.store8 (i32.add (local.get $p) (i32.const 1)) (i32.add (i32.const 48) (i32.rem_u (i32.div_u (local.get $v) (i32.const 100)) (i32.const 10))))
    (i32.store8 (i32.add (local.get $p) (i32.const 2)) (i32.add (i32.const 48) (i32.rem_u (i32.div_u (local.get $v) (i32.const 10)) (i32.const 10))))
    (i32.store8 (i32.add (local.get $p) (i32.const 3)) (i32.add (i32.const 48) (i32.rem_u (local.get $v) (i32.const 10)))))

  (func $copy3 (param $dst i32) (param $src i32)
    (i32.store8 (local.get $dst) (i32.load8_u (local.get $src)))
    (i32.store8 (i32.add (local.get $dst) (i32.const 1)) (i32.load8_u (i32.add (local.get $src) (i32.const 1))))
    (i32.store8 (i32.add (local.get $dst) (i32.const 2)) (i32.load8_u (i32.add (local.get $src) (i32.const 2)))))

  (func $wday_name_ptr (param $weekday i32) (result i32)
    (i32.add (i32.const 1280) (i32.mul (i32.sub (local.get $weekday) (i32.const 1)) (i32.const 3))))

  (func $month_name_ptr (param $month i32) (result i32)
    (i32.add (i32.const 1304) (i32.mul (i32.sub (local.get $month) (i32.const 1)) (i32.const 3))))

  (func (export "http_date_format") (param $lo i32) (param $hi i32) (param $out i32) (param $cap i32) (result i64)
    (local $secs i64) (local $days i64) (local $sod i64)
    (local $year i32) (local $month i32) (local $mdays i32)
    (local $day i32) (local $hour i32) (local $minute i32) (local $second i32) (local $weekday i32)
    (if (i32.lt_u (local.get $cap) (i32.const 29)) (then (return (call $pack (i32.const 2) (i32.const 0)))))
    (local.set $secs (i64.or (i64.extend_i32_u (local.get $lo)) (i64.shl (i64.extend_i32_u (local.get $hi)) (i64.const 32))))
    (if (i64.ge_u (local.get $secs) (i64.const 253402300800)) (then (return (call $pack (i32.const 4) (i32.const 0)))))
    (local.set $days (i64.div_u (local.get $secs) (i64.const 86400)))
    (local.set $sod (i64.rem_u (local.get $secs) (i64.const 86400)))
    (local.set $weekday (i32.add (i32.rem_u (i32.add (i32.wrap_i64 (local.get $days)) (i32.const 3)) (i32.const 7)) (i32.const 1)))
    (local.set $hour (i32.wrap_i64 (i64.div_u (local.get $sod) (i64.const 3600))))
    (local.set $minute (i32.wrap_i64 (i64.div_u (i64.rem_u (local.get $sod) (i64.const 3600)) (i64.const 60))))
    (local.set $second (i32.wrap_i64 (i64.rem_u (local.get $sod) (i64.const 60))))
    (local.set $year (i32.const 1970))
    (loop $year_loop
      (local.set $mdays (select (i32.const 366) (i32.const 365) (call $m108is_leap (local.get $year))))
      (if (i64.lt_u (local.get $days) (i64.extend_i32_u (local.get $mdays))) (then))
      (if (i64.ge_u (local.get $days) (i64.extend_i32_u (local.get $mdays)))
        (then
          (local.set $days (i64.sub (local.get $days) (i64.extend_i32_u (local.get $mdays)))
          )
          (local.set $year (i32.add (local.get $year) (i32.const 1)))
          br $year_loop)))
    (local.set $month (i32.const 1))
    (loop $month_loop
      (local.set $mdays (call $m108month_days (local.get $year) (local.get $month)))
      (if (i64.lt_u (local.get $days) (i64.extend_i32_u (local.get $mdays))) (then))
      (if (i64.ge_u (local.get $days) (i64.extend_i32_u (local.get $mdays)))
        (then
          (local.set $days (i64.sub (local.get $days) (i64.extend_i32_u (local.get $mdays))))
          (local.set $month (i32.add (local.get $month) (i32.const 1)))
          br $month_loop)))
    (local.set $day (i32.add (i32.wrap_i64 (local.get $days)) (i32.const 1)))

    (call $copy3 (local.get $out) (call $wday_name_ptr (local.get $weekday)))
    (i32.store8 (i32.add (local.get $out) (i32.const 3)) (i32.const 44))
    (i32.store8 (i32.add (local.get $out) (i32.const 4)) (i32.const 32))
    (call $m108put2 (i32.add (local.get $out) (i32.const 5)) (local.get $day))
    (i32.store8 (i32.add (local.get $out) (i32.const 7)) (i32.const 32))
    (call $copy3 (i32.add (local.get $out) (i32.const 8)) (call $month_name_ptr (local.get $month)))
    (i32.store8 (i32.add (local.get $out) (i32.const 11)) (i32.const 32))
    (call $m108put4 (i32.add (local.get $out) (i32.const 12)) (local.get $year))
    (i32.store8 (i32.add (local.get $out) (i32.const 16)) (i32.const 32))
    (call $m108put2 (i32.add (local.get $out) (i32.const 17)) (local.get $hour))
    (i32.store8 (i32.add (local.get $out) (i32.const 19)) (i32.const 58))
    (call $m108put2 (i32.add (local.get $out) (i32.const 20)) (local.get $minute))
    (i32.store8 (i32.add (local.get $out) (i32.const 22)) (i32.const 58))
    (call $m108put2 (i32.add (local.get $out) (i32.const 23)) (local.get $second))
    (i32.store8 (i32.add (local.get $out) (i32.const 25)) (i32.const 32))
    (i32.store8 (i32.add (local.get $out) (i32.const 26)) (i32.const 71))
    (i32.store8 (i32.add (local.get $out) (i32.const 27)) (i32.const 77))
    (i32.store8 (i32.add (local.get $out) (i32.const 28)) (i32.const 84))
    (call $pack (i32.const 0) (i32.const 29)))

  ;; Static strings used by parser/formatter.


(func $byte_lower_at (param $ptr i32) (param $off i32) (result i32)
    (call $to_lower (i32.load8_u (i32.add (local.get $ptr) (local.get $off)))))

  (func $is_ctl_or_space (param $b i32) (result i32)
    (i32.or (i32.le_u (local.get $b) (i32.const 32)) (i32.eq (local.get $b) (i32.const 127))))

  (func (export "http_header_name_valid") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (if (i32.eqz (local.get $len)) (then (return (i32.const 0))))
    (loop $scan
      (if (i32.ge_u (local.get $i) (local.get $len)) (then (return (i32.const 1))))
      (if (i32.eqz (call $is_tchar (i32.load8_u (i32.add (local.get $ptr) (local.get $i)))))
        (then (return (i32.const 0))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      br $scan)
    i32.const 1)

  (func (export "http_header_value_valid") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $b i32)
    (loop $scan
      (if (i32.ge_u (local.get $i) (local.get $len)) (then (return (i32.const 1))))
      (local.set $b (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
      (if
        (i32.eqz
          (i32.or
            (i32.eq (local.get $b) (i32.const 9))
            (i32.and (i32.ge_u (local.get $b) (i32.const 32)) (i32.le_u (local.get $b) (i32.const 126)))))
        (then (return (i32.const 0))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      br $scan)
    i32.const 1)

  ;; Method classes: 0 invalid, 1 GET, 2 HEAD, 3 POST, 4 PUT, 5 DELETE,
  ;; 6 CONNECT, 7 OPTIONS, 8 TRACE, 9 PATCH, 255 extension token.
  (func (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (if (i32.eqz (local.get $len)) (then (return (i32.const 0))))
    (block $valid_done
      (loop $valid
        (if (i32.ge_u (local.get $i) (local.get $len)) (then (br $valid_done)))
        (if (i32.eqz (call $is_tchar (i32.load8_u (i32.add (local.get $ptr) (local.get $i)))))
          (then (return (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        br $valid))
    (if
      (i32.and (i32.eq (local.get $len) (i32.const 3))
        (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 0)) (i32.const 103))
          (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 1)) (i32.const 101))
            (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 2)) (i32.const 116)))))
      (then (return (i32.const 1))))
    (if
      (i32.and (i32.eq (local.get $len) (i32.const 4))
        (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 0)) (i32.const 104))
          (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 1)) (i32.const 101))
            (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 2)) (i32.const 97))
              (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 3)) (i32.const 100))))))
      (then (return (i32.const 2))))
    (if
      (i32.and (i32.eq (local.get $len) (i32.const 4))
        (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 0)) (i32.const 112))
          (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 1)) (i32.const 111))
            (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 2)) (i32.const 115))
              (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 3)) (i32.const 116))))))
      (then (return (i32.const 3))))
    (if
      (i32.and (i32.eq (local.get $len) (i32.const 3))
        (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 0)) (i32.const 112))
          (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 1)) (i32.const 117))
            (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 2)) (i32.const 116)))))
      (then (return (i32.const 4))))
    (if
      (i32.and (i32.eq (local.get $len) (i32.const 6))
        (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 0)) (i32.const 100))
          (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 1)) (i32.const 101))
            (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 2)) (i32.const 108))
              (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 3)) (i32.const 101))
                (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 4)) (i32.const 116))
                  (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 5)) (i32.const 101))))))))
      (then (return (i32.const 5))))
    (if
      (i32.and (i32.eq (local.get $len) (i32.const 7))
        (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 0)) (i32.const 99))
          (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 1)) (i32.const 111))
            (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 2)) (i32.const 110))
              (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 3)) (i32.const 110))
                (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 4)) (i32.const 101))
                  (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 5)) (i32.const 99))
                    (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 6)) (i32.const 116)))))))))
      (then (return (i32.const 6))))
    (if
      (i32.and (i32.eq (local.get $len) (i32.const 7))
        (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 0)) (i32.const 111))
          (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 1)) (i32.const 112))
            (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 2)) (i32.const 116))
              (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 3)) (i32.const 105))
                (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 4)) (i32.const 111))
                  (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 5)) (i32.const 110))
                    (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 6)) (i32.const 115)))))))))
      (then (return (i32.const 7))))
    (if
      (i32.and (i32.eq (local.get $len) (i32.const 5))
        (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 0)) (i32.const 116))
          (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 1)) (i32.const 114))
            (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 2)) (i32.const 97))
              (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 3)) (i32.const 99))
                (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 4)) (i32.const 101)))))))
      (then (return (i32.const 8))))
    (if
      (i32.and (i32.eq (local.get $len) (i32.const 5))
        (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 0)) (i32.const 112))
          (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 1)) (i32.const 97))
            (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 2)) (i32.const 116))
              (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 3)) (i32.const 99))
                (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 4)) (i32.const 104)))))))
      (then (return (i32.const 9))))
    i32.const 255)

  ;; bit 0: method can carry a request body; bit 1: method expects response body.
  (func (export "http_method_body_flags") (param $method_class i32) (result i32)
    (local $flags i32)
    (local.set $flags (i32.const 2))
    (if (i32.eq (local.get $method_class) (i32.const 2)) (then (local.set $flags (i32.const 0))))
    (if
      (i32.or
        (i32.or (i32.eq (local.get $method_class) (i32.const 3)) (i32.eq (local.get $method_class) (i32.const 4)))
        (i32.or (i32.eq (local.get $method_class) (i32.const 6)) (i32.or (i32.eq (local.get $method_class) (i32.const 9)) (i32.eq (local.get $method_class) (i32.const 255)))))
      (then (local.set $flags (i32.or (local.get $flags) (i32.const 1)))))
    local.get $flags)

  ;; Status classes: 0 invalid, 1 informational, 2 success, 3 redirection,
  ;; 4 client error, 5 server error.
  (func (export "http_status_classify") (param $code i32) (result i32)
    (if (i32.lt_u (local.get $code) (i32.const 100)) (then (return (i32.const 0))))
    (if (i32.ge_u (local.get $code) (i32.const 600)) (then (return (i32.const 0))))
    (i32.div_u (local.get $code) (i32.const 100)))

  ;; Request target classes: 0 invalid, 1 origin-form path, 2 absolute http(s),
  ;; 3 asterisk, 4 relative target accepted by node builder normalization.
  (func (export "http_request_target_classify") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $b i32)
    (if (i32.eqz (local.get $len)) (then (return (i32.const 0))))
    (block $scan_done
      (loop $scan
        (if (i32.ge_u (local.get $i) (local.get $len)) (then (br $scan_done)))
        (local.set $b (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
        (if (call $is_ctl_or_space (local.get $b)) (then (return (i32.const 0))))
        (if (i32.eq (local.get $b) (i32.const 35)) (then (return (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        br $scan))
    (if
      (i32.and (i32.eq (local.get $len) (i32.const 1)) (i32.eq (i32.load8_u (local.get $ptr)) (i32.const 42)))
      (then (return (i32.const 3))))
    (if (i32.eq (i32.load8_u (local.get $ptr)) (i32.const 47)) (then (return (i32.const 1))))
    (if
      (i32.and
        (i32.ge_u (local.get $len) (i32.const 7))
        (i32.and
          (i32.and
            (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 0)) (i32.const 104))
              (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 1)) (i32.const 116)))
            (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 2)) (i32.const 116))
              (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 3)) (i32.const 112))))
          (i32.or
            (i32.and
              (i32.eq (i32.load8_u (i32.add (local.get $ptr) (i32.const 4))) (i32.const 58))
              (i32.and
                (i32.eq (i32.load8_u (i32.add (local.get $ptr) (i32.const 5))) (i32.const 47))
                (i32.eq (i32.load8_u (i32.add (local.get $ptr) (i32.const 6))) (i32.const 47))))
            (i32.and
              (i32.ge_u (local.get $len) (i32.const 8))
              (i32.and
                (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 4)) (i32.const 115))
                (i32.and
                  (i32.eq (i32.load8_u (i32.add (local.get $ptr) (i32.const 5))) (i32.const 58))
                  (i32.and
                    (i32.eq (i32.load8_u (i32.add (local.get $ptr) (i32.const 6))) (i32.const 47))
                    (i32.eq (i32.load8_u (i32.add (local.get $ptr) (i32.const 7))) (i32.const 47)))))))))
      (then (return (i32.const 2))))
    i32.const 4)

  ;; HTTP/1 body states: 0 no body, 1 fixed empty, 2 fixed length,
  ;; 3 chunked, 4 close-delimited.
  (func (export "http1_body_state")
    (param $method_class i32) (param $status_code i32) (param $has_chunked i32)
    (param $content_length_present i32) (param $content_length_low32 i32)
    (result i32)
    (if (i32.eq (local.get $method_class) (i32.const 2)) (then (return (i32.const 0))))
    (if (i32.and (i32.ge_u (local.get $status_code) (i32.const 100)) (i32.lt_u (local.get $status_code) (i32.const 200)))
      (then (return (i32.const 0))))
    (if (i32.or (i32.eq (local.get $status_code) (i32.const 204)) (i32.eq (local.get $status_code) (i32.const 304)))
      (then (return (i32.const 0))))
    (if (local.get $has_chunked) (then (return (i32.const 3))))
    (if (local.get $content_length_present)
      (then
        (if (i32.eqz (local.get $content_length_low32))
          (then (return (i32.const 1)))
          (else (return (i32.const 2))))))
    i32.const 4)

  (func (param $frame_type i32) (result i32)
    (if (result i32) (i32.le_u (local.get $frame_type) (i32.const 9))
      (then (local.get $frame_type))
      (else (i32.const 255))))

  (func (export "http2_error_classify") (param $code i32) (result i32)
    (if
      (i32.or
        (i32.or (i32.le_u (local.get $code) (i32.const 9)) (i32.eq (local.get $code) (i32.const 11)))
        (i32.or (i32.eq (local.get $code) (i32.const 12)) (i32.eq (local.get $code) (i32.const 13))))
      (then (return (local.get $code))))
    i32.const 2)

  ;; Returns HTTP/2 error code: 0 ok, 1 PROTOCOL_ERROR, 6 FRAME_SIZE_ERROR.
  (func (export "http2_frame_semantics")
    (param $frame_class i32) (param $flags i32) (param $stream_id i32)
    (param $payload_len i32) (param $first_u32 i32)
    (result i32)
    (if (i32.eq (local.get $frame_class) (i32.const 255)) (then (return (i32.const 0))))
    (if
      (i32.or
        (i32.or (i32.eq (local.get $frame_class) (i32.const 0)) (i32.eq (local.get $frame_class) (i32.const 1)))
        (i32.or (i32.eq (local.get $frame_class) (i32.const 2)) (i32.or (i32.eq (local.get $frame_class) (i32.const 3)) (i32.eq (local.get $frame_class) (i32.const 9)))))
      (then (if (i32.eqz (local.get $stream_id)) (then (return (i32.const 1))))))
    (if (i32.eq (local.get $frame_class) (i32.const 2))
      (then (if (i32.ne (local.get $payload_len) (i32.const 5)) (then (return (i32.const 6))))))
    (if (i32.eq (local.get $frame_class) (i32.const 3))
      (then (if (i32.ne (local.get $payload_len) (i32.const 4)) (then (return (i32.const 6))))))
    (if (i32.eq (local.get $frame_class) (i32.const 4))
      (then
        (if (i32.ne (local.get $stream_id) (i32.const 0)) (then (return (i32.const 1))))
        (if (i32.and (i32.and (local.get $flags) (i32.const 1)) (i32.ne (local.get $payload_len) (i32.const 0)))
          (then (return (i32.const 6))))
        (if (i32.and (i32.eqz (i32.and (local.get $flags) (i32.const 1))) (i32.ne (i32.rem_u (local.get $payload_len) (i32.const 6)) (i32.const 0)))
          (then (return (i32.const 6))))))
    (if (i32.eq (local.get $frame_class) (i32.const 5))
      (then
        (if (i32.eqz (local.get $stream_id)) (then (return (i32.const 1))))
        (if (i32.ge_u (local.get $payload_len) (i32.const 4))
          (then
            (if
              (i32.or
                (i32.eqz (i32.and (local.get $first_u32) (i32.const 0x7fffffff)))
                (i32.eqz (i32.and (local.get $first_u32) (i32.const 1))))
              (then (return (i32.const 1))))))))
    (if (i32.eq (local.get $frame_class) (i32.const 6))
      (then
        (if (i32.ne (local.get $stream_id) (i32.const 0)) (then (return (i32.const 1))))
        (if (i32.ne (local.get $payload_len) (i32.const 8)) (then (return (i32.const 6))))))
    (if (i32.eq (local.get $frame_class) (i32.const 7))
      (then
        (if (i32.ne (local.get $stream_id) (i32.const 0)) (then (return (i32.const 1))))
        (if (i32.lt_u (local.get $payload_len) (i32.const 8)) (then (return (i32.const 6))))))
    (if (i32.eq (local.get $frame_class) (i32.const 8))
      (then
        (if (i32.ne (local.get $payload_len) (i32.const 4)) (then (return (i32.const 6))))
        (if (i32.eqz (i32.and (local.get $first_u32) (i32.const 0x7fffffff))) (then (return (i32.const 1))))))
    i32.const 0)

  ;; HTTP/3 frame classes: 0 DATA, 1 HEADERS, 2 SETTINGS, 3 CANCEL_PUSH,
  ;; 4 PUSH_PROMISE, 5 MAX_PUSH_ID, 6 GOAWAY, 7 STREAMS_BLOCKED,
  ;; 8 reserved/grease, 9 unknown extension.
  (func (param $value i64) (result i32)
    (if (i64.eq (local.get $value) (i64.const 0)) (then (return (i32.const 0))))
    (if (i64.eq (local.get $value) (i64.const 1)) (then (return (i32.const 1))))
    (if (i64.eq (local.get $value) (i64.const 3)) (then (return (i32.const 3))))
    (if (i64.eq (local.get $value) (i64.const 4)) (then (return (i32.const 2))))
    (if (i64.eq (local.get $value) (i64.const 5)) (then (return (i32.const 4))))
    (if (i64.eq (local.get $value) (i64.const 7)) (then (return (i32.const 5))))
    (if (i64.eq (local.get $value) (i64.const 8)) (then (return (i32.const 6))))
    (if (i64.eq (local.get $value) (i64.const 9)) (then (return (i32.const 7))))
    (if
      (i32.or
        (i64.eq (i64.rem_u (local.get $value) (i64.const 31)) (i64.const 2))
        (i64.eq (i64.rem_u (local.get $value) (i64.const 31)) (i64.const 6)))
      (then (return (i32.const 8))))
    i32.const 9)

  ;; Returns 0 ok or 10 H3_FRAME_UNEXPECTED.
  ;; stream_kind: 0 control, 1 request/response, 2 push, 3 qpack.
  (func (export "http3_frame_stream_semantics") (param $stream_kind i32) (param $frame_class i32) (result i32)
    (if (i32.eq (local.get $stream_kind) (i32.const 0))
      (then
        (if
          (i32.or
            (i32.or (i32.eq (local.get $frame_class) (i32.const 2)) (i32.eq (local.get $frame_class) (i32.const 6)))
            (i32.or (i32.eq (local.get $frame_class) (i32.const 5)) (i32.eq (local.get $frame_class) (i32.const 3))))
          (then (return (i32.const 0)))
          (else (return (i32.const 10))))))
    (if
      (i32.or (i32.eq (local.get $stream_kind) (i32.const 1)) (i32.eq (local.get $stream_kind) (i32.const 2)))
      (then
        (if (i32.or (i32.eq (local.get $frame_class) (i32.const 0)) (i32.eq (local.get $frame_class) (i32.const 1)))
          (then (return (i32.const 0)))
          (else (return (i32.const 10))))))
    i32.const 10)

  ;; Route state from node HTTP/3 path/goaway fields:
  ;; 0 no active path, 1 active path and stream allowed, 2 active but going away
  ;; still allows this stream, 3 active but GOAWAY rejects this stream.
  (func (export "http3_route_state")
    (param $active_path_present i32) (param $going_away_present i32)
    (param $stream_id i64) (param $goaway_stream_id i64)
    (result i32)
    (if (i32.eqz (local.get $active_path_present)) (then (return (i32.const 0))))
    (if (i32.eqz (local.get $going_away_present)) (then (return (i32.const 1))))
    (if (i64.le_u (local.get $stream_id) (local.get $goaway_stream_id))
      (then (return (i32.const 2))))
    i32.const 3)

  (func (export "http3_status_classify") (param $code i32) (result i32)
    (call $http_status_classify_public (local.get $code)))

  (func $http_status_classify_public (param $code i32) (result i32)
    (if (i32.lt_u (local.get $code) (i32.const 100)) (then (return (i32.const 0))))
    (if (i32.ge_u (local.get $code) (i32.const 600)) (then (return (i32.const 0))))
    (i32.div_u (local.get $code) (i32.const 100)))


;; Decode output record, little-endian:
  ;; 0:u32 flags_high_bits
  ;; 4:u32 consumed
  ;; 8:u64 value
  (func $prefix_decode
    (param $in_ptr i32) (param $in_len i32) (param $prefix_bits i32) (param $out_ptr i32)
    (param $octet_limit i32)
    (result i32)
    (local $first i32)
    (local $mask i32)
    (local $flags i32)
    (local $value i64)
    (local $consumed i32)
    (local $shift i32)
    (local $byte i32)
    (if
      (i32.or
        (i32.lt_u (local.get $prefix_bits) (i32.const 1))
        (i32.gt_u (local.get $prefix_bits) (i32.const 8)))
      (then (return (i32.const 3))))
    (if (i32.eqz (local.get $in_len))
      (then (return (i32.const 1))))
    (local.set $mask (call $prefix_mask (local.get $prefix_bits)))
    (local.set $first (i32.load8_u (local.get $in_ptr)))
    (local.set $flags (i32.shr_u (local.get $first) (local.get $prefix_bits)))
    (local.set $value (i64.extend_i32_u (i32.and (local.get $first) (local.get $mask))))
    (if (i64.lt_u (local.get $value) (i64.extend_i32_u (local.get $mask)))
      (then
        (i32.store (local.get $out_ptr) (local.get $flags))
        (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (i32.const 1))
        (i64.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $value))
        (return (i32.const 0))))
    (local.set $consumed (i32.const 1))
    (loop $again
      (if (i32.ge_u (local.get $consumed) (local.get $octet_limit))
        (then (return (i32.const 4))))
      (if (i32.ge_u (local.get $consumed) (local.get $in_len))
        (then (return (i32.const 1))))
      (local.set $byte (i32.load8_u (i32.add (local.get $in_ptr) (local.get $consumed))))
      (if
        (i32.and
          (i32.ge_u (local.get $shift) (i32.const 63))
          (i32.ne (i32.and (local.get $byte) (i32.const 127)) (i32.const 0)))
        (then (return (i32.const 4))))
      (local.set $value
        (i64.add
          (local.get $value)
          (i64.shl
            (i64.extend_i32_u (i32.and (local.get $byte) (i32.const 127)))
            (i64.extend_i32_u (local.get $shift)))))
      (local.set $consumed (i32.add (local.get $consumed) (i32.const 1)))
      (if (i32.eqz (i32.and (local.get $byte) (i32.const 128)))
        (then
          (i32.store (local.get $out_ptr) (local.get $flags))
          (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $consumed))
          (i64.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $value))
          (return (i32.const 0))))
      (local.set $shift (i32.add (local.get $shift) (i32.const 7)))
      (br $again))
    (i32.const 1))

(func $http_prefix_encode
    (param $value i64) (param $prefix_bits i32) (param $prefix_high_bits i32)
    (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $mask i32)
    (local $flags i32)
    (local $remaining i64)
    (local $written i32)
    (local $byte i32)
    (if
      (i32.or
        (i32.lt_u (local.get $prefix_bits) (i32.const 1))
        (i32.gt_u (local.get $prefix_bits) (i32.const 8)))
      (then (return (call $pack (i32.const 3) (i32.const 0)))))
    (if (i32.eqz (local.get $out_cap))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))
    (local.set $mask (call $prefix_mask (local.get $prefix_bits)))
    (if
      (i32.ge_u
        (local.get $prefix_high_bits)
        (if (result i32)
          (i32.eq (local.get $prefix_bits) (i32.const 8))
          (then (i32.const 1))
          (else (i32.shl (i32.const 1) (i32.sub (i32.const 8) (local.get $prefix_bits))))))
      (then (return (call $pack (i32.const 3) (i32.const 0)))))
    (local.set $flags (i32.shl (local.get $prefix_high_bits) (local.get $prefix_bits)))
    (if (i64.lt_u (local.get $value) (i64.extend_i32_u (local.get $mask)))
      (then
        (i32.store8
          (local.get $out_ptr)
          (i32.or (local.get $flags) (i32.wrap_i64 (local.get $value))))
        (return (call $pack (i32.const 0) (i32.const 1)))))
    (i32.store8 (local.get $out_ptr) (i32.or (local.get $flags) (local.get $mask)))
    (local.set $remaining (i64.sub (local.get $value) (i64.extend_i32_u (local.get $mask))))
    (local.set $written (i32.const 1))
    (loop $again
      (if (i32.ge_u (local.get $written) (local.get $out_cap))
        (then (return (call $pack (i32.const 2) (local.get $written)))))
      (local.set $byte (i32.and (i32.wrap_i64 (local.get $remaining)) (i32.const 127)))
      (local.set $remaining (i64.shr_u (local.get $remaining) (i64.const 7)))
      (if (i64.ne (local.get $remaining) (i64.const 0))
        (then (local.set $byte (i32.or (local.get $byte) (i32.const 128)))))
      (i32.store8 (i32.add (local.get $out_ptr) (local.get $written)) (local.get $byte))
      (local.set $written (i32.add (local.get $written) (i32.const 1)))
      (br_if $again (i64.ne (local.get $remaining) (i64.const 0))))
    (call $pack (i32.const 0) (local.get $written)))

  (func (export "hpack_prefix_int_decode")
    (param $in_ptr i32) (param $in_len i32) (param $prefix_bits i32) (param $out_ptr i32)
    (result i32)
    (call $prefix_decode
      (local.get $in_ptr) (local.get $in_len) (local.get $prefix_bits) (local.get $out_ptr)
      (i32.const 5)))

  (func (export "hpack_prefix_int_encode")
    (param $value i64) (param $prefix_bits i32) (param $prefix_high_bits i32)
    (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (call $http_prefix_encode
      (local.get $value) (local.get $prefix_bits) (local.get $prefix_high_bits)
      (local.get $out_ptr) (local.get $out_cap)))

  (func (export "qpack_prefix_int_decode")
    (param $in_ptr i32) (param $in_len i32) (param $prefix_bits i32) (param $out_ptr i32)
    (result i32)
    (call $prefix_decode
      (local.get $in_ptr) (local.get $in_len) (local.get $prefix_bits) (local.get $out_ptr)
      (i32.const 10)))

  (func (export "qpack_prefix_int_encode")
    (param $value i64) (param $prefix_bits i32) (param $prefix_high_bits i32)
    (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (call $http_prefix_encode
      (local.get $value) (local.get $prefix_bits) (local.get $prefix_high_bits)
      (local.get $out_ptr) (local.get $out_cap)))

(data (i32.const 65500) "18446744073709551615")
  (data (i32.const 65440) "content-length")
  (data (i32.const 65456) "transfer-encoding")
  (data (i32.const 65480) "chunked")
(data (i32.const 65380) "18446744073709551615")
  (data (i32.const 65408) "content-length")
  (data (i32.const 65424) "transfer-encoding")
  (data (i32.const 65448) "chunked")
  (data (i32.const 65456) "host")
  (data (i32.const 8192) "\00\00\00\00\00\00\00\00\00\02\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\02\21\20\21\21\21\21\21\20\20\21\21\20\21\21\20\35\35\35\35\35\35\35\35\35\35\20\20\20\20\20\20\20\39\39\39\39\39\39\29\29\29\29\29\29\29\29\29\29\29\29\29\29\29\29\29\29\29\29\20\20\20\21\21\21\39\39\39\39\39\39\29\29\29\29\29\29\29\29\29\29\29\29\29\29\29\29\29\29\29\29\20\21\20\21\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00")
  (data (i32.const 1200) "Monday, Tuesday, Wednesday, Thursday, Friday, Saturday, Sunday,  GMT")
  (data (i32.const 1280) "MonTueWedThuFriSatSun")
  (data (i32.const 1304) "JanFebMarAprMayJunJulAugSepOctNovDec")
