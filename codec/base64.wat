(func $m85b64_char (param $n i32) (result i32)
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
            i32.const 43
          else
            i32.const 47
          end
        end
      end
    end)

  (func $m85b64_value (param $c i32) (result i32)
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
          i32.const 43
          i32.eq
          if (result i32)
            i32.const 62
          else
            local.get $c
            i32.const 47
            i32.eq
            if (result i32)
              i32.const 63
            else
              i32.const -1
            end
          end
        end
      end
    end)

  (func $base64_encode
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $groups i32)
    (local $rem i32)
    (local $out_len i32)
    (local $i i32)
    (local $j i32)
    (local $b0 i32)
    (local $b1 i32)
    (local $b2 i32)
    local.get $in_len
    i32.const 3
    i32.div_u
    local.set $groups
    local.get $in_len
    i32.const 3
    i32.rem_u
    local.set $rem
    local.get $groups
    i32.const 1073741823
    i32.gt_u
    if (result i64)
      i32.const 4
      i32.const 0
      call $pack
    else
      local.get $groups
      i32.const 1073741823
      i32.eq
      local.get $rem
      i32.const 0
      i32.ne
      i32.and
      if (result i64)
        i32.const 4
        i32.const 0
        call $pack
      else
        local.get $groups
        local.get $rem
        i32.const 0
        i32.ne
        i32.add
        i32.const 2
        i32.shl
        local.set $out_len
        local.get $out_len
        local.get $out_cap
        i32.gt_u
        if (result i64)
          i32.const 2
          i32.const 0
          call $pack
        else
          loop $loop
            local.get $i
            local.get $groups
            i32.lt_u
            if
              local.get $in_ptr
              local.get $i
              i32.const 3
              i32.mul
              i32.add
              i32.load8_u
              local.set $b0
              local.get $in_ptr
              local.get $i
              i32.const 3
              i32.mul
              i32.add
              i32.const 1
              i32.add
              i32.load8_u
              local.set $b1
              local.get $in_ptr
              local.get $i
              i32.const 3
              i32.mul
              i32.add
              i32.const 2
              i32.add
              i32.load8_u
              local.set $b2
              local.get $out_ptr
              local.get $j
              i32.add
              local.get $b0
              i32.const 2
              i32.shr_u
              call $m85b64_char
              i32.store8
              local.get $out_ptr
              local.get $j
              i32.add
              i32.const 1
              i32.add
              local.get $b0
              i32.const 4
              i32.shl
              local.get $b1
              i32.const 4
              i32.shr_u
              i32.or
              i32.const 63
              i32.and
              call $m85b64_char
              i32.store8
              local.get $out_ptr
              local.get $j
              i32.add
              i32.const 2
              i32.add
              local.get $b1
              i32.const 2
              i32.shl
              local.get $b2
              i32.const 6
              i32.shr_u
              i32.or
              i32.const 63
              i32.and
              call $m85b64_char
              i32.store8
              local.get $out_ptr
              local.get $j
              i32.add
              i32.const 3
              i32.add
              local.get $b2
              i32.const 63
              i32.and
              call $m85b64_char
              i32.store8
              local.get $i
              i32.const 1
              i32.add
              local.set $i
              local.get $j
              i32.const 4
              i32.add
              local.set $j
              br $loop
            end
          end
          local.get $rem
          i32.const 1
          i32.eq
          if
            local.get $in_ptr
            local.get $groups
            i32.const 3
            i32.mul
            i32.add
            i32.load8_u
            local.set $b0
            local.get $out_ptr
            local.get $j
            i32.add
            local.get $b0
            i32.const 2
            i32.shr_u
            call $m85b64_char
            i32.store8
            local.get $out_ptr
            local.get $j
            i32.add
            i32.const 1
            i32.add
            local.get $b0
            i32.const 4
            i32.shl
            i32.const 63
            i32.and
            call $m85b64_char
            i32.store8
            local.get $out_ptr
            local.get $j
            i32.add
            i32.const 2
            i32.add
            i32.const 61
            i32.store8
            local.get $out_ptr
            local.get $j
            i32.add
            i32.const 3
            i32.add
            i32.const 61
            i32.store8
          end
          local.get $rem
          i32.const 2
          i32.eq
          if
            local.get $in_ptr
            local.get $groups
            i32.const 3
            i32.mul
            i32.add
            i32.load8_u
            local.set $b0
            local.get $in_ptr
            local.get $groups
            i32.const 3
            i32.mul
            i32.add
            i32.const 1
            i32.add
            i32.load8_u
            local.set $b1
            local.get $out_ptr
            local.get $j
            i32.add
            local.get $b0
            i32.const 2
            i32.shr_u
            call $m85b64_char
            i32.store8
            local.get $out_ptr
            local.get $j
            i32.add
            i32.const 1
            i32.add
            local.get $b0
            i32.const 4
            i32.shl
            local.get $b1
            i32.const 4
            i32.shr_u
            i32.or
            i32.const 63
            i32.and
            call $m85b64_char
            i32.store8
            local.get $out_ptr
            local.get $j
            i32.add
            i32.const 2
            i32.add
            local.get $b1
            i32.const 2
            i32.shl
            i32.const 63
            i32.and
            call $m85b64_char
            i32.store8
            local.get $out_ptr
            local.get $j
            i32.add
            i32.const 3
            i32.add
            i32.const 61
            i32.store8
          end
          i32.const 0
          local.get $out_len
          call $pack
        end
      end
    end)

  (func $base64_decode (export "base64_standard_decode")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $groups i32)
    (local $full_groups i32)
    (local $pad i32)
    (local $out_len i32)
    (local $i i32)
    (local $j i32)
    (local $v0 i32)
    (local $v1 i32)
    (local $v2 i32)
    (local $v3 i32)
    local.get $in_len
    i32.const 3
    i32.and
    if (result i64)
      i32.const 3
      i32.const 0
      call $pack
    else
      local.get $in_len
      i32.const 4
      i32.div_u
      local.set $groups
      local.get $groups
      local.set $full_groups
      local.get $in_len
      if
        local.get $in_ptr
        local.get $in_len
        i32.add
        i32.const 1
        i32.sub
        i32.load8_u
        i32.const 61
        i32.eq
        if
          i32.const 1
          local.set $pad
          local.get $in_ptr
          local.get $in_len
          i32.add
          i32.const 2
          i32.sub
          i32.load8_u
          i32.const 61
          i32.eq
          if
            i32.const 2
            local.set $pad
          end
        end
      end
      local.get $pad
      if
        local.get $groups
        i32.const 0
        i32.eq
        if
          i32.const 3
          i32.const 0
          call $pack
          return
        end
        local.get $groups
        i32.const 1
        i32.sub
        local.set $full_groups
      end
      local.get $groups
      i32.const 3
      i32.mul
      local.get $pad
      i32.sub
      local.set $out_len
      local.get $out_len
      local.get $out_cap
      i32.gt_u
      if (result i64)
        i32.const 2
        i32.const 0
        call $pack
      else
        loop $loop
          local.get $i
          local.get $full_groups
          i32.lt_u
          if
            local.get $in_ptr
            local.get $i
            i32.const 4
            i32.mul
            i32.add
            i32.load8_u
            call $m85b64_value
            local.tee $v0
            i32.const 0
            i32.lt_s
            if
              i32.const 3
              i32.const 0
              call $pack
              return
            end
            local.get $in_ptr
            local.get $i
            i32.const 4
            i32.mul
            i32.add
            i32.const 1
            i32.add
            i32.load8_u
            call $m85b64_value
            local.tee $v1
            i32.const 0
            i32.lt_s
            if
              i32.const 3
              i32.const 0
              call $pack
              return
            end
            local.get $in_ptr
            local.get $i
            i32.const 4
            i32.mul
            i32.add
            i32.const 2
            i32.add
            i32.load8_u
            call $m85b64_value
            local.tee $v2
            i32.const 0
            i32.lt_s
            if
              i32.const 3
              i32.const 0
              call $pack
              return
            end
            local.get $in_ptr
            local.get $i
            i32.const 4
            i32.mul
            i32.add
            i32.const 3
            i32.add
            i32.load8_u
            call $m85b64_value
            local.tee $v3
            i32.const 0
            i32.lt_s
            if
              i32.const 3
              i32.const 0
              call $pack
              return
            end
            local.get $out_ptr
            local.get $j
            i32.add
            local.get $v0
            i32.const 2
            i32.shl
            local.get $v1
            i32.const 4
            i32.shr_u
            i32.or
            i32.store8
            local.get $out_ptr
            local.get $j
            i32.add
            i32.const 1
            i32.add
            local.get $v1
            i32.const 4
            i32.shl
            local.get $v2
            i32.const 2
            i32.shr_u
            i32.or
            i32.store8
            local.get $out_ptr
            local.get $j
            i32.add
            i32.const 2
            i32.add
            local.get $v2
            i32.const 6
            i32.shl
            local.get $v3
            i32.or
            i32.store8
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            local.get $j
            i32.const 3
            i32.add
            local.set $j
            br $loop
          end
        end
        local.get $pad
        i32.const 1
        i32.eq
        if
          local.get $in_ptr
          local.get $full_groups
          i32.const 4
          i32.mul
          i32.add
          i32.load8_u
          call $m85b64_value
          local.tee $v0
          i32.const 0
          i32.lt_s
          if
            i32.const 3
            i32.const 0
            call $pack
            return
          end
          local.get $in_ptr
          local.get $full_groups
          i32.const 4
          i32.mul
          i32.add
          i32.const 1
          i32.add
          i32.load8_u
          call $m85b64_value
          local.tee $v1
          i32.const 0
          i32.lt_s
          if
            i32.const 3
            i32.const 0
            call $pack
            return
          end
          local.get $in_ptr
          local.get $full_groups
          i32.const 4
          i32.mul
          i32.add
          i32.const 2
          i32.add
          i32.load8_u
          call $m85b64_value
          local.tee $v2
          i32.const 0
          i32.lt_s
          if
            i32.const 3
            i32.const 0
            call $pack
            return
          end
          local.get $v2
          i32.const 3
          i32.and
          if
            i32.const 3
            i32.const 0
            call $pack
            return
          end
          local.get $out_ptr
          local.get $j
          i32.add
          local.get $v0
          i32.const 2
          i32.shl
          local.get $v1
          i32.const 4
          i32.shr_u
          i32.or
          i32.store8
          local.get $out_ptr
          local.get $j
          i32.add
          i32.const 1
          i32.add
          local.get $v1
          i32.const 4
          i32.shl
          local.get $v2
          i32.const 2
          i32.shr_u
          i32.or
          i32.store8
        end
        local.get $pad
        i32.const 2
        i32.eq
        if
          local.get $in_ptr
          local.get $full_groups
          i32.const 4
          i32.mul
          i32.add
          i32.load8_u
          call $m85b64_value
          local.tee $v0
          i32.const 0
          i32.lt_s
          if
            i32.const 3
            i32.const 0
            call $pack
            return
          end
          local.get $in_ptr
          local.get $full_groups
          i32.const 4
          i32.mul
          i32.add
          i32.const 1
          i32.add
          i32.load8_u
          call $m85b64_value
          local.tee $v1
          i32.const 0
          i32.lt_s
          if
            i32.const 3
            i32.const 0
            call $pack
            return
          end
          local.get $v1
          i32.const 15
          i32.and
          if
            i32.const 3
            i32.const 0
            call $pack
            return
          end
          local.get $out_ptr
          local.get $j
          i32.add
          local.get $v0
          i32.const 2
          i32.shl
          local.get $v1
          i32.const 4
          i32.shr_u
          i32.or
          i32.store8
        end
        i32.const 0
        local.get $out_len
        call $pack
            end
    end)


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
    i32.const 0
    local.get $o
    call $pack)

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

    i32.const 0
    local.get $o
    call $pack)

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

  (func $b32hex_char (param $n i32) (result i32)
    local.get $n
    i32.const 10
    i32.lt_u
    if (result i32)
      local.get $n
      i32.const 48
      i32.add
    else
      local.get $n
      i32.const 10
      i32.sub
      i32.const 97
      i32.add
    end)

  (func $b32hex_value (param $c i32) (result i32)
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
      i32.const 97
      i32.ge_u
      local.get $c
      i32.const 118
      i32.le_u
      i32.and
      if (result i32)
        local.get $c
        i32.const 97
        i32.sub
        i32.const 10
        i32.add
      else
        local.get $c
        i32.const 65
        i32.ge_u
        local.get $c
        i32.const 86
        i32.le_u
        i32.and
        if (result i32)
          local.get $c
          i32.const 65
          i32.sub
          i32.const 10
          i32.add
        else
          i32.const -1
        end
      end
    end)

  (func $valid_unpadded_len (param $len i32) (result i32)
    local.get $len
    i32.const 8
    i32.rem_u
    i32.const 0
    i32.eq
    local.get $len
    i32.const 8
    i32.rem_u
    i32.const 2
    i32.eq
    i32.or
    local.get $len
    i32.const 8
    i32.rem_u
    i32.const 4
    i32.eq
    i32.or
    local.get $len
    i32.const 8
    i32.rem_u
    i32.const 5
    i32.eq
    i32.or
    local.get $len
    i32.const 8
    i32.rem_u
    i32.const 7
    i32.eq
    i32.or)

  (func $base32hex_encode (export "base32hex_encode")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $needed i32)
    (local $i i32)
    (local $written i32)
    (local $bits i64)
    (local $bit_len i32)
    local.get $in_len
    i32.const 536870911
    i32.gt_u
    if
      i32.const 4
      i32.const 0
      call $pack
      return
    end
    local.get $in_len
    i32.const 3
    i32.shl
    i32.const 4
    i32.add
    i32.const 5
    i32.div_u
    local.set $needed
    local.get $needed
    local.get $out_cap
    i32.gt_u
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end
    loop $bytes
      local.get $i
      local.get $in_len
      i32.lt_u
      if
        local.get $bits
        i64.const 8
        i64.shl
        local.get $in_ptr
        local.get $i
        i32.add
        i32.load8_u
        i64.extend_i32_u
        i64.or
        local.set $bits
        local.get $bit_len
        i32.const 8
        i32.add
        local.set $bit_len
        loop $chunks
          local.get $bit_len
          i32.const 5
          i32.ge_u
          if
            local.get $bit_len
            i32.const 5
            i32.sub
            local.set $bit_len
            local.get $out_ptr
            local.get $written
            i32.add
            local.get $bits
            local.get $bit_len
            i64.extend_i32_u
            i64.shr_u
            i64.const 31
            i64.and
            i32.wrap_i64
            call $b32hex_char
            i32.store8
            local.get $written
            i32.const 1
            i32.add
            local.set $written
            br $chunks
          end
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $bytes
      end
    end
    local.get $bit_len
    i32.const 0
    i32.gt_u
    if
      local.get $out_ptr
      local.get $written
      i32.add
      local.get $bits
      i32.const 5
      local.get $bit_len
      i32.sub
      i64.extend_i32_u
      i64.shl
      i64.const 31
      i64.and
      i32.wrap_i64
      call $b32hex_char
      i32.store8
      local.get $written
      i32.const 1
      i32.add
      local.set $written
    end
    i32.const 0
    local.get $written
    call $pack)

  (func $base32hex_decode (export "base32hex_decode")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $needed i32)
    (local $i i32)
    (local $written i32)
    (local $bits i64)
    (local $bit_len i32)
    (local $value i32)
    local.get $in_len
    call $valid_unpadded_len
    i32.eqz
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end
    local.get $in_len
    i32.const 858993459
    i32.gt_u
    if
      i32.const 4
      i32.const 0
      call $pack
      return
    end
    local.get $in_len
    i32.const 5
    i32.mul
    i32.const 3
    i32.shr_u
    local.set $needed
    local.get $needed
    local.get $out_cap
    i32.gt_u
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end
    loop $chars
      local.get $i
      local.get $in_len
      i32.lt_u
      if
        local.get $in_ptr
        local.get $i
        i32.add
        i32.load8_u
        call $b32hex_value
        local.set $value
        local.get $value
        i32.const 0
        i32.lt_s
        if
          i32.const 3
          i32.const 0
          call $pack
          return
        end
        local.get $bits
        i64.const 5
        i64.shl
        local.get $value
        i64.extend_i32_u
        i64.or
        local.set $bits
        local.get $bit_len
        i32.const 5
        i32.add
        local.set $bit_len
        loop $bytes
          local.get $bit_len
          i32.const 8
          i32.ge_u
          if
            local.get $bit_len
            i32.const 8
            i32.sub
            local.set $bit_len
            local.get $out_ptr
            local.get $written
            i32.add
            local.get $bits
            local.get $bit_len
            i64.extend_i32_u
            i64.shr_u
            i32.wrap_i64
            i32.store8
            local.get $written
            i32.const 1
            i32.add
            local.set $written
            br $bytes
          end
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $chars
      end
    end
    i32.const 0
    local.get $written
    call $pack)



  (func (param $ptr i32) (param $len i32) (param $offset i32) (result i64)
    (if (result i64)
      (call $bounds_check (local.get $len) (local.get $offset) (i32.const 2))
      (then
        (call $pack
          (i32.const 0)
          (i32.or
            (i32.load8_u (i32.add (local.get $ptr) (local.get $offset)))
            (i32.shl
              (i32.load8_u
                (i32.add
                  (local.get $ptr)
                  (i32.add (local.get $offset) (i32.const 1))))
              (i32.const 8)))))
      (else (call $pack (i32.const 1) (i32.const 0)))))

  (func (param $ptr i32) (param $len i32) (param $offset i32) (result i64)
    (if (result i64)
      (call $bounds_check (local.get $len) (local.get $offset) (i32.const 4))
      (then
        (call $pack
          (i32.const 0)
          (i32.or
            (i32.or
              (i32.load8_u (i32.add (local.get $ptr) (local.get $offset)))
              (i32.shl
                (i32.load8_u
                  (i32.add
                    (local.get $ptr)
                    (i32.add (local.get $offset) (i32.const 1))))
                (i32.const 8)))
            (i32.or
              (i32.shl
                (i32.load8_u
                  (i32.add
                    (local.get $ptr)
                    (i32.add (local.get $offset) (i32.const 2))))
                (i32.const 16))
              (i32.shl
                (i32.load8_u
                  (i32.add
                    (local.get $ptr)
                    (i32.add (local.get $offset) (i32.const 3))))
                (i32.const 24))))))
      (else (call $pack (i32.const 1) (i32.const 0)))))

  (func
    (param $value_low i32) (param $value_high i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $value i64)
    (local $next i64)
    (local $written i32)
    (local $byte i32)
    (local.set $value
      (i64.or
        (i64.extend_i32_u (local.get $value_low))
        (i64.shl (i64.extend_i32_u (local.get $value_high)) (i64.const 32))))
    (loop $again
      (if (i32.ge_u (local.get $written) (local.get $out_cap))
        (then (return (call $pack (i32.const 2) (local.get $written)))))
      (local.set $next (i64.shr_u (local.get $value) (i64.const 7)))
      (local.set $byte (i32.and (i32.wrap_i64 (local.get $value)) (i32.const 127)))
      (if (i64.ne (local.get $next) (i64.const 0))
        (then (local.set $byte (i32.or (local.get $byte) (i32.const 128)))))
      (i32.store8
        (i32.add (local.get $out_ptr) (local.get $written))
        (local.get $byte))
      (local.set $written (i32.add (local.get $written) (i32.const 1)))
      (local.set $value (local.get $next))
      (br_if $again (i64.ne (local.get $value) (i64.const 0))))
    (call $pack (i32.const 0) (local.get $written)))

  (func
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32)
    (result i64)
    (local $i i32)
    (local $shift i32)
    (local $b i32)
    (local $result i64)
    (if (i32.eqz (local.get $in_len))
      (then (return (call $pack (i32.const 1) (i32.const 0)))))
    (loop $again
      (if (i32.ge_u (local.get $i) (local.get $in_len))
        (then (return (call $pack (i32.const 5) (local.get $i)))))
      (if (i32.ge_u (local.get $i) (i32.const 10))
        (then (return (call $pack (i32.const 6) (local.get $i)))))
      (local.set $b (i32.load8_u (i32.add (local.get $in_ptr) (local.get $i))))
      (if
        (i32.and
          (i32.eq (local.get $shift) (i32.const 63))
          (i32.ne (i32.and (local.get $b) (i32.const 126)) (i32.const 0)))
        (then (return (call $pack (i32.const 4) (local.get $i)))))
      (local.set $result
        (i64.or
          (local.get $result)
          (i64.shl
            (i64.extend_i32_u (i32.and (local.get $b) (i32.const 127)))
            (i64.extend_i32_u (local.get $shift)))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (if (i32.eqz (i32.and (local.get $b) (i32.const 128)))
        (then
          (i64.store (local.get $out_ptr) (local.get $result))
          (return (call $pack (i32.const 0) (local.get $i)))))
      (if (i32.eq (local.get $i) (i32.const 10))
        (then (return (call $pack (i32.const 6) (local.get $i)))))
      (local.set $shift (i32.add (local.get $shift) (i32.const 7)))
      (br $again))
    (call $pack (i32.const 5) (local.get $i)))

  (func
    (param $value_low i32) (param $value_high i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $value i64)
    (local.set $value
      (i64.or
        (i64.extend_i32_u (local.get $value_low))
        (i64.shl (i64.extend_i32_u (local.get $value_high)) (i64.const 32))))
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
          (i32.wrap_i64 (local.get $value)))
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

  (func
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32)
    (result i64)
    (local $first i32)
    (local $need i32)
    (local $i i32)
    (local $value i64)
    (if (i32.eqz (local.get $in_len))
      (then (return (call $pack (i32.const 1) (i32.const 0)))))
    (local.set $first (i32.load8_u (local.get $in_ptr)))
    (local.set $need
      (i32.shl
        (i32.const 1)
        (i32.shr_u (local.get $first) (i32.const 6))))
    (if (i32.lt_u (local.get $in_len) (local.get $need))
      (then (return (call $pack (i32.const 5) (i32.const 0)))))
    (local.set $value (i64.extend_i32_u (i32.and (local.get $first) (i32.const 63))))
    (local.set $i (i32.const 1))
    (loop $again
      (if (i32.lt_u (local.get $i) (local.get $need))
        (then
          (local.set $value
            (i64.or
              (i64.shl (local.get $value) (i64.const 8))
              (i64.extend_i32_u
                (i32.load8_u
                  (i32.add (local.get $in_ptr) (local.get $i))))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $again))))
    (i64.store (local.get $out_ptr) (local.get $value))
    (call $pack (i32.const 0) (local.get $need)))

  (func (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $j i32)
    (local $crc i32)
    (local.set $crc (i32.const -1))
    (loop $bytes
      (if (i32.lt_u (local.get $i) (local.get $len))
        (then
          (local.set $crc
            (i32.xor
              (local.get $crc)
              (i32.load8_u (i32.add (local.get $ptr) (local.get $i)))))
          (local.set $j (i32.const 0))
          (loop $bits
            (if (i32.lt_u (local.get $j) (i32.const 8))
              (then
                (if (i32.and (local.get $crc) (i32.const 1))
                  (then
                    (local.set $crc
                      (i32.xor
                        (i32.shr_u (local.get $crc) (i32.const 1))
                        (i32.const 0xedb88320))))
                  (else
                    (local.set $crc
                      (i32.shr_u (local.get $crc) (i32.const 1)))))
                (local.set $j (i32.add (local.get $j) (i32.const 1)))
                (br $bits))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $bytes))))
    (i32.xor (local.get $crc) (i32.const -1)))

  

  (func (param $ptr i32) (param $len i32) (result i32)
    (call $adler32_update (i32.const 1) (local.get $ptr) (local.get $len)))

  (func (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $j i32)
    (local $k i32)
    (local $crc i32)
    (local $word i32)
    (local.set $crc (i32.const -1))
    (loop $loop
      (if (i32.lt_u (local.get $i) (local.get $len))
        (then
          (if (i32.le_u (i32.add (local.get $i) (i32.const 4)) (local.get $len))
            (then
              (local.set $word
                (i32.load (i32.add (local.get $ptr) (local.get $i))))
              (local.set $k (i32.const 0))
              (loop $quad
                (if (i32.lt_u (local.get $k) (i32.const 4))
                  (then
                    (local.set $crc
                      (i32.xor
                        (local.get $crc)
                        (i32.and (local.get $word) (i32.const 0xff))))
                    (local.set $j (i32.const 0))
                    (loop $bits
                      (if (i32.lt_u (local.get $j) (i32.const 8))
                        (then
                          (if (i32.and (local.get $crc) (i32.const 1))
                            (then
                              (local.set $crc
                                (i32.xor
                                  (i32.shr_u (local.get $crc) (i32.const 1))
                                  (i32.const 0xedb88320))))
                            (else
                              (local.set $crc
                                (i32.shr_u (local.get $crc) (i32.const 1)))))
                          (local.set $j (i32.add (local.get $j) (i32.const 1)))
                          (br $bits))))
                    (local.set $word
                      (i32.shr_u (local.get $word) (i32.const 8)))
                    (local.set $k (i32.add (local.get $k) (i32.const 1)))
                    (br $quad))))
              (local.set $i (i32.add (local.get $i) (i32.const 4)))
              (br $loop))
            (else
              (local.set $crc
                (i32.xor
                  (local.get $crc)
                  (i32.load8_u (i32.add (local.get $ptr) (local.get $i)))))
              (local.set $j (i32.const 0))
              (loop $bits_tail
                (if (i32.lt_u (local.get $j) (i32.const 8))
                  (then
                    (if (i32.and (local.get $crc) (i32.const 1))
                      (then
                        (local.set $crc
                          (i32.xor
                            (i32.shr_u (local.get $crc) (i32.const 1))
                            (i32.const 0xedb88320))))
                      (else
                        (local.set $crc
                          (i32.shr_u (local.get $crc) (i32.const 1)))))
                    (local.set $j (i32.add (local.get $j) (i32.const 1)))
                    (br $bits_tail))))
              (local.set $i (i32.add (local.get $i) (i32.const 1)))
              (br $loop))))))
    (i32.xor (local.get $crc) (i32.const -1)))

  (func (param $ptr i32) (param $len i32) (result i32)
    (call $adler32_update_vec (i32.const 1) (local.get $ptr) (local.get $len)))

  

  ;; ── Byte-at-a-time CRC-32 update ──
  ;; crc32_update_byte(crc, byte) → crc
  

  ;; ── Byte-at-a-time Adler-32 update ──
  ;; adler32_update_byte(adler, byte) → adler
  
  ;; Encoding Text — hex + base64url pipeline stages

    ;; Standard ID removed — merged into single module


  (func $hex_char (param $n i32) (result i32)
    local.get $n
    i32.const 10
    i32.lt_u
    if (result i32)
      local.get $n
      i32.const 48
      i32.add
    else
      local.get $n
      i32.const 87
      i32.add
    end)

  (func $m90hex_nibble (param $c i32) (result i32)
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
          i32.const -1
        end
      end
    end)

  ;; base64url functions imported from encoding-base64url as $b64_encode/$b64_decode

  (func $hex_encode_lower (export "hex_encode_lower")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $i i32)
    (local $out_len i32)
    (local $b i32)
    local.get $in_len
    i32.const 2147483647
    i32.gt_u
    if (result i64)
      i32.const 4
      i32.const 0
      call $pack
    else
      local.get $in_len
      i32.const 1
      i32.shl
      local.tee $out_len
      local.get $out_cap
      i32.gt_u
      if (result i64)
        i32.const 2
        i32.const 0
        call $pack
      else
        loop $loop
          local.get $i
          local.get $in_len
          i32.lt_u
          if
            local.get $in_ptr
            local.get $i
            i32.add
            i32.load8_u
            local.set $b
            local.get $out_ptr
            local.get $i
            i32.const 1
            i32.shl
            i32.add
            local.get $b
            i32.const 4
            i32.shr_u
            call $hex_char
            i32.store8
            local.get $out_ptr
            local.get $i
            i32.const 1
            i32.shl
            i32.add
            i32.const 1
            i32.add
            local.get $b
            i32.const 15
            i32.and
            call $hex_char
            i32.store8
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $loop
          end
        end
        i32.const 0
        local.get $out_len
        call $pack
      end
    end)

  (func $hex_decode_strict (export "hex_decode_strict")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $i i32)
    (local $out_len i32)
    (local $hi i32)
    (local $lo i32)
    local.get $in_len
    i32.const 1
    i32.and
    if (result i64)
      i32.const 3
      i32.const 0
      call $pack
    else
      local.get $in_len
      i32.const 1
      i32.shr_u
      local.tee $out_len
      local.get $out_cap
      i32.gt_u
      if (result i64)
        i32.const 2
        i32.const 0
        call $pack
      else
        loop $loop
          local.get $i
          local.get $out_len
          i32.lt_u
          if
            local.get $in_ptr
            local.get $i
            i32.const 1
            i32.shl
            i32.add
            i32.load8_u
            call $m90hex_nibble
            local.tee $hi
            i32.const 0
            i32.lt_s
            if
              i32.const 3
              i32.const 0
              call $pack
              return
            end
            local.get $in_ptr
            local.get $i
            i32.const 1
            i32.shl
            i32.add
            i32.const 1
            i32.add
            i32.load8_u
            call $m90hex_nibble
            local.tee $lo
            i32.const 0
            i32.lt_s
            if
              i32.const 3
              i32.const 0
              call $pack
              return
            end
            local.get $out_ptr
            local.get $i
            i32.add
            local.get $hi
            i32.const 4
            i32.shl
            local.get $lo
            i32.or
            i32.store8
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $loop
          end
        end
        i32.const 0
        local.get $out_len
        call $pack
      end
    end)

  ;; ── SIMD hex encode: 16 bytes → 32 hex chars ──
  (func $hex_encode_simd (export "hex_encode_simd")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $i i32) (local $o i32) (local $b i32)
    (local $v v128) (local $nibbles_hi v128) (local $nibbles_lo v128)
    (local $gt9 v128) (local $delta v128)
    (local $chars_hi v128) (local $chars_lo v128)
    (local $out0 v128) (local $out1 v128)

    (if (i32.gt_u (i32.shl (local.get $in_len) (i32.const 1)) (local.get $out_cap))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))

    (block $loop_end
      (br_if $loop_end (i32.lt_u (local.get $in_len) (i32.const 16)))
      (loop $loop
        (br_if $loop_end
          (i32.ge_u (local.get $i) (i32.sub (local.get $in_len) (i32.const 15))))
        (local.set $v (v128.load (i32.add (local.get $in_ptr) (local.get $i))))
        (local.set $nibbles_hi
          (v128.and
            (i8x16.shr_u (local.get $v) (i32.const 4))
            (i8x16.splat (i32.const 15))))
        (local.set $nibbles_lo
          (v128.and (local.get $v) (i8x16.splat (i32.const 15))))
        (local.set $gt9
          (i8x16.gt_u (local.get $nibbles_hi) (i8x16.splat (i32.const 9))))
        (local.set $delta
          (v128.bitselect
            (i8x16.splat (i32.const 87)) (i8x16.splat (i32.const 48)) (local.get $gt9)))
        (local.set $chars_hi
          (i8x16.add (local.get $nibbles_hi) (local.get $delta)))
        (local.set $gt9
          (i8x16.gt_u (local.get $nibbles_lo) (i8x16.splat (i32.const 9))))
        (local.set $delta
          (v128.bitselect
            (i8x16.splat (i32.const 87)) (i8x16.splat (i32.const 48)) (local.get $gt9)))
        (local.set $chars_lo
          (i8x16.add (local.get $nibbles_lo) (local.get $delta)))
        (local.set $out0
          (i8x16.shuffle 0 16 1 17 2 18 3 19 4 20 5 21 6 22 7 23
            (local.get $chars_hi) (local.get $chars_lo)))
        (local.set $out1
          (i8x16.shuffle 8 24 9 25 10 26 11 27 12 28 13 29 14 15 30 31
            (local.get $chars_hi) (local.get $chars_lo)))
        (v128.store (i32.add (local.get $out_ptr) (local.get $o)) (local.get $out0))
        (v128.store (i32.add (local.get $out_ptr) (i32.add (local.get $o) (i32.const 16))) (local.get $out1))
        (local.set $i (i32.add (local.get $i) (i32.const 16)))
        (local.set $o (i32.add (local.get $o) (i32.const 32)))
        (br $loop)))

    (block $tail_end
      (loop $tail
        (br_if $tail_end (i32.ge_u (local.get $i) (local.get $in_len)))
        (local.set $b (i32.load8_u (i32.add (local.get $in_ptr) (local.get $i))))
        (i32.store8
          (i32.add (local.get $out_ptr) (local.get $o))
          (call $hex_char (i32.shr_u (local.get $b) (i32.const 4))))
        (local.set $o (i32.add (local.get $o) (i32.const 1)))
        (i32.store8
          (i32.add (local.get $out_ptr) (local.get $o))
          (call $hex_char (i32.and (local.get $b) (i32.const 15))))
        (local.set $o (i32.add (local.get $o) (i32.const 1)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $tail)))
    (call $pack (i32.const 0) (local.get $o)))

  ;; ── SIMD hex decode: 32 hex chars → 16 bytes ──
  (func $hex_decode_simd (export "hex_decode_simd")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $i i32) (local $o i32) (local $c i32) (local $hi i32) (local $lo i32)
    (local $v0 v128) (local $v1 v128)
    (local $digit0 v128) (local $lower0 v128) (local $upper0 v128)
    (local $digit1 v128) (local $lower1 v128) (local $upper1 v128)
    (local $valid0 v128) (local $valid1 v128)
    (local $val0 v128) (local $val1 v128)
    (local $evens v128) (local $odds v128) (local $bytes v128)

    (if (i32.and (local.get $in_len) (i32.const 1))
      (then (return (call $pack (i32.const 3) (i32.const 0)))))
    (if (i32.gt_u (i32.shr_u (local.get $in_len) (i32.const 1)) (local.get $out_cap))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))

    (block $loop_end
      (br_if $loop_end (i32.lt_u (local.get $in_len) (i32.const 32)))
      (loop $loop
        (br_if $loop_end
          (i32.ge_u (local.get $i) (i32.sub (local.get $in_len) (i32.const 31))))
        (local.set $v0 (v128.load (i32.add (local.get $in_ptr) (local.get $i))))
        (local.set $v1 (v128.load (i32.add (local.get $in_ptr) (i32.add (local.get $i) (i32.const 16)))))

        (local.set $digit0
          (v128.and
            (i8x16.ge_u (local.get $v0) (i8x16.splat (i32.const 48)))
            (i8x16.le_u (local.get $v0) (i8x16.splat (i32.const 57)))))
        (local.set $digit1
          (v128.and
            (i8x16.ge_u (local.get $v1) (i8x16.splat (i32.const 48)))
            (i8x16.le_u (local.get $v1) (i8x16.splat (i32.const 57)))))

        (local.set $lower0
          (v128.and
            (i8x16.ge_u (local.get $v0) (i8x16.splat (i32.const 97)))
            (i8x16.le_u (local.get $v0) (i8x16.splat (i32.const 102)))))
        (local.set $lower1
          (v128.and
            (i8x16.ge_u (local.get $v1) (i8x16.splat (i32.const 97)))
            (i8x16.le_u (local.get $v1) (i8x16.splat (i32.const 102)))))

        (local.set $upper0
          (v128.and
            (i8x16.ge_u (local.get $v0) (i8x16.splat (i32.const 65)))
            (i8x16.le_u (local.get $v0) (i8x16.splat (i32.const 70)))))
        (local.set $upper1
          (v128.and
            (i8x16.ge_u (local.get $v1) (i8x16.splat (i32.const 65)))
            (i8x16.le_u (local.get $v1) (i8x16.splat (i32.const 70)))))

        (local.set $valid0 (v128.or (v128.or (local.get $digit0) (local.get $lower0)) (local.get $upper0)))
        (local.set $valid1 (v128.or (v128.or (local.get $digit1) (local.get $lower1)) (local.get $upper1)))
        (if (i32.eqz (i32.and (i8x16.all_true (local.get $valid0)) (i8x16.all_true (local.get $valid1))))
          (then (return (call $pack (i32.const 3) (local.get $i)))))

        ;; nibble = digit ? (v-48) : upper ? (v-55) : (v-87)
        (local.set $val0
          (v128.bitselect
            (i8x16.sub (local.get $v0) (i8x16.splat (i32.const 48)))
            (v128.bitselect
              (i8x16.sub (local.get $v0) (i8x16.splat (i32.const 55)))
              (i8x16.sub (local.get $v0) (i8x16.splat (i32.const 87)))
              (local.get $upper0))
            (local.get $digit0)))
        (local.set $val1
          (v128.bitselect
            (i8x16.sub (local.get $v1) (i8x16.splat (i32.const 48)))
            (v128.bitselect
              (i8x16.sub (local.get $v1) (i8x16.splat (i32.const 55)))
              (i8x16.sub (local.get $v1) (i8x16.splat (i32.const 87)))
              (local.get $upper1))
            (local.get $digit1)))

        ;; Deinterleave: pairs (n0,n1)→byte0, etc.
        (local.set $evens
          (i8x16.shuffle 0 2 4 6 8 10 12 14 16 18 20 22 24 26 28 30
            (local.get $val0) (local.get $val1)))
        (local.set $odds
          (i8x16.shuffle 1 3 5 7 9 11 13 15 17 19 21 23 25 27 29 31
            (local.get $val0) (local.get $val1)))
        (local.set $bytes
          (v128.or (i8x16.shl (local.get $evens) (i32.const 4)) (local.get $odds)))
        (v128.store (i32.add (local.get $out_ptr) (local.get $o)) (local.get $bytes))

        (local.set $i (i32.add (local.get $i) (i32.const 32)))
        (local.set $o (i32.add (local.get $o) (i32.const 16)))
        (br $loop)))

    ;; Scalar tail
    (block $tail_end
      (loop $tail
        (br_if $tail_end (i32.ge_u (local.get $i) (local.get $in_len)))
        (local.set $c (i32.load8_u (i32.add (local.get $in_ptr) (local.get $i))))
        (local.set $hi (call $m90hex_nibble (local.get $c)))
        (if (i32.lt_s (local.get $hi) (i32.const 0))
          (then (return (call $pack (i32.const 3) (local.get $i)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (local.set $c (i32.load8_u (i32.add (local.get $in_ptr) (local.get $i))))
        (local.set $lo (call $m90hex_nibble (local.get $c)))
        (if (i32.lt_s (local.get $lo) (i32.const 0))
          (then (return (call $pack (i32.const 3) (local.get $i)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (i32.store8
          (i32.add (local.get $out_ptr) (local.get $o))
          (i32.or (i32.shl (local.get $hi) (i32.const 4)) (local.get $lo)))
        (local.set $o (i32.add (local.get $o) (i32.const 1)))
        (br $tail)))
    (call $pack (i32.const 0) (local.get $o)))

  ;; ── Pipeline stage: hex encode (zero-copy input, SIMD accelerated) ──
  ;; Reads directly from pipe buffer via pipe_read_ptr, writes output to scratch.
  (func $process_hex_encode (export "process_hex_encode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $len_slot i32) (local $in_ptr i32) (local $read i32)
    (local $result i64) (local $out_len i32) (local $status i32)
    (local.set $len_slot (i32.sub (i32.add (local.get $scratch) (local.get $scap)) (i32.const 4)))
    (local.set $in_ptr (call $pipe_read_ptr (local.get $input) (local.get $len_slot)))
    (local.set $read (i32.load (local.get $len_slot)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $result (call $hex_encode_simd
      (local.get $in_ptr) (local.get $read)
      (local.get $scratch) (local.get $scap)))
    (local.set $status (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $out_len (i32.wrap_i64 (local.get $result)))
    (call $pipe_advance (local.get $input) (local.get $read))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (local.get $out_len)))
    (return (local.get $out_len)))

  ;; ── Pipeline stage: base64url nopad encode (zero-copy input) ──
  (func (export "process_b64_encode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $len_slot i32) (local $in_ptr i32) (local $read i32)
    (local $result i64) (local $out_len i32) (local $status i32)
    (local.set $len_slot (i32.sub (i32.add (local.get $scratch) (local.get $scap)) (i32.const 4)))
    (local.set $in_ptr (call $pipe_read_ptr (local.get $input) (local.get $len_slot)))
    (local.set $read (i32.load (local.get $len_slot)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $result (call $b64_encode
      (local.get $in_ptr) (local.get $read)
      (local.get $scratch) (local.get $scap)))
    (local.set $status (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $out_len (i32.wrap_i64 (local.get $result)))
    (call $pipe_advance (local.get $input) (local.get $read))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (local.get $out_len)))
    (return (local.get $out_len)))

  ;; ── Pipeline stage: base64url nopad decode (zero-copy input) ──
  (func (export "process_b64_decode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $len_slot i32) (local $in_ptr i32) (local $read i32)
    (local $result i64) (local $out_len i32) (local $status i32)
    (local.set $len_slot (i32.sub (i32.add (local.get $scratch) (local.get $scap)) (i32.const 4)))
    (local.set $in_ptr (call $pipe_read_ptr (local.get $input) (local.get $len_slot)))
    (local.set $read (i32.load (local.get $len_slot)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $result (call $b64_decode
      (local.get $in_ptr) (local.get $read)
      (local.get $scratch) (local.get $scap)))
    (local.set $status (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $out_len (i32.wrap_i64 (local.get $result)))
    (call $pipe_advance (local.get $input) (local.get $read))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (local.get $out_len)))
    (return (local.get $out_len)))

  ;; ── Pipeline stage: hex decode (zero-copy input, SIMD accelerated) ──
  (func $process_hex_decode (export "process_hex_decode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $len_slot i32) (local $in_ptr i32) (local $read i32)
    (local $result i64) (local $out_len i32) (local $status i32)
    (local.set $len_slot (i32.sub (i32.add (local.get $scratch) (local.get $scap)) (i32.const 4)))
    (local.set $in_ptr (call $pipe_read_ptr (local.get $input) (local.get $len_slot)))
    (local.set $read (i32.load (local.get $len_slot)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $result (call $hex_decode_simd
      (local.get $in_ptr) (local.get $read)
      (local.get $scratch) (local.get $scap)))
    (local.set $status (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $out_len (i32.wrap_i64 (local.get $result)))
    (call $pipe_advance (local.get $input) (local.get $read))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (local.get $out_len)))
    (return (local.get $out_len)))
