  ;; Base64url encoding (RFC 4648 §5) — no padding, -_ alphabet.

    ;; Standard ID removed — merged into single module

  (func $m86b64url_char (param $n i32) (result i32)
    local.get $n
    i32.const 26
    i32.lt_u
    if (result i32)
      local.get $n
      i32.const 65
      i32.add
    else
      local.get $n
      i32.const 52
      i32.lt_u
      if (result i32)
        local.get $n
        i32.const 26
        i32.sub
        i32.const 97
        i32.add
      else
        local.get $n
        i32.const 62
        i32.lt_u
        if (result i32)
          local.get $n
          i32.const 52
          i32.sub
          i32.const 48
          i32.add
        else
          local.get $n
          i32.const 62
          i32.eq
          if (result i32)
            i32.const 45           ;; '-' instead of '+'
          else
            i32.const 95           ;; '_' instead of '/'
          end
        end
      end
    end)

  ;; base64url_encode(src_ptr, src_len, dst_ptr, dcap) -> status:i32, written:i32 packed as i64
  ;; No padding. Returns (0,written) on success, (negative,0) on error.
  (func $b64_encode (export "base64url_encode") (param $src i32) (param $slen i32) (param $dst i32) (param $dcap i32) (result i64)
    (local $i i32)
    (local $o i32)
    (local $b0 i32)
    (local $b1 i32)
    (local $b2 i32)
    (local $triple i32)

    ;; estimate output: ((slen + 2) / 3) * 4
    local.get $slen
    i32.const 2
    i32.add
    i32.const 3
    i32.div_u
    i32.const 4
    i32.mul
    local.set $o
    local.get $dcap
    local.get $o
    i32.lt_u
    if
      i64.const -1
      return
    end

    i32.const 0
    local.set $o

    block $done
    loop $main
      local.get $i
      local.get $slen
      i32.ge_u
      br_if $done

      ;; load 3 bytes
      local.get $src
      local.get $i
      i32.add
      i32.load8_u
      local.set $b0
      local.get $i
      i32.const 1
      i32.add
      local.tee $i
      local.get $slen
      i32.ge_u
      if
        ;; 1 byte left → 2 chars
        local.get $b0
        i32.const 2
        i32.shl
        i32.const 255
        i32.and
        call $m86b64url_char
        local.set $b2
        local.get $dst
        local.get $o
        i32.add
        local.get $b2
        i32.store8
        local.get $o
        i32.const 1
        i32.add
        local.set $o
        local.get $b0
        i32.const 4
        i32.shr_u
        i32.const 15
        i32.and
        call $m86b64url_char
        local.set $b2
        local.get $dst
        local.get $o
        i32.add
        local.get $b2
        i32.store8
        local.get $o
        i32.const 1
        i32.add
        local.set $o
        br $done
      end

      local.get $src
      local.get $i
      i32.add
      i32.load8_u
      local.set $b1
      local.get $i
      i32.const 1
      i32.add
      local.tee $i
      local.get $slen
      i32.ge_u
      if
        ;; 2 bytes → 3 chars
        local.get $b0
        i32.const 2
        i32.shl
        local.get $b1
        i32.const 6
        i32.shr_u
        i32.or
        i32.const 255
        i32.and
        call $m86b64url_char
        local.set $b2
        local.get $dst
        local.get $o
        i32.add
        local.get $b2
        i32.store8
        local.get $o
        i32.const 1
        i32.add
        local.set $o
        local.get $b1
        i32.const 2
        i32.shl
        i32.const 63
        i32.and
        call $m86b64url_char
        local.set $b2
        local.get $dst
        local.get $o
        i32.add
        local.get $b2
        i32.store8
        local.get $o
        i32.const 1
        i32.add
        local.set $o
        local.get $b1
        i32.const 4
        i32.shr_u
        i32.const 15
        i32.and
        call $m86b64url_char
        local.set $b2
        local.get $dst
        local.get $o
        i32.add
        local.get $b2
        i32.store8
        local.get $o
        i32.const 1
        i32.add
        local.set $o
        br $done
      end

      ;; 3 bytes → 4 chars
      local.get $src
      local.get $i
      i32.add
      i32.load8_u
      local.set $b2
      local.get $i
      i32.const 1
      i32.add
      local.set $i

      local.get $b0
      i32.const 16
      i32.shl
      local.get $b1
      i32.const 8
      i32.shl
      i32.or
      local.get $b2
      i32.or
      local.set $triple

      local.get $triple
      i32.const 18
      i32.shr_u
      i32.const 63
      i32.and
      call $m86b64url_char
      local.set $b0
      local.get $dst
      local.get $o
      i32.add
      local.get $b0
      i32.store8
      local.get $o
      i32.const 1
      i32.add
      local.set $o

      local.get $triple
      i32.const 12
      i32.shr_u
      i32.const 63
      i32.and
      call $m86b64url_char
      local.set $b0
      local.get $dst
      local.get $o
      i32.add
      local.get $b0
      i32.store8
      local.get $o
      i32.const 1
      i32.add
      local.set $o

      local.get $triple
      i32.const 6
      i32.shr_u
      i32.const 63
      i32.and
      call $m86b64url_char
      local.set $b0
      local.get $dst
      local.get $o
      i32.add
      local.get $b0
      i32.store8
      local.get $o
      i32.const 1
      i32.add
      local.set $o

      local.get $triple
      i32.const 63
      i32.and
      call $m86b64url_char
      local.set $b0
      local.get $dst
      local.get $o
      i32.add
      local.get $b0
      i32.store8
      local.get $o
      i32.const 1
      i32.add
      local.set $o

      br $main
    end
    end

    ;; result = status=0, written=o
    i64.const 0
    local.get $o
    i64.extend_i32_u
    i64.const 32
    i64.shl
    i64.or)

  ;; base64url_decode(src_ptr, src_len, dst_ptr, dcap) -> status:i32, written:i32 packed as i64
  ;; Decodes base64url (no padding). Rejects invalid chars.
  (func $b64_decode (export "base64url_decode") (param $src i32) (param $slen i32) (param $dst i32) (param $dcap i32) (result i64)
    (local $i i32)
    (local $o i32)
    (local $c0 i32)
    (local $c1 i32)
    (local $c2 i32)
    (local $c3 i32)
    (local $quad i32)

    ;; estimate max output: (slen / 4) * 3
    local.get $slen
    i32.const 3
    i32.mul
    i32.const 2
    i32.shr_u
    local.set $o
    local.get $dcap
    local.get $o
    i32.lt_u
    if
      i64.const -2
      return
    end

    i32.const 0
    local.set $o

    block $done
    loop $main
      local.get $i
      i32.const 4
      i32.add
      local.tee $i
      local.get $slen
      i32.gt_u
      br_if $done

      ;; decode 4 chars
      local.get $src
      local.get $i
      i32.const 4
      i32.sub
      i32.add
      i32.load8_u
      call $m86b64url_value
      local.tee $c0
      i32.const 0
      i32.lt_s
      br_if $done

      local.get $src
      local.get $i
      i32.const 3
      i32.sub
      i32.add
      i32.load8_u
      call $m86b64url_value
      local.tee $c1
      i32.const 0
      i32.lt_s
      br_if $done

      local.get $src
      local.get $i
      i32.const 2
      i32.sub
      i32.add
      i32.load8_u
      call $m86b64url_value
      local.tee $c2
      i32.const 0
      i32.lt_s
      br_if $done

      local.get $src
      local.get $i
      i32.const 1
      i32.sub
      i32.add
      i32.load8_u
      call $m86b64url_value
      local.tee $c3
      i32.const 0
      i32.lt_s
      br_if $done

      ;; reconstruct 4*6 = 24 bits → 3 bytes
      local.get $c0
      i32.const 18
      i32.shl
      local.get $c1
      i32.const 12
      i32.shl
      i32.or
      local.get $c2
      i32.const 6
      i32.shl
      i32.or
      local.get $c3
      i32.or
      local.set $quad

      local.get $dst
      local.get $o
      i32.add
      local.get $quad
      i32.const 16
      i32.shr_u
      i32.const 255
      i32.and
      i32.store8
      local.get $o
      i32.const 1
      i32.add
      local.set $o

      local.get $dst
      local.get $o
      i32.add
      local.get $quad
      i32.const 8
      i32.shr_u
      i32.const 255
      i32.and
      i32.store8
      local.get $o
      i32.const 1
      i32.add
      local.set $o

      local.get $dst
      local.get $o
      i32.add
      local.get $quad
      i32.const 255
      i32.and
      i32.store8
      local.get $o
      i32.const 1
      i32.add
      local.set $o

      br $main
    end
    end

    i64.const 0
    local.get $o
    i64.extend_i32_u
    i64.const 32
    i64.shl
    i64.or)

  (func $m86b64url_value (param $c i32) (result i32)
    local.get $c
    i32.const 65
    i32.ge_u
    local.get $c
    i32.const 90
    i32.le_u
    i32.and
    if (result i32)
      local.get $c
      i32.const 65
      i32.sub
    else
      local.get $c
      i32.const 97
      i32.ge_u
      local.get $c
      i32.const 122
      i32.le_u
      i32.and
      if (result i32)
        local.get $c
        i32.const 97
        i32.sub
        i32.const 26
        i32.add
      else
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
          i32.const 52
          i32.add
        else
          local.get $c
          i32.const 45           ;; '-'
          i32.eq
          if (result i32)
            i32.const 62
          else
            local.get $c
          i32.const 95         ;; '_'
          i32.eq
          if (result i32)
            i32.const 63
          else
            i32.const -1
          end
        end
      end
    end
    end
  )