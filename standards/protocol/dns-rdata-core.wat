
  (import "dns" "is_label_byte" (func $is_label_byte (param i32) (result i32)))
  (import "binary" "read_u16_be" (func $read_u16_be (param $ptr i32) (result i32)))

;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid, 6 too_long.
  ;; Output record:
  ;; 0:u32 consumed_wire_bytes_at_start
  ;; 4:u32 label_count
  ;; 8:u32 normalized_name_len
  ;; 12:u32 pointer_count
  ;; 16:u32 terminal_offset
  ;; 20..: repeated u32 label_offset_from_message, u32 label_len
  (func $m81dns_name_follow
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

  ;; A output: octet0, octet1, octet2, octet3, ipv4_be.
  (func (export "dns_rdata_a_decode")
    (param $ptr i32) (param $msg_len i32) (param $rdata_start i32) (param $rdlen i32) (param $out_ptr i32)
    (result i32)
    (local $b0 i32) (local $b1 i32) (local $b2 i32) (local $b3 i32)
    (if (i32.ne (local.get $rdlen) (i32.const 4))
      (then (return (i32.const 3))))
    (if (i32.gt_u (i32.add (local.get $rdata_start) (i32.const 4)) (local.get $msg_len))
      (then (return (i32.const 1))))
    (local.set $b0 (i32.load8_u (i32.add (local.get $ptr) (local.get $rdata_start))))
    (local.set $b1 (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $rdata_start)) (i32.const 1))))
    (local.set $b2 (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $rdata_start)) (i32.const 2))))
    (local.set $b3 (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $rdata_start)) (i32.const 3))))
    (i32.store (local.get $out_ptr) (local.get $b0))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $b1))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $b2))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 12)) (local.get $b3))
    (i32.store
      (i32.add (local.get $out_ptr) (i32.const 16))
      (i32.or
        (i32.or (i32.shl (local.get $b0) (i32.const 24)) (i32.shl (local.get $b1) (i32.const 16)))
        (i32.or (i32.shl (local.get $b2) (i32.const 8)) (local.get $b3))))
    (i32.const 0))

  ;; AAAA output: eight u16 words in network order.
  (func (export "dns_rdata_aaaa_decode")
    (param $ptr i32) (param $msg_len i32) (param $rdata_start i32) (param $rdlen i32) (param $out_ptr i32)
    (result i32)
    (local $i i32)
    (if (i32.ne (local.get $rdlen) (i32.const 16))
      (then (return (i32.const 3))))
    (if (i32.gt_u (i32.add (local.get $rdata_start) (i32.const 16)) (local.get $msg_len))
      (then (return (i32.const 1))))
    (loop $words
      (if (i32.lt_u (local.get $i) (i32.const 8))
        (then
          (i32.store
            (i32.add (local.get $out_ptr) (i32.mul (local.get $i) (i32.const 4)))
            (call $read_u16_be
              (i32.add
                (local.get $ptr)
                (i32.add (local.get $rdata_start) (i32.mul (local.get $i) (i32.const 2))))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $words))))
    (i32.const 0))

  ;; Name output: rdata_start, rdlen, consumed_wire_len, label_count,
  ;; normalized_name_len, pointer_count, terminal_offset, then label spans.
  (func $dns_rdata_name_decode (export "dns_rdata_name_decode")
    (param $ptr i32) (param $msg_len i32) (param $rdata_start i32) (param $rdlen i32) (param $out_ptr i32) (param $out_cap i32)
    (result i32)
    (local $tmp i32)
    (local $status i32)
    (local $consumed i32)
    (local $labels i32)
    (local $i i32)
    (if (i32.lt_u (local.get $out_cap) (i32.const 28))
      (then (return (i32.const 2))))
    (if (i32.gt_u (i32.add (local.get $rdata_start) (local.get $rdlen)) (local.get $msg_len))
      (then (return (i32.const 1))))
    (local.set $tmp (i32.const 8192))
    (local.set $status (call $m81dns_name_follow (local.get $ptr) (local.get $msg_len) (local.get $rdata_start) (local.get $tmp) (i32.const 2048)))
    (if (i32.ne (local.get $status) (i32.const 0))
      (then (return (local.get $status))))
    (local.set $consumed (i32.load (local.get $tmp)))
    (if (i32.ne (local.get $consumed) (local.get $rdlen))
      (then (return (i32.const 3))))
    (local.set $labels (i32.load (i32.add (local.get $tmp) (i32.const 4))))
    (if (i32.lt_u (local.get $out_cap) (i32.add (i32.const 28) (i32.mul (local.get $labels) (i32.const 8))))
      (then (return (i32.const 2))))
    (i32.store (local.get $out_ptr) (local.get $rdata_start))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $rdlen))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $consumed))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 12)) (local.get $labels))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 16)) (i32.load (i32.add (local.get $tmp) (i32.const 8))))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 20)) (i32.load (i32.add (local.get $tmp) (i32.const 12))))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 24)) (i32.load (i32.add (local.get $tmp) (i32.const 16))))
    (loop $copy
      (if (i32.lt_u (local.get $i) (local.get $labels))
        (then
          (i32.store
            (i32.add (local.get $out_ptr) (i32.add (i32.const 28) (i32.mul (local.get $i) (i32.const 8))))
            (i32.load (i32.add (local.get $tmp) (i32.add (i32.const 20) (i32.mul (local.get $i) (i32.const 8))))))
          (i32.store
            (i32.add (local.get $out_ptr) (i32.add (i32.const 32) (i32.mul (local.get $i) (i32.const 8))))
            (i32.load (i32.add (local.get $tmp) (i32.add (i32.const 24) (i32.mul (local.get $i) (i32.const 8))))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $copy))))
    (i32.const 0))

  ;; MX output: preference, exchange_start, exchange_wire_len, label_count,
  ;; normalized_name_len, pointer_count, terminal_offset.
  (func (export "dns_rdata_mx_decode")
    (param $ptr i32) (param $msg_len i32) (param $rdata_start i32) (param $rdlen i32) (param $out_ptr i32) (param $out_cap i32)
    (result i32)
    (local $name_out i32)
    (local $status i32)
    (if (i32.lt_u (local.get $out_cap) (i32.const 28))
      (then (return (i32.const 2))))
    (if (i32.lt_u (local.get $rdlen) (i32.const 3))
      (then (return (i32.const 3))))
    (if (i32.gt_u (i32.add (local.get $rdata_start) (local.get $rdlen)) (local.get $msg_len))
      (then (return (i32.const 1))))
    (local.set $name_out (i32.const 6144))
    (local.set $status
      (call $dns_rdata_name_decode
        (local.get $ptr)
        (local.get $msg_len)
        (i32.add (local.get $rdata_start) (i32.const 2))
        (i32.sub (local.get $rdlen) (i32.const 2))
        (local.get $name_out)
        (i32.const 2048)))
    (if (i32.ne (local.get $status) (i32.const 0))
      (then (return (local.get $status))))
    (i32.store (local.get $out_ptr) (call $read_u16_be (i32.add (local.get $ptr) (local.get $rdata_start))))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (i32.load (local.get $name_out)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (i32.load (i32.add (local.get $name_out) (i32.const 4))))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 12)) (i32.load (i32.add (local.get $name_out) (i32.const 12))))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 16)) (i32.load (i32.add (local.get $name_out) (i32.const 16))))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 20)) (i32.load (i32.add (local.get $name_out) (i32.const 20))))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 24)) (i32.load (i32.add (local.get $name_out) (i32.const 24))))
    (i32.const 0))

  ;; TXT output:
  ;; 0:u32 string_count, 4:u32 total_text_bytes, 8:u32 first_offset,
  ;; 12:u32 first_len, 16:u32 last_offset, 20:u32 last_len,
  ;; 24..: repeated u32 string_offset_from_message, u32 string_len.
  (func (export "dns_rdata_txt_walk")
    (param $ptr i32) (param $msg_len i32) (param $rdata_start i32) (param $rdlen i32) (param $out_ptr i32) (param $out_cap i32)
    (result i32)
    (local $end i32)
    (local $pos i32)
    (local $len i32)
    (local $count i32)
    (local $total i32)
    (local $need i32)
    (if (i32.lt_u (local.get $out_cap) (i32.const 24))
      (then (return (i32.const 2))))
    (if (i32.gt_u (i32.add (local.get $rdata_start) (local.get $rdlen)) (local.get $msg_len))
      (then (return (i32.const 1))))
    (local.set $end (i32.add (local.get $rdata_start) (local.get $rdlen)))
    (local.set $pos (local.get $rdata_start))
    (loop $walk
      (if (i32.lt_u (local.get $pos) (local.get $end))
        (then
          (local.set $len (i32.load8_u (i32.add (local.get $ptr) (local.get $pos))))
          (if (i32.gt_u (i32.add (i32.add (local.get $pos) (i32.const 1)) (local.get $len)) (local.get $end))
            (then (return (i32.const 1))))
          (local.set $need (i32.add (i32.const 24) (i32.mul (i32.add (local.get $count) (i32.const 1)) (i32.const 8))))
          (if (i32.lt_u (local.get $out_cap) (local.get $need))
            (then (return (i32.const 2))))
          (if (i32.eqz (local.get $count))
            (then
              (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (i32.add (local.get $pos) (i32.const 1)))
              (i32.store (i32.add (local.get $out_ptr) (i32.const 12)) (local.get $len))))
          (i32.store (i32.add (local.get $out_ptr) (i32.const 16)) (i32.add (local.get $pos) (i32.const 1)))
          (i32.store (i32.add (local.get $out_ptr) (i32.const 20)) (local.get $len))
          (i32.store
            (i32.add (local.get $out_ptr) (i32.add (i32.const 24) (i32.mul (local.get $count) (i32.const 8))))
            (i32.add (local.get $pos) (i32.const 1)))
          (i32.store
            (i32.add (local.get $out_ptr) (i32.add (i32.const 28) (i32.mul (local.get $count) (i32.const 8))))
            (local.get $len))
          (local.set $count (i32.add (local.get $count) (i32.const 1)))
          (local.set $total (i32.add (local.get $total) (local.get $len)))
          (local.set $pos (i32.add (i32.add (local.get $pos) (i32.const 1)) (local.get $len)))
          (br $walk))))
    (i32.store (local.get $out_ptr) (local.get $count))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $total))
    (if (i32.eqz (local.get $count))
      (then
        (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (i32.const 0))
        (i32.store (i32.add (local.get $out_ptr) (i32.const 12)) (i32.const 0))
        (i32.store (i32.add (local.get $out_ptr) (i32.const 16)) (i32.const 0))
        (i32.store (i32.add (local.get $out_ptr) (i32.const 20)) (i32.const 0))))
    (i32.const 0))