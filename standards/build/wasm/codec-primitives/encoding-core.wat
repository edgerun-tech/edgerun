(module
  (memory (export "memory") 1)

  (func $pack (param $status i32) (param $value i32) (result i64)
    (i64.or
      (i64.extend_i32_u (local.get $status))
      (i64.shl (i64.extend_i32_u (local.get $value)) (i64.const 32))))

  (func (export "proto_abi_version") (result i32)
    (i32.const 2))

  (func (export "proto_standard_id") (result i32)
    (i32.const 300001))

  (func $has (param $len i32) (param $offset i32) (param $need i32) (result i32)
    (if (result i32)
      (i32.lt_u (local.get $len) (local.get $need))
      (then (i32.const 0))
      (else
        (i32.le_u
          (local.get $offset)
          (i32.sub (local.get $len) (local.get $need))))))

  (func (export "read_u16_be") (param $ptr i32) (param $len i32) (param $offset i32) (result i64)
    (if (result i64)
      (call $has (local.get $len) (local.get $offset) (i32.const 2))
      (then
        (call $pack
          (i32.const 0)
          (i32.or
            (i32.shl
              (i32.load8_u (i32.add (local.get $ptr) (local.get $offset)))
              (i32.const 8))
            (i32.load8_u
              (i32.add
                (local.get $ptr)
                (i32.add (local.get $offset) (i32.const 1)))))))
      (else (call $pack (i32.const 1) (i32.const 0)))))

  (func (export "read_u24_be") (param $ptr i32) (param $len i32) (param $offset i32) (result i64)
    (if (result i64)
      (call $has (local.get $len) (local.get $offset) (i32.const 3))
      (then
        (call $pack
          (i32.const 0)
          (i32.or
            (i32.or
              (i32.shl
                (i32.load8_u (i32.add (local.get $ptr) (local.get $offset)))
                (i32.const 16))
              (i32.shl
                (i32.load8_u
                  (i32.add
                    (local.get $ptr)
                    (i32.add (local.get $offset) (i32.const 1))))
                (i32.const 8)))
            (i32.load8_u
              (i32.add
                (local.get $ptr)
                (i32.add (local.get $offset) (i32.const 2)))))))
      (else (call $pack (i32.const 1) (i32.const 0)))))

  (func (export "read_u32_be") (param $ptr i32) (param $len i32) (param $offset i32) (result i64)
    (if (result i64)
      (call $has (local.get $len) (local.get $offset) (i32.const 4))
      (then
        (call $pack
          (i32.const 0)
          (i32.or
            (i32.or
              (i32.shl
                (i32.load8_u (i32.add (local.get $ptr) (local.get $offset)))
                (i32.const 24))
              (i32.shl
                (i32.load8_u
                  (i32.add
                    (local.get $ptr)
                    (i32.add (local.get $offset) (i32.const 1))))
                (i32.const 16)))
            (i32.or
              (i32.shl
                (i32.load8_u
                  (i32.add
                    (local.get $ptr)
                    (i32.add (local.get $offset) (i32.const 2))))
                (i32.const 8))
              (i32.load8_u
                (i32.add
                  (local.get $ptr)
                  (i32.add (local.get $offset) (i32.const 3))))))))
      (else (call $pack (i32.const 1) (i32.const 0)))))

  (func (export "read_u16_le") (param $ptr i32) (param $len i32) (param $offset i32) (result i64)
    (if (result i64)
      (call $has (local.get $len) (local.get $offset) (i32.const 2))
      (then
        (call $pack
          (i32.const 0)
          (i32.or
            (i32.load8_u (i32.add (local.get $ptr) (local.get $offset)))
            (i32.shl
              (i32.load8_u
                (i32.add
                  (local.get $ptr)
                  (i32.add (local.get $offset) (i32.const 1))))
              (i32.const 8)))))
      (else (call $pack (i32.const 1) (i32.const 0)))))

  (func (export "read_u32_le") (param $ptr i32) (param $len i32) (param $offset i32) (result i64)
    (if (result i64)
      (call $has (local.get $len) (local.get $offset) (i32.const 4))
      (then
        (call $pack
          (i32.const 0)
          (i32.or
            (i32.or
              (i32.load8_u (i32.add (local.get $ptr) (local.get $offset)))
              (i32.shl
                (i32.load8_u
                  (i32.add
                    (local.get $ptr)
                    (i32.add (local.get $offset) (i32.const 1))))
                (i32.const 8)))
            (i32.or
              (i32.shl
                (i32.load8_u
                  (i32.add
                    (local.get $ptr)
                    (i32.add (local.get $offset) (i32.const 2))))
                (i32.const 16))
              (i32.shl
                (i32.load8_u
                  (i32.add
                    (local.get $ptr)
                    (i32.add (local.get $offset) (i32.const 3))))
                (i32.const 24))))))
      (else (call $pack (i32.const 1) (i32.const 0)))))

  (func (export "varint_encode_u64")
    (param $value_low i32) (param $value_high i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $value i64)
    (local $next i64)
    (local $written i32)
    (local $byte i32)
    (local.set $value
      (i64.or
        (i64.extend_i32_u (local.get $value_low))
        (i64.shl (i64.extend_i32_u (local.get $value_high)) (i64.const 32))))
    (loop $again
      (if (i32.ge_u (local.get $written) (local.get $out_cap))
        (then (return (call $pack (i32.const 2) (local.get $written)))))
      (local.set $next (i64.shr_u (local.get $value) (i64.const 7)))
      (local.set $byte (i32.and (i32.wrap_i64 (local.get $value)) (i32.const 127)))
      (if (i64.ne (local.get $next) (i64.const 0))
        (then (local.set $byte (i32.or (local.get $byte) (i32.const 128)))))
      (i32.store8
        (i32.add (local.get $out_ptr) (local.get $written))
        (local.get $byte))
      (local.set $written (i32.add (local.get $written) (i32.const 1)))
      (local.set $value (local.get $next))
      (br_if $again (i64.ne (local.get $value) (i64.const 0))))
    (call $pack (i32.const 0) (local.get $written)))

  (func (export "varint_decode_u64")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32)
    (result i64)
    (local $i i32)
    (local $shift i32)
    (local $b i32)
    (local $result i64)
    (if (i32.eqz (local.get $in_len))
      (then (return (call $pack (i32.const 1) (i32.const 0)))))
    (loop $again
      (if (i32.ge_u (local.get $i) (local.get $in_len))
        (then (return (call $pack (i32.const 5) (local.get $i)))))
      (if (i32.ge_u (local.get $i) (i32.const 10))
        (then (return (call $pack (i32.const 6) (local.get $i)))))
      (local.set $b (i32.load8_u (i32.add (local.get $in_ptr) (local.get $i))))
      (if
        (i32.and
          (i32.eq (local.get $shift) (i32.const 63))
          (i32.ne (i32.and (local.get $b) (i32.const 126)) (i32.const 0)))
        (then (return (call $pack (i32.const 4) (local.get $i)))))
      (local.set $result
        (i64.or
          (local.get $result)
          (i64.shl
            (i64.extend_i32_u (i32.and (local.get $b) (i32.const 127)))
            (i64.extend_i32_u (local.get $shift)))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (if (i32.eqz (i32.and (local.get $b) (i32.const 128)))
        (then
          (i64.store (local.get $out_ptr) (local.get $result))
          (return (call $pack (i32.const 0) (local.get $i)))))
      (if (i32.eq (local.get $i) (i32.const 10))
        (then (return (call $pack (i32.const 6) (local.get $i)))))
      (local.set $shift (i32.add (local.get $shift) (i32.const 7)))
      (br $again))
    (call $pack (i32.const 5) (local.get $i)))

  (func (export "quic_varint_encode_u64")
    (param $value_low i32) (param $value_high i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $value i64)
    (local.set $value
      (i64.or
        (i64.extend_i32_u (local.get $value_low))
        (i64.shl (i64.extend_i32_u (local.get $value_high)) (i64.const 32))))
    (if (i64.gt_u (local.get $value) (i64.const 4611686018427387903))
      (then (return (call $pack (i32.const 4) (i32.const 0)))))
    (if (i64.le_u (local.get $value) (i64.const 63))
      (then
        (if (i32.lt_u (local.get $out_cap) (i32.const 1))
          (then (return (call $pack (i32.const 2) (i32.const 0)))))
        (i32.store8 (local.get $out_ptr) (i32.wrap_i64 (local.get $value)))
        (return (call $pack (i32.const 0) (i32.const 1)))))
    (if (i64.le_u (local.get $value) (i64.const 16383))
      (then
        (if (i32.lt_u (local.get $out_cap) (i32.const 2))
          (then (return (call $pack (i32.const 2) (i32.const 0)))))
        (i32.store8
          (local.get $out_ptr)
          (i32.or
            (i32.const 64)
            (i32.and
              (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 8)))
              (i32.const 63))))
        (i32.store8
          (i32.add (local.get $out_ptr) (i32.const 1))
          (i32.wrap_i64 (local.get $value)))
        (return (call $pack (i32.const 0) (i32.const 2)))))
    (if (i64.le_u (local.get $value) (i64.const 1073741823))
      (then
        (if (i32.lt_u (local.get $out_cap) (i32.const 4))
          (then (return (call $pack (i32.const 2) (i32.const 0)))))
        (i32.store8
          (local.get $out_ptr)
          (i32.or
            (i32.const 128)
            (i32.and
              (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 24)))
              (i32.const 63))))
        (i32.store8
          (i32.add (local.get $out_ptr) (i32.const 1))
          (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 16))))
        (i32.store8
          (i32.add (local.get $out_ptr) (i32.const 2))
          (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 8))))
        (i32.store8
          (i32.add (local.get $out_ptr) (i32.const 3))
          (i32.wrap_i64 (local.get $value)))
        (return (call $pack (i32.const 0) (i32.const 4)))))
    (if (i32.lt_u (local.get $out_cap) (i32.const 8))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))
    (i32.store8
      (local.get $out_ptr)
      (i32.or
        (i32.const 192)
        (i32.and
          (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 56)))
          (i32.const 63))))
    (i32.store8
      (i32.add (local.get $out_ptr) (i32.const 1))
      (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 48))))
    (i32.store8
      (i32.add (local.get $out_ptr) (i32.const 2))
      (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 40))))
    (i32.store8
      (i32.add (local.get $out_ptr) (i32.const 3))
      (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 32))))
    (i32.store8
      (i32.add (local.get $out_ptr) (i32.const 4))
      (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 24))))
    (i32.store8
      (i32.add (local.get $out_ptr) (i32.const 5))
      (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 16))))
    (i32.store8
      (i32.add (local.get $out_ptr) (i32.const 6))
      (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 8))))
    (i32.store8
      (i32.add (local.get $out_ptr) (i32.const 7))
      (i32.wrap_i64 (local.get $value)))
    (call $pack (i32.const 0) (i32.const 8)))

  (func (export "quic_varint_decode_u64")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32)
    (result i64)
    (local $first i32)
    (local $need i32)
    (local $i i32)
    (local $value i64)
    (if (i32.eqz (local.get $in_len))
      (then (return (call $pack (i32.const 1) (i32.const 0)))))
    (local.set $first (i32.load8_u (local.get $in_ptr)))
    (local.set $need
      (i32.shl
        (i32.const 1)
        (i32.shr_u (local.get $first) (i32.const 6))))
    (if (i32.lt_u (local.get $in_len) (local.get $need))
      (then (return (call $pack (i32.const 5) (i32.const 0)))))
    (local.set $value (i64.extend_i32_u (i32.and (local.get $first) (i32.const 63))))
    (local.set $i (i32.const 1))
    (loop $again
      (if (i32.lt_u (local.get $i) (local.get $need))
        (then
          (local.set $value
            (i64.or
              (i64.shl (local.get $value) (i64.const 8))
              (i64.extend_i32_u
                (i32.load8_u
                  (i32.add (local.get $in_ptr) (local.get $i))))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $again))))
    (i64.store (local.get $out_ptr) (local.get $value))
    (call $pack (i32.const 0) (local.get $need)))

  (func (export "crc32") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $j i32)
    (local $crc i32)
    (local.set $crc (i32.const -1))
    (loop $bytes
      (if (i32.lt_u (local.get $i) (local.get $len))
        (then
          (local.set $crc
            (i32.xor
              (local.get $crc)
              (i32.load8_u (i32.add (local.get $ptr) (local.get $i)))))
          (local.set $j (i32.const 0))
          (loop $bits
            (if (i32.lt_u (local.get $j) (i32.const 8))
              (then
                (if (i32.and (local.get $crc) (i32.const 1))
                  (then
                    (local.set $crc
                      (i32.xor
                        (i32.shr_u (local.get $crc) (i32.const 1))
                        (i32.const 0xedb88320))))
                  (else
                    (local.set $crc
                      (i32.shr_u (local.get $crc) (i32.const 1)))))
                (local.set $j (i32.add (local.get $j) (i32.const 1)))
                (br $bits))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $bytes))))
    (i32.xor (local.get $crc) (i32.const -1)))

  (func $adler32_update (export "adler32_update") (param $initial i32) (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $a i32)
    (local $b i32)
    (local.set $a (i32.and (local.get $initial) (i32.const 65535)))
    (local.set $b
      (i32.and
        (i32.shr_u (local.get $initial) (i32.const 16))
        (i32.const 65535)))
    (loop $bytes
      (if (i32.lt_u (local.get $i) (local.get $len))
        (then
          (local.set $a
            (i32.rem_u
              (i32.add
                (local.get $a)
                (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
              (i32.const 65521)))
          (local.set $b
            (i32.rem_u
              (i32.add (local.get $b) (local.get $a))
              (i32.const 65521)))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $bytes))))
    (i32.or
      (i32.shl (local.get $b) (i32.const 16))
      (local.get $a)))

  (func (export "adler32") (param $ptr i32) (param $len i32) (result i32)
    (call $adler32_update (i32.const 1) (local.get $ptr) (local.get $len)))
)
