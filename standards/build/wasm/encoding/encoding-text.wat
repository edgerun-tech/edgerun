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
  (func (export "process_hex_encode")
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
    local.get $out_len)

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
    local.get $out_len)

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
    local.get $out_len)

  ;; ── Pipeline stage: hex decode (zero-copy input, SIMD accelerated) ──
  (func (export "process_hex_decode")
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
    local.get $out_len)