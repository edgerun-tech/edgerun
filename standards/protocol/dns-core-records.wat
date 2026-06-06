
  (import "dns" "is_label_byte" (func $is_label_byte (param i32) (result i32)))

(func (export "proto_standard_id") (result i32)
    i32.const 300162)

  ;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid, 6 too_long.
  ;; DNSSEC result values mirror dnssec.rs: 0 Valid, 1 Expired, 2 NoSignature,
  ;; 3 NoKey, 4 BadSignature, 5 ChainBroken, 6 Insecure.

  (func $m78read_u16_be (param $ptr i32) (result i32)
    (i32.or
      (i32.shl (i32.load8_u (local.get $ptr)) (i32.const 8))
      (i32.load8_u (i32.add (local.get $ptr) (i32.const 1)))))

  (func $m78read_u32_be (param $ptr i32) (result i32)
    (i32.or
      (i32.or
        (i32.shl (i32.load8_u (local.get $ptr)) (i32.const 24))
        (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))) (i32.const 16)))
      (i32.or
        (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 2))) (i32.const 8))
        (i32.load8_u (i32.add (local.get $ptr) (i32.const 3))))))

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
    (local.set $rtype (call $m78read_u16_be (i32.add (local.get $in_ptr) (local.get $rr_start))))
    (local.set $rclass (call $m78read_u16_be (i32.add (local.get $in_ptr) (i32.add (local.get $rr_start) (i32.const 2)))))
    (local.set $ttl (call $m78read_u32_be (i32.add (local.get $in_ptr) (i32.add (local.get $rr_start) (i32.const 4)))))
    (local.set $rdlength (call $m78read_u16_be (i32.add (local.get $in_ptr) (i32.add (local.get $rr_start) (i32.const 8)))))
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
