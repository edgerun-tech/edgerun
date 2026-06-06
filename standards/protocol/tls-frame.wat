(func (export "proto_standard_id") (result i32)
    i32.const 300006)

  ;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid, 4 overflow.
  (func $valid_record_content_type (param $content_type i32) (result i32)
    (i32.or
      (i32.or
        (i32.eq (local.get $content_type) (i32.const 20))
        (i32.eq (local.get $content_type) (i32.const 21)))
      (i32.or
        (i32.eq (local.get $content_type) (i32.const 22))
        (i32.eq (local.get $content_type) (i32.const 23)))))

  (func $valid_record_version (param $version i32) (result i32)
    (i32.and
      (i32.ge_u (local.get $version) (i32.const 768))
      (i32.le_u (local.get $version) (i32.const 772))))

  (func $valid_record_fragment_len (param $fragment_len i32) (result i32)
    (i32.le_u (local.get $fragment_len) (i32.const 16384)))
  ;;
  ;; tls_record_header_decode return bits:
  ;; low16=status, next8=content_type, next16=version, next16=fragment_len.
  (func (export "tls_record_header_decode") (param $in_ptr i32) (param $in_len i32) (result i64)
    (local $content_type i32)
    (local $version i32)
    (local $fragment_len i32)
    (if (i32.lt_u (local.get $in_len) (i32.const 5))
      (then (return (i64.const 1))))
    (local.set $content_type (i32.load8_u (local.get $in_ptr)))
    (local.set $version
      (i32.or
        (i32.shl (i32.load8_u (i32.add (local.get $in_ptr) (i32.const 1))) (i32.const 8))
        (i32.load8_u (i32.add (local.get $in_ptr) (i32.const 2)))))
    (local.set $fragment_len
      (i32.or
        (i32.shl (i32.load8_u (i32.add (local.get $in_ptr) (i32.const 3))) (i32.const 8))
        (i32.load8_u (i32.add (local.get $in_ptr) (i32.const 4)))))
    (if
      (i32.eqz
        (i32.and
          (i32.and
            (call $valid_record_content_type (local.get $content_type))
            (call $valid_record_version (local.get $version)))
          (call $valid_record_fragment_len (local.get $fragment_len))))
      (then (return (i64.const 3))))
    (i64.or
      (i64.or
        (i64.shl (i64.extend_i32_u (local.get $fragment_len)) (i64.const 40))
        (i64.shl (i64.extend_i32_u (local.get $version)) (i64.const 24)))
      (i64.shl (i64.extend_i32_u (local.get $content_type)) (i64.const 16))))

  ;; Return bits: low32=status, high32=written.
  (func (export "tls_record_header_encode")
    (param $content_type i32) (param $version i32) (param $fragment_len i32)
    (param $out_ptr i32) (param $out_cap i32) (result i64)
    (if (i32.lt_u (local.get $out_cap) (i32.const 5))
      (then (return (i64.const 2))))
    (if
      (i32.or
        (i32.or
          (i32.gt_u (local.get $content_type) (i32.const 255))
          (i32.gt_u (local.get $version) (i32.const 65535)))
        (i32.gt_u (local.get $fragment_len) (i32.const 65535)))
      (then (return (i64.const 4))))
    (if
      (i32.eqz
        (i32.and
          (i32.and
            (call $valid_record_content_type (local.get $content_type))
            (call $valid_record_version (local.get $version)))
          (call $valid_record_fragment_len (local.get $fragment_len))))
      (then (return (i64.const 3))))
    (i32.store8 (local.get $out_ptr) (local.get $content_type))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 1)) (i32.shr_u (local.get $version) (i32.const 8)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 2)) (local.get $version))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 3)) (i32.shr_u (local.get $fragment_len) (i32.const 8)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $fragment_len))
    (i64.const 21474836480))

  ;; tls_handshake_header_decode return bits:
  ;; low16=status, next8=handshake_type, next24=body_len.
  (func (export "tls_handshake_header_decode") (param $in_ptr i32) (param $in_len i32) (result i64)
    (local $handshake_type i32)
    (local $body_len i32)
    (if (i32.lt_u (local.get $in_len) (i32.const 4))
      (then (return (i64.const 1))))
    (local.set $handshake_type (i32.load8_u (local.get $in_ptr)))
    (local.set $body_len
      (i32.or
        (i32.or
          (i32.shl (i32.load8_u (i32.add (local.get $in_ptr) (i32.const 1))) (i32.const 16))
          (i32.shl (i32.load8_u (i32.add (local.get $in_ptr) (i32.const 2))) (i32.const 8)))
        (i32.load8_u (i32.add (local.get $in_ptr) (i32.const 3)))))
    (i64.or
      (i64.shl (i64.extend_i32_u (local.get $body_len)) (i64.const 24))
      (i64.shl (i64.extend_i32_u (local.get $handshake_type)) (i64.const 16))))

  ;; Return bits: low32=status, high32=written.
  (func (export "tls_handshake_header_encode")
    (param $handshake_type i32) (param $body_len i32)
    (param $out_ptr i32) (param $out_cap i32) (result i64)
    (if (i32.lt_u (local.get $out_cap) (i32.const 4))
      (then (return (i64.const 2))))
    (if
      (i32.or
        (i32.gt_u (local.get $handshake_type) (i32.const 255))
        (i32.gt_u (local.get $body_len) (i32.const 16777215)))
      (then (return (i64.const 4))))
    (i32.store8 (local.get $out_ptr) (local.get $handshake_type))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 1)) (i32.shr_u (local.get $body_len) (i32.const 16)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 2)) (i32.shr_u (local.get $body_len) (i32.const 8)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 3)) (local.get $body_len))
    (i64.const 17179869184))
