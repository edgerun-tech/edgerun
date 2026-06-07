(func $m68is_supported_tag (param $tag i32) (result i32)
    (if (i32.and (i32.ge_u (local.get $tag) (i32.const 64)) (i32.le_u (local.get $tag) (i32.const 126)))
      (then (return (i32.const 1))))
    (if (i32.and (i32.ge_u (local.get $tag) (i32.const 128)) (i32.le_u (local.get $tag) (i32.const 190)))
      (then (return (i32.const 1))))
    (if (i32.and (i32.ge_u (local.get $tag) (i32.const 192)) (i32.le_u (local.get $tag) (i32.const 254)))
      (then (return (i32.const 1))))
    (if (i32.or
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
              (i32.eq (local.get $tag) (i32.const 12))
              (i32.eq (local.get $tag) (i32.const 48))))
          (i32.eq (local.get $tag) (i32.const 49)))
      (then (return (i32.const 1))))
    i32.const 0)

  ;; Internal DER header decoder.
  ;; Return bits: low16=status, next16=header_len, next8=tag, high24=length.
  (func $m68header_decode (param $ptr i32) (param $len i32) (result i64)
    (local $tag i32)
    (local $first i32)
    (local $len_len i32)
    (local $i i32)
    (local $value i64)
    (local $consumed i32)
    (if (i32.lt_u (local.get $len) (i32.const 2))
      (then (return (i64.const 1))))
    (local.set $tag (i32.load8_u (local.get $ptr)))
    (if (i32.eqz (call $m68is_supported_tag (local.get $tag)))
      (then (return (i64.const 3))))
    (local.set $first (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))))
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

  (func $m68write_span (param $out i32) (param $value_ptr i32) (param $value_len i32) (param $header_len i32) (param $total_len i32)
    (i32.store (local.get $out) (local.get $value_ptr))
    (i32.store (i32.add (local.get $out) (i32.const 4)) (local.get $value_len))
    (i32.store (i32.add (local.get $out) (i32.const 8)) (local.get $header_len))
    (i32.store (i32.add (local.get $out) (i32.const 12)) (local.get $total_len)))

  ;; out record: value_ptr:u32, value_len:u32, header_len:u32, total_len:u32, negative:u32.
  (func $der_asn1_integer_decode (export "der_asn1_integer_decode") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $h i64) (local $status i32) (local $hdr i32) (local $tag i32) (local $vlen i32)
    (local $vptr i32) (local $first i32) (local $second i32)
    (local.set $h (call $m68header_decode (local.get $ptr) (local.get $len)))
    (local.set $status (i32.wrap_i64 (i64.and (local.get $h) (i64.const 65535))))
    (if (local.get $status) (then (return (local.get $status))))
    (local.set $hdr (i32.wrap_i64 (i64.and (i64.shr_u (local.get $h) (i64.const 16)) (i64.const 65535))))
    (local.set $tag (i32.wrap_i64 (i64.and (i64.shr_u (local.get $h) (i64.const 32)) (i64.const 255))))
    (local.set $vlen (i32.wrap_i64 (i64.and (i64.shr_u (local.get $h) (i64.const 40)) (i64.const 16777215))))
    (if (i32.ne (local.get $tag) (i32.const 2)) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $vlen)) (then (return (i32.const 3))))
    (if (i32.lt_u (local.get $len) (i32.add (local.get $hdr) (local.get $vlen))) (then (return (i32.const 1))))
    (local.set $vptr (i32.add (local.get $ptr) (local.get $hdr)))
    (local.set $first (i32.load8_u (local.get $vptr)))
    (if (i32.gt_u (local.get $vlen) (i32.const 1))
      (then
        (local.set $second (i32.load8_u (i32.add (local.get $vptr) (i32.const 1))))
        (if (i32.and (i32.eqz (local.get $first)) (i32.eqz (i32.and (local.get $second) (i32.const 128))))
          (then (return (i32.const 3))))
        (if (i32.and
              (i32.eq (local.get $first) (i32.const 255))
              (i32.ne (i32.and (local.get $second) (i32.const 128)) (i32.const 0)))
          (then (return (i32.const 3))))))
    (call $m68write_span (local.get $out) (local.get $vptr) (local.get $vlen) (local.get $hdr) (i32.add (local.get $hdr) (local.get $vlen)))
    (i32.store (i32.add (local.get $out) (i32.const 16)) (select (i32.const 1) (i32.const 0) (i32.and (local.get $first) (i32.const 128))))
    i32.const 0)

  ;; out record: payload_ptr:u32, payload_len:u32, header_len:u32, total_len:u32, unused_bits:u32.
  (func $der_asn1_bit_string_decode (export "der_asn1_bit_string_decode") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $h i64) (local $status i32) (local $hdr i32) (local $tag i32) (local $vlen i32)
    (local $vptr i32) (local $unused i32)
    (local.set $h (call $m68header_decode (local.get $ptr) (local.get $len)))
    (local.set $status (i32.wrap_i64 (i64.and (local.get $h) (i64.const 65535))))
    (if (local.get $status) (then (return (local.get $status))))
    (local.set $hdr (i32.wrap_i64 (i64.and (i64.shr_u (local.get $h) (i64.const 16)) (i64.const 65535))))
    (local.set $tag (i32.wrap_i64 (i64.and (i64.shr_u (local.get $h) (i64.const 32)) (i64.const 255))))
    (local.set $vlen (i32.wrap_i64 (i64.and (i64.shr_u (local.get $h) (i64.const 40)) (i64.const 16777215))))
    (if (i32.ne (local.get $tag) (i32.const 3)) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $vlen)) (then (return (i32.const 3))))
    (if (i32.lt_u (local.get $len) (i32.add (local.get $hdr) (local.get $vlen))) (then (return (i32.const 1))))
    (local.set $vptr (i32.add (local.get $ptr) (local.get $hdr)))
    (local.set $unused (i32.load8_u (local.get $vptr)))
    (if (i32.gt_u (local.get $unused) (i32.const 7)) (then (return (i32.const 3))))
    (if (i32.and (i32.ne (local.get $unused) (i32.const 0)) (i32.eq (local.get $vlen) (i32.const 1)))
      (then (return (i32.const 3))))
    (call $m68write_span (local.get $out) (i32.add (local.get $vptr) (i32.const 1)) (i32.sub (local.get $vlen) (i32.const 1)) (local.get $hdr) (i32.add (local.get $hdr) (local.get $vlen)))
    (i32.store (i32.add (local.get $out) (i32.const 16)) (local.get $unused))
    i32.const 0)

  (func $der_asn1_octet_string_decode (export "der_asn1_octet_string_decode") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $h i64) (local $status i32) (local $hdr i32) (local $tag i32) (local $vlen i32)
    (local.set $h (call $m68header_decode (local.get $ptr) (local.get $len)))
    (local.set $status (i32.wrap_i64 (i64.and (local.get $h) (i64.const 65535))))
    (if (local.get $status) (then (return (local.get $status))))
    (local.set $hdr (i32.wrap_i64 (i64.and (i64.shr_u (local.get $h) (i64.const 16)) (i64.const 65535))))
    (local.set $tag (i32.wrap_i64 (i64.and (i64.shr_u (local.get $h) (i64.const 32)) (i64.const 255))))
    (local.set $vlen (i32.wrap_i64 (i64.and (i64.shr_u (local.get $h) (i64.const 40)) (i64.const 16777215))))
    (if (i32.ne (local.get $tag) (i32.const 4)) (then (return (i32.const 3))))
    (if (i32.lt_u (local.get $len) (i32.add (local.get $hdr) (local.get $vlen))) (then (return (i32.const 1))))
    (call $m68write_span (local.get $out) (i32.add (local.get $ptr) (local.get $hdr)) (local.get $vlen) (local.get $hdr) (i32.add (local.get $hdr) (local.get $vlen)))
    i32.const 0)

  (func (export "der_asn1_null_decode") (param $ptr i32) (param $len i32) (result i32)
    (local $h i64) (local $status i32) (local $hdr i32) (local $tag i32) (local $vlen i32)
    (local.set $h (call $m68header_decode (local.get $ptr) (local.get $len)))
    (local.set $status (i32.wrap_i64 (i64.and (local.get $h) (i64.const 65535))))
    (if (local.get $status) (then (return (local.get $status))))
    (local.set $hdr (i32.wrap_i64 (i64.and (i64.shr_u (local.get $h) (i64.const 16)) (i64.const 65535))))
    (local.set $tag (i32.wrap_i64 (i64.and (i64.shr_u (local.get $h) (i64.const 32)) (i64.const 255))))
    (local.set $vlen (i32.wrap_i64 (i64.and (i64.shr_u (local.get $h) (i64.const 40)) (i64.const 16777215))))
    (if (i32.ne (local.get $tag) (i32.const 5)) (then (return (i32.const 3))))
    (if (i32.ne (local.get $vlen) (i32.const 0)) (then (return (i32.const 3))))
    (if (i32.lt_u (local.get $len) (local.get $hdr)) (then (return (i32.const 1))))
    i32.const 0)

  ;; out record: body_ptr:u32, body_len:u32, header_len:u32, total_len:u32.
  (func $der_asn1_sequence_decode (export "der_asn1_sequence_decode") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $h i64) (local $status i32) (local $hdr i32) (local $tag i32) (local $vlen i32)
    (local.set $h (call $m68header_decode (local.get $ptr) (local.get $len)))
    (local.set $status (i32.wrap_i64 (i64.and (local.get $h) (i64.const 65535))))
    (if (local.get $status) (then (return (local.get $status))))
    (local.set $hdr (i32.wrap_i64 (i64.and (i64.shr_u (local.get $h) (i64.const 16)) (i64.const 65535))))
    (local.set $tag (i32.wrap_i64 (i64.and (i64.shr_u (local.get $h) (i64.const 32)) (i64.const 255))))
    (local.set $vlen (i32.wrap_i64 (i64.and (i64.shr_u (local.get $h) (i64.const 40)) (i64.const 16777215))))
    (if (i32.ne (local.get $tag) (i32.const 48)) (then (return (i32.const 3))))
    (if (i32.lt_u (local.get $len) (i32.add (local.get $hdr) (local.get $vlen))) (then (return (i32.const 1))))
    (call $m68write_span (local.get $out) (i32.add (local.get $ptr) (local.get $hdr)) (local.get $vlen) (local.get $hdr) (i32.add (local.get $hdr) (local.get $vlen)))
    i32.const 0)

  ;; Iterate one DER child inside a decoded sequence body.
  ;; out record: tag:u32, header_len:u32, value_ptr:u32, value_len:u32, total_len:u32, next_offset:u32.
  (func (export "der_asn1_sequence_next_child") (param $body_ptr i32) (param $body_len i32) (param $offset i32) (param $out i32) (result i32)
    (local $h i64) (local $status i32) (local $hdr i32) (local $tag i32) (local $vlen i32) (local $remaining i32)
    (if (i32.eq (local.get $offset) (local.get $body_len)) (then (return (i32.const 5))))
    (if (i32.gt_u (local.get $offset) (local.get $body_len)) (then (return (i32.const 3))))
    (local.set $remaining (i32.sub (local.get $body_len) (local.get $offset)))
    (local.set $h (call $m68header_decode (i32.add (local.get $body_ptr) (local.get $offset)) (local.get $remaining)))
    (local.set $status (i32.wrap_i64 (i64.and (local.get $h) (i64.const 65535))))
    (if (local.get $status) (then (return (local.get $status))))
    (local.set $hdr (i32.wrap_i64 (i64.and (i64.shr_u (local.get $h) (i64.const 16)) (i64.const 65535))))
    (local.set $tag (i32.wrap_i64 (i64.and (i64.shr_u (local.get $h) (i64.const 32)) (i64.const 255))))
    (local.set $vlen (i32.wrap_i64 (i64.and (i64.shr_u (local.get $h) (i64.const 40)) (i64.const 16777215))))
    (if (i32.lt_u (local.get $remaining) (i32.add (local.get $hdr) (local.get $vlen))) (then (return (i32.const 1))))
    (i32.store (local.get $out) (local.get $tag))
    (i32.store (i32.add (local.get $out) (i32.const 4)) (local.get $hdr))
    (i32.store (i32.add (local.get $out) (i32.const 8)) (i32.add (i32.add (local.get $body_ptr) (local.get $offset)) (local.get $hdr)))
    (i32.store (i32.add (local.get $out) (i32.const 12)) (local.get $vlen))
    (i32.store (i32.add (local.get $out) (i32.const 16)) (i32.add (local.get $hdr) (local.get $vlen)))
    (i32.store (i32.add (local.get $out) (i32.const 20)) (i32.add (local.get $offset) (i32.add (local.get $hdr) (local.get $vlen))))
    i32.const 0)

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

;; status: 0 ok, 1 input_short, 2 output_short, 3 invalid, 4 overflow, 5 done.
  ;; Decode the compact first+second OID root octet used by local const_oid.
  ;; out record: first_arc:u32, second_arc:u32, next_offset:u32.
  (func $der_oid_root_decode (export "der_oid_root_decode") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $octet i32)
    (if (i32.eqz (local.get $len))
      (then (return (i32.const 1))))
    (local.set $octet (i32.load8_u (local.get $ptr)))
    (if (i32.gt_u (local.get $octet) (i32.const 119))
      (then (return (i32.const 3))))
    (i32.store (local.get $out) (i32.div_u (local.get $octet) (i32.const 40)))
    (i32.store (i32.add (local.get $out) (i32.const 4))
      (i32.rem_u (local.get $octet) (i32.const 40)))
    (i32.store (i32.add (local.get $out) (i32.const 8)) (i32.const 1))
    i32.const 0)

  ;; Iterate one body arc after the root octet.
  ;; out record: arc:u32, next_offset:u32, encoded_len:u32.
  (func $der_oid_next_arc (export "der_oid_next_arc") (param $ptr i32) (param $len i32) (param $offset i32) (param $out i32) (result i32)
    (local $cursor i32)
    (local $byte i32)
    (local $count i32)
    (local $arc i64)
    (local $start i32)
    (if (i32.ge_u (local.get $offset) (local.get $len))
      (then (return (i32.const 5))))
    (if (i32.eqz (local.get $offset))
      (then (return (i32.const 3))))
    (local.set $cursor (local.get $offset))
    (local.set $start (local.get $offset))
    (local.set $arc (i64.const 0))
    (local.set $count (i32.const 0))
    (block $done
      (loop $loop
        (if (i32.ge_u (local.get $cursor) (local.get $len))
          (then (return (i32.const 1))))
        (local.set $byte (i32.load8_u (i32.add (local.get $ptr) (local.get $cursor))))
        (if (i32.and
              (i32.and
                (i32.eqz (local.get $count))
                (i32.eq (local.get $byte) (i32.const 128)))
              (i32.lt_u (i32.add (local.get $cursor) (i32.const 1)) (local.get $len)))
          (then (return (i32.const 3))))
        (local.set $count (i32.add (local.get $count) (i32.const 1)))
        (if (i32.gt_u (local.get $count) (i32.const 5))
          (then (return (i32.const 4))))
        (local.set $arc
          (i64.or
            (i64.shl (local.get $arc) (i64.const 7))
            (i64.extend_i32_u (i32.and (local.get $byte) (i32.const 127)))))
        (if (i64.gt_u (local.get $arc) (i64.const 4294967295))
          (then (return (i32.const 4))))
        (local.set $cursor (i32.add (local.get $cursor) (i32.const 1)))
        (br_if $done (i32.eqz (i32.and (local.get $byte) (i32.const 128))))
        (br $loop)))
    (i32.store (local.get $out) (i32.wrap_i64 (local.get $arc)))
    (i32.store (i32.add (local.get $out) (i32.const 4)) (local.get $cursor))
    (i32.store (i32.add (local.get $out) (i32.const 8))
      (i32.sub (local.get $cursor) (local.get $start)))
    i32.const 0)

  (func (export "der_oid_value_validate") (param $ptr i32) (param $len i32) (param $scratch i32) (result i32)
    (local $status i32)
    (local $offset i32)
    (local.set $status (call $der_oid_root_decode (local.get $ptr) (local.get $len) (local.get $scratch)))
    (if (local.get $status) (then (return (local.get $status))))
    (local.set $offset (i32.load (i32.add (local.get $scratch) (i32.const 8))))
    (block $done
      (loop $loop
        (local.set $status
          (call $der_oid_next_arc (local.get $ptr) (local.get $len) (local.get $offset) (local.get $scratch)))
        (if (i32.eq (local.get $status) (i32.const 5))
          (then (br $done)))
        (if (local.get $status)
          (then (return (local.get $status))))
        (local.set $offset (i32.load (i32.add (local.get $scratch) (i32.const 4))))
        (br $loop)))
    i32.const 0)

  ;; Return bits: low32=status, high32=written.
  (func (export "der_oid_root_encode") (param $first i32) (param $second i32) (param $out i32) (param $cap i32) (result i64)
    (if (i32.or (i32.gt_u (local.get $first) (i32.const 2)) (i32.gt_u (local.get $second) (i32.const 39)))
      (then (return (i64.const 3))))
    (if (i32.lt_u (local.get $cap) (i32.const 1))
      (then (return (i64.const 2))))
    (i32.store8 (local.get $out)
      (i32.add (i32.mul (local.get $first) (i32.const 40)) (local.get $second)))
    i64.const 4294967296)

  ;; Encode one u32 body arc as base-128.
  ;; Return bits: low32=status, high32=written.
  (func (export "der_oid_arc_encode") (param $arc i32) (param $out i32) (param $cap i32) (result i64)
    (local $needed i32)
    (local $i i32)
    (local $shift i32)
    (local $byte i32)
    (local.set $needed
      (select
        (i32.const 1)
        (select
          (i32.const 2)
          (select
            (i32.const 3)
            (select
              (i32.const 4)
              (i32.const 5)
              (i32.le_u (local.get $arc) (i32.const 268435455)))
            (i32.le_u (local.get $arc) (i32.const 2097151)))
          (i32.le_u (local.get $arc) (i32.const 16383)))
        (i32.le_u (local.get $arc) (i32.const 127))))
    (if (i32.lt_u (local.get $cap) (local.get $needed))
      (then (return (i64.const 2))))
    (local.set $i (i32.const 0))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $needed)))
        (local.set $shift
          (i32.mul
            (i32.sub (i32.sub (local.get $needed) (local.get $i)) (i32.const 1))
            (i32.const 7)))
        (local.set $byte
          (i32.and (i32.shr_u (local.get $arc) (local.get $shift)) (i32.const 127)))
        (if (i32.lt_u (local.get $i) (i32.sub (local.get $needed) (i32.const 1)))
          (then (local.set $byte (i32.or (local.get $byte) (i32.const 128)))))
        (i32.store8 (i32.add (local.get $out) (local.get $i)) (local.get $byte))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (i64.shl (i64.extend_i32_u (local.get $needed)) (i64.const 32)))

;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid, 4 overflow.
  ;; Record:
  ;; 0 kind: 1 UTCTime, 2 GeneralizedTime
  ;; 4 year, 8 month, 12 day, 16 hour, 20 minute, 24 second
  ;; 28 unix_seconds_lo, 32 unix_seconds_hi, 36 header_len, 40 total_len

  (func $m71digit (param $ptr i32) (param $end i32) (result i32)
    (local $b i32)
    (if (i32.ge_u (local.get $ptr) (local.get $end))
      (then (return (i32.const -1))))
    (local.set $b (i32.load8_u (local.get $ptr)))
    (if (i32.or (i32.lt_u (local.get $b) (i32.const 48)) (i32.gt_u (local.get $b) (i32.const 57)))
      (then (return (i32.const -1))))
    (i32.sub (local.get $b) (i32.const 48)))

  (func $m71two (param $ptr i32) (param $end i32) (result i32)
    (local $a i32)
    (local $b i32)
    (local.set $a (call $m71digit (local.get $ptr) (local.get $end)))
    (if (i32.lt_s (local.get $a) (i32.const 0)) (then (return (i32.const -1))))
    (local.set $b (call $m71digit (i32.add (local.get $ptr) (i32.const 1)) (local.get $end)))
    (if (i32.lt_s (local.get $b) (i32.const 0)) (then (return (i32.const -1))))
    (i32.add (i32.mul (local.get $a) (i32.const 10)) (local.get $b)))

  (func $m71four (param $ptr i32) (param $end i32) (result i32)
    (local $a i32)
    (local $b i32)
    (local.set $a (call $m71two (local.get $ptr) (local.get $end)))
    (if (i32.lt_s (local.get $a) (i32.const 0)) (then (return (i32.const -1))))
    (local.set $b (call $m71two (i32.add (local.get $ptr) (i32.const 2)) (local.get $end)))
    (if (i32.lt_s (local.get $b) (i32.const 0)) (then (return (i32.const -1))))
    (i32.add (i32.mul (local.get $a) (i32.const 100)) (local.get $b)))

  (func $m71is_leap (param $year i32) (result i32)
    (if (i32.ne (i32.rem_u (local.get $year) (i32.const 4)) (i32.const 0))
      (then (return (i32.const 0))))
    (if (i32.ne (i32.rem_u (local.get $year) (i32.const 100)) (i32.const 0))
      (then (return (i32.const 1))))
    (select
      (i32.const 1)
      (i32.const 0)
      (i32.eq (i32.rem_u (local.get $year) (i32.const 400)) (i32.const 0))))

  (func $m71month_days (param $year i32) (param $month i32) (result i32)
    (if (i32.eq (local.get $month) (i32.const 2))
      (then
        (return
          (select (i32.const 29) (i32.const 28) (call $m71is_leap (local.get $year))))))
    (if
      (i32.or
        (i32.or
          (i32.or
            (i32.eq (local.get $month) (i32.const 1))
            (i32.eq (local.get $month) (i32.const 3)))
          (i32.or
            (i32.eq (local.get $month) (i32.const 5))
            (i32.eq (local.get $month) (i32.const 7))))
        (i32.or
          (i32.or
            (i32.eq (local.get $month) (i32.const 8))
            (i32.eq (local.get $month) (i32.const 10)))
          (i32.eq (local.get $month) (i32.const 12))))
      (then (return (i32.const 31))))
    i32.const 30)

  (func $m71days_before_year (param $year i32) (result i64)
    (local $y i64)
    (local.set $y (i64.extend_i32_s (i32.sub (local.get $year) (i32.const 1))))
    (i64.sub
      (i64.sub
        (i64.add
          (i64.mul (local.get $y) (i64.const 365))
          (i64.div_s (local.get $y) (i64.const 4)))
        (i64.div_s (local.get $y) (i64.const 100)))
      (i64.mul (i64.div_s (local.get $y) (i64.const 400)) (i64.const -1))))

  (func $m71days_since_epoch (param $year i32) (param $month i32) (param $day i32) (result i64)
    (local $m i32)
    (local $days i64)
    (local.set $days
      (i64.sub
        (call $m71days_before_year (local.get $year))
        (i64.const 719162)))
    (local.set $m (i32.const 1))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $m) (local.get $month)))
        (local.set $days
          (i64.add (local.get $days) (i64.extend_i32_u (call $m71month_days (local.get $year) (local.get $m)))))
        (local.set $m (i32.add (local.get $m) (i32.const 1)))
        (br $loop)))
    (i64.add (local.get $days) (i64.extend_i32_u (i32.sub (local.get $day) (i32.const 1)))))

  (func $m71unix_seconds
    (param $year i32) (param $month i32) (param $day i32) (param $hour i32) (param $minute i32) (param $second i32)
    (result i64)
    (i64.add
      (i64.add
        (i64.mul (call $m71days_since_epoch (local.get $year) (local.get $month) (local.get $day)) (i64.const 86400))
        (i64.mul (i64.extend_i32_u (local.get $hour)) (i64.const 3600)))
      (i64.add
        (i64.mul (i64.extend_i32_u (local.get $minute)) (i64.const 60))
        (i64.extend_i32_u (local.get $second)))))

  (func $m71valid_fields
    (param $year i32) (param $month i32) (param $day i32) (param $hour i32) (param $minute i32) (param $second i32)
    (result i32)
    (if (i32.or (i32.lt_u (local.get $month) (i32.const 1)) (i32.gt_u (local.get $month) (i32.const 12)))
      (then (return (i32.const 0))))
    (if (i32.or
          (i32.lt_u (local.get $day) (i32.const 1))
          (i32.gt_u (local.get $day) (call $m71month_days (local.get $year) (local.get $month))))
      (then (return (i32.const 0))))
    (if (i32.gt_u (local.get $hour) (i32.const 23)) (then (return (i32.const 0))))
    (if (i32.gt_u (local.get $minute) (i32.const 59)) (then (return (i32.const 0))))
    (if (i32.gt_u (local.get $second) (i32.const 59)) (then (return (i32.const 0))))
    i32.const 1)

  (func $parse_len (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $first i32)
    (local $value i32)
    (if (i32.lt_u (local.get $len) (i32.const 2))
      (then (return (i32.const 1))))
    (local.set $first (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))))
    (if (i32.lt_u (local.get $first) (i32.const 128))
      (then
        (i32.store (local.get $out) (i32.const 2))
        (i32.store (i32.add (local.get $out) (i32.const 4)) (local.get $first))
        (return (i32.const 0))))
    (if (i32.eq (local.get $first) (i32.const 129))
      (then
        (if (i32.lt_u (local.get $len) (i32.const 3)) (then (return (i32.const 1))))
        (local.set $value (i32.load8_u (i32.add (local.get $ptr) (i32.const 2))))
        (if (i32.lt_u (local.get $value) (i32.const 128)) (then (return (i32.const 3))))
        (i32.store (local.get $out) (i32.const 3))
        (i32.store (i32.add (local.get $out) (i32.const 4)) (local.get $value))
        (return (i32.const 0))))
    (return (i32.const 3)))

  (func $der_time_decode (export "der_time_decode") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $tag i32)
    (local $status i32)
    (local $hdr i32)
    (local $vlen i32)
    (local $body i32)
    (local $end i32)
    (local $kind i32)
    (local $year i32)
    (local $month i32)
    (local $day i32)
    (local $hour i32)
    (local $minute i32)
    (local $second i32)
    (local $unix i64)
    (local.set $status (call $parse_len (local.get $ptr) (local.get $len) (local.get $out)))
    (if (local.get $status) (then (return (local.get $status))))
    (local.set $hdr (i32.load (local.get $out)))
    (local.set $vlen (i32.load (i32.add (local.get $out) (i32.const 4))))
    (if (i32.lt_u (local.get $len) (i32.add (local.get $hdr) (local.get $vlen)))
      (then (return (i32.const 1))))
    (if (i32.ne (local.get $len) (i32.add (local.get $hdr) (local.get $vlen)))
      (then (return (i32.const 3))))
    (local.set $tag (i32.load8_u (local.get $ptr)))
    (local.set $body (i32.add (local.get $ptr) (local.get $hdr)))
    (local.set $end (i32.add (local.get $body) (local.get $vlen)))
    local.get $tag
    i32.const 23
    i32.eq
    if
      (if (i32.ne (local.get $vlen) (i32.const 13)) (then (return (i32.const 3))))
      (local.set $kind (i32.const 1))
      (local.set $year (call $m71two (local.get $body) (local.get $end)))
      (if (i32.lt_s (local.get $year) (i32.const 0)) (then (return (i32.const 3))))
      (local.set $year
        (select
          (i32.add (local.get $year) (i32.const 1900))
          (i32.add (local.get $year) (i32.const 2000))
          (i32.ge_u (local.get $year) (i32.const 50))))
      (local.set $month (call $m71two (i32.add (local.get $body) (i32.const 2)) (local.get $end)))
      (local.set $day (call $m71two (i32.add (local.get $body) (i32.const 4)) (local.get $end)))
      (local.set $hour (call $m71two (i32.add (local.get $body) (i32.const 6)) (local.get $end)))
      (local.set $minute (call $m71two (i32.add (local.get $body) (i32.const 8)) (local.get $end)))
      (local.set $second (call $m71two (i32.add (local.get $body) (i32.const 10)) (local.get $end)))
      (if (i32.ne (i32.load8_u (i32.add (local.get $body) (i32.const 12))) (i32.const 90))
        (then (return (i32.const 3))))
    else
      (if (i32.ne (local.get $tag) (i32.const 24)) (then (return (i32.const 3))))
      (if (i32.ne (local.get $vlen) (i32.const 15)) (then (return (i32.const 3))))
      (local.set $kind (i32.const 2))
      (local.set $year (call $m71four (local.get $body) (local.get $end)))
      (local.set $month (call $m71two (i32.add (local.get $body) (i32.const 4)) (local.get $end)))
      (local.set $day (call $m71two (i32.add (local.get $body) (i32.const 6)) (local.get $end)))
      (local.set $hour (call $m71two (i32.add (local.get $body) (i32.const 8)) (local.get $end)))
      (local.set $minute (call $m71two (i32.add (local.get $body) (i32.const 10)) (local.get $end)))
      (local.set $second (call $m71two (i32.add (local.get $body) (i32.const 12)) (local.get $end)))
      (if (i32.ne (i32.load8_u (i32.add (local.get $body) (i32.const 14))) (i32.const 90))
        (then (return (i32.const 3))))
    end
    (if
      (i32.or
        (i32.or
          (i32.or (i32.lt_s (local.get $month) (i32.const 0)) (i32.lt_s (local.get $day) (i32.const 0)))
          (i32.or (i32.lt_s (local.get $hour) (i32.const 0)) (i32.lt_s (local.get $minute) (i32.const 0))))
        (i32.lt_s (local.get $second) (i32.const 0)))
      (then (return (i32.const 3))))
    (if (i32.eqz (call $m71valid_fields (local.get $year) (local.get $month) (local.get $day) (local.get $hour) (local.get $minute) (local.get $second)))
      (then (return (i32.const 3))))
    (local.set $unix
      (call $m71unix_seconds
        (local.get $year) (local.get $month) (local.get $day)
        (local.get $hour) (local.get $minute) (local.get $second)))
    (i32.store (local.get $out) (local.get $kind))
    (i32.store (i32.add (local.get $out) (i32.const 4)) (local.get $year))
    (i32.store (i32.add (local.get $out) (i32.const 8)) (local.get $month))
    (i32.store (i32.add (local.get $out) (i32.const 12)) (local.get $day))
    (i32.store (i32.add (local.get $out) (i32.const 16)) (local.get $hour))
    (i32.store (i32.add (local.get $out) (i32.const 20)) (local.get $minute))
    (i32.store (i32.add (local.get $out) (i32.const 24)) (local.get $second))
    (i32.store (i32.add (local.get $out) (i32.const 28)) (i32.wrap_i64 (local.get $unix)))
    (i32.store (i32.add (local.get $out) (i32.const 32)) (i32.wrap_i64 (i64.shr_u (local.get $unix) (i64.const 32))))
    (i32.store (i32.add (local.get $out) (i32.const 36)) (local.get $hdr))
    (i32.store (i32.add (local.get $out) (i32.const 40)) (local.get $len))
    i32.const 0)

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
