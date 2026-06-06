(module
  (import "edgerun" "string_eq" (func $m30eq (param i32 i32 i32 i32) (result i32)))
  (import "edgerun" "is_upper" (func $m169is_upper (param i32) (result i32)))
  (import "edgerun" "is_lower" (func $m169is_lower (param i32) (result i32)))
  (import "edgerun" "load8_u" (func $m76byte (param i32 i32) (result i32)))
  (import "edgerun" "to_lower" (func $m28lower (param i32) (result i32)))
  (import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))
  (import "edgerun" "lo" (func $lo (param i64) (result i32)))
  (import "edgerun" "hi" (func $hi (param i64) (result i32)))
  (import "edgerun" "is_digit" (func $is_digit (param i32) (result i32)))
  (import "edgerun" "is_hex" (func $is_hex (param i32) (result i32)))
  (memory (export "memory") 1)

  ;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid, 4 overflow.

  (func $m200is_supported_tag (param $m200tag i32) (result i32)
    (if (i32.eq (i32.and (local.get $m200tag) (i32.const 31)) (i32.const 31))
      (then (return (i32.const 0))))
    i32.const 1)

  ;; DER header decoder for single-octet tags and definite lengths.
  ;; Return bits: low16=status, next16=header_len, next8=tag, high24=length.
  (func $m200header_decode (param $ptr i32) (param $len i32) (result i64)
    (local $m200tag i32)
    (local $first i32)
    (local $len_len i32)
    (local $i i32)
    (local $value i64)
    (local $consumed i32)
    (if (i32.lt_u (local.get $len) (i32.const 2))
      (then (return (i64.const 1))))
    (local.set $m200tag (i32.load8_u (local.get $ptr)))
    (if (i32.eqz (call $m200is_supported_tag (local.get $m200tag)))
      (then (return (i64.const 3))))
    (local.set $first (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))))
    (if (i32.lt_u (local.get $first) (i32.const 128))
      (then
        (return
          (i64.or
            (i64.or
              (i64.shl (i64.extend_i32_u (local.get $first)) (i64.const 40))
              (i64.shl (i64.extend_i32_u (local.get $m200tag)) (i64.const 32)))
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
          (i64.shl (i64.extend_i32_u (local.get $m200tag)) (i64.const 32)))
        (i64.shl (i64.extend_i32_u (local.get $consumed)) (i64.const 16)))
      (i64.const 0)))

  (func $m200status (param $h i64) (result i32)
    (i32.wrap_i64 (i64.and (local.get $h) (i64.const 65535))))

  (func $m200hdr_len (param $h i64) (result i32)
    (i32.wrap_i64 (i64.and (i64.shr_u (local.get $h) (i64.const 16)) (i64.const 65535))))

  (func $m200tag (param $h i64) (result i32)
    (i32.wrap_i64 (i64.and (i64.shr_u (local.get $h) (i64.const 32)) (i64.const 255))))

  (func $m200value_len (param $h i64) (result i32)
    (i32.wrap_i64 (i64.and (i64.shr_u (local.get $h) (i64.const 40)) (i64.const 16777215))))

  (func $m200write_span (param $out i32) (param $ptr i32) (param $len i32) (param $hdr i32) (param $total i32)
    (i32.store (local.get $out) (local.get $ptr))
    (i32.store (i32.add (local.get $out) (i32.const 4)) (local.get $len))
    (i32.store (i32.add (local.get $out) (i32.const 8)) (local.get $hdr))
    (i32.store (i32.add (local.get $out) (i32.const 12)) (local.get $total)))

  (func $read_child (param $body_ptr i32) (param $body_len i32) (param $offset i32) (param $out i32) (result i32)
    (local $h i64)
    (local $m200status i32)
    (local $hdr i32)
    (local $vlen i32)
    (local $total i32)
    (local $remaining i32)
    (if (i32.ge_u (local.get $offset) (local.get $body_len))
      (then (return (i32.const 1))))
    (local.set $remaining (i32.sub (local.get $body_len) (local.get $offset)))
    (local.set $h (call $m200header_decode (i32.add (local.get $body_ptr) (local.get $offset)) (local.get $remaining)))
    (local.set $m200status (call $m200status (local.get $h)))
    (if (local.get $m200status) (then (return (local.get $m200status))))
    (local.set $hdr (call $m200hdr_len (local.get $h)))
    (local.set $vlen (call $m200value_len (local.get $h)))
    (local.set $total (i32.add (local.get $hdr) (local.get $vlen)))
    (if (i32.lt_u (local.get $remaining) (local.get $total))
      (then (return (i32.const 1))))
    (i32.store (local.get $out) (call $m200tag (local.get $h)))
    (i32.store (i32.add (local.get $out) (i32.const 4)) (i32.add (local.get $body_ptr) (local.get $offset)))
    (i32.store (i32.add (local.get $out) (i32.const 8)) (local.get $hdr))
    (i32.store (i32.add (local.get $out) (i32.const 12)) (local.get $vlen))
    (i32.store (i32.add (local.get $out) (i32.const 16)) (local.get $total))
    (i32.store (i32.add (local.get $out) (i32.const 20)) (i32.add (local.get $offset) (local.get $total)))
    i32.const 0)

  ;; Scan a DER X.509 Certificate.
  ;; out record:
  ;; cert_body_ptr, cert_body_len, cert_header_len, cert_total_len,
  ;; tbs_ptr, tbs_total_len, tbs_header_len, tbs_body_len,
  ;; signature_algorithm_ptr, signature_algorithm_total_len, signature_algorithm_header_len, signature_algorithm_body_len,
  ;; signature_value_ptr, signature_value_total_len, signature_value_header_len,
  ;; signature_value_payload_ptr, signature_value_payload_len, signature_value_unused_bits.
  (func (export "x509_certificate_scan") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $h i64)
    (local $m200status i32)
    (local $hdr i32)
    (local $vlen i32)
    (local $body_ptr i32)
    (local $offset i32)
    (local $m200tag i32)
    (local $child_ptr i32)
    (local $child_hdr i32)
    (local $child_vlen i32)
    (local $child_total i32)
    (local $bit_payload_ptr i32)
    (local $bit_payload_len i32)
    (local $unused i32)
    (local.set $h (call $m200header_decode (local.get $ptr) (local.get $len)))
    (local.set $m200status (call $m200status (local.get $h)))
    (if (local.get $m200status) (then (return (local.get $m200status))))
    (local.set $hdr (call $m200hdr_len (local.get $h)))
    (local.set $m200tag (call $m200tag (local.get $h)))
    (local.set $vlen (call $m200value_len (local.get $h)))
    (if (i32.ne (local.get $m200tag) (i32.const 48)) (then (return (i32.const 3))))
    (if (i32.lt_u (local.get $len) (i32.add (local.get $hdr) (local.get $vlen))) (then (return (i32.const 1))))
    (if (i32.ne (local.get $len) (i32.add (local.get $hdr) (local.get $vlen))) (then (return (i32.const 3))))
    (local.set $body_ptr (i32.add (local.get $ptr) (local.get $hdr)))
    (call $m200write_span (local.get $out) (local.get $body_ptr) (local.get $vlen) (local.get $hdr) (local.get $len))

    (local.set $m200status (call $read_child (local.get $body_ptr) (local.get $vlen) (i32.const 0) (i32.add (local.get $out) (i32.const 72))))
    (if (local.get $m200status) (then (return (local.get $m200status))))
    (local.set $m200tag (i32.load (i32.add (local.get $out) (i32.const 72))))
    (local.set $child_ptr (i32.load (i32.add (local.get $out) (i32.const 76))))
    (local.set $child_hdr (i32.load (i32.add (local.get $out) (i32.const 80))))
    (local.set $child_vlen (i32.load (i32.add (local.get $out) (i32.const 84))))
    (local.set $child_total (i32.load (i32.add (local.get $out) (i32.const 88))))
    (local.set $offset (i32.load (i32.add (local.get $out) (i32.const 92))))
    (if (i32.ne (local.get $m200tag) (i32.const 48)) (then (return (i32.const 3))))
    (i32.store (i32.add (local.get $out) (i32.const 16)) (local.get $child_ptr))
    (i32.store (i32.add (local.get $out) (i32.const 20)) (local.get $child_total))
    (i32.store (i32.add (local.get $out) (i32.const 24)) (local.get $child_hdr))
    (i32.store (i32.add (local.get $out) (i32.const 28)) (local.get $child_vlen))

    (local.set $m200status (call $read_child (local.get $body_ptr) (local.get $vlen) (local.get $offset) (i32.add (local.get $out) (i32.const 72))))
    (if (local.get $m200status) (then (return (local.get $m200status))))
    (local.set $m200tag (i32.load (i32.add (local.get $out) (i32.const 72))))
    (local.set $child_ptr (i32.load (i32.add (local.get $out) (i32.const 76))))
    (local.set $child_hdr (i32.load (i32.add (local.get $out) (i32.const 80))))
    (local.set $child_vlen (i32.load (i32.add (local.get $out) (i32.const 84))))
    (local.set $child_total (i32.load (i32.add (local.get $out) (i32.const 88))))
    (local.set $offset (i32.load (i32.add (local.get $out) (i32.const 92))))
    (if (i32.ne (local.get $m200tag) (i32.const 48)) (then (return (i32.const 3))))
    (i32.store (i32.add (local.get $out) (i32.const 32)) (local.get $child_ptr))
    (i32.store (i32.add (local.get $out) (i32.const 36)) (local.get $child_total))
    (i32.store (i32.add (local.get $out) (i32.const 40)) (local.get $child_hdr))
    (i32.store (i32.add (local.get $out) (i32.const 44)) (local.get $child_vlen))

    (local.set $m200status (call $read_child (local.get $body_ptr) (local.get $vlen) (local.get $offset) (i32.add (local.get $out) (i32.const 72))))
    (if (local.get $m200status) (then (return (local.get $m200status))))
    (local.set $m200tag (i32.load (i32.add (local.get $out) (i32.const 72))))
    (local.set $child_ptr (i32.load (i32.add (local.get $out) (i32.const 76))))
    (local.set $child_hdr (i32.load (i32.add (local.get $out) (i32.const 80))))
    (local.set $child_vlen (i32.load (i32.add (local.get $out) (i32.const 84))))
    (local.set $child_total (i32.load (i32.add (local.get $out) (i32.const 88))))
    (local.set $offset (i32.load (i32.add (local.get $out) (i32.const 92))))
    (if (i32.ne (local.get $m200tag) (i32.const 3)) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $child_vlen)) (then (return (i32.const 3))))
    (local.set $bit_payload_ptr (i32.add (i32.add (local.get $child_ptr) (local.get $child_hdr)) (i32.const 1)))
    (local.set $bit_payload_len (i32.sub (local.get $child_vlen) (i32.const 1)))
    (local.set $unused (i32.load8_u (i32.add (local.get $child_ptr) (local.get $child_hdr))))
    (if (i32.gt_u (local.get $unused) (i32.const 7)) (then (return (i32.const 3))))
    (if (i32.and (i32.ne (local.get $unused) (i32.const 0)) (i32.eq (local.get $child_vlen) (i32.const 1)))
      (then (return (i32.const 3))))
    (i32.store (i32.add (local.get $out) (i32.const 48)) (local.get $child_ptr))
    (i32.store (i32.add (local.get $out) (i32.const 52)) (local.get $child_total))
    (i32.store (i32.add (local.get $out) (i32.const 56)) (local.get $child_hdr))
    (i32.store (i32.add (local.get $out) (i32.const 60)) (local.get $bit_payload_ptr))
    (i32.store (i32.add (local.get $out) (i32.const 64)) (local.get $bit_payload_len))
    (i32.store (i32.add (local.get $out) (i32.const 68)) (local.get $unused))
    (if (i32.ne (local.get $offset) (local.get $vlen)) (then (return (i32.const 3))))
    i32.const 0)

  ;; Scan TBSCertificate and return exact spans for required fields.
  ;; Optional version and extensions spans are zeroed when absent.
  (func (export "x509_tbs_certificate_scan") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $h i64)
    (local $m200status i32)
    (local $hdr i32)
    (local $vlen i32)
    (local $body_ptr i32)
    (local $offset i32)
    (local $m200tag i32)
    (local $child_ptr i32)
    (local $child_hdr i32)
    (local $child_vlen i32)
    (local $child_total i32)
    (local $extensions_seen i32)
    (local.set $h (call $m200header_decode (local.get $ptr) (local.get $len)))
    (local.set $m200status (call $m200status (local.get $h)))
    (if (local.get $m200status) (then (return (local.get $m200status))))
    (local.set $hdr (call $m200hdr_len (local.get $h)))
    (local.set $m200tag (call $m200tag (local.get $h)))
    (local.set $vlen (call $m200value_len (local.get $h)))
    (if (i32.ne (local.get $m200tag) (i32.const 48)) (then (return (i32.const 3))))
    (if (i32.lt_u (local.get $len) (i32.add (local.get $hdr) (local.get $vlen))) (then (return (i32.const 1))))
    (local.set $body_ptr (i32.add (local.get $ptr) (local.get $hdr)))
    (call $m200write_span (local.get $out) (local.get $body_ptr) (local.get $vlen) (local.get $hdr) (i32.add (local.get $hdr) (local.get $vlen)))
    (i32.store (i32.add (local.get $out) (i32.const 16)) (i32.const 0))
    (i32.store (i32.add (local.get $out) (i32.const 20)) (i32.const 0))
    (i32.store (i32.add (local.get $out) (i32.const 24)) (i32.const 0))
    (i32.store (i32.add (local.get $out) (i32.const 28)) (i32.const 0))
    (i32.store (i32.add (local.get $out) (i32.const 32)) (i32.const 0))
    (i32.store (i32.add (local.get $out) (i32.const 132)) (i32.const 0))
    (i32.store (i32.add (local.get $out) (i32.const 136)) (i32.const 0))
    (i32.store (i32.add (local.get $out) (i32.const 140)) (i32.const 0))
    (i32.store (i32.add (local.get $out) (i32.const 144)) (i32.const 0))
    (i32.store (i32.add (local.get $out) (i32.const 148)) (i32.const 0))

    (local.set $m200status (call $read_child (local.get $body_ptr) (local.get $vlen) (i32.const 0) (i32.add (local.get $out) (i32.const 152))))
    (if (local.get $m200status) (then (return (local.get $m200status))))
    (local.set $m200tag (i32.load (i32.add (local.get $out) (i32.const 152))))
    (local.set $offset (i32.const 0))
    (if (i32.eq (local.get $m200tag) (i32.const 160))
      (then
        (local.set $child_ptr (i32.load (i32.add (local.get $out) (i32.const 156))))
        (local.set $child_hdr (i32.load (i32.add (local.get $out) (i32.const 160))))
        (local.set $child_vlen (i32.load (i32.add (local.get $out) (i32.const 164))))
        (local.set $child_total (i32.load (i32.add (local.get $out) (i32.const 168))))
        (local.set $offset (i32.load (i32.add (local.get $out) (i32.const 172))))
        (call $m200write_span (i32.add (local.get $out) (i32.const 16)) (local.get $child_ptr) (local.get $child_vlen) (local.get $child_hdr) (local.get $child_total))
        (i32.store (i32.add (local.get $out) (i32.const 32)) (i32.const 1))))

    (local.set $m200status (call $read_child (local.get $body_ptr) (local.get $vlen) (local.get $offset) (i32.add (local.get $out) (i32.const 152))))
    (if (local.get $m200status) (then (return (local.get $m200status))))
    (if (i32.ne (i32.load (i32.add (local.get $out) (i32.const 152))) (i32.const 2)) (then (return (i32.const 3))))
    (call $m200write_span (i32.add (local.get $out) (i32.const 36)) (i32.load (i32.add (local.get $out) (i32.const 156))) (i32.load (i32.add (local.get $out) (i32.const 164))) (i32.load (i32.add (local.get $out) (i32.const 160))) (i32.load (i32.add (local.get $out) (i32.const 168))))
    (local.set $offset (i32.load (i32.add (local.get $out) (i32.const 172))))

    (local.set $m200status (call $read_child (local.get $body_ptr) (local.get $vlen) (local.get $offset) (i32.add (local.get $out) (i32.const 152))))
    (if (local.get $m200status) (then (return (local.get $m200status))))
    (if (i32.ne (i32.load (i32.add (local.get $out) (i32.const 152))) (i32.const 48)) (then (return (i32.const 3))))
    (call $m200write_span (i32.add (local.get $out) (i32.const 52)) (i32.load (i32.add (local.get $out) (i32.const 156))) (i32.load (i32.add (local.get $out) (i32.const 164))) (i32.load (i32.add (local.get $out) (i32.const 160))) (i32.load (i32.add (local.get $out) (i32.const 168))))
    (local.set $offset (i32.load (i32.add (local.get $out) (i32.const 172))))

    (local.set $m200status (call $read_child (local.get $body_ptr) (local.get $vlen) (local.get $offset) (i32.add (local.get $out) (i32.const 152))))
    (if (local.get $m200status) (then (return (local.get $m200status))))
    (if (i32.ne (i32.load (i32.add (local.get $out) (i32.const 152))) (i32.const 48)) (then (return (i32.const 3))))
    (call $m200write_span (i32.add (local.get $out) (i32.const 68)) (i32.load (i32.add (local.get $out) (i32.const 156))) (i32.load (i32.add (local.get $out) (i32.const 164))) (i32.load (i32.add (local.get $out) (i32.const 160))) (i32.load (i32.add (local.get $out) (i32.const 168))))
    (local.set $offset (i32.load (i32.add (local.get $out) (i32.const 172))))

    (local.set $m200status (call $read_child (local.get $body_ptr) (local.get $vlen) (local.get $offset) (i32.add (local.get $out) (i32.const 152))))
    (if (local.get $m200status) (then (return (local.get $m200status))))
    (if (i32.ne (i32.load (i32.add (local.get $out) (i32.const 152))) (i32.const 48)) (then (return (i32.const 3))))
    (call $m200write_span (i32.add (local.get $out) (i32.const 84)) (i32.load (i32.add (local.get $out) (i32.const 156))) (i32.load (i32.add (local.get $out) (i32.const 164))) (i32.load (i32.add (local.get $out) (i32.const 160))) (i32.load (i32.add (local.get $out) (i32.const 168))))
    (local.set $offset (i32.load (i32.add (local.get $out) (i32.const 172))))

    (local.set $m200status (call $read_child (local.get $body_ptr) (local.get $vlen) (local.get $offset) (i32.add (local.get $out) (i32.const 152))))
    (if (local.get $m200status) (then (return (local.get $m200status))))
    (if (i32.ne (i32.load (i32.add (local.get $out) (i32.const 152))) (i32.const 48)) (then (return (i32.const 3))))
    (call $m200write_span (i32.add (local.get $out) (i32.const 100)) (i32.load (i32.add (local.get $out) (i32.const 156))) (i32.load (i32.add (local.get $out) (i32.const 164))) (i32.load (i32.add (local.get $out) (i32.const 160))) (i32.load (i32.add (local.get $out) (i32.const 168))))
    (local.set $offset (i32.load (i32.add (local.get $out) (i32.const 172))))

    (local.set $m200status (call $read_child (local.get $body_ptr) (local.get $vlen) (local.get $offset) (i32.add (local.get $out) (i32.const 152))))
    (if (local.get $m200status) (then (return (local.get $m200status))))
    (if (i32.ne (i32.load (i32.add (local.get $out) (i32.const 152))) (i32.const 48)) (then (return (i32.const 3))))
    (call $m200write_span (i32.add (local.get $out) (i32.const 116)) (i32.load (i32.add (local.get $out) (i32.const 156))) (i32.load (i32.add (local.get $out) (i32.const 164))) (i32.load (i32.add (local.get $out) (i32.const 160))) (i32.load (i32.add (local.get $out) (i32.const 168))))
    (local.set $offset (i32.load (i32.add (local.get $out) (i32.const 172))))

    (block $done
      (loop $loop
        (br_if $done (i32.eq (local.get $offset) (local.get $vlen)))
        (local.set $m200status (call $read_child (local.get $body_ptr) (local.get $vlen) (local.get $offset) (i32.add (local.get $out) (i32.const 152))))
        (if (local.get $m200status) (then (return (local.get $m200status))))
        (local.set $m200tag (i32.load (i32.add (local.get $out) (i32.const 152))))
        (if (i32.eq (local.get $m200tag) (i32.const 163))
          (then
            (if (local.get $extensions_seen) (then (return (i32.const 3))))
            (local.set $extensions_seen (i32.const 1))
            (call $m200write_span (i32.add (local.get $out) (i32.const 132)) (i32.load (i32.add (local.get $out) (i32.const 156))) (i32.load (i32.add (local.get $out) (i32.const 164))) (i32.load (i32.add (local.get $out) (i32.const 160))) (i32.load (i32.add (local.get $out) (i32.const 168))))
            (i32.store (i32.add (local.get $out) (i32.const 148)) (i32.const 1)))
          (else
            (if (i32.and
                  (i32.ne (local.get $m200tag) (i32.const 129))
                  (i32.ne (local.get $m200tag) (i32.const 130)))
              (then (return (i32.const 3))))))
        (local.set $offset (i32.load (i32.add (local.get $out) (i32.const 172))))
        (br $loop)))
    i32.const 0)


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

  (func (export "acme_account_status_code") (param $ptr i32) (param $len i32) (result i32)
    (if (call $m30eq (local.get $ptr) (local.get $len) (i32.const 16) (i32.const 5)) (then (return (i32.const 1))))
    (if (call $m30eq (local.get $ptr) (local.get $len) (i32.const 21) (i32.const 11)) (then (return (i32.const 2))))
    (if (call $m30eq (local.get $ptr) (local.get $len) (i32.const 32) (i32.const 7)) (then (return (i32.const 3))))
    i32.const 0)

  (func (export "acme_order_status_code") (param $ptr i32) (param $len i32) (result i32)
    (if (call $m30eq (local.get $ptr) (local.get $len) (i32.const 48) (i32.const 7)) (then (return (i32.const 1))))
    (if (call $m30eq (local.get $ptr) (local.get $len) (i32.const 55) (i32.const 5)) (then (return (i32.const 2))))
    (if (call $m30eq (local.get $ptr) (local.get $len) (i32.const 60) (i32.const 10)) (then (return (i32.const 3))))
    (if (call $m30eq (local.get $ptr) (local.get $len) (i32.const 16) (i32.const 5)) (then (return (i32.const 4))))
    (if (call $m30eq (local.get $ptr) (local.get $len) (i32.const 70) (i32.const 7)) (then (return (i32.const 5))))
    i32.const 0)

  (func (export "acme_authorization_status_code") (param $ptr i32) (param $len i32) (result i32)
    (if (call $m30eq (local.get $ptr) (local.get $len) (i32.const 48) (i32.const 7)) (then (return (i32.const 1))))
    (if (call $m30eq (local.get $ptr) (local.get $len) (i32.const 16) (i32.const 5)) (then (return (i32.const 2))))
    (if (call $m30eq (local.get $ptr) (local.get $len) (i32.const 70) (i32.const 7)) (then (return (i32.const 3))))
    (if (call $m30eq (local.get $ptr) (local.get $len) (i32.const 21) (i32.const 11)) (then (return (i32.const 4))))
    (if (call $m30eq (local.get $ptr) (local.get $len) (i32.const 77) (i32.const 7)) (then (return (i32.const 5))))
    (if (call $m30eq (local.get $ptr) (local.get $len) (i32.const 32) (i32.const 7)) (then (return (i32.const 6))))
    i32.const 0)

  (func (export "acme_challenge_status_code") (param $ptr i32) (param $len i32) (result i32)
    (if (call $m30eq (local.get $ptr) (local.get $len) (i32.const 48) (i32.const 7)) (then (return (i32.const 1))))
    (if (call $m30eq (local.get $ptr) (local.get $len) (i32.const 60) (i32.const 10)) (then (return (i32.const 2))))
    (if (call $m30eq (local.get $ptr) (local.get $len) (i32.const 16) (i32.const 5)) (then (return (i32.const 3))))
    (if (call $m30eq (local.get $ptr) (local.get $len) (i32.const 70) (i32.const 7)) (then (return (i32.const 4))))
    i32.const 0)

  (func (export "acme_challenge_type_code") (param $ptr i32) (param $len i32) (result i32)
    (if (call $m30eq (local.get $ptr) (local.get $len) (i32.const 96) (i32.const 7)) (then (return (i32.const 1))))
    (if (call $m30eq (local.get $ptr) (local.get $len) (i32.const 103) (i32.const 6)) (then (return (i32.const 2))))
    (if (call $m30eq (local.get $ptr) (local.get $len) (i32.const 109) (i32.const 11)) (then (return (i32.const 3))))
    i32.const 4)

  (func (export "acme_challenge_material_kind") (param $type_code i32) (result i32)
    (if (i32.eq (local.get $type_code) (i32.const 1)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $type_code) (i32.const 2)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $type_code) (i32.const 3)) (then (return (i32.const 3))))
    i32.const 0)

  (func (export "acme_key_authorization_parts_valid") (param $token_len i32) (param $thumbprint_len i32) (result i32)
    (i32.and (i32.gt_u (local.get $token_len) (i32.const 0)) (i32.gt_u (local.get $thumbprint_len) (i32.const 0))))

  (func (export "acme_directory_builtin_code") (param $ptr i32) (param $len i32) (result i32)
    (if (call $m30eq (local.get $ptr) (local.get $len) (i32.const 128) (i32.const 11)) (then (return (i32.const 1))))
    (if (call $m30eq (local.get $ptr) (local.get $len) (i32.const 139) (i32.const 18)) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "acme_jwk_kind") (param $has_n i32) (param $has_e i32) (param $has_crv i32) (param $has_x i32) (param $has_y i32) (result i32)
    (if (i32.and (local.get $has_n) (local.get $has_e)) (then (return (i32.const 1))))
    (if (i32.and (local.get $has_crv) (i32.and (local.get $has_x) (local.get $has_y))) (then (return (i32.const 2))))
    i32.const 0)




  (func $m169is_space (param $b i32) (result i32)
    local.get $b
    i32.const 32
    i32.eq
    local.get $b
    i32.const 9
    i32.eq
    i32.or)

  (func $is_eol (param $b i32) (result i32)
    local.get $b
    i32.const 10
    i32.eq
    local.get $b
    i32.const 13
    i32.eq
    i32.or)

  (func $is_token_byte (param $b i32) (result i32)
    local.get $b
    call $m169is_space
    i32.eqz
    local.get $b
    call $is_eol
    i32.eqz
    i32.and)


  (func $is_b64_data (param $b i32) (result i32)
    local.get $b
    call $m169is_upper
    local.get $b
    call $m169is_lower
    i32.or
    local.get $b
    call $is_digit
    i32.or
    local.get $b
    i32.const 43
    i32.eq
    i32.or
    local.get $b
    i32.const 47
    i32.eq
    i32.or)

  (func (export "ssh_authorized_key_scan")
    (param $ptr i32) (param $len i32) (param $out i32)
    (result i32)
    (local $end i32)
    (local $i i32)
    (local $b i32)
    (local $type_start i32)
    (local $type_end i32)
    (local $b64_start i32)
    (local $b64_end i32)
    (local $comment_start i32)
    (local $pad_count i32)
    (local $saw_data i32)
    (local $in_padding i32)

    local.get $ptr
    local.get $len
    i32.add
    local.set $end
    local.get $ptr
    local.set $i

    ;; Leading indentation is allowed.
    (block $leading_done
      (loop $leading
        local.get $i
        local.get $end
        i32.ge_u
        br_if $leading_done
        local.get $i
        i32.load8_u
        local.set $b
        local.get $b
        call $m169is_space
        i32.eqz
        br_if $leading_done
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $leading))

    ;; Blank lines and comment lines are not authorized-key records.
    local.get $i
    local.get $end
    i32.ge_u
    if
      i32.const 3
      return
    end
    local.get $i
    i32.load8_u
    local.tee $b
    call $is_eol
    if
      i32.const 3
      return
    end
    local.get $b
    i32.const 35
    i32.eq
    if
      i32.const 3
      return
    end

    local.get $i
    local.set $type_start
    (block $type_done
      (loop $type_loop
        local.get $i
        local.get $end
        i32.ge_u
        br_if $type_done
        local.get $i
        i32.load8_u
        call $is_token_byte
        i32.eqz
        br_if $type_done
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $type_loop))
    local.get $i
    local.set $type_end
    local.get $type_end
    local.get $type_start
    i32.eq
    if
      i32.const 3
      return
    end

    ;; The key type must be followed by at least one space or tab.
    local.get $i
    local.get $end
    i32.ge_u
    if
      i32.const 3
      return
    end
    local.get $i
    i32.load8_u
    call $m169is_space
    i32.eqz
    if
      i32.const 3
      return
    end
    (block $after_type_ws
      (loop $type_ws
        local.get $i
        local.get $end
        i32.ge_u
        br_if $after_type_ws
        local.get $i
        i32.load8_u
        call $m169is_space
        i32.eqz
        br_if $after_type_ws
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $type_ws))

    local.get $i
    local.set $b64_start
    (block $b64_done
      (loop $b64_loop
        local.get $i
        local.get $end
        i32.ge_u
        br_if $b64_done
        local.get $i
        i32.load8_u
        local.set $b

        local.get $b
        call $m169is_space
        local.get $b
        call $is_eol
        i32.or
        br_if $b64_done

        local.get $b
        i32.const 61
        i32.eq
        if
          local.get $saw_data
          i32.eqz
          if
            i32.const 3
            return
          end
          local.get $pad_count
          i32.const 2
          i32.ge_u
          if
            i32.const 3
            return
          end
          local.get $pad_count
          i32.const 1
          i32.add
          local.set $pad_count
          i32.const 1
          local.set $in_padding
        else
          local.get $b
          call $is_b64_data
          i32.eqz
          if
            i32.const 3
            return
          end
          local.get $in_padding
          if
            i32.const 3
            return
          end
          i32.const 1
          local.set $saw_data
        end

        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $b64_loop))
    local.get $i
    local.set $b64_end
    local.get $saw_data
    i32.eqz
    if
      i32.const 3
      return
    end
    local.get $b64_end
    local.get $b64_start
    i32.eq
    if
      i32.const 3
      return
    end

    ;; Optional comment begins after separating spaces and runs to CR/LF or end.
    (block $after_b64_ws
      (loop $b64_ws
        local.get $i
        local.get $end
        i32.ge_u
        br_if $after_b64_ws
        local.get $i
        i32.load8_u
        call $m169is_space
        i32.eqz
        br_if $after_b64_ws
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $b64_ws))
    local.get $i
    local.set $comment_start
    (block $comment_done
      (loop $comment_loop
        local.get $i
        local.get $end
        i32.ge_u
        br_if $comment_done
        local.get $i
        i32.load8_u
        call $is_eol
        br_if $comment_done
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $comment_loop))

    local.get $out
    local.get $type_start
    local.get $ptr
    i32.sub
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $type_end
    local.get $type_start
    i32.sub
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $b64_start
    local.get $ptr
    i32.sub
    i32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $b64_end
    local.get $b64_start
    i32.sub
    i32.store
    local.get $out
    i32.const 16
    i32.add
    local.get $comment_start
    local.get $ptr
    i32.sub
    i32.store
    local.get $out
    i32.const 20
    i32.add
    local.get $i
    local.get $comment_start
    i32.sub
    i32.store

    i32.const 0)


  ;; Status values: 0 ok, 2 output_short.
  ;; Packed return: low u32 status, high u32 bytes written.


  (func $m76put (param $out_ptr i32) (param $out_cap i32) (param $written i32) (param $c i32) (result i64)
    (if (i32.ge_u (local.get $written) (local.get $out_cap))
      (then (return (call $pack (i32.const 2) (local.get $written)))))
    (i32.store8 (i32.add (local.get $out_ptr) (local.get $written)) (local.get $c))
    (call $pack (i32.const 0) (i32.add (local.get $written) (i32.const 1))))

  (func $emit_crlf (param $out_ptr i32) (param $out_cap i32) (param $written i32) (result i64)
    (local $packed i64)
    (local.set $packed
      (call $m76put
        (local.get $out_ptr)
        (local.get $out_cap)
        (local.get $written)
        (i32.const 13)))
    (if (i32.ne (i32.wrap_i64 (local.get $packed)) (i32.const 0))
      (then (return (local.get $packed))))
    (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
    (call $m76put
      (local.get $out_ptr)
      (local.get $out_cap)
      (local.get $written)
      (i32.const 10)))

  (func $emit_blank_lines (param $out_ptr i32) (param $out_cap i32) (param $written i32) (param $count i32) (result i64)
    (local $packed i64)
    (loop $again
      (if (i32.eqz (local.get $count))
        (then (return (call $pack (i32.const 0) (local.get $written)))))
      (local.set $packed
        (call $emit_crlf
          (local.get $out_ptr)
          (local.get $out_cap)
          (local.get $written)))
      (if (i32.ne (i32.wrap_i64 (local.get $packed)) (i32.const 0))
        (then (return (local.get $packed))))
      (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
      (local.set $count (i32.sub (local.get $count) (i32.const 1)))
      (br $again))
    (call $pack (i32.const 0) (local.get $written)))

  (func $m76is_wsp (param $c i32) (result i32)
    (i32.or
      (i32.eq (local.get $c) (i32.const 32))
      (i32.eq (local.get $c) (i32.const 9))))

  (func $is_next_lf (param $in_ptr i32) (param $in_len i32) (param $i i32) (result i32)
    (if (i32.ge_u (i32.add (local.get $i) (i32.const 1)) (local.get $in_len))
      (then (return (i32.const 0))))
    (i32.eq
      (call $m76byte (local.get $in_ptr) (i32.add (local.get $i) (i32.const 1)))
      (i32.const 10)))

  (func (export "dkim_body_simple") (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32) (result i64)
    (local $i i32)
    (local $c i32)
    (local $written i32)
    (local $packed i64)
    (local $pending_blank_lines i32)
    (local $line_len i32)
    (local $seen_nonempty i32)

    (loop $again
      (if (i32.ge_u (local.get $i) (local.get $in_len))
        (then
          (if (i32.ne (local.get $line_len) (i32.const 0))
            (then
              (local.set $packed
                (call $emit_crlf
                  (local.get $out_ptr)
                  (local.get $out_cap)
                  (local.get $written)))
              (if (i32.ne (i32.wrap_i64 (local.get $packed)) (i32.const 0))
                (then (return (local.get $packed))))
              (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
              (local.set $seen_nonempty (i32.const 1))))
          (if (i32.eqz (local.get $seen_nonempty))
            (then
              (local.set $packed
                (call $emit_crlf
                  (local.get $out_ptr)
                  (local.get $out_cap)
                  (local.get $written)))
              (if (i32.ne (i32.wrap_i64 (local.get $packed)) (i32.const 0))
                (then (return (local.get $packed))))
              (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))))
          (return (call $pack (i32.const 0) (local.get $written)))))

      (local.set $c (call $m76byte (local.get $in_ptr) (local.get $i)))

      (if (i32.or (i32.eq (local.get $c) (i32.const 13)) (i32.eq (local.get $c) (i32.const 10)))
        (then
          (if (i32.and
                (i32.eq (local.get $c) (i32.const 13))
                (call $is_next_lf (local.get $in_ptr) (local.get $in_len) (local.get $i)))
            (then (local.set $i (i32.add (local.get $i) (i32.const 1)))))
          (if (i32.eqz (local.get $line_len))
            (then
              (local.set $pending_blank_lines (i32.add (local.get $pending_blank_lines) (i32.const 1))))
            (else
              (local.set $packed
                (call $emit_crlf
                  (local.get $out_ptr)
                  (local.get $out_cap)
                  (local.get $written)))
              (if (i32.ne (i32.wrap_i64 (local.get $packed)) (i32.const 0))
                (then (return (local.get $packed))))
              (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
              (local.set $seen_nonempty (i32.const 1))
              (local.set $line_len (i32.const 0))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $again)))

      (if (i32.eqz (local.get $line_len))
        (then
          (local.set $packed
            (call $emit_blank_lines
              (local.get $out_ptr)
              (local.get $out_cap)
              (local.get $written)
              (local.get $pending_blank_lines)))
          (if (i32.ne (i32.wrap_i64 (local.get $packed)) (i32.const 0))
            (then (return (local.get $packed))))
          (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
          (local.set $pending_blank_lines (i32.const 0))))

      (local.set $packed
        (call $m76put
          (local.get $out_ptr)
          (local.get $out_cap)
          (local.get $written)
          (local.get $c)))
      (if (i32.ne (i32.wrap_i64 (local.get $packed)) (i32.const 0))
        (then (return (local.get $packed))))
      (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
      (local.set $line_len (i32.add (local.get $line_len) (i32.const 1)))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $again))

    (call $pack (i32.const 0) (local.get $written)))

  (func (export "dkim_body_relaxed") (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32) (result i64)
    (local $i i32)
    (local $c i32)
    (local $written i32)
    (local $packed i64)
    (local $pending_blank_lines i32)
    (local $pending_space i32)
    (local $line_has_content i32)
    (local $seen_nonempty i32)

    (loop $again
      (if (i32.ge_u (local.get $i) (local.get $in_len))
        (then
          (if (local.get $line_has_content)
            (then
              (local.set $packed
                (call $emit_crlf
                  (local.get $out_ptr)
                  (local.get $out_cap)
                  (local.get $written)))
              (if (i32.ne (i32.wrap_i64 (local.get $packed)) (i32.const 0))
                (then (return (local.get $packed))))
              (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
              (local.set $seen_nonempty (i32.const 1))))
          (if (i32.eqz (local.get $seen_nonempty))
            (then
              (local.set $packed
                (call $emit_crlf
                  (local.get $out_ptr)
                  (local.get $out_cap)
                  (local.get $written)))
              (if (i32.ne (i32.wrap_i64 (local.get $packed)) (i32.const 0))
                (then (return (local.get $packed))))
              (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))))
          (return (call $pack (i32.const 0) (local.get $written)))))

      (local.set $c (call $m76byte (local.get $in_ptr) (local.get $i)))

      (if (i32.or (i32.eq (local.get $c) (i32.const 13)) (i32.eq (local.get $c) (i32.const 10)))
        (then
          (if (i32.and
                (i32.eq (local.get $c) (i32.const 13))
                (call $is_next_lf (local.get $in_ptr) (local.get $in_len) (local.get $i)))
            (then (local.set $i (i32.add (local.get $i) (i32.const 1)))))
          (if (i32.eqz (local.get $line_has_content))
            (then
              (local.set $pending_blank_lines (i32.add (local.get $pending_blank_lines) (i32.const 1))))
            (else
              (local.set $packed
                (call $emit_crlf
                  (local.get $out_ptr)
                  (local.get $out_cap)
                  (local.get $written)))
              (if (i32.ne (i32.wrap_i64 (local.get $packed)) (i32.const 0))
                (then (return (local.get $packed))))
              (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
              (local.set $seen_nonempty (i32.const 1))
              (local.set $line_has_content (i32.const 0))
              (local.set $pending_space (i32.const 0))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $again)))

      (if (call $m76is_wsp (local.get $c))
        (then
          (if (local.get $line_has_content)
            (then (local.set $pending_space (i32.const 1))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $again)))

      (if (i32.eqz (local.get $line_has_content))
        (then
          (local.set $packed
            (call $emit_blank_lines
              (local.get $out_ptr)
              (local.get $out_cap)
              (local.get $written)
              (local.get $pending_blank_lines)))
          (if (i32.ne (i32.wrap_i64 (local.get $packed)) (i32.const 0))
            (then (return (local.get $packed))))
          (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
          (local.set $pending_blank_lines (i32.const 0))
          (local.set $line_has_content (i32.const 1)))
        (else
          (if (local.get $pending_space)
            (then
              (local.set $packed
                (call $m76put
                  (local.get $out_ptr)
                  (local.get $out_cap)
                  (local.get $written)
                  (i32.const 32)))
              (if (i32.ne (i32.wrap_i64 (local.get $packed)) (i32.const 0))
                (then (return (local.get $packed))))
              (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
              (local.set $pending_space (i32.const 0))))))

      (local.set $packed
        (call $m76put
          (local.get $out_ptr)
          (local.get $out_cap)
          (local.get $written)
          (local.get $c)))
      (if (i32.ne (i32.wrap_i64 (local.get $packed)) (i32.const 0))
        (then (return (local.get $packed))))
      (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $again))

    (call $pack (i32.const 0) (local.get $written)))

;; Authority storage semantics plundered from edgerun-vfs and edgerun-virtual-disk.
  (func (export "authority_vfs_wire_abi_version") (result i32)
    i32.const 1)

  (func (export "authority_vfs_default_object_packet_bytes") (result i32)
    i32.const 65536)

  (func (export "authority_vfs_compression_none") (result i32)
    i32.const 0)

  (func (export "authority_vfs_compression_deflate_raw") (result i32)
    i32.const 1)

  (func (export "authority_vfs_seal_aes256_gcm") (result i32)
    i32.const 1)

  (func (export "authority_vfs_wire_record_tag") (param $kind i32) (result i32)
    ;; 0 packet, 1 file ref, 2 transform ref, 3 seal request,
    ;; 4 unseal request, 5 tree manifest.
    (if (result i32) (i32.le_u (local.get $kind) (i32.const 5))
      (then local.get $kind)
      (else i32.const -1)))

  (func (export "authority_vfs_packet_count") (param $object_len i32) (param $max_payload i32) (result i32)
    (if (i32.eqz (local.get $max_payload)) (then (return (i32.const -1))))
    (if (i32.eqz (local.get $object_len)) (then (return (i32.const 1))))
    (i32.add
      (i32.div_u (i32.sub (local.get $object_len) (i32.const 1)) (local.get $max_payload))
      (i32.const 1)))

  (func (export "authority_vfs_packet_offset") (param $packet_index i32) (param $max_payload i32) (result i32)
    (i32.mul (local.get $packet_index) (local.get $max_payload)))

  (func (export "authority_vfs_packet_shape_result")
    (param $abi i32) (param $object_len i32) (param $packet_index i32)
    (param $packet_count i32) (param $offset i32) (param $payload_len i32)
    (result i32)
    ;; 0 ok, 1 invalid shape, 2 missing packet/index, 3 object too large.
    (if (i32.or (i32.ne (local.get $abi) (i32.const 1)) (i32.eqz (local.get $packet_count)))
      (then (return (i32.const 1))))
    (if (i32.ge_u (local.get $packet_index) (local.get $packet_count))
      (then (return (i32.const 2))))
    (if (i32.lt_u (i32.add (local.get $offset) (local.get $payload_len)) (local.get $offset))
      (then (return (i32.const 3))))
    (if (i32.gt_u (i32.add (local.get $offset) (local.get $payload_len)) (local.get $object_len))
      (then (return (i32.const 1))))
    (if (i32.and
          (i32.lt_u (i32.add (local.get $packet_index) (i32.const 1)) (local.get $packet_count))
          (i32.eqz (local.get $payload_len)))
      (then (return (i32.const 1))))
    i32.const 0)

  (func (export "authority_vfs_transform_result")
    (param $abi i32) (param $compression i32) (param $seal i32) (param $hash_matches i32)
    (result i32)
    ;; validate_transform: ABI 1, AES-256-GCM seal, none/raw-deflate compression, hash match.
    (if (i32.ne (local.get $abi) (i32.const 1)) (then (return (i32.const 1))))
    (if (i32.ne (local.get $seal) (i32.const 1)) (then (return (i32.const 1))))
    (if (i32.eqz
          (i32.or
            (i32.eq (local.get $compression) (i32.const 0))
            (i32.eq (local.get $compression) (i32.const 1))))
      (then (return (i32.const 1))))
    (if (i32.eqz (local.get $hash_matches)) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "authority_vfs_file_ref_result")
    (param $abi i32) (param $path_ok i32) (param $file_hash_matches i32)
    (param $object_hash_matches i32) (param $object_len_matches i32)
    (result i32)
    (if (i32.ne (local.get $abi) (i32.const 1)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $path_ok)) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $file_hash_matches)) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $object_hash_matches)) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $object_len_matches)) (then (return (i32.const 3))))
    i32.const 0)

  (func (export "authority_vfs_manifest_order") (param $left_path_cmp i32) (result i32)
    ;; files_to_manifest sorts VfsFileRef entries by path before hashing.
    (if (result i32) (i32.le_s (local.get $left_path_cmp) (i32.const 0))
      (then i32.const 0)
      (else i32.const 1)))

  (func (export "authority_vfs_seal_request_payload_result")
    (param $compression i32) (param $adapter_available i32) (result i32)
    ;; none keeps payload bytes; raw deflate requires an explicit WAT adapter and fails closed.
    (if (i32.eq (local.get $compression) (i32.const 0)) (then (return (i32.const 0))))
    (if (i32.eq (local.get $compression) (i32.const 1))
      (then
        (if (local.get $adapter_available)
          (then (return (i32.const 0)))
          (else (return (i32.const 1))))))
    (return (i32.const 2)))

  (func (export "authority_vfs_memory_write_usage") (param $old_total i32) (param $old_size i32) (param $new_size i32) (result i32)
    (i32.add (i32.sub (local.get $old_total) (local.get $old_size)) (local.get $new_size)))

  (func (export "authority_vfs_text_edit_result") (param $exists i32) (param $deleted i32) (param $is_text i32) (result i32)
    ;; 0 ok, 1 deleted, 2 not found, 3 binary.
    (if (local.get $deleted) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $exists)) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $is_text)) (then (return (i32.const 3))))
    i32.const 0)

  (func (export "authority_vfs_path_result") (param $ptr i32) (param $len i32) (result i32)
    (local $start i32) (local $end i32) (local $i i32) (local $seg_start i32) (local $seg_len i32) (local $ch i32)
    (local.set $start (local.get $ptr))
    (local.set $end (i32.add (local.get $ptr) (local.get $len)))
    (loop $trim_left
      (if (i32.and (i32.lt_u (local.get $start) (local.get $end)) (i32.eq (i32.load8_u (local.get $start)) (i32.const 47)))
        (then
          (local.set $start (i32.add (local.get $start) (i32.const 1)))
          (br $trim_left))))
    (loop $trim_right
      (if (i32.and (i32.lt_u (local.get $start) (local.get $end)) (i32.eq (i32.load8_u (i32.sub (local.get $end) (i32.const 1))) (i32.const 47)))
        (then
          (local.set $end (i32.sub (local.get $end) (i32.const 1)))
          (br $trim_right))))
    (if (i32.eq (local.get $start) (local.get $end)) (then (return (i32.const 1))))
    (local.set $i (local.get $start))
    (local.set $seg_start (local.get $start))
    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (local.get $end)))
        (local.set $ch (i32.load8_u (local.get $i)))
        (if (i32.eq (local.get $ch) (i32.const 92)) (then (return (i32.const 1))))
        (if (i32.eq (local.get $ch) (i32.const 47))
          (then
            (local.set $seg_len (i32.sub (local.get $i) (local.get $seg_start)))
            (if (call $authority_bad_path_segment (local.get $seg_start) (local.get $seg_len))
              (then (return (i32.const 1))))
            (local.set $seg_start (i32.add (local.get $i) (i32.const 1)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    (local.set $seg_len (i32.sub (local.get $end) (local.get $seg_start)))
    (if (call $authority_bad_path_segment (local.get $seg_start) (local.get $seg_len))
      (then (return (i32.const 1))))
    i32.const 0)

  (func $authority_bad_path_segment (param $ptr i32) (param $len i32) (result i32)
    (if (i32.eqz (local.get $len)) (then (return (i32.const 1))))
    (if (i32.and
          (i32.eq (local.get $len) (i32.const 1))
          (i32.eq (i32.load8_u (local.get $ptr)) (i32.const 46)))
      (then (return (i32.const 1))))
    (if (i32.and
          (i32.eq (local.get $len) (i32.const 2))
          (i32.and
            (i32.eq (i32.load8_u (local.get $ptr)) (i32.const 46))
            (i32.eq (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))) (i32.const 46))))
      (then (return (i32.const 1))))
    i32.const 0)

  (func (export "authority_disk_format_from_path") (param $ptr i32) (param $len i32) (result i32)
    ;; raw=1, qcow2=2, vhd=3, vhdx=4; unknown/no extension defaults raw.
    (if (call $suffix4 (local.get $ptr) (local.get $len) (i32.const 46) (i32.const 118) (i32.const 104) (i32.const 100)) (then (return (i32.const 3))))
    (if (call $suffix5 (local.get $ptr) (local.get $len) (i32.const 46) (i32.const 118) (i32.const 104) (i32.const 100) (i32.const 120)) (then (return (i32.const 4))))
    (if (call $suffix5 (local.get $ptr) (local.get $len) (i32.const 46) (i32.const 113) (i32.const 99) (i32.const 111) (i32.const 119)) (then (return (i32.const 2))))
    (if (call $suffix6 (local.get $ptr) (local.get $len) (i32.const 46) (i32.const 113) (i32.const 99) (i32.const 111) (i32.const 119) (i32.const 50)) (then (return (i32.const 2))))
    i32.const 1)

  (func $suffix4 (param $ptr i32) (param $len i32) (param $a i32) (param $b i32) (param $c i32) (param $d i32) (result i32)
    (if (i32.lt_u (local.get $len) (i32.const 4)) (then (return (i32.const 0))))
    (i32.and
      (i32.and (i32.eq (call $m28lower (i32.load8_u (i32.add (local.get $ptr) (i32.sub (local.get $len) (i32.const 4))))) (local.get $a))
               (i32.eq (call $m28lower (i32.load8_u (i32.add (local.get $ptr) (i32.sub (local.get $len) (i32.const 3))))) (local.get $b)))
      (i32.and (i32.eq (call $m28lower (i32.load8_u (i32.add (local.get $ptr) (i32.sub (local.get $len) (i32.const 2))))) (local.get $c))
               (i32.eq (call $m28lower (i32.load8_u (i32.add (local.get $ptr) (i32.sub (local.get $len) (i32.const 1))))) (local.get $d)))))

  (func $suffix5 (param $ptr i32) (param $len i32) (param $a i32) (param $b i32) (param $c i32) (param $d i32) (param $e i32) (result i32)
    (if (i32.lt_u (local.get $len) (i32.const 5)) (then (return (i32.const 0))))
    (i32.and
      (call $suffix4 (i32.add (local.get $ptr) (i32.const 1)) (i32.sub (local.get $len) (i32.const 1)) (local.get $b) (local.get $c) (local.get $d) (local.get $e))
      (i32.eq (call $m28lower (i32.load8_u (i32.add (local.get $ptr) (i32.sub (local.get $len) (i32.const 5))))) (local.get $a))))

  (func $suffix6 (param $ptr i32) (param $len i32) (param $a i32) (param $b i32) (param $c i32) (param $d i32) (param $e i32) (param $f i32) (result i32)
    (if (i32.lt_u (local.get $len) (i32.const 6)) (then (return (i32.const 0))))
    (i32.and
      (call $suffix5 (i32.add (local.get $ptr) (i32.const 1)) (i32.sub (local.get $len) (i32.const 1)) (local.get $b) (local.get $c) (local.get $d) (local.get $e) (local.get $f))
      (i32.eq (call $m28lower (i32.load8_u (i32.add (local.get $ptr) (i32.sub (local.get $len) (i32.const 6))))) (local.get $a))))


  (func $authority_disk_format_requires_qemu (export "authority_disk_format_requires_qemu") (param $format i32) (result i32)
    (i32.or (i32.eq (local.get $format) (i32.const 2)) (i32.or (i32.eq (local.get $format) (i32.const 3)) (i32.eq (local.get $format) (i32.const 4)))))

  (func (export "authority_disk_validate_spec_result") (param $path_len i32) (param $size_bytes i32) (param $format i32) (param $qemu_available i32) (result i32)
    ;; 0 ok, 1 invalid argument, 2 command missing.
    (if (i32.eqz (local.get $size_bytes)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $path_len)) (then (return (i32.const 1))))
    (if (i32.and (call $authority_disk_format_requires_qemu (local.get $format)) (i32.eqz (local.get $qemu_available)))
      (then (return (i32.const 2))))
    i32.const 0)

  (func (export "authority_disk_qemu_size_unit") (param $size_bytes i32) (result i32)
    ;; 3 GiB, 2 MiB, 1 KiB, 0 raw bytes.
    (if (i32.eqz (i32.rem_u (local.get $size_bytes) (i32.const 1073741824))) (then (return (i32.const 3))))
    (if (i32.eqz (i32.rem_u (local.get $size_bytes) (i32.const 1048576))) (then (return (i32.const 2))))
    (if (i32.eqz (i32.rem_u (local.get $size_bytes) (i32.const 1024))) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "authority_block_total_size") (param $block_size i32) (param $block_count i32) (result i64)
    (i64.mul (i64.extend_i32_u (local.get $block_size)) (i64.extend_i32_u (local.get $block_count))))

  (func (export "authority_block_transfer_result")
    (param $block_size i32) (param $block_count i32) (param $readonly i32)
    (param $op i32) (param $lba i32) (param $blocks i32) (param $buffer_len i32)
    (result i32)
    ;; op: 1 read, 2 write, 3 discard, 4 write_zeroes. 0 ok, 1 bad device,
    ;; 2 readonly, 3 out of range, 4 length mismatch.
    (if (i32.or (i32.eqz (local.get $block_size)) (i32.eqz (local.get $block_count)))
      (then (return (i32.const 1))))
    (if (i32.eqz (local.get $blocks)) (then (return (i32.const 1))))
    (if (i32.and (local.get $readonly) (i32.or (i32.eq (local.get $op) (i32.const 2)) (i32.or (i32.eq (local.get $op) (i32.const 3)) (i32.eq (local.get $op) (i32.const 4)))))
      (then (return (i32.const 2))))
    (if (i32.gt_u (i32.add (local.get $lba) (local.get $blocks)) (local.get $block_count))
      (then (return (i32.const 3))))
    (if (i32.ne (local.get $buffer_len) (i32.mul (local.get $block_size) (local.get $blocks)))
      (then (return (i32.const 4))))
    i32.const 0)

  (func (export "authority_block_next_request_id") (param $current i32) (result i32)
    (if (result i32) (i32.eq (local.get $current) (i32.const -1))
      (then i32.const -1)
      (else (i32.add (local.get $current) (i32.const 1)))))

  (func (export "authority_file_backend_open_result") (param $exists i32) (param $is_file i32) (param $block_size i32) (param $file_len i32) (result i32)
    ;; 0 ok, 1 invalid argument, 2 misaligned.
    (if (i32.eqz (local.get $block_size)) (then (return (i32.const 1))))
    (if (i32.eqz (i32.and (local.get $exists) (local.get $is_file))) (then (return (i32.const 1))))
    (if (i32.ne (i32.rem_u (local.get $file_len) (local.get $block_size)) (i32.const 0))
      (then (return (i32.const 2))))
    i32.const 0)

  (func (export "authority_nbd_ioctl_code") (param $which i32) (result i32)
    (if (i32.eq (local.get $which) (i32.const 1)) (then (return (i32.const 0xab00))))
    (if (i32.eq (local.get $which) (i32.const 2)) (then (return (i32.const 0xab01))))
    (if (i32.eq (local.get $which) (i32.const 3)) (then (return (i32.const 0xab03))))
    (if (i32.eq (local.get $which) (i32.const 4)) (then (return (i32.const 0xab04))))
    (if (i32.eq (local.get $which) (i32.const 5)) (then (return (i32.const 0xab05))))
    (if (i32.eq (local.get $which) (i32.const 6)) (then (return (i32.const 0xab07))))
    (if (i32.eq (local.get $which) (i32.const 7)) (then (return (i32.const 0xab08))))
    (if (i32.eq (local.get $which) (i32.const 8)) (then (return (i32.const 0xab0a))))
    i32.const 0)

  (func (export "authority_nbd_attach_result") (param $size_bytes i32) (param $block_size i32) (param $read_only i32) (param $flags i32) (result i32)
    ;; 0 ok, 1 misaligned; read_only ORs the negotiated flags with read-only bit 2.
    (drop (local.get $flags))
    (if (i32.eqz (local.get $block_size)) (then (return (i32.const 1))))
    (if (i32.ne (i32.rem_u (local.get $size_bytes) (local.get $block_size)) (i32.const 0))
      (then (return (i32.const 1))))
    (if (result i32) (local.get $read_only)
      (then i32.const 2)
      (else i32.const 0)))

  (func (export "authority_nbd_len") (param $which i32) (result i32)
    ;; 1 server handshake, 2 client flags, 3 option header, 4 export info,
    ;; 5 request header, 6 reply header, 7 option reply header.
    (if (i32.eq (local.get $which) (i32.const 1)) (then (return (i32.const 18))))
    (if (i32.eq (local.get $which) (i32.const 2)) (then (return (i32.const 4))))
    (if (i32.eq (local.get $which) (i32.const 3)) (then (return (i32.const 16))))
    (if (i32.eq (local.get $which) (i32.const 4)) (then (return (i32.const 134))))
    (if (i32.eq (local.get $which) (i32.const 5)) (then (return (i32.const 28))))
    (if (i32.eq (local.get $which) (i32.const 6)) (then (return (i32.const 16))))
    (if (i32.eq (local.get $which) (i32.const 7)) (then (return (i32.const 20))))
    i32.const 0)

  (data (i32.const 16) "validdeactivatedrevoked")
  (data (i32.const 48) "pendingreadyprocessinginvalidexpired")
  (data (i32.const 96) "http-01dns-01tls-alpn-01")
  (data (i32.const 128) "letsencryptletsencryptstaging")
)
