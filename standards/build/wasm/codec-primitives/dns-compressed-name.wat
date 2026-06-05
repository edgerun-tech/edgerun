(module
  (memory (export "memory") 1)

  (func (export "proto_abi_version") (result i32)
    i32.const 2)

  (func (export "proto_standard_id") (result i32)
    i32.const 300033)

  ;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid, 6 too_long.
  (func $pack (param $status i32) (param $value i32) (result i64)
    (i64.or
      (i64.extend_i32_u (local.get $status))
      (i64.shl (i64.extend_i32_u (local.get $value)) (i64.const 32))))

  (func $is_digit (param $b i32) (result i32)
    (i32.and
      (i32.ge_u (local.get $b) (i32.const 48))
      (i32.le_u (local.get $b) (i32.const 57))))

  (func $is_alpha (param $b i32) (result i32)
    (i32.or
      (i32.and
        (i32.ge_u (local.get $b) (i32.const 65))
        (i32.le_u (local.get $b) (i32.const 90)))
      (i32.and
        (i32.ge_u (local.get $b) (i32.const 97))
        (i32.le_u (local.get $b) (i32.const 122)))))

  (func $is_label_byte (param $b i32) (result i32)
    (i32.or
      (i32.or
        (call $is_alpha (local.get $b))
        (call $is_digit (local.get $b)))
      (i32.or
        (i32.eq (local.get $b) (i32.const 45))
        (i32.eq (local.get $b) (i32.const 95)))))

  (func $lower_ascii (param $b i32) (result i32)
    (if (result i32)
      (i32.and
        (i32.ge_u (local.get $b) (i32.const 65))
        (i32.le_u (local.get $b) (i32.const 90)))
      (then (i32.add (local.get $b) (i32.const 32)))
      (else (local.get $b))))

  (func $u32_at (param $ptr i32) (param $index i32) (result i32)
    (i32.load (i32.add (local.get $ptr) (i32.mul (local.get $index) (i32.const 4)))))

  ;; Return bits: low32=status, high32=consumed bytes at start.
  (func (export "dns_name_wire_len") (param $ptr i32) (param $msg_len i32) (param $start i32) (result i64)
    (local $scratch i32)
    (local $status i32)
    (local.set $scratch (i32.const 4096))
    (local.set $status
      (call $dns_name_follow
        (local.get $ptr)
        (local.get $msg_len)
        (local.get $start)
        (local.get $scratch)
        (i32.const 2048)))
    (if (i32.ne (local.get $status) (i32.const 0))
      (then (return (call $pack (local.get $status) (i32.const 0)))))
    (call $pack (i32.const 0) (i32.load (local.get $scratch))))

  ;; Output record, little-endian:
  ;; 0:u32 consumed_wire_bytes_at_start
  ;; 4:u32 label_count
  ;; 8:u32 normalized_name_len
  ;; 12:u32 pointer_count
  ;; 16:u32 terminal_offset
  ;; 20..: repeated u32 label_offset_from_message, u32 label_len
  (func $dns_name_follow (export "dns_name_follow")
    (param $ptr i32) (param $msg_len i32) (param $start i32) (param $out_ptr i32) (param $out_cap i32)
    (result i32)
    (local $pos i32)
    (local $len i32)
    (local $next i32)
    (local $target i32)
    (local $consumed i32)
    (local $jumped i32)
    (local $labels i32)
    (local $name_len i32)
    (local $pointers i32)
    (local $hops i32)
    (local $record_need i32)
    (local $j i32)
    (local $b i32)
    (local $last i32)
    (local $i i32)
    (local $label_off i32)
    (local $label_len i32)
    (if (i32.lt_u (local.get $out_cap) (i32.const 20))
      (then (return (i32.const 2))))
    (if (i32.ge_u (local.get $start) (local.get $msg_len))
      (then (return (i32.const 1))))
    (local.set $pos (local.get $start))
    (loop $scan
      (if (i32.ge_u (local.get $pos) (local.get $msg_len))
        (then (return (i32.const 1))))
      (local.set $len (i32.load8_u (i32.add (local.get $ptr) (local.get $pos))))
      (if (i32.eq (local.get $len) (i32.const 0))
        (then
          (if (i32.eqz (local.get $jumped))
            (then (local.set $consumed (i32.add (local.get $consumed) (i32.const 1)))))
          (i32.store (local.get $out_ptr) (local.get $consumed))
          (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $labels))
          (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $name_len))
          (i32.store (i32.add (local.get $out_ptr) (i32.const 12)) (local.get $pointers))
          (i32.store (i32.add (local.get $out_ptr) (i32.const 16)) (local.get $pos))
          (return (i32.const 0))))
      (if (i32.eq (i32.and (local.get $len) (i32.const 0xc0)) (i32.const 0xc0))
        (then
          (if (i32.ge_u (i32.add (local.get $pos) (i32.const 1)) (local.get $msg_len))
            (then (return (i32.const 1))))
          (local.set $next (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $pos)) (i32.const 1))))
          (local.set $target
            (i32.or
              (i32.shl (i32.and (local.get $len) (i32.const 0x3f)) (i32.const 8))
              (local.get $next)))
          (if (i32.ge_u (local.get $target) (local.get $msg_len))
            (then (return (i32.const 3))))
          (if (i32.eq (local.get $target) (local.get $pos))
            (then (return (i32.const 3))))
          (local.set $i (i32.const 0))
          (loop $range_check
            (if (i32.lt_u (local.get $i) (local.get $labels))
              (then
                (local.set $label_off
                  (i32.load
                    (i32.add
                      (local.get $out_ptr)
                      (i32.add (i32.const 20) (i32.mul (local.get $i) (i32.const 8))))))
                (local.set $label_len
                  (i32.load
                    (i32.add
                      (local.get $out_ptr)
                      (i32.add (i32.const 24) (i32.mul (local.get $i) (i32.const 8))))))
                (if
                  (i32.or
                    (i32.eq (local.get $target) (i32.sub (local.get $label_off) (i32.const 1)))
                    (i32.and
                      (i32.ge_u (local.get $target) (local.get $label_off))
                      (i32.lt_u (local.get $target) (i32.add (local.get $label_off) (local.get $label_len)))))
                  (then (return (i32.const 3))))
                (local.set $i (i32.add (local.get $i) (i32.const 1)))
                (br $range_check))))
          (if (i32.eqz (local.get $jumped))
            (then
              (local.set $consumed (i32.add (local.get $consumed) (i32.const 2)))
              (local.set $jumped (i32.const 1))))
          (local.set $pointers (i32.add (local.get $pointers) (i32.const 1)))
          (local.set $hops (i32.add (local.get $hops) (i32.const 1)))
          (if (i32.gt_u (local.get $hops) (i32.const 16))
            (then (return (i32.const 3))))
          (local.set $pos (local.get $target))
          (br $scan)))
      (if (i32.ne (i32.and (local.get $len) (i32.const 0xc0)) (i32.const 0))
        (then (return (i32.const 3))))
      (if (i32.gt_u (local.get $len) (i32.const 63))
        (then (return (i32.const 3))))
      (if (i32.gt_u (i32.add (i32.add (local.get $pos) (i32.const 1)) (local.get $len)) (local.get $msg_len))
        (then (return (i32.const 1))))
      (local.set $record_need
        (i32.add (i32.const 20) (i32.mul (i32.add (local.get $labels) (i32.const 1)) (i32.const 8))))
      (if (i32.lt_u (local.get $out_cap) (local.get $record_need))
        (then (return (i32.const 2))))
      (local.set $j (i32.const 0))
      (loop $label
        (if (i32.lt_u (local.get $j) (local.get $len))
          (then
            (local.set $b
              (i32.load8_u
                (i32.add
                  (local.get $ptr)
                  (i32.add (i32.add (local.get $pos) (i32.const 1)) (local.get $j)))))
            (if (i32.eqz (call $is_label_byte (local.get $b)))
              (then (return (i32.const 3))))
            (if
              (i32.and
                (i32.eqz (local.get $j))
                (i32.eq (local.get $b) (i32.const 45)))
              (then (return (i32.const 3))))
            (local.set $last (local.get $b))
            (local.set $j (i32.add (local.get $j) (i32.const 1)))
            (br $label))))
      (if (i32.eq (local.get $last) (i32.const 45))
        (then (return (i32.const 3))))
      (i32.store
        (i32.add (local.get $out_ptr) (i32.add (i32.const 20) (i32.mul (local.get $labels) (i32.const 8))))
        (i32.add (local.get $pos) (i32.const 1)))
      (i32.store
        (i32.add (local.get $out_ptr) (i32.add (i32.const 24) (i32.mul (local.get $labels) (i32.const 8))))
        (local.get $len))
      (local.set $name_len
        (i32.add
          (local.get $name_len)
          (i32.add (local.get $len) (if (result i32) (i32.eqz (local.get $labels)) (then (i32.const 0)) (else (i32.const 1))))))
      (if (i32.gt_u (local.get $name_len) (i32.const 253))
        (then (return (i32.const 6))))
      (local.set $labels (i32.add (local.get $labels) (i32.const 1)))
      (if (i32.eqz (local.get $jumped))
        (then (local.set $consumed (i32.add (local.get $consumed) (i32.add (local.get $len) (i32.const 1))))))
      (local.set $pos (i32.add (i32.add (local.get $pos) (i32.const 1)) (local.get $len)))
      (br $scan))
    (i32.const 3))

  ;; Return bits: low32=status, high32=written. Writes a lowercase dotted name.
  (func (export "dns_name_to_lower_ascii_compressed")
    (param $ptr i32) (param $msg_len i32) (param $start i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $meta i32)
    (local $status i32)
    (local $labels i32)
    (local $name_len i32)
    (local $i i32)
    (local $j i32)
    (local $off i32)
    (local $len i32)
    (local $written i32)
    (local $b i32)
    (local.set $meta (i32.const 4096))
    (local.set $status
      (call $dns_name_follow
        (local.get $ptr)
        (local.get $msg_len)
        (local.get $start)
        (local.get $meta)
        (i32.const 2048)))
    (if (i32.ne (local.get $status) (i32.const 0))
      (then (return (call $pack (local.get $status) (i32.const 0)))))
    (local.set $labels (i32.load (i32.add (local.get $meta) (i32.const 4))))
    (local.set $name_len (i32.load (i32.add (local.get $meta) (i32.const 8))))
    (if (i32.lt_u (local.get $out_cap) (local.get $name_len))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))
    (loop $labels_loop
      (if (i32.lt_u (local.get $i) (local.get $labels))
        (then
          (if (i32.ne (local.get $i) (i32.const 0))
            (then
              (i32.store8 (i32.add (local.get $out_ptr) (local.get $written)) (i32.const 46))
              (local.set $written (i32.add (local.get $written) (i32.const 1)))))
          (local.set $off (call $u32_at (local.get $meta) (i32.add (i32.const 5) (i32.mul (local.get $i) (i32.const 2)))))
          (local.set $len (call $u32_at (local.get $meta) (i32.add (i32.const 6) (i32.mul (local.get $i) (i32.const 2)))))
          (local.set $j (i32.const 0))
          (loop $copy_label
            (if (i32.lt_u (local.get $j) (local.get $len))
              (then
                (local.set $b
                  (call $lower_ascii
                    (i32.load8_u
                      (i32.add (local.get $ptr) (i32.add (local.get $off) (local.get $j))))))
                (i32.store8 (i32.add (local.get $out_ptr) (local.get $written)) (local.get $b))
                (local.set $written (i32.add (local.get $written) (i32.const 1)))
                (local.set $j (i32.add (local.get $j) (i32.const 1)))
                (br $copy_label))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $labels_loop))))
    (call $pack (i32.const 0) (local.get $written)))
)
