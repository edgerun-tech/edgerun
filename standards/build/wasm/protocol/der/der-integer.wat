  (func (export "proto_abi_version") (result i32)
    i32.const 2)

  (func (export "proto_standard_id") (result i32)
    i32.const 300801)

  ;; Status: 0 ok, 1 unsupported length, 2 output short, 3 invalid DER.
  ;; Return bits: low16=status, next16=header_len, high32=value_len.
  (func $m69header_decode (param $ptr i32) (param $len i32) (result i64)
    (local $first i32)
    (local $len_len i32)
    (local $i i32)
    (local $value i64)
    (if (i32.lt_u (local.get $len) (i32.const 2))
      (then (return (i64.const 1))))
    (if (i32.ne (i32.load8_u (local.get $ptr)) (i32.const 2))
      (then (return (i64.const 3))))
    (local.set $first (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))))
    (if (i32.lt_u (local.get $first) (i32.const 128))
      (then
        (return
          (i64.or
            (i64.shl (i64.extend_i32_u (local.get $first)) (i64.const 32))
            (i64.const 131072)))))
    (if (i32.eq (local.get $first) (i32.const 128))
      (then (return (i64.const 3))))
    (local.set $len_len (i32.and (local.get $first) (i32.const 127)))
    (if (i32.gt_u (local.get $len_len) (i32.const 4))
      (then (return (i64.const 1))))
    (if (i32.lt_u (local.get $len) (i32.add (i32.const 2) (local.get $len_len)))
      (then (return (i64.const 1))))
    (if (i32.eqz (i32.load8_u (i32.add (local.get $ptr) (i32.const 2))))
      (then (return (i64.const 3))))
    (local.set $i (i32.const 0))
    (local.set $value (i64.const 0))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $len_len)))
        (local.set $value
          (i64.or
            (i64.shl (local.get $value) (i64.const 8))
            (i64.extend_i32_u
              (i32.load8_u
                (i32.add (i32.add (local.get $ptr) (i32.const 2)) (local.get $i))))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (if (i64.lt_u (local.get $value) (i64.const 128))
      (then (return (i64.const 3))))
    (if (i64.gt_u (local.get $value) (i64.const 268435455))
      (then (return (i64.const 1))))
    (i64.or
      (i64.shl (local.get $value) (i64.const 32))
      (i64.shl (i64.extend_i32_u (i32.add (local.get $len_len) (i32.const 2))) (i64.const 16))))

  (func $validate_parts (param $ptr i32) (param $len i32) (result i64)
    (local $h i64)
    (local $status i32)
    (local $hdr i32)
    (local $vlen i32)
    (local $vptr i32)
    (local $first i32)
    (local $second i32)
    (local.set $h (call $m69header_decode (local.get $ptr) (local.get $len)))
    (local.set $status (i32.wrap_i64 (i64.and (local.get $h) (i64.const 65535))))
    (if (local.get $status)
      (then (return (local.get $h))))
    (local.set $hdr (i32.wrap_i64 (i64.and (i64.shr_u (local.get $h) (i64.const 16)) (i64.const 65535))))
    (local.set $vlen (i32.wrap_i64 (i64.shr_u (local.get $h) (i64.const 32))))
    (if (i32.eqz (local.get $vlen))
      (then (return (i64.const 3))))
    (if (i32.ne (local.get $len) (i32.add (local.get $hdr) (local.get $vlen)))
      (then (return (i64.const 1))))
    (local.set $vptr (i32.add (local.get $ptr) (local.get $hdr)))
    (local.set $first (i32.load8_u (local.get $vptr)))
    (if (i32.and
          (i32.ne (local.get $first) (i32.const 0))
          (i32.ne (i32.and (local.get $first) (i32.const 128)) (i32.const 0)))
      (then (return (i64.const 3))))
    (if (i32.gt_u (local.get $vlen) (i32.const 1))
      (then
        (local.set $second (i32.load8_u (i32.add (local.get $vptr) (i32.const 1))))
        (if (i32.and
              (i32.eqz (local.get $first))
              (i32.eqz (i32.and (local.get $second) (i32.const 128))))
          (then (return (i64.const 3))))))
    (i64.or
      (i64.or
        (i64.shl (i64.extend_i32_u (local.get $vptr)) (i64.const 32))
        (i64.shl (i64.extend_i32_u (local.get $vlen)) (i64.const 16)))
      (i64.const 0)))

  (func (export "der_integer_validate") (param $ptr i32) (param $len i32) (result i32)
    (i32.wrap_i64 (i64.and (call $validate_parts (local.get $ptr) (local.get $len)) (i64.const 65535))))

  (func (export "der_integer_payload")
    (param $ptr i32) (param $len i32) (param $out_ptr i32) (param $out_cap i32) (result i64)
    (local $p i64)
    (local $status i32)
    (local $vlen i32)
    (local $vptr i32)
    (local $copy_ptr i32)
    (local $copy_len i32)
    (local $i i32)
    (local.set $p (call $validate_parts (local.get $ptr) (local.get $len)))
    (local.set $status (i32.wrap_i64 (i64.and (local.get $p) (i64.const 65535))))
    (if (local.get $status)
      (then (return (i64.extend_i32_u (local.get $status)))))
    (local.set $vlen (i32.wrap_i64 (i64.and (i64.shr_u (local.get $p) (i64.const 16)) (i64.const 65535))))
    (local.set $vptr (i32.wrap_i64 (i64.shr_u (local.get $p) (i64.const 32))))
    (local.set $copy_ptr (local.get $vptr))
    (local.set $copy_len (local.get $vlen))
    (if (i32.and
          (i32.gt_u (local.get $vlen) (i32.const 1))
          (i32.eqz (i32.load8_u (local.get $vptr))))
      (then
        (local.set $copy_ptr (i32.add (local.get $vptr) (i32.const 1)))
        (local.set $copy_len (i32.sub (local.get $vlen) (i32.const 1)))))
    (if (i32.lt_u (local.get $out_cap) (local.get $copy_len))
      (then (return (i64.const 2))))
    (local.set $i (i32.const 0))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $copy_len)))
        (i32.store8
          (i32.add (local.get $out_ptr) (local.get $i))
          (i32.load8_u (i32.add (local.get $copy_ptr) (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (i64.shl (i64.extend_i32_u (local.get $copy_len)) (i64.const 32)))

  (func $encoded_len_len (param $value i32) (result i32)
    (if (i32.lt_u (local.get $value) (i32.const 128))
      (then (return (i32.const 1))))
    (if (i32.le_u (local.get $value) (i32.const 255))
      (then (return (i32.const 2))))
    (if (i32.le_u (local.get $value) (i32.const 65535))
      (then (return (i32.const 3))))
    (if (i32.le_u (local.get $value) (i32.const 16777215))
      (then (return (i32.const 4))))
    i32.const 5)

  (func $write_len (param $value i32) (param $out_ptr i32)
    (local $needed i32)
    (local $i i32)
    (if (i32.lt_u (local.get $value) (i32.const 128))
      (then
        (i32.store8 (local.get $out_ptr) (local.get $value))
        (return)))
    (local.set $needed (i32.sub (call $encoded_len_len (local.get $value)) (i32.const 1)))
    (i32.store8 (local.get $out_ptr) (i32.or (i32.const 128) (local.get $needed)))
    (local.set $i (i32.const 0))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $needed)))
        (i32.store8
          (i32.add (i32.add (local.get $out_ptr) (i32.const 1)) (local.get $i))
          (i32.and
            (i32.shr_u
              (local.get $value)
              (i32.mul
                (i32.sub (i32.sub (local.get $needed) (local.get $i)) (i32.const 1))
                (i32.const 8)))
            (i32.const 255)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop))))

  (func (export "der_integer_emit_unsigned")
    (param $value_ptr i32) (param $value_len i32) (param $out_ptr i32) (param $out_cap i32) (result i64)
    (local $start i32)
    (local $body_len i32)
    (local $len_len i32)
    (local $total i32)
    (local $needs_pad i32)
    (local $i i32)
    (local.set $start (local.get $value_ptr))
    (local.set $body_len (local.get $value_len))
    (block $trim_done
      (loop $trim
        (br_if $trim_done (i32.le_u (local.get $body_len) (i32.const 1)))
        (br_if $trim_done (i32.ne (i32.load8_u (local.get $start)) (i32.const 0)))
        (local.set $start (i32.add (local.get $start) (i32.const 1)))
        (local.set $body_len (i32.sub (local.get $body_len) (i32.const 1)))
        (br $trim)))
    (if (i32.eqz (local.get $value_len))
      (then
        (local.set $body_len (i32.const 1))
        (local.set $needs_pad (i32.const 0)))
      (else
        (local.set $needs_pad
          (i32.and (i32.load8_u (local.get $start)) (i32.const 128)))))
    (if (local.get $needs_pad)
      (then (local.set $body_len (i32.add (local.get $body_len) (i32.const 1)))))
    (if (i32.gt_u (local.get $body_len) (i32.const 268435455))
      (then (return (i64.const 1))))
    (local.set $len_len (call $encoded_len_len (local.get $body_len)))
    (local.set $total (i32.add (i32.add (i32.const 1) (local.get $len_len)) (local.get $body_len)))
    (if (i32.lt_u (local.get $out_cap) (local.get $total))
      (then (return (i64.const 2))))
    (i32.store8 (local.get $out_ptr) (i32.const 2))
    (call $write_len (local.get $body_len) (i32.add (local.get $out_ptr) (i32.const 1)))
    (if (local.get $needs_pad)
      (then
        (i32.store8 (i32.add (i32.add (local.get $out_ptr) (i32.const 1)) (local.get $len_len)) (i32.const 0))
        (local.set $i (i32.const 1)))
      (else
        (local.set $i (i32.const 0))))
    (if (i32.eqz (local.get $value_len))
      (then
        (i32.store8 (i32.add (i32.add (local.get $out_ptr) (i32.const 1)) (local.get $len_len)) (i32.const 0))
        (return (i64.shl (i64.extend_i32_u (local.get $total)) (i64.const 32)))))
    (block $copy_done
      (loop $copy
        (br_if $copy_done (i32.ge_u (i32.sub (local.get $i) (select (i32.const 1) (i32.const 0) (local.get $needs_pad))) (i32.sub (local.get $body_len) (select (i32.const 1) (i32.const 0) (local.get $needs_pad)))))
        (i32.store8
          (i32.add (i32.add (i32.add (local.get $out_ptr) (i32.const 1)) (local.get $len_len)) (local.get $i))
          (i32.load8_u
            (i32.add
              (local.get $start)
              (i32.sub (local.get $i) (select (i32.const 1) (i32.const 0) (local.get $needs_pad))))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $copy)))
    (i64.shl (i64.extend_i32_u (local.get $total)) (i64.const 32)))