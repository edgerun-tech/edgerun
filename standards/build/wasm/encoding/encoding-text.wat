(module
  (import "edgerun-core" "memory" (memory 1))
  (import "edgerun-core" "pack" (func $pack (param i32 i32) (result i64)))
  (import "pipe-core" "pipe_read" (func $pipe_read (param i32 i32 i32) (result i32)))
  (import "pipe-core" "pipe_write" (func $pipe_write (param i32 i32 i32) (result i32)))

  (func (export "proto_standard_id") (result i32)
    i32.const 300002)


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

  (func $m90b64url_char (param $n i32) (result i32)
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
            i32.const 45
          else
            i32.const 95
          end
        end
      end
    end)

  (func $m90b64url_value (param $c i32) (result i32)
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
          i32.const 45
          i32.eq
          if (result i32)
            i32.const 62
          else
            local.get $c
            i32.const 95
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

  (func $b64_encode (export "base64url_nopad_encode")
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
    local.tee $groups
    i32.const 1073741823
    i32.gt_u
    if (result i64)
      i32.const 4
      i32.const 0
      call $pack
    else
      local.get $in_len
      i32.const 3
      i32.rem_u
      local.set $rem
      local.get $groups
      i32.const 2
      i32.shl
      local.set $out_len
      local.get $rem
      if
        local.get $out_len
        local.get $rem
        i32.const 1
        i32.add
        i32.add
        local.set $out_len
      end
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
            call $m90b64url_char
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
            call $m90b64url_char
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
            call $m90b64url_char
            i32.store8
            local.get $out_ptr
            local.get $j
            i32.add
            i32.const 3
            i32.add
            local.get $b2
            i32.const 63
            i32.and
            call $m90b64url_char
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
          call $m90b64url_char
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
          call $m90b64url_char
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
          call $m90b64url_char
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
          call $m90b64url_char
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
          call $m90b64url_char
          i32.store8
        end
        i32.const 0
        local.get $out_len
        call $pack
      end
    end)

  (func $b64_decode (export "base64url_nopad_decode")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $groups i32)
    (local $rem i32)
    (local $out_len i32)
    (local $i i32)
    (local $j i32)
    (local $v0 i32)
    (local $v1 i32)
    (local $v2 i32)
    (local $v3 i32)
    local.get $in_len
    i32.const 4
    i32.rem_u
    local.tee $rem
    i32.const 1
    i32.eq
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
      i32.const 3
      i32.mul
      local.set $out_len
      local.get $rem
      i32.const 2
      i32.eq
      if
        local.get $out_len
        i32.const 1
        i32.add
        local.set $out_len
      end
      local.get $rem
      i32.const 3
      i32.eq
      if
        local.get $out_len
        i32.const 2
        i32.add
        local.set $out_len
      end
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
            i32.const 4
            i32.mul
            i32.add
            i32.load8_u
            call $m90b64url_value
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
            call $m90b64url_value
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
            call $m90b64url_value
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
            call $m90b64url_value
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
        local.get $rem
        i32.const 2
        i32.eq
        if
          local.get $in_ptr
          local.get $groups
          i32.const 4
          i32.mul
          i32.add
          i32.load8_u
          call $m90b64url_value
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
          local.get $groups
          i32.const 4
          i32.mul
          i32.add
          i32.const 1
          i32.add
          i32.load8_u
          call $m90b64url_value
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
        local.get $rem
        i32.const 3
        i32.eq
        if
          local.get $in_ptr
          local.get $groups
          i32.const 4
          i32.mul
          i32.add
          i32.load8_u
          call $m90b64url_value
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
          local.get $groups
          i32.const 4
          i32.mul
          i32.add
          i32.const 1
          i32.add
          i32.load8_u
          call $m90b64url_value
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
          local.get $groups
          i32.const 4
          i32.mul
          i32.add
          i32.const 2
          i32.add
          i32.load8_u
          call $m90b64url_value
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
         i32.const 0
        local.get $out_len
        call $pack
      end
    end)

  ;; ── Pipeline stage: hex encode ──
  ;; (input_pipe, output_pipe, config, clen, scratch, scap) → bytes_written | error
  (func (export "process_hex_encode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (result i32)
    (local $max_in i32) (local $read i32) (local $result i64) (local $out_len i32) (local $status i32)
    (local.set $max_in (i32.div_u (local.get $scap) (i32.const 3)))
    (local.set $read (call $pipe_read (local.get $input) (local.get $scratch) (local.get $max_in)))
    (if (i32.le_s (local.get $read) (i32.const 0)) (then (return (local.get $read))))
    (local.set $result (call $hex_encode_lower
      (local.get $scratch) (local.get $read)
      (i32.add (local.get $scratch) (local.get $max_in))
      (i32.sub (local.get $scap) (local.get $max_in))))
    (local.set $status (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $out_len (i32.wrap_i64 (local.get $result)))
    (drop (call $pipe_write (local.get $output)
      (i32.add (local.get $scratch) (local.get $max_in)) (local.get $out_len)))
    local.get $out_len)

  ;; ── Pipeline stage: base64url nopad encode ──
  ;; (input_pipe, output_pipe, config, clen, scratch, scap) → bytes_written | error
  (func (export "process_b64_encode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (result i32)
    (local $max_in i32) (local $read i32) (local $result i64) (local $out_len i32) (local $status i32)
    (local.set $max_in (i32.div_u (local.get $scap) (i32.const 3)))
    (local.set $read (call $pipe_read (local.get $input) (local.get $scratch) (local.get $max_in)))
    (if (i32.le_s (local.get $read) (i32.const 0)) (then (return (local.get $read))))
    (local.set $result (call $b64_encode
      (local.get $scratch) (local.get $read)
      (i32.add (local.get $scratch) (local.get $max_in))
      (i32.sub (local.get $scap) (local.get $max_in))))
    (local.set $status (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $out_len (i32.wrap_i64 (local.get $result)))
    (drop (call $pipe_write (local.get $output)
      (i32.add (local.get $scratch) (local.get $max_in)) (local.get $out_len)))
    local.get $out_len)

  ;; ── Pipeline stage: base64url nopad decode ──
  ;; (input_pipe, output_pipe, config, clen, scratch, scap) → bytes_written | error
  (func (export "process_b64_decode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (result i32)
    (local $max_in i32) (local $read i32) (local $result i64) (local $out_len i32) (local $status i32)
    (local.set $max_in (i32.div_u (local.get $scap) (i32.const 2)))
    (local.set $read (call $pipe_read (local.get $input) (local.get $scratch) (local.get $max_in)))
    (if (i32.le_s (local.get $read) (i32.const 0)) (then (return (local.get $read))))
    (local.set $result (call $b64_decode
      (local.get $scratch) (local.get $read)
      (i32.add (local.get $scratch) (local.get $max_in))
      (i32.sub (local.get $scap) (local.get $max_in))))
    (local.set $status (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $out_len (i32.wrap_i64 (local.get $result)))
    (drop (call $pipe_write (local.get $output)
      (i32.add (local.get $scratch) (local.get $max_in)) (local.get $out_len)))
    local.get $out_len)

  ;; ── Pipeline stage: hex decode ──
  ;; (input_pipe, output_pipe, config, clen, scratch, scap) → bytes_written | error
  (func (export "process_hex_decode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (result i32)
    (local $max_in i32) (local $read i32) (local $result i64) (local $out_len i32) (local $status i32)
    (local.set $max_in (i32.div_u (i32.mul (local.get $scap) (i32.const 2)) (i32.const 3)))
    (local.set $read (call $pipe_read (local.get $input) (local.get $scratch) (local.get $max_in)))
    (if (i32.le_s (local.get $read) (i32.const 0)) (then (return (local.get $read))))
    (local.set $result (call $hex_decode_strict
      (local.get $scratch) (local.get $read)
      (i32.add (local.get $scratch) (local.get $max_in))
      (i32.sub (local.get $scap) (local.get $max_in))))
    (local.set $status (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $out_len (i32.wrap_i64 (local.get $result)))
    (drop (call $pipe_write (local.get $output)
      (i32.add (local.get $scratch) (local.get $max_in)) (local.get $out_len)))
    local.get $out_len)
)