;; ═════════════════════════════════════════════════════════════════════
  ;; DNS Core Primitives — shared across all DNS parsers
  ;; ═════════════════════════════════════════════════════════════════════

;; Valid DNS label byte: ALPHA, DIGIT, '-' (0x2D), or '_' (0x5F).
  


;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid, 6 too_long.

  (func $u32_at (param $ptr i32) (param $index i32) (result i32)
    (i32.load (i32.add (local.get $ptr) (i32.mul (local.get $index) (i32.const 4)))))

  ;; Return bits: low32=status, high32=consumed bytes at start.
  (func (export "dns_name_wire_len") (param $ptr i32) (param $msg_len i32) (param $start i32) (result i64)
    (local $scratch i32)
    (local $status i32)
    (local.set $scratch (i32.const 4096))
    (local.set $status
      (call $m77dns_name_follow
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
  (func $m77dns_name_follow (export "dns_name_follow")
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
  (func $dns_name_to_lower_ascii_compressed (export "dns_name_to_lower_ascii_compressed")
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
      (call $m77dns_name_follow
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
                  (call $to_lower
                    (i32.load8_u
                      (i32.add (local.get $ptr) (i32.add (local.get $off) (local.get $j))))))
                (i32.store8 (i32.add (local.get $out_ptr) (local.get $written)) (local.get $b))
                (local.set $written (i32.add (local.get $written) (i32.const 1)))
                (local.set $j (i32.add (local.get $j) (i32.const 1)))
                (br $copy_label))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $labels_loop))))
    (call $pack (i32.const 0) (local.get $written)))



;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid, 6 too_long.
  ;; DNSSEC result values mirror dnssec.rs: 0 Valid, 1 Expired, 2 NoSignature,
  ;; 3 NoKey, 4 BadSignature, 5 ChainBroken, 6 Insecure.

  (func $dns_record_type_status (export "dns_record_type_status") (param $rtype i32) (result i32)
    (if (i32.ne (call $dns_record_type_family (local.get $rtype)) (i32.const 0))
      (then (return (i32.const 0))))
    (i32.const 3))

  ;; Families: 0 unknown, 1 address, 2 name, 3 text, 4 service/locator,
  ;; 5 DNSSEC, 6 EDNS/TSIG/meta, 7 route/authority data.
  (func $dns_record_type_family (export "dns_record_type_family") (param $rtype i32) (result i32)
    (if (i32.or (i32.eq (local.get $rtype) (i32.const 1)) (i32.eq (local.get $rtype) (i32.const 28)))
      (then (return (i32.const 1))))
    (if (i32.or
          (i32.or (i32.eq (local.get $rtype) (i32.const 2)) (i32.eq (local.get $rtype) (i32.const 5)))
          (i32.or (i32.eq (local.get $rtype) (i32.const 12)) (i32.eq (local.get $rtype) (i32.const 17))))
      (then (return (i32.const 2))))
    (if (i32.or
          (i32.or (i32.eq (local.get $rtype) (i32.const 16)) (i32.eq (local.get $rtype) (i32.const 99)))
          (i32.eq (local.get $rtype) (i32.const 13)))
      (then (return (i32.const 3))))
    (if (i32.or
          (i32.or
            (i32.or (i32.eq (local.get $rtype) (i32.const 15)) (i32.eq (local.get $rtype) (i32.const 33)))
            (i32.or (i32.eq (local.get $rtype) (i32.const 35)) (i32.eq (local.get $rtype) (i32.const 52))))
          (i32.or
            (i32.or (i32.eq (local.get $rtype) (i32.const 64)) (i32.eq (local.get $rtype) (i32.const 65)))
            (i32.or (i32.eq (local.get $rtype) (i32.const 256)) (i32.eq (local.get $rtype) (i32.const 257)))))
      (then (return (i32.const 4))))
    (if (i32.or
          (i32.or (i32.eq (local.get $rtype) (i32.const 43)) (i32.eq (local.get $rtype) (i32.const 46)))
          (i32.or
            (i32.or (i32.eq (local.get $rtype) (i32.const 47)) (i32.eq (local.get $rtype) (i32.const 48)))
            (i32.eq (local.get $rtype) (i32.const 50))))
      (then (return (i32.const 5))))
    (if (i32.or
          (i32.or (i32.eq (local.get $rtype) (i32.const 41)) (i32.eq (local.get $rtype) (i32.const 250)))
          (i32.or (i32.eq (local.get $rtype) (i32.const 252)) (i32.eq (local.get $rtype) (i32.const 255))))
      (then (return (i32.const 6))))
    (if (i32.or
          (i32.or (i32.eq (local.get $rtype) (i32.const 6)) (i32.eq (local.get $rtype) (i32.const 18)))
          (i32.eq (local.get $rtype) (i32.const 29)))
      (then (return (i32.const 7))))
    (i32.const 0))

  (func $dns_class_status (export "dns_class_status") (param $rclass i32) (param $rtype i32) (result i32)
    (if (i32.eq (local.get $rtype) (i32.const 41))
      (then
        (if (i32.and (i32.ge_u (local.get $rclass) (i32.const 512)) (i32.le_u (local.get $rclass) (i32.const 4096)))
          (then (return (i32.const 0))))
        (return (i32.const 3))))
    (if (result i32)
      (i32.or (i32.eq (local.get $rclass) (i32.const 1)) (i32.eq (local.get $rclass) (i32.const 255)))
      (then (i32.const 0))
      (else (i32.const 3))))

  (func $dns_opcode_status (export "dns_opcode_status") (param $opcode i32) (result i32)
    (if (result i32)
      (i32.or
        (i32.or (i32.eq (local.get $opcode) (i32.const 0)) (i32.eq (local.get $opcode) (i32.const 1)))
        (i32.or
          (i32.eq (local.get $opcode) (i32.const 2))
          (i32.or (i32.eq (local.get $opcode) (i32.const 4)) (i32.eq (local.get $opcode) (i32.const 5)))))
      (then (i32.const 0))
      (else (i32.const 3))))

  (func $dns_rcode_status (export "dns_rcode_status") (param $rcode i32) (result i32)
    (if (result i32)
      (i32.le_u (local.get $rcode) (i32.const 10))
      (then (i32.const 0))
      (else (i32.const 3))))

  ;; Return bits: low32=status, high32=flags.
  (func (export "dns_header_flags_pack")
    (param $qr i32) (param $opcode i32) (param $aa i32) (param $tc i32)
    (param $rd i32) (param $ra i32) (param $rcode i32)
    (result i64)
    (local $flags i32)
    (if (i32.or
          (i32.or (i32.gt_u (local.get $qr) (i32.const 1)) (i32.gt_u (local.get $aa) (i32.const 1)))
          (i32.or
            (i32.or (i32.gt_u (local.get $tc) (i32.const 1)) (i32.gt_u (local.get $rd) (i32.const 1)))
            (i32.gt_u (local.get $ra) (i32.const 1))))
      (then (return (call $pack (i32.const 3) (i32.const 0)))))
    (if (i32.ne (call $dns_opcode_status (local.get $opcode)) (i32.const 0))
      (then (return (call $pack (i32.const 3) (i32.const 0)))))
    (if (i32.ne (call $dns_rcode_status (local.get $rcode)) (i32.const 0))
      (then (return (call $pack (i32.const 3) (i32.const 0)))))
    (local.set $flags
      (i32.or
        (i32.or
          (i32.shl (local.get $qr) (i32.const 15))
          (i32.shl (i32.and (local.get $opcode) (i32.const 15)) (i32.const 11)))
        (i32.or
          (i32.or (i32.shl (local.get $aa) (i32.const 10)) (i32.shl (local.get $tc) (i32.const 9)))
          (i32.or
            (i32.or (i32.shl (local.get $rd) (i32.const 8)) (i32.shl (local.get $ra) (i32.const 7)))
            (i32.and (local.get $rcode) (i32.const 15))))))
    (call $pack (i32.const 0) (local.get $flags)))

  ;; Output record: 0:qr 4:opcode 8:aa 12:tc 16:rd 20:ra 24:rcode.
  (func (export "dns_header_flags_unpack") (param $flags i32) (param $out_ptr i32) (param $out_cap i32) (result i32)
    (local $opcode i32)
    (local $rcode i32)
    (if (i32.lt_u (local.get $out_cap) (i32.const 28))
      (then (return (i32.const 2))))
    (local.set $opcode (i32.and (i32.shr_u (local.get $flags) (i32.const 11)) (i32.const 15)))
    (local.set $rcode (i32.and (local.get $flags) (i32.const 15)))
    (if (i32.or
          (i32.ne (call $dns_opcode_status (local.get $opcode)) (i32.const 0))
          (i32.ne (call $dns_rcode_status (local.get $rcode)) (i32.const 0)))
      (then (return (i32.const 3))))
    (i32.store (local.get $out_ptr) (i32.shr_u (i32.and (local.get $flags) (i32.const 0x8000)) (i32.const 15)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $opcode))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (i32.shr_u (i32.and (local.get $flags) (i32.const 0x0400)) (i32.const 10)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 12)) (i32.shr_u (i32.and (local.get $flags) (i32.const 0x0200)) (i32.const 9)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 16)) (i32.shr_u (i32.and (local.get $flags) (i32.const 0x0100)) (i32.const 8)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 20)) (i32.shr_u (i32.and (local.get $flags) (i32.const 0x0080)) (i32.const 7)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 24)) (local.get $rcode))
    (i32.const 0))

  (func $dns_label_validate (export "dns_label_validate") (param $in_ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $b i32)
    (local $last i32)
    (if (i32.or (i32.eqz (local.get $len)) (i32.gt_u (local.get $len) (i32.const 63)))
      (then (return (i32.const 3))))
    (loop $scan
      (if (i32.lt_u (local.get $i) (local.get $len))
        (then
          (local.set $b (i32.load8_u (i32.add (local.get $in_ptr) (local.get $i))))
          (if (i32.eqz (call $is_label_byte (local.get $b)))
            (then (return (i32.const 3))))
          (if (i32.and (i32.eqz (local.get $i)) (i32.eq (local.get $b) (i32.const 45)))
            (then (return (i32.const 3))))
          (local.set $last (local.get $b))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $scan))))
    (if (i32.eq (local.get $last) (i32.const 45))
      (then (return (i32.const 3))))
    (i32.const 0))

  ;; Output record: 0:consumed 4:labels 8:normalized dotted length.
  (func (export "dns_name_scan")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i32)
    (local $pos i32)
    (local $len i32)
    (local $labels i32)
    (local $name_len i32)
    (local $record_need i32)
    (if (i32.lt_u (local.get $out_cap) (i32.const 12))
      (then (return (i32.const 2))))
    (loop $scan
      (if (i32.ge_u (local.get $pos) (local.get $in_len))
        (then (return (i32.const 1))))
      (local.set $len (i32.load8_u (i32.add (local.get $in_ptr) (local.get $pos))))
      (if (i32.eq (local.get $len) (i32.const 0))
        (then
          (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
          (if (i32.gt_u (local.get $pos) (i32.const 255))
            (then (return (i32.const 6))))
          (i32.store (local.get $out_ptr) (local.get $pos))
          (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $labels))
          (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $name_len))
          (return (i32.const 0))))
      (if (i32.ne (i32.and (local.get $len) (i32.const 0xc0)) (i32.const 0))
        (then (return (i32.const 3))))
      (if (i32.gt_u (i32.add (i32.add (local.get $pos) (i32.const 1)) (local.get $len)) (local.get $in_len))
        (then (return (i32.const 1))))
      (if (i32.gt_u (i32.add (i32.add (local.get $pos) (i32.const 1)) (local.get $len)) (i32.const 255))
        (then (return (i32.const 6))))
      (if (i32.ne (call $dns_label_validate (i32.add (local.get $in_ptr) (i32.add (local.get $pos) (i32.const 1))) (local.get $len)) (i32.const 0))
        (then (return (i32.const 3))))
      (local.set $record_need
        (i32.add (i32.const 12) (i32.mul (i32.add (local.get $labels) (i32.const 1)) (i32.const 8))))
      (if (i32.lt_u (local.get $out_cap) (local.get $record_need))
        (then (return (i32.const 2))))
      (i32.store
        (i32.add (local.get $out_ptr) (i32.add (i32.const 12) (i32.mul (local.get $labels) (i32.const 8))))
        (i32.add (local.get $pos) (i32.const 1)))
      (i32.store
        (i32.add (local.get $out_ptr) (i32.add (i32.const 16) (i32.mul (local.get $labels) (i32.const 8))))
        (local.get $len))
      (local.set $name_len
        (i32.add
          (local.get $name_len)
          (i32.add (local.get $len) (if (result i32) (i32.eqz (local.get $labels)) (then (i32.const 0)) (else (i32.const 1))))))
      (if (i32.gt_u (local.get $name_len) (i32.const 253))
        (then (return (i32.const 6))))
      (local.set $labels (i32.add (local.get $labels) (i32.const 1)))
      (local.set $pos (i32.add (i32.add (local.get $pos) (i32.const 1)) (local.get $len)))
      (br $scan))
    (i32.const 3))

  (func $dnssec_algorithm_status (export "dnssec_algorithm_status") (param $algorithm i32) (result i32)
    (if (result i32)
      (i32.or
        (i32.or (i32.eq (local.get $algorithm) (i32.const 8)) (i32.eq (local.get $algorithm) (i32.const 13)))
        (i32.or (i32.eq (local.get $algorithm) (i32.const 15)) (i32.eq (local.get $algorithm) (i32.const 16))))
      (then (i32.const 0))
      (else (i32.const 6))))

  (func $dnssec_digest_status (export "dnssec_digest_status") (param $digest_type i32) (result i32)
    (if (result i32)
      (i32.or
        (i32.eq (local.get $digest_type) (i32.const 1))
        (i32.or (i32.eq (local.get $digest_type) (i32.const 2)) (i32.eq (local.get $digest_type) (i32.const 4))))
      (then (i32.const 0))
      (else (i32.const 5))))

  (func (export "dnssec_result_status") (param $result i32) (result i32)
    (if (result i32)
      (i32.le_u (local.get $result) (i32.const 6))
      (then (i32.const 0))
      (else (i32.const 3))))

  (func (export "dnssec_dnskey_header_status") (param $protocol i32) (param $algorithm i32) (param $public_key_len i32) (result i32)
    (if (i32.ne (local.get $protocol) (i32.const 3))
      (then (return (i32.const 3))))
    (if (i32.eqz (local.get $public_key_len))
      (then (return (i32.const 3))))
    (call $dnssec_algorithm_status (local.get $algorithm)))

  (func (export "dnssec_ds_digest_length_status") (param $digest_type i32) (param $digest_len i32) (result i32)
    (if (i32.ne (call $dnssec_digest_status (local.get $digest_type)) (i32.const 0))
      (then (return (i32.const 5))))
    (if (result i32)
      (i32.or
        (i32.and (i32.eq (local.get $digest_type) (i32.const 1)) (i32.eq (local.get $digest_len) (i32.const 20)))
        (i32.or
          (i32.and (i32.eq (local.get $digest_type) (i32.const 2)) (i32.eq (local.get $digest_len) (i32.const 32)))
          (i32.and (i32.eq (local.get $digest_type) (i32.const 4)) (i32.eq (local.get $digest_len) (i32.const 48)))))
      (then (i32.const 0))
      (else (i32.const 3))))

  ;; Output record: 0:type 4:class 8:ttl 12:rdlength 16:rdata_start 20:end
  ;; 24:type_status 28:class_status.
  (func (export "dns_rr_trailer_decode")
    (param $in_ptr i32) (param $msg_len i32) (param $rr_start i32) (param $out_ptr i32) (param $out_cap i32)
    (result i32)
    (local $rtype i32)
    (local $rclass i32)
    (local $ttl i32)
    (local $rdlength i32)
    (local $rdata_start i32)
    (local $rr_end i32)
    (if (i32.lt_u (local.get $out_cap) (i32.const 32))
      (then (return (i32.const 2))))
    (if (i32.gt_u (i32.add (local.get $rr_start) (i32.const 10)) (local.get $msg_len))
      (then (return (i32.const 1))))
    (local.set $rtype (call $read_u16_be (i32.add (local.get $in_ptr) (local.get $rr_start))))
    (local.set $rclass (call $read_u16_be (i32.add (local.get $in_ptr) (i32.add (local.get $rr_start) (i32.const 2)))))
    (local.set $ttl (call $read_u32_be (i32.add (local.get $in_ptr) (i32.add (local.get $rr_start) (i32.const 4)))))
    (local.set $rdlength (call $read_u16_be (i32.add (local.get $in_ptr) (i32.add (local.get $rr_start) (i32.const 8)))))
    (local.set $rdata_start (i32.add (local.get $rr_start) (i32.const 10)))
    (local.set $rr_end (i32.add (local.get $rdata_start) (local.get $rdlength)))
    (if (i32.lt_u (local.get $rr_end) (local.get $rdata_start))
      (then (return (i32.const 3))))
    (if (i32.gt_u (local.get $rr_end) (local.get $msg_len))
      (then (return (i32.const 1))))
    (i32.store (local.get $out_ptr) (local.get $rtype))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $rclass))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $ttl))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 12)) (local.get $rdlength))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 16)) (local.get $rdata_start))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 20)) (local.get $rr_end))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 24)) (call $dns_record_type_status (local.get $rtype)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 28)) (call $dns_class_status (local.get $rclass) (local.get $rtype)))
    (if (result i32)
      (i32.or
        (i32.ne (call $dns_record_type_status (local.get $rtype)) (i32.const 0))
        (i32.ne (call $dns_class_status (local.get $rclass) (local.get $rtype)) (i32.const 0)))
      (then (i32.const 3))
      (else (i32.const 0))))


;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid.

  (func $valid_counts (param $qd i32) (param $an i32) (param $ns i32) (param $ar i32) (result i32)
    (i32.and
      (i32.le_u (local.get $qd) (i32.const 16))
      (i32.and
        (i32.le_u (local.get $an) (i32.const 256))
        (i32.and
          (i32.le_u (local.get $ns) (i32.const 256))
          (i32.le_u (local.get $ar) (i32.const 256))))))

  ;; Packed classifier:
  ;; bit0 QR, bits1..4 opcode, bit5 AA, bit6 TC, bit7 RD, bit8 RA, bits9..12 RCODE.
  (func (export "dns_header_flags_classify") (param $flags i32) (result i32)
    (i32.or
      (i32.or
        (i32.shr_u (i32.and (local.get $flags) (i32.const 0x8000)) (i32.const 15))
        (i32.shl (i32.and (i32.shr_u (local.get $flags) (i32.const 11)) (i32.const 0x0f)) (i32.const 1)))
      (i32.or
        (i32.or
          (i32.shr_u (i32.and (local.get $flags) (i32.const 0x0400)) (i32.const 5))
          (i32.shr_u (i32.and (local.get $flags) (i32.const 0x0200)) (i32.const 3)))
        (i32.or
          (i32.or
            (i32.shr_u (i32.and (local.get $flags) (i32.const 0x0100)) (i32.const 1))
            (i32.shl (i32.shr_u (i32.and (local.get $flags) (i32.const 0x0080)) (i32.const 7)) (i32.const 8)))
          (i32.shl (i32.and (local.get $flags) (i32.const 0x000f)) (i32.const 9))))))

  ;; Output record, little-endian:
  ;; 0:id, 4:flags, 8:qr, 12:opcode, 16:aa, 20:tc, 24:rd, 28:ra,
  ;; 32:rcode, 36:qdcount, 40:ancount, 44:nscount, 48:arcount.
  (func $dns_header_decode (export "dns_header_decode")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32)
    (result i32)
    (local $id i32)
    (local $flags i32)
    (local $qd i32)
    (local $an i32)
    (local $ns i32)
    (local $ar i32)
    (if (i32.lt_u (local.get $in_len) (i32.const 12))
      (then (return (i32.const 1))))
    (local.set $id (call $read_u16_be (local.get $in_ptr)))
    (local.set $flags (call $read_u16_be (i32.add (local.get $in_ptr) (i32.const 2))))
    (local.set $qd (call $read_u16_be (i32.add (local.get $in_ptr) (i32.const 4))))
    (local.set $an (call $read_u16_be (i32.add (local.get $in_ptr) (i32.const 6))))
    (local.set $ns (call $read_u16_be (i32.add (local.get $in_ptr) (i32.const 8))))
    (local.set $ar (call $read_u16_be (i32.add (local.get $in_ptr) (i32.const 10))))
    (if (i32.eqz (call $valid_counts (local.get $qd) (local.get $an) (local.get $ns) (local.get $ar)))
      (then (return (i32.const 3))))
    (i32.store (local.get $out_ptr) (local.get $id))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $flags))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 8))
      (i32.shr_u (i32.and (local.get $flags) (i32.const 0x8000)) (i32.const 15)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 12))
      (i32.and (i32.shr_u (local.get $flags) (i32.const 11)) (i32.const 0x0f)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 16))
      (i32.shr_u (i32.and (local.get $flags) (i32.const 0x0400)) (i32.const 10)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 20))
      (i32.shr_u (i32.and (local.get $flags) (i32.const 0x0200)) (i32.const 9)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 24))
      (i32.shr_u (i32.and (local.get $flags) (i32.const 0x0100)) (i32.const 8)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 28))
      (i32.shr_u (i32.and (local.get $flags) (i32.const 0x0080)) (i32.const 7)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 32))
      (i32.and (local.get $flags) (i32.const 0x000f)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 36)) (local.get $qd))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 40)) (local.get $an))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 44)) (local.get $ns))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 48)) (local.get $ar))
    (i32.const 0))

  ;; Return bits: low32=status, high32=written.
  (func $dns_header_encode (export "dns_header_encode")
    (param $id i32) (param $flags i32) (param $qd i32) (param $an i32) (param $ns i32) (param $ar i32)
    (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (if (i32.lt_u (local.get $out_cap) (i32.const 12))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))
    (if
      (i32.or
        (i32.or (i32.gt_u (local.get $id) (i32.const 65535)) (i32.gt_u (local.get $flags) (i32.const 65535)))
        (i32.eqz (call $valid_counts (local.get $qd) (local.get $an) (local.get $ns) (local.get $ar))))
      (then (return (call $pack (i32.const 3) (i32.const 0)))))
    (call $write_u16_be (local.get $out_ptr) (local.get $id))
    (call $write_u16_be (i32.add (local.get $out_ptr) (i32.const 2)) (local.get $flags))
    (call $write_u16_be (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $qd))
    (call $write_u16_be (i32.add (local.get $out_ptr) (i32.const 6)) (local.get $an))
    (call $write_u16_be (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $ns))
    (call $write_u16_be (i32.add (local.get $out_ptr) (i32.const 10)) (local.get $ar))
    (call $pack (i32.const 0) (i32.const 12)))


;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid, 6 too_long.

  ;; Output record, little-endian:
  ;; 0:u32 consumed wire bytes
  ;; 4:u32 label count
  ;; 8:u32 normalized dotted-name length, excluding root dot
  ;; 12..: repeated u32 label_offset_from_input, u32 label_len
  ;;
  ;; This scanner accepts uncompressed RFC1035 wire names only. Compression
  ;; pointers and reserved top-bit label forms are detected and rejected.
  (func $dns_name_scan 
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i32)
    (local $pos i32)
    (local $len i32)
    (local $labels i32)
    (local $name_len i32)
    (local $record_need i32)
    (local $j i32)
    (local $b i32)
    (local $last i32)
    (if (i32.lt_u (local.get $out_cap) (i32.const 12))
      (then (return (i32.const 2))))
    (loop $scan
      (if (i32.ge_u (local.get $pos) (local.get $in_len))
        (then (return (i32.const 1))))
      (local.set $len (i32.load8_u (i32.add (local.get $in_ptr) (local.get $pos))))
      (if (i32.eq (local.get $len) (i32.const 0))
        (then
          (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
          (if (i32.gt_u (local.get $pos) (i32.const 255))
            (then (return (i32.const 6))))
          (i32.store (local.get $out_ptr) (local.get $pos))
          (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $labels))
          (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $name_len))
          (return (i32.const 0))))
      (if (i32.ne (i32.and (local.get $len) (i32.const 0xc0)) (i32.const 0))
        (then (return (i32.const 3))))
      (if (i32.gt_u (local.get $len) (i32.const 63))
        (then (return (i32.const 3))))
      (if (i32.gt_u (i32.add (i32.add (local.get $pos) (i32.const 1)) (local.get $len)) (local.get $in_len))
        (then (return (i32.const 1))))
      (if (i32.gt_u (i32.add (i32.add (local.get $pos) (i32.const 1)) (local.get $len)) (i32.const 255))
        (then (return (i32.const 6))))
      (local.set $record_need
        (i32.add (i32.const 12) (i32.mul (i32.add (local.get $labels) (i32.const 1)) (i32.const 8))))
      (if (i32.lt_u (local.get $out_cap) (local.get $record_need))
        (then (return (i32.const 2))))
      (local.set $j (i32.const 0))
      (loop $label
        (if (i32.lt_u (local.get $j) (local.get $len))
          (then
            (local.set $b
              (i32.load8_u
                (i32.add
                  (local.get $in_ptr)
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
        (i32.add (local.get $out_ptr) (i32.add (i32.const 12) (i32.mul (local.get $labels) (i32.const 8))))
        (i32.add (local.get $pos) (i32.const 1)))
      (i32.store
        (i32.add (local.get $out_ptr) (i32.add (i32.const 16) (i32.mul (local.get $labels) (i32.const 8))))
        (local.get $len))
      (local.set $name_len
        (i32.add
          (local.get $name_len)
          (i32.add (local.get $len) (if (result i32) (i32.eqz (local.get $labels)) (then (i32.const 0)) (else (i32.const 1))))))
      (if (i32.gt_u (local.get $name_len) (i32.const 253))
        (then (return (i32.const 6))))
      (local.set $labels (i32.add (local.get $labels) (i32.const 1)))
      (local.set $pos (i32.add (i32.add (local.get $pos) (i32.const 1)) (local.get $len)))
      (br $scan))
    (i32.const 3))

  ;; Return bits: low32=status, high32=written. Writes a lowercase dotted name
  ;; from an uncompressed wire name. Root writes zero bytes.
  (func (export "dns_name_to_lower_ascii")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32)
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
    (local.set $status (call $dns_name_scan (local.get $in_ptr) (local.get $in_len) (local.get $meta) (i32.const 512)))
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
          (local.set $off (i32.load (i32.add (local.get $meta) (i32.add (i32.const 12) (i32.mul (local.get $i) (i32.const 8))))))
          (local.set $len (i32.load (i32.add (local.get $meta) (i32.add (i32.const 16) (i32.mul (local.get $i) (i32.const 8))))))
          (local.set $j (i32.const 0))
          (loop $copy_label
            (if (i32.lt_u (local.get $j) (local.get $len))
              (then
                (local.set $b (call $to_lower (i32.load8_u (i32.add (local.get $in_ptr) (i32.add (local.get $off) (local.get $j))))))
                (i32.store8 (i32.add (local.get $out_ptr) (local.get $written)) (local.get $b))
                (local.set $written (i32.add (local.get $written) (i32.const 1)))
                (local.set $j (i32.add (local.get $j) (i32.const 1)))
                (br $copy_label))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $labels_loop))))
    (call $pack (i32.const 0) (local.get $written)))



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
;; DNS A-record resolver — self-contained, uses abstract socket for UDP.
  ;; dns_resolve_a(host_ptr, host_len,
  ;;               dns_host_ptr, dns_host_len, dns_port,
  ;;               out_ip_ptr, out_ip_cap) -> i64
  ;;
  ;; Builds DNS A-record query, sends via UDP abstract socket,
  ;; parses response for A records. out_ip stores 4-byte IPs.
  ;; Returns packed (status, ip_count).
  (func (export "dns_resolve_a")
    (param $host i32) (param $hlen i32)
    (param $dns_host i32) (param $dns_hlen i32) (param $dns_port i32)
    (param $out i32) (param $ocap i32)
    (result i64)
    (local $qbuf i32) (local $rbuf i32) (local $qlen i32)
    (local $fd i32) (local $rc i32) (local $rlen i32)
    (local $i i32) (local $j i32) (local $b i32)
    (local $llen i32) (local $ip_count i32)
    (local $ans_count i32)

    i32.const 4096 local.set $qbuf
    i32.const 5120 local.set $rbuf

    ;; ── 1. Build DNS header (12 bytes) ──
    local.get $qbuf i32.const 0 i32.add i32.const 0x1234 i32.store16  ;; ID
    local.get $qbuf i32.const 2 i32.add i32.const 0x0100 i32.store16  ;; flags: RD=1
    local.get $qbuf i32.const 4 i32.add i32.const 1 i32.store16       ;; qdcount=1
    local.get $qbuf i32.const 6 i32.add i32.const 0 i32.store16       ;; ancount=0
    local.get $qbuf i32.const 8 i32.add i32.const 0 i32.store16       ;; nscount=0
    local.get $qbuf i32.const 10 i32.add i32.const 0 i32.store16      ;; arcount=0
    i32.const 12 local.set $qlen

    ;; ── 2. Encode hostname as DNS name ──
    i32.const 0 local.set $i
    block $encode_done
    loop $encode
      local.get $i local.get $hlen i32.ge_u
      if
        local.get $qbuf local.get $qlen i32.add i32.const 0 i32.store8
        local.get $qlen i32.const 1 i32.add local.set $qlen
        br $encode_done
      end
      i32.const 0 local.set $llen
      block $llen_done
      loop $llen_loop
        local.get $i local.get $llen i32.add local.get $hlen i32.ge_u
        br_if $llen_done
        local.get $host local.get $i local.get $llen i32.add i32.add i32.load8_u
        i32.const 46 i32.eq
        br_if $llen_done
        local.get $llen i32.const 1 i32.add local.set $llen
        br $llen_loop
      end
      end
      local.get $qbuf local.get $qlen i32.add local.get $llen i32.store8
      local.get $qlen i32.const 1 i32.add local.set $qlen
      i32.const 0 local.set $j
      block $lbl_done
      loop $lbl_loop
        local.get $j local.get $llen i32.ge_u br_if $lbl_done
        local.get $qbuf local.get $qlen i32.add
        local.get $host local.get $i local.get $j i32.add i32.add i32.load8_u
        i32.store8
        local.get $qlen i32.const 1 i32.add local.set $qlen
        local.get $j i32.const 1 i32.add local.set $j
        br $lbl_loop
      end
      end
      local.get $i local.get $llen i32.add i32.const 1 i32.add local.set $i
      br $encode
    end
    end

    ;; ── 3. Append QTYPE=A(1), QCLASS=IN(1) ──
    local.get $qbuf local.get $qlen i32.add i32.const 0 i32.store8
    local.get $qbuf local.get $qlen i32.const 1 i32.add i32.add i32.const 1 i32.store8
    local.get $qlen i32.const 2 i32.add local.set $qlen
    local.get $qbuf local.get $qlen i32.add i32.const 0 i32.store8
    local.get $qbuf local.get $qlen i32.const 1 i32.add i32.add i32.const 1 i32.store8
    local.get $qlen i32.const 2 i32.add local.set $qlen

    ;; ── 4. Build UDP config right after query ──
    local.get $qbuf local.get $qlen i32.add i32.const 0 i32.add local.get $dns_host i32.store
    local.get $qbuf local.get $qlen i32.add i32.const 4 i32.add local.get $dns_hlen i32.store
    local.get $qbuf local.get $qlen i32.add i32.const 8 i32.add local.get $dns_port i32.store16

    ;; ── 5. sock_open(SOCK_UDP=5, cfg, 12) ──
    i32.const 5 local.get $qbuf local.get $qlen i32.add i32.const 12 i32.const 0 call $sock_open
    local.tee $fd
    i32.const 0 i32.lt_s
    if i64.const -2 return end

    ;; ── 6. sock_send(fd, qbuf, qlen) ──
    local.get $fd local.get $qbuf local.get $qlen call $sock_send
    i32.const 0 i32.lt_s
    if local.get $fd call $sock_close i64.const -3 return end

    ;; ── 7. sock_recv into rbuf ──
    local.get $fd local.get $rbuf i32.const 512 call $sock_recv
    local.tee $rc
    i32.const 0 i32.le_s
    if local.get $fd call $sock_close i64.const -4 return end
    local.get $rc local.set $rlen
    local.get $fd call $sock_close

    ;; ── 8. Parse response header ──
    local.get $rlen i32.const 12 i32.lt_u
    if i64.const -5 return end
    local.get $rbuf i32.load16_u i32.const 0x1234 i32.ne
    if i64.const -5 return end
    local.get $rbuf i32.load8_u offset=2 i32.const 0x80 i32.and i32.eqz
    if i64.const -5 return end
    local.get $rbuf i32.load8_u offset=3 i32.const 15 i32.and i32.const 0 i32.ne
    if i64.const -5 return end
    local.get $rbuf i32.load16_u offset=6
    local.tee $ans_count
    i32.eqz
    if i64.const -6 return end

    ;; ── 9. Skip question section ──
    local.get $rbuf i32.load16_u offset=4
    local.set $rc
    i32.const 12 local.set $i
    block $q_done
    loop $q_loop
      local.get $rc i32.eqz br_if $q_done
      local.get $i local.get $rlen i32.ge_u
      if i64.const -5 return end
      local.get $rbuf local.get $i i32.add i32.load8_u
      i32.const 0xc0 i32.and i32.const 0xc0 i32.eq
      if
        local.get $i i32.const 2 i32.add local.set $i
      else
        block $qname_break
        loop $qname_loop
          local.get $i local.get $rlen i32.ge_u
          if i64.const -5 return end
          local.get $rbuf local.get $i i32.add i32.load8_u
          local.tee $b
          i32.eqz
          if
            local.get $i i32.const 1 i32.add local.set $i
            br $qname_break
          end
          local.get $i local.get $b i32.add i32.const 1 i32.add local.set $i
          br $qname_loop
        end
        end
      end
      local.get $i i32.const 4 i32.add local.set $i
      local.get $rc i32.const 1 i32.sub local.set $rc
      br $q_loop
    end
    end

    ;; ── 10. Parse answer section — extract A records ──
    i32.const 0 local.set $ip_count
    block $ans_done
    loop $ans_loop
      local.get $ans_count i32.eqz br_if $ans_done
      local.get $i i32.const 12 i32.add local.get $rlen i32.gt_u
      br_if $ans_done

      ;; skip RR name (compressed or inline)
      local.get $rbuf local.get $i i32.add i32.load8_u
      i32.const 0xc0 i32.and i32.const 0xc0 i32.eq
      if
        local.get $i i32.const 2 i32.add local.set $i
      else
        block $rr_name_break
        loop $rr_name_loop
          local.get $i local.get $rlen i32.ge_u br_if $ans_done
          local.get $rbuf local.get $i i32.add i32.load8_u
          local.tee $b
          i32.eqz
          if
            local.get $i i32.const 1 i32.add local.set $i
            br $rr_name_break
          end
          local.get $i local.get $b i32.add i32.const 1 i32.add local.set $i
          br $rr_name_loop
        end
        end
      end

      ;; read TYPE(2) + CLASS(2) + TTL(4) + RDLENGTH(2) starting at $i
      local.get $i i32.const 10 i32.add local.get $rlen i32.gt_u
      br_if $ans_done
      local.get $rbuf local.get $i i32.add i32.load16_u
      i32.const 1 i32.ne
      if
        ;; not A record — skip by rdlength
        local.get $rbuf local.get $i i32.const 8 i32.add i32.add i32.load16_u
        local.set $b
        local.get $i i32.const 10 i32.add local.get $b i32.add local.set $i
        local.get $ans_count i32.const 1 i32.sub local.set $ans_count
        br $ans_loop
      end

      ;; TYPE is A — skip CLASS(2) and TTL(4) to rdlength at $i+6
      local.get $rbuf local.get $i i32.const 6 i32.add i32.add i32.load16_u
      local.tee $b
      i32.const 4 i32.ne
      if
        local.get $i i32.const 8 i32.add local.get $b i32.add local.set $i
        local.get $ans_count i32.const 1 i32.sub local.set $ans_count
        br $ans_loop
      end

      ;; rdlength=4 — read IP at $i+8
      local.get $i i32.const 8 i32.add local.get $rlen i32.gt_u
      br_if $ans_done

      local.get $ip_count local.get $ocap i32.ge_u
      br_if $ans_done

      local.get $out local.get $ip_count i32.const 2 i32.shl i32.add
      local.get $i i32.const 8 i32.add local.get $rbuf i32.add i32.load
      i32.store

      local.get $ip_count i32.const 1 i32.add local.set $ip_count
      local.get $i i32.const 12 i32.add local.set $i
      local.get $ans_count i32.const 1 i32.sub local.set $ans_count
      br $ans_loop
    end
    end

    local.get $ip_count i32.eqz
    if i64.const -6 return end

    i64.const 0
    local.get $ip_count
    i64.extend_i32_u
    i64.const 32
    i64.shl
    i64.or)


;; Status values: 0 ok, 1 input_short, 3 invalid, 6 too_long.
  (func $name_len (param $ptr i32) (param $msg_len i32) (param $start i32) (result i64)
    (local $pos i32)
    (local $len i32)
    (local $target i32)
    (local $consumed i32)
    (local $jumped i32)
    (local $hops i32)
    (local $name_len i32)
    (if (i32.ge_u (local.get $start) (local.get $msg_len))
      (then (return (i64.const 1))))
    (local.set $pos (local.get $start))
    (loop $scan
      (if (i32.ge_u (local.get $pos) (local.get $msg_len))
        (then (return (i64.const 1))))
      (local.set $len (i32.load8_u (i32.add (local.get $ptr) (local.get $pos))))
      (if (i32.eqz (local.get $len))
        (then
          (if (i32.eqz (local.get $jumped))
            (then (local.set $consumed (i32.add (local.get $consumed) (i32.const 1)))))
          (return
            (i64.or
              (i64.extend_i32_u (i32.const 0))
              (i64.shl (i64.extend_i32_u (local.get $consumed)) (i64.const 32))))))
      (if (i32.eq (i32.and (local.get $len) (i32.const 0xc0)) (i32.const 0xc0))
        (then
          (if (i32.ge_u (i32.add (local.get $pos) (i32.const 1)) (local.get $msg_len))
            (then (return (i64.const 1))))
          (local.set $target
            (i32.or
              (i32.shl (i32.and (local.get $len) (i32.const 0x3f)) (i32.const 8))
              (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $pos)) (i32.const 1)))))
          (if (i32.ge_u (local.get $target) (local.get $msg_len))
            (then (return (i64.const 3))))
          (if (i32.eq (local.get $target) (local.get $pos))
            (then (return (i64.const 3))))
          (if (i32.eqz (local.get $jumped))
            (then
              (local.set $consumed (i32.add (local.get $consumed) (i32.const 2)))
              (local.set $jumped (i32.const 1))))
          (local.set $hops (i32.add (local.get $hops) (i32.const 1)))
          (if (i32.gt_u (local.get $hops) (i32.const 16))
            (then (return (i64.const 3))))
          (local.set $pos (local.get $target))
          (br $scan)))
      (if (i32.ne (i32.and (local.get $len) (i32.const 0xc0)) (i32.const 0))
        (then (return (i64.const 3))))
      (if (i32.gt_u (local.get $len) (i32.const 63))
        (then (return (i64.const 3))))
      (if (i32.gt_u (i32.add (i32.add (local.get $pos) (i32.const 1)) (local.get $len)) (local.get $msg_len))
        (then (return (i64.const 1))))
      (local.set $name_len
        (i32.add
          (local.get $name_len)
          (i32.add (local.get $len) (i32.const 1))))
      (if (i32.gt_u (local.get $name_len) (i32.const 255))
        (then (return (i64.const 6))))
      (if (i32.eqz (local.get $jumped))
        (then
          (local.set $consumed
            (i32.add (local.get $consumed) (i32.add (local.get $len) (i32.const 1))))))
      (local.set $pos (i32.add (i32.add (local.get $pos) (i32.const 1)) (local.get $len)))
      (br $scan))
    (i64.const 3))

  ;; Question record: name_start, name_wire_len, qtype, qclass, next_offset.
  (func $dns_question_next (export "dns_question_next")
    (param $ptr i32) (param $msg_len i32) (param $start i32) (param $out_ptr i32)
    (result i32)
    (local $packed i64)
    (local $status i32)
    (local $name_wire_len i32)
    (local $pos i32)
    (local.set $packed (call $name_len (local.get $ptr) (local.get $msg_len) (local.get $start)))
    (local.set $status (i32.wrap_i64 (local.get $packed)))
    (if (i32.ne (local.get $status) (i32.const 0))
      (then (return (local.get $status))))
    (local.set $name_wire_len (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
    (local.set $pos (i32.add (local.get $start) (local.get $name_wire_len)))
    (if (i32.gt_u (i32.add (local.get $pos) (i32.const 4)) (local.get $msg_len))
      (then (return (i32.const 1))))
    (i32.store (local.get $out_ptr) (local.get $start))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $name_wire_len))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (call $read_u16_be (i32.add (local.get $ptr) (local.get $pos))))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 12)) (call $read_u16_be (i32.add (i32.add (local.get $ptr) (local.get $pos)) (i32.const 2))))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 16)) (i32.add (local.get $pos) (i32.const 4)))
    (i32.const 0))

  ;; RR record: name_start, name_wire_len, type, class, ttl, rdlen, rdata_start, next_offset, is_opt.
  (func $dns_rr_next (export "dns_rr_next")
    (param $ptr i32) (param $msg_len i32) (param $start i32) (param $out_ptr i32)
    (result i32)
    (local $packed i64)
    (local $status i32)
    (local $name_wire_len i32)
    (local $pos i32)
    (local $typ i32)
    (local $rdlen i32)
    (local $rdata i32)
    (local $next i32)
    (local.set $packed (call $name_len (local.get $ptr) (local.get $msg_len) (local.get $start)))
    (local.set $status (i32.wrap_i64 (local.get $packed)))
    (if (i32.ne (local.get $status) (i32.const 0))
      (then (return (local.get $status))))
    (local.set $name_wire_len (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
    (local.set $pos (i32.add (local.get $start) (local.get $name_wire_len)))
    (if (i32.gt_u (i32.add (local.get $pos) (i32.const 10)) (local.get $msg_len))
      (then (return (i32.const 1))))
    (local.set $typ (call $read_u16_be (i32.add (local.get $ptr) (local.get $pos))))
    (local.set $rdlen (call $read_u16_be (i32.add (i32.add (local.get $ptr) (local.get $pos)) (i32.const 8))))
    (local.set $rdata (i32.add (local.get $pos) (i32.const 10)))
    (local.set $next (i32.add (local.get $rdata) (local.get $rdlen)))
    (if (i32.gt_u (local.get $next) (local.get $msg_len))
      (then (return (i32.const 1))))
    (i32.store (local.get $out_ptr) (local.get $start))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $name_wire_len))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $typ))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 12)) (call $read_u16_be (i32.add (i32.add (local.get $ptr) (local.get $pos)) (i32.const 2))))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 16)) (call $read_u32_be (i32.add (i32.add (local.get $ptr) (local.get $pos)) (i32.const 4))))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 20)) (local.get $rdlen))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 24)) (local.get $rdata))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 28)) (local.get $next))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 32)) (i32.eq (local.get $typ) (i32.const 41)))
    (i32.const 0))

  ;; Summary record: q_start, q_end, an_start, an_end, ns_start, ns_end,
  ;; ar_start, ar_end, q_count, rr_count, opt_count, error_offset.
  (func (export "dns_section_walk")
    (param $ptr i32) (param $msg_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i32)
    (local $qd i32) (local $an i32) (local $ns i32) (local $ar i32)
    (local $pos i32) (local $i i32) (local $status i32) (local $tmp i32)
    (local $an_start i32) (local $ns_start i32) (local $ar_start i32)
    (local $opt_count i32)
    (if (i32.lt_u (local.get $out_cap) (i32.const 48))
      (then (return (i32.const 2))))
    (if (i32.lt_u (local.get $msg_len) (i32.const 12))
      (then
        (i32.store (i32.add (local.get $out_ptr) (i32.const 44)) (local.get $msg_len))
        (return (i32.const 1))))
    (local.set $qd (call $read_u16_be (i32.add (local.get $ptr) (i32.const 4))))
    (local.set $an (call $read_u16_be (i32.add (local.get $ptr) (i32.const 6))))
    (local.set $ns (call $read_u16_be (i32.add (local.get $ptr) (i32.const 8))))
    (local.set $ar (call $read_u16_be (i32.add (local.get $ptr) (i32.const 10))))
    (if (i32.or
          (i32.gt_u (local.get $qd) (i32.const 16))
          (i32.or
            (i32.gt_u (local.get $an) (i32.const 256))
            (i32.or (i32.gt_u (local.get $ns) (i32.const 256)) (i32.gt_u (local.get $ar) (i32.const 256)))))
      (then
        (i32.store (i32.add (local.get $out_ptr) (i32.const 44)) (i32.const 12))
        (return (i32.const 3))))
    (local.set $pos (i32.const 12))
    (i32.store (local.get $out_ptr) (local.get $pos))
    (local.set $tmp (i32.const 8192))
    (local.set $i (i32.const 0))
    (loop $questions
      (if (i32.lt_u (local.get $i) (local.get $qd))
        (then
          (local.set $status (call $dns_question_next (local.get $ptr) (local.get $msg_len) (local.get $pos) (local.get $tmp)))
          (if (i32.ne (local.get $status) (i32.const 0))
            (then
              (i32.store (i32.add (local.get $out_ptr) (i32.const 44)) (local.get $pos))
              (return (local.get $status))))
          (local.set $pos (i32.load (i32.add (local.get $tmp) (i32.const 16))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $questions))))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $pos))
    (local.set $an_start (local.get $pos))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $an_start))
    (local.set $i (i32.const 0))
    (loop $answers
      (if (i32.lt_u (local.get $i) (local.get $an))
        (then
          (local.set $status (call $dns_rr_next (local.get $ptr) (local.get $msg_len) (local.get $pos) (local.get $tmp)))
          (if (i32.ne (local.get $status) (i32.const 0))
            (then
              (i32.store (i32.add (local.get $out_ptr) (i32.const 44)) (local.get $pos))
              (return (local.get $status))))
          (local.set $pos (i32.load (i32.add (local.get $tmp) (i32.const 28))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $answers))))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 12)) (local.get $pos))
    (local.set $ns_start (local.get $pos))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 16)) (local.get $ns_start))
    (local.set $i (i32.const 0))
    (loop $authority
      (if (i32.lt_u (local.get $i) (local.get $ns))
        (then
          (local.set $status (call $dns_rr_next (local.get $ptr) (local.get $msg_len) (local.get $pos) (local.get $tmp)))
          (if (i32.ne (local.get $status) (i32.const 0))
            (then
              (i32.store (i32.add (local.get $out_ptr) (i32.const 44)) (local.get $pos))
              (return (local.get $status))))
          (local.set $pos (i32.load (i32.add (local.get $tmp) (i32.const 28))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $authority))))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 20)) (local.get $pos))
    (local.set $ar_start (local.get $pos))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 24)) (local.get $ar_start))
    (local.set $i (i32.const 0))
    (loop $additional
      (if (i32.lt_u (local.get $i) (local.get $ar))
        (then
          (local.set $status (call $dns_rr_next (local.get $ptr) (local.get $msg_len) (local.get $pos) (local.get $tmp)))
          (if (i32.ne (local.get $status) (i32.const 0))
            (then
              (i32.store (i32.add (local.get $out_ptr) (i32.const 44)) (local.get $pos))
              (return (local.get $status))))
          (local.set $opt_count (i32.add (local.get $opt_count) (i32.load (i32.add (local.get $tmp) (i32.const 32)))))
          (local.set $pos (i32.load (i32.add (local.get $tmp) (i32.const 28))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $additional))))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 28)) (local.get $pos))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 32)) (local.get $qd))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 36)) (i32.add (i32.add (local.get $an) (local.get $ns)) (local.get $ar)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 40)) (local.get $opt_count))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 44)) (i32.const 0))
    (if (i32.ne (local.get $pos) (local.get $msg_len))
      (then
        (i32.store (i32.add (local.get $out_ptr) (i32.const 44)) (local.get $pos))
        (return (i32.const 3))))
    (i32.const 0))
