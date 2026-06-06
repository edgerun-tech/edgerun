
  ;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid, 4 overflow.
  ;; x509_name_scan out record:
  ;; 0 name_body_ptr, 4 name_body_len, 8 name_header_len, 12 name_total_len,
  ;; 16 rdn_count, 20 attribute_count,
  ;; 24 commonName_value_ptr, 28 commonName_value_len, 32 commonName_value_tag, 36 commonName_tlv_total_len,
  ;; 40 organization_value_ptr, 44 organization_value_len, 48 organization_value_tag, 52 organization_tlv_total_len.

  (func $m201is_supported_tag (param $m201tag i32) (result i32)
    (if (i32.eq (i32.and (local.get $m201tag) (i32.const 31)) (i32.const 31))
      (then (return (i32.const 0))))
    i32.const 1)

  ;; DER header decoder for single-octet tags and definite lengths.
  ;; Return bits: low16=status, next16=header_len, next8=tag, high24=length.
  (func $m201header_decode (param $ptr i32) (param $len i32) (result i64)
    (local $m201tag i32)
    (local $first i32)
    (local $len_len i32)
    (local $i i32)
    (local $value i64)
    (local $consumed i32)
    (if (i32.lt_u (local.get $len) (i32.const 2))
      (then (return (i64.const 1))))
    (local.set $m201tag (i32.load8_u (local.get $ptr)))
    (if (i32.eqz (call $m201is_supported_tag (local.get $m201tag)))
      (then (return (i64.const 3))))
    (local.set $first (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))))
    (if (i32.lt_u (local.get $first) (i32.const 128))
      (then
        (return
          (i64.or
            (i64.or
              (i64.shl (i64.extend_i32_u (local.get $first)) (i64.const 40))
              (i64.shl (i64.extend_i32_u (local.get $m201tag)) (i64.const 32)))
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
          (i64.shl (i64.extend_i32_u (local.get $m201tag)) (i64.const 32)))
        (i64.shl (i64.extend_i32_u (local.get $consumed)) (i64.const 16)))
      (i64.const 0)))

  (func $m201status (param $h i64) (result i32)
    (i32.wrap_i64 (i64.and (local.get $h) (i64.const 65535))))

  (func $m201hdr_len (param $h i64) (result i32)
    (i32.wrap_i64 (i64.and (i64.shr_u (local.get $h) (i64.const 16)) (i64.const 65535))))

  (func $m201tag (param $h i64) (result i32)
    (i32.wrap_i64 (i64.and (i64.shr_u (local.get $h) (i64.const 32)) (i64.const 255))))

  (func $m201value_len (param $h i64) (result i32)
    (i32.wrap_i64 (i64.and (i64.shr_u (local.get $h) (i64.const 40)) (i64.const 16777215))))

  (func $is_directory_string_tag (param $m201tag i32) (result i32)
    (if
      (i32.or
        (i32.or
          (i32.eq (local.get $m201tag) (i32.const 12))
          (i32.eq (local.get $m201tag) (i32.const 19)))
        (i32.or
          (i32.or
            (i32.eq (local.get $m201tag) (i32.const 20))
            (i32.eq (local.get $m201tag) (i32.const 30)))
          (i32.eq (local.get $m201tag) (i32.const 22))))
      (then (return (i32.const 1))))
    i32.const 0)

  (func $validate_oid_value (param $ptr i32) (param $len i32) (result i32)
    (local $offset i32)
    (local $byte i32)
    (local $count i32)
    (local $arc i64)
    (if (i32.eqz (local.get $len))
      (then (return (i32.const 1))))
    (if (i32.gt_u (i32.load8_u (local.get $ptr)) (i32.const 119))
      (then (return (i32.const 3))))
    (local.set $offset (i32.const 1))
    (block $done
      (loop $arc_loop
        (br_if $done (i32.ge_u (local.get $offset) (local.get $len)))
        (local.set $count (i32.const 0))
        (local.set $arc (i64.const 0))
        (block $arc_done
          (loop $byte_loop
            (if (i32.ge_u (local.get $offset) (local.get $len))
              (then (return (i32.const 1))))
            (local.set $byte (i32.load8_u (i32.add (local.get $ptr) (local.get $offset))))
            (if
              (i32.and
                (i32.and
                  (i32.eqz (local.get $count))
                  (i32.eq (local.get $byte) (i32.const 128)))
                (i32.lt_u (i32.add (local.get $offset) (i32.const 1)) (local.get $len)))
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
            (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
            (br_if $arc_done (i32.eqz (i32.and (local.get $byte) (i32.const 128))))
            (br $byte_loop)))
        (br $arc_loop)))
    i32.const 0)

  (func $oid_is_common_name (param $ptr i32) (param $len i32) (result i32)
    (if (i32.ne (local.get $len) (i32.const 3))
      (then (return (i32.const 0))))
    (if (i32.ne (i32.load8_u (local.get $ptr)) (i32.const 85))
      (then (return (i32.const 0))))
    (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))) (i32.const 4))
      (then (return (i32.const 0))))
    (select
      (i32.const 1)
      (i32.const 0)
      (i32.eq (i32.load8_u (i32.add (local.get $ptr) (i32.const 2))) (i32.const 3))))

  (func $oid_is_organization (param $ptr i32) (param $len i32) (result i32)
    (if (i32.ne (local.get $len) (i32.const 3))
      (then (return (i32.const 0))))
    (if (i32.ne (i32.load8_u (local.get $ptr)) (i32.const 85))
      (then (return (i32.const 0))))
    (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))) (i32.const 4))
      (then (return (i32.const 0))))
    (select
      (i32.const 1)
      (i32.const 0)
      (i32.eq (i32.load8_u (i32.add (local.get $ptr) (i32.const 2))) (i32.const 10))))

  (func $store_value_span (param $out i32) (param $base i32) (param $value_ptr i32) (param $m201value_len i32) (param $m201tag i32) (param $total i32)
    (i32.store (i32.add (local.get $out) (local.get $base)) (local.get $value_ptr))
    (i32.store (i32.add (local.get $out) (i32.add (local.get $base) (i32.const 4))) (local.get $m201value_len))
    (i32.store (i32.add (local.get $out) (i32.add (local.get $base) (i32.const 8))) (local.get $m201tag))
    (i32.store (i32.add (local.get $out) (i32.add (local.get $base) (i32.const 12))) (local.get $total))
  )

  (func (export "x509_name_scan") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $h i64)
    (local $m201status i32)
    (local $hdr i32)
    (local $vlen i32)
    (local $body_ptr i32)
    (local $body_end i32)
    (local $rdn_ptr i32)
    (local $rdn_hdr i32)
    (local $rdn_len i32)
    (local $rdn_end i32)
    (local $attr_ptr i32)
    (local $attr_hdr i32)
    (local $attr_len i32)
    (local $attr_end i32)
    (local $oid_ptr i32)
    (local $oid_hdr i32)
    (local $oid_len i32)
    (local $oid_value_ptr i32)
    (local $value_ptr i32)
    (local $value_hdr i32)
    (local $m201value_len i32)
    (local $value_tag i32)
    (local $rdn_count i32)
    (local $attr_count i32)

    (i64.store (local.get $out) (i64.const 0))
    (i64.store (i32.add (local.get $out) (i32.const 8)) (i64.const 0))
    (i64.store (i32.add (local.get $out) (i32.const 16)) (i64.const 0))
    (i64.store (i32.add (local.get $out) (i32.const 24)) (i64.const 0))
    (i64.store (i32.add (local.get $out) (i32.const 32)) (i64.const 0))
    (i64.store (i32.add (local.get $out) (i32.const 40)) (i64.const 0))
    (i64.store (i32.add (local.get $out) (i32.const 48)) (i64.const 0))

    (local.set $h (call $m201header_decode (local.get $ptr) (local.get $len)))
    (local.set $m201status (call $m201status (local.get $h)))
    (if (local.get $m201status) (then (return (local.get $m201status))))
    (local.set $hdr (call $m201hdr_len (local.get $h)))
    (local.set $vlen (call $m201value_len (local.get $h)))
    (if (i32.ne (call $m201tag (local.get $h)) (i32.const 48))
      (then (return (i32.const 3))))
    (if (i32.lt_u (local.get $len) (i32.add (local.get $hdr) (local.get $vlen)))
      (then (return (i32.const 1))))
    (if (i32.ne (local.get $len) (i32.add (local.get $hdr) (local.get $vlen)))
      (then (return (i32.const 3))))

    (local.set $body_ptr (i32.add (local.get $ptr) (local.get $hdr)))
    (local.set $body_end (i32.add (local.get $body_ptr) (local.get $vlen)))
    (i32.store (local.get $out) (local.get $body_ptr))
    (i32.store (i32.add (local.get $out) (i32.const 4)) (local.get $vlen))
    (i32.store (i32.add (local.get $out) (i32.const 8)) (local.get $hdr))
    (i32.store (i32.add (local.get $out) (i32.const 12)) (local.get $len))

    (local.set $rdn_ptr (local.get $body_ptr))
    (block $done
      (loop $rdn_loop
        (br_if $done (i32.ge_u (local.get $rdn_ptr) (local.get $body_end)))
        (local.set $h (call $m201header_decode (local.get $rdn_ptr) (i32.sub (local.get $body_end) (local.get $rdn_ptr))))
        (local.set $m201status (call $m201status (local.get $h)))
        (if (local.get $m201status) (then (return (local.get $m201status))))
        (if (i32.ne (call $m201tag (local.get $h)) (i32.const 49))
          (then (return (i32.const 3))))
        (local.set $rdn_hdr (call $m201hdr_len (local.get $h)))
        (local.set $rdn_len (call $m201value_len (local.get $h)))
        (if (i32.lt_u (i32.sub (local.get $body_end) (local.get $rdn_ptr)) (i32.add (local.get $rdn_hdr) (local.get $rdn_len)))
          (then (return (i32.const 1))))
        (local.set $attr_ptr (i32.add (local.get $rdn_ptr) (local.get $rdn_hdr)))
        (local.set $rdn_end (i32.add (local.get $attr_ptr) (local.get $rdn_len)))
        (if (i32.eq (local.get $attr_ptr) (local.get $rdn_end))
          (then (return (i32.const 3))))
        (local.set $rdn_count (i32.add (local.get $rdn_count) (i32.const 1)))

        (block $attrs_done
          (loop $attrs_loop
            (br_if $attrs_done (i32.ge_u (local.get $attr_ptr) (local.get $rdn_end)))
            (local.set $h (call $m201header_decode (local.get $attr_ptr) (i32.sub (local.get $rdn_end) (local.get $attr_ptr))))
            (local.set $m201status (call $m201status (local.get $h)))
            (if (local.get $m201status) (then (return (local.get $m201status))))
            (if (i32.ne (call $m201tag (local.get $h)) (i32.const 48))
              (then (return (i32.const 3))))
            (local.set $attr_hdr (call $m201hdr_len (local.get $h)))
            (local.set $attr_len (call $m201value_len (local.get $h)))
            (if (i32.lt_u (i32.sub (local.get $rdn_end) (local.get $attr_ptr)) (i32.add (local.get $attr_hdr) (local.get $attr_len)))
              (then (return (i32.const 1))))
            (local.set $oid_ptr (i32.add (local.get $attr_ptr) (local.get $attr_hdr)))
            (local.set $attr_end (i32.add (local.get $oid_ptr) (local.get $attr_len)))

            (local.set $h (call $m201header_decode (local.get $oid_ptr) (i32.sub (local.get $attr_end) (local.get $oid_ptr))))
            (local.set $m201status (call $m201status (local.get $h)))
            (if (local.get $m201status) (then (return (local.get $m201status))))
            (if (i32.ne (call $m201tag (local.get $h)) (i32.const 6))
              (then (return (i32.const 3))))
            (local.set $oid_hdr (call $m201hdr_len (local.get $h)))
            (local.set $oid_len (call $m201value_len (local.get $h)))
            (if (i32.lt_u (i32.sub (local.get $attr_end) (local.get $oid_ptr)) (i32.add (local.get $oid_hdr) (local.get $oid_len)))
              (then (return (i32.const 1))))
            (local.set $oid_value_ptr (i32.add (local.get $oid_ptr) (local.get $oid_hdr)))
            (local.set $m201status (call $validate_oid_value (local.get $oid_value_ptr) (local.get $oid_len)))
            (if (local.get $m201status) (then (return (local.get $m201status))))

            (local.set $value_ptr (i32.add (local.get $oid_value_ptr) (local.get $oid_len)))
            (if (i32.ge_u (local.get $value_ptr) (local.get $attr_end))
              (then (return (i32.const 3))))
            (local.set $h (call $m201header_decode (local.get $value_ptr) (i32.sub (local.get $attr_end) (local.get $value_ptr))))
            (local.set $m201status (call $m201status (local.get $h)))
            (if (local.get $m201status) (then (return (local.get $m201status))))
            (local.set $value_tag (call $m201tag (local.get $h)))
            (local.set $value_hdr (call $m201hdr_len (local.get $h)))
            (local.set $m201value_len (call $m201value_len (local.get $h)))
            (if (i32.lt_u (i32.sub (local.get $attr_end) (local.get $value_ptr)) (i32.add (local.get $value_hdr) (local.get $m201value_len)))
              (then (return (i32.const 1))))
            (if (i32.ne (i32.add (i32.add (local.get $value_ptr) (local.get $value_hdr)) (local.get $m201value_len)) (local.get $attr_end))
              (then (return (i32.const 3))))
            (if (i32.eqz (call $is_directory_string_tag (local.get $value_tag)))
              (then (return (i32.const 3))))

            (if
              (i32.and
                (i32.eqz (i32.load (i32.add (local.get $out) (i32.const 28))))
                (call $oid_is_common_name (local.get $oid_value_ptr) (local.get $oid_len)))
              (then
                (call $store_value_span
                  (local.get $out) (i32.const 24)
                  (i32.add (local.get $value_ptr) (local.get $value_hdr))
                  (local.get $m201value_len)
                  (local.get $value_tag)
                  (i32.add (local.get $value_hdr) (local.get $m201value_len)))))
            (if
              (i32.and
                (i32.eqz (i32.load (i32.add (local.get $out) (i32.const 44))))
                (call $oid_is_organization (local.get $oid_value_ptr) (local.get $oid_len)))
              (then
                (call $store_value_span
                  (local.get $out) (i32.const 40)
                  (i32.add (local.get $value_ptr) (local.get $value_hdr))
                  (local.get $m201value_len)
                  (local.get $value_tag)
                  (i32.add (local.get $value_hdr) (local.get $m201value_len)))))

            (local.set $attr_count (i32.add (local.get $attr_count) (i32.const 1)))
            (local.set $attr_ptr (local.get $attr_end))
            (br $attrs_loop)))

        (local.set $rdn_ptr (local.get $rdn_end))
        (br $rdn_loop)))

    (i32.store (i32.add (local.get $out) (i32.const 16)) (local.get $rdn_count))
    (i32.store (i32.add (local.get $out) (i32.const 20)) (local.get $attr_count))
    i32.const 0)
