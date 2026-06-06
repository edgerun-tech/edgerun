(func $m72is_supported_tag (param $tag i32) (result i32)
    (if (i32.and
          (i32.ge_u (local.get $tag) (i32.const 64))
          (i32.le_u (local.get $tag) (i32.const 126)))
      (then (return (i32.const 1))))
    (if (i32.and
          (i32.ge_u (local.get $tag) (i32.const 128))
          (i32.le_u (local.get $tag) (i32.const 190)))
      (then (return (i32.const 1))))
    (if (i32.and
          (i32.ge_u (local.get $tag) (i32.const 192))
          (i32.le_u (local.get $tag) (i32.const 254)))
      (then (return (i32.const 1))))
    (if (i32.or
          (i32.or
            (i32.or
              (i32.or
                (i32.or
                  (i32.or
                    (i32.eq (local.get $tag) (i32.const 1))
                    (i32.eq (local.get $tag) (i32.const 2)))
                  (i32.or
                    (i32.eq (local.get $tag) (i32.const 3))
                    (i32.eq (local.get $tag) (i32.const 4))))
                (i32.or
                  (i32.eq (local.get $tag) (i32.const 5))
                  (i32.eq (local.get $tag) (i32.const 6))))
              (i32.or
                (i32.eq (local.get $tag) (i32.const 9))
                (i32.eq (local.get $tag) (i32.const 10))))
            (i32.or
              (i32.or
                (i32.eq (local.get $tag) (i32.const 12))
                (i32.and
                  (i32.ge_u (local.get $tag) (i32.const 18))
                  (i32.le_u (local.get $tag) (i32.const 24))))
              (i32.eq (local.get $tag) (i32.const 26))))
          (i32.or
            (i32.eq (local.get $tag) (i32.const 30))
            (i32.or
              (i32.eq (local.get $tag) (i32.const 48))
              (i32.eq (local.get $tag) (i32.const 49)))))
      (then (return (i32.const 1))))
    i32.const 0)

  ;; DER definite length decoder.
  ;; Return bits: low16=status, next16=consumed, high32=value.
  ;; status: 0 ok, 1 input_short, 2 output_short, 3 invalid, 4 overflow.
  (func (export "der_len_decode") (param $in_ptr i32) (param $in_len i32) (result i64)
    (local $first i32)
    (local $len_len i32)
    (local $i i32)
    (local $value i64)
    (if (i32.eqz (local.get $in_len))
      (then (return (i64.const 1))))
    (local.set $first (i32.load8_u (local.get $in_ptr)))
    (if (i32.lt_u (local.get $first) (i32.const 128))
      (then
        (return
          (i64.or
            (i64.shl (i64.extend_i32_u (local.get $first)) (i64.const 32))
            (i64.const 65536)))))
    (if (i32.eq (local.get $first) (i32.const 128))
      (then (return (i64.const 3))))
    (local.set $len_len (i32.and (local.get $first) (i32.const 127)))
    (if (i32.gt_u (local.get $len_len) (i32.const 4))
      (then (return (i64.const 3))))
    (if (i32.lt_u (local.get $in_len) (i32.add (i32.const 1) (local.get $len_len)))
      (then (return (i64.const 1))))
    (if (i32.eqz (i32.load8_u (i32.add (local.get $in_ptr) (i32.const 1))))
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
                (i32.add
                  (i32.add (local.get $in_ptr) (i32.const 1))
                  (local.get $i))))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (if (i64.lt_u (local.get $value) (i64.const 128))
      (then (return (i64.const 3))))
    (if (i64.gt_u (local.get $value) (i64.const 268435455))
      (then (return (i64.const 4))))
    (i64.or
      (i64.shl (local.get $value) (i64.const 32))
      (i64.shl
        (i64.extend_i32_u (i32.add (local.get $len_len) (i32.const 1)))
        (i64.const 16))))

  ;; DER definite length encoder.
  ;; Return bits: low32=status, high32=written.
  (func (export "der_len_encode") (param $value i32) (param $out_ptr i32) (param $out_cap i32) (result i64)
    (local $needed i32)
    (local $i i32)
    (if (i32.gt_u (local.get $value) (i32.const 268435455))
      (then (return (i64.const 4))))
    (if (i32.lt_u (local.get $value) (i32.const 128))
      (then
        (if (i32.lt_u (local.get $out_cap) (i32.const 1))
          (then (return (i64.const 2))))
        (i32.store8 (local.get $out_ptr) (local.get $value))
        (return (i64.const 4294967296))))
    (local.set $needed
      (select
        (i32.const 1)
        (select
          (i32.const 2)
          (select
            (i32.const 3)
            (i32.const 4)
            (i32.le_u (local.get $value) (i32.const 16777215)))
          (i32.le_u (local.get $value) (i32.const 65535)))
        (i32.le_u (local.get $value) (i32.const 255))))
    (if (i32.lt_u (local.get $out_cap) (i32.add (local.get $needed) (i32.const 1)))
      (then (return (i64.const 2))))
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
        (br $loop)))
    (i64.shl
      (i64.extend_i32_u (i32.add (local.get $needed) (i32.const 1)))
      (i64.const 32)))

  ;; Return bits: low32=status, high32=tag_octet.
  (func (export "der_tag_decode") (param $in_ptr i32) (param $in_len i32) (result i64)
    (local $tag i32)
    (if (i32.eqz (local.get $in_len))
      (then (return (i64.const 1))))
    (local.set $tag (i32.load8_u (local.get $in_ptr)))
    (if (i32.eqz (call $m72is_supported_tag (local.get $tag)))
      (then (return (i64.const 3))))
    (i64.shl
      (i64.extend_i32_u (local.get $tag))
      (i64.const 32)))

  ;; DER header decoder.
  ;; Return bits: low16=status, next16=consumed, next8=tag, high24=length.
  (func (export "der_header_decode") (param $in_ptr i32) (param $in_len i32) (result i64)
    (local $tag i32)
    (local $first i32)
    (local $len_len i32)
    (local $i i32)
    (local $value i64)
    (local $consumed i32)
    (if (i32.lt_u (local.get $in_len) (i32.const 2))
      (then (return (i64.const 1))))
    (local.set $tag (i32.load8_u (local.get $in_ptr)))
    (if (i32.eqz (call $m72is_supported_tag (local.get $tag)))
      (then (return (i64.const 3))))
    (local.set $first (i32.load8_u (i32.add (local.get $in_ptr) (i32.const 1))))
    (if (i32.lt_u (local.get $first) (i32.const 128))
      (then
        (return
          (i64.or
            (i64.or
              (i64.shl (i64.extend_i32_u (local.get $first)) (i64.const 40))
              (i64.shl (i64.extend_i32_u (local.get $tag)) (i64.const 32)))
            (i64.const 131072)))))
    (if (i32.eq (local.get $first) (i32.const 128))
      (then (return (i64.const 3))))
    (local.set $len_len (i32.and (local.get $first) (i32.const 127)))
    (if (i32.gt_u (local.get $len_len) (i32.const 4))
      (then (return (i64.const 3))))
    (if (i32.lt_u (local.get $in_len) (i32.add (i32.const 2) (local.get $len_len)))
      (then (return (i64.const 1))))
    (if (i32.eqz (i32.load8_u (i32.add (local.get $in_ptr) (i32.const 2))))
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
                (i32.add
                  (i32.add (local.get $in_ptr) (i32.const 2))
                  (local.get $i))))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (if (i64.lt_u (local.get $value) (i64.const 128))
      (then (return (i64.const 3))))
    (if (i64.gt_u (local.get $value) (i64.const 16777215))
      (then (return (i64.const 4))))
    (local.set $consumed (i32.add (local.get $len_len) (i32.const 2)))
    (i64.or
      (i64.or
        (i64.or
          (i64.shl (local.get $value) (i64.const 40))
          (i64.shl (i64.extend_i32_u (local.get $tag)) (i64.const 32)))
        (i64.shl (i64.extend_i32_u (local.get $consumed)) (i64.const 16)))
      (i64.const 0)))
