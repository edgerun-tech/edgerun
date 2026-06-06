  ;; Status values: 0 ok, 2 output_short.
  ;; Packed return: low u32 status, high u32 bytes written.
  (func (export "proto_abi_version") (result i32)
    i32.const 2)

  (func (export "proto_standard_id") (result i32)
    i32.const 300047)


  (func $m76byte (param $ptr i32) (param $off i32) (result i32)
    (i32.load8_u (i32.add (local.get $ptr) (local.get $off))))

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