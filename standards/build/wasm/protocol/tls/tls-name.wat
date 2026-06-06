  (func (export "proto_abi_version") (result i32)
    i32.const 2)

  (func (export "proto_standard_id") (result i32)
    i32.const 300014)

  ;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid, 4 overflow.

  (func $m183is_space (param $b i32) (result i32)
    (i32.or
      (i32.eq (local.get $b) (i32.const 32))
      (i32.and
        (i32.ge_u (local.get $b) (i32.const 9))
        (i32.le_u (local.get $b) (i32.const 13)))))




  (func $m183lower_ascii (param $b i32) (result i32)
    (if (result i32)
      (i32.and
        (i32.ge_u (local.get $b) (i32.const 65))
        (i32.le_u (local.get $b) (i32.const 90)))
      (then (i32.add (local.get $b) (i32.const 32)))
      (else (local.get $b))))

  (func $validate_normalized (param $ptr i32) (param $len i32) (param $allow_wildcard i32) (result i32)
    (local $i i32)
    (local $b i32)
    (local $label_len i32)
    (local $labels i32)
    (local $last i32)
    (local $all_digit_dot i32)
    (if (i32.or (i32.eqz (local.get $len)) (i32.gt_u (local.get $len) (i32.const 253)))
      (then (return (i32.const 0))))
    (local.set $all_digit_dot (i32.const 1))
    (loop $scan
      (if (i32.lt_u (local.get $i) (local.get $len))
        (then
          (local.set $b (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
          (if
            (i32.or
              (i32.or (i32.eqz (local.get $b)) (call $m183is_space (local.get $b)))
              (i32.eq (local.get $b) (i32.const 58)))
            (then (return (i32.const 0))))
          (if
            (i32.eqz
              (i32.or
                (call $is_digit (local.get $b))
                (i32.eq (local.get $b) (i32.const 46))))
            (then (local.set $all_digit_dot (i32.const 0))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $scan))))
    (if (local.get $all_digit_dot)
      (then (return (i32.const 0))))
    (local.set $i (i32.const 0))
    (local.set $label_len (i32.const 0))
    (loop $labels_loop
      (if (i32.lt_u (local.get $i) (local.get $len))
        (then
          (local.set $b (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
          (if (i32.eq (local.get $b) (i32.const 46))
            (then
              (if
                (i32.or
                  (i32.or (i32.eqz (local.get $label_len)) (i32.gt_u (local.get $label_len) (i32.const 63)))
                  (i32.eq (local.get $last) (i32.const 45)))
                (then (return (i32.const 0))))
              (local.set $labels (i32.add (local.get $labels) (i32.const 1)))
              (local.set $label_len (i32.const 0)))
            (else
              (if
                (i32.and
                  (local.get $allow_wildcard)
                  (i32.and
                    (i32.eqz (local.get $i))
                    (i32.eq (local.get $b) (i32.const 42))))
                (then
                  (if (i32.ne (local.get $len) (i32.const 1))
                    (then
                      (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))) (i32.const 46))
                        (then (return (i32.const 0)))))
                    (else (return (i32.const 0))))
                  (local.set $label_len (i32.add (local.get $label_len) (i32.const 1))))
                (else
                  (if
                    (i32.eqz
                      (i32.or
                        (call $is_alnum (local.get $b))
                        (i32.eq (local.get $b) (i32.const 45))))
                    (then (return (i32.const 0))))
                  (if
                    (i32.and
                      (i32.eqz (local.get $label_len))
                      (i32.eq (local.get $b) (i32.const 45)))
                    (then (return (i32.const 0))))
                  (local.set $label_len (i32.add (local.get $label_len) (i32.const 1)))))
              (local.set $last (local.get $b))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $labels_loop))))
    (if
      (i32.or
        (i32.or (i32.eqz (local.get $label_len)) (i32.gt_u (local.get $label_len) (i32.const 63)))
        (i32.eq (local.get $last) (i32.const 45)))
      (then (return (i32.const 0))))
    (local.set $labels (i32.add (local.get $labels) (i32.const 1)))
    (i32.ge_u (local.get $labels) (i32.const 2)))

  (func $validate_label (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $b i32)
    (if (i32.or (i32.eqz (local.get $len)) (i32.gt_u (local.get $len) (i32.const 63)))
      (then (return (i32.const 0))))
    (loop $scan
      (if (i32.lt_u (local.get $i) (local.get $len))
        (then
          (local.set $b (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
          (if
            (i32.eqz
              (i32.or
                (call $is_alnum (local.get $b))
                (i32.eq (local.get $b) (i32.const 45))))
            (then (return (i32.const 0))))
          (if
            (i32.and
              (i32.eqz (local.get $i))
              (i32.eq (local.get $b) (i32.const 45)))
            (then (return (i32.const 0))))
          (if
            (i32.and
              (i32.eq (i32.add (local.get $i) (i32.const 1)) (local.get $len))
              (i32.eq (local.get $b) (i32.const 45)))
            (then (return (i32.const 0))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $scan))))
    (i32.const 1))

  ;; Return bits: low32=status, high32=written. Writes lowercase normalized DNS name.
  (func $tls_dns_name_normalize (export "tls_dns_name_normalize")
    (param $input_ptr i32) (param $input_len i32) (param $out_ptr i32) (param $out_cap i32) (result i64)
    (local $start i32)
    (local $end i32)
    (local $written i32)
    (local $b i32)
    (local $valid i32)
    (local.set $start (i32.const 0))
    (local.set $end (local.get $input_len))
    (loop $trim_start
      (if
        (i32.and
          (i32.lt_u (local.get $start) (local.get $end))
          (call $m183is_space (i32.load8_u (i32.add (local.get $input_ptr) (local.get $start)))))
        (then
          (local.set $start (i32.add (local.get $start) (i32.const 1)))
          (br $trim_start))))
    (loop $trim_end
      (if
        (i32.and
          (i32.gt_u (local.get $end) (local.get $start))
          (call $m183is_space (i32.load8_u (i32.add (local.get $input_ptr) (i32.sub (local.get $end) (i32.const 1))))))
        (then
          (local.set $end (i32.sub (local.get $end) (i32.const 1)))
          (br $trim_end))))
    (loop $trim_dot
      (if
        (i32.and
          (i32.gt_u (local.get $end) (local.get $start))
          (i32.eq (i32.load8_u (i32.add (local.get $input_ptr) (i32.sub (local.get $end) (i32.const 1)))) (i32.const 46)))
        (then
          (local.set $end (i32.sub (local.get $end) (i32.const 1)))
          (br $trim_dot))))
    (if (i32.lt_u (local.get $out_cap) (i32.sub (local.get $end) (local.get $start)))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))
    (loop $copy
      (if (i32.lt_u (i32.add (local.get $start) (local.get $written)) (local.get $end))
        (then
          (local.set $b
            (call $m183lower_ascii
              (i32.load8_u
                (i32.add
                  (local.get $input_ptr)
                  (i32.add (local.get $start) (local.get $written))))))
          (i32.store8 (i32.add (local.get $out_ptr) (local.get $written)) (local.get $b))
          (local.set $written (i32.add (local.get $written) (i32.const 1)))
          (br $copy))))
    (local.set $valid (call $validate_normalized (local.get $out_ptr) (local.get $written) (i32.const 0)))
    (if (i32.eqz (local.get $valid))
      (then (return (call $pack (i32.const 3) (i32.const 0)))))
    (call $pack (i32.const 0) (local.get $written)))

  (func $normalized_match (param $a_ptr i32) (param $a_len i32) (param $b_ptr i32) (param $b_len i32) (result i32)
    (local $i i32)
    (if (i32.ne (local.get $a_len) (local.get $b_len))
      (then (return (i32.const 0))))
    (loop $cmp
      (if (i32.lt_u (local.get $i) (local.get $a_len))
        (then
          (if
            (i32.ne
              (i32.load8_u (i32.add (local.get $a_ptr) (local.get $i)))
              (i32.load8_u (i32.add (local.get $b_ptr) (local.get $i))))
            (then (return (i32.const 0))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $cmp))))
    (i32.const 1))

  (func $strip_and_lower_pattern (param $ptr i32) (param $len i32) (param $out_ptr i32) (result i32)
    (local $start i32)
    (local $end i32)
    (local $written i32)
    (local.set $end (local.get $len))
    (loop $trim_start
      (if
        (i32.and
          (i32.lt_u (local.get $start) (local.get $end))
          (call $m183is_space (i32.load8_u (i32.add (local.get $ptr) (local.get $start)))))
        (then
          (local.set $start (i32.add (local.get $start) (i32.const 1)))
          (br $trim_start))))
    (loop $trim_end
      (if
        (i32.and
          (i32.gt_u (local.get $end) (local.get $start))
          (call $m183is_space (i32.load8_u (i32.add (local.get $ptr) (i32.sub (local.get $end) (i32.const 1))))))
        (then
          (local.set $end (i32.sub (local.get $end) (i32.const 1)))
          (br $trim_end))))
    (loop $trim_dot
      (if
        (i32.and
          (i32.gt_u (local.get $end) (local.get $start))
          (i32.eq (i32.load8_u (i32.add (local.get $ptr) (i32.sub (local.get $end) (i32.const 1)))) (i32.const 46)))
        (then
          (local.set $end (i32.sub (local.get $end) (i32.const 1)))
          (br $trim_dot))))
    (loop $copy
      (if (i32.lt_u (i32.add (local.get $start) (local.get $written)) (local.get $end))
        (then
          (i32.store8
            (i32.add (local.get $out_ptr) (local.get $written))
            (call $m183lower_ascii
              (i32.load8_u
                (i32.add
                  (local.get $ptr)
                  (i32.add (local.get $start) (local.get $written))))))
          (local.set $written (i32.add (local.get $written) (i32.const 1)))
          (br $copy))))
    (local.get $written))

  ;; Scratch areas 4096..4351 for host normalization and 4352..4607 for pattern normalization.
  ;; Return 0 for no match/invalid, 1 for exact or single-label wildcard match.
  (func (export "tls_dns_name_matches")
    (param $pattern_ptr i32) (param $pattern_len i32) (param $host_ptr i32) (param $host_len i32) (result i32)
    (local $host_pack i64)
    (local $host_status i32)
    (local $host_norm_len i32)
    (local $pattern_norm_len i32)
    (local $suffix_ptr i32)
    (local $suffix_len i32)
    (local $prefix_len i32)
    (local $i i32)
    (local $b i32)
    (local.set $host_pack
      (call $tls_dns_name_normalize
        (local.get $host_ptr)
        (local.get $host_len)
        (i32.const 4096)
        (i32.const 256)))
    (local.set $host_status (i32.wrap_i64 (local.get $host_pack)))
    (if (i32.ne (local.get $host_status) (i32.const 0))
      (then (return (i32.const 0))))
    (local.set $host_norm_len
      (i32.wrap_i64
        (i64.shr_u (local.get $host_pack) (i64.const 32))))
    (local.set $pattern_norm_len
      (call $strip_and_lower_pattern
        (local.get $pattern_ptr)
        (local.get $pattern_len)
        (i32.const 4352)))
    (if (call $normalized_match (i32.const 4352) (local.get $pattern_norm_len) (i32.const 4096) (local.get $host_norm_len))
      (then (return (i32.const 1))))
    (if
      (i32.or
        (i32.lt_u (local.get $pattern_norm_len) (i32.const 3))
        (i32.or
          (i32.ne (i32.load8_u (i32.const 4352)) (i32.const 42))
          (i32.ne (i32.load8_u (i32.const 4353)) (i32.const 46))))
      (then (return (i32.const 0))))
    (local.set $suffix_ptr (i32.const 4354))
    (local.set $suffix_len (i32.sub (local.get $pattern_norm_len) (i32.const 2)))
    (if (i32.eqz (call $validate_normalized (local.get $suffix_ptr) (local.get $suffix_len) (i32.const 0)))
      (then (return (i32.const 0))))
    (if (i32.le_u (local.get $host_norm_len) (local.get $suffix_len))
      (then (return (i32.const 0))))
    (local.set $prefix_len (i32.sub (i32.sub (local.get $host_norm_len) (local.get $suffix_len)) (i32.const 1)))
    (if (i32.eqz (local.get $prefix_len))
      (then (return (i32.const 0))))
    (if (i32.ne (i32.load8_u (i32.add (i32.const 4096) (local.get $prefix_len))) (i32.const 46))
      (then (return (i32.const 0))))
    (if (i32.eqz (call $normalized_match (local.get $suffix_ptr) (local.get $suffix_len) (i32.add (i32.const 4096) (i32.add (local.get $prefix_len) (i32.const 1))) (local.get $suffix_len)))
      (then (return (i32.const 0))))
    (loop $prefix_scan
      (if (i32.lt_u (local.get $i) (local.get $prefix_len))
        (then
          (local.set $b (i32.load8_u (i32.add (i32.const 4096) (local.get $i))))
          (if (i32.eq (local.get $b) (i32.const 46))
            (then (return (i32.const 0))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $prefix_scan))))
    (call $validate_label (i32.const 4096) (local.get $prefix_len)))