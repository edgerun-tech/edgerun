(func (export "read_u16_le") (param $ptr i32) (param $len i32) (param $offset i32) (result i64)
    (if (result i64)
      (call $bounds_check (local.get $len) (local.get $offset) (i32.const 2))
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
      (call $bounds_check (local.get $len) (local.get $offset) (i32.const 4))
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

  (func $crc32 (export "crc32") (param $ptr i32) (param $len i32) (result i32)
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

  

  (func $adler32 (export "adler32") (param $ptr i32) (param $len i32) (result i32)
    (call $adler32_update (i32.const 1) (local.get $ptr) (local.get $len)))

  (func (export "crc32_unrolled") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $j i32)
    (local $k i32)
    (local $crc i32)
    (local $word i32)
    (local.set $crc (i32.const -1))
    (loop $loop
      (if (i32.lt_u (local.get $i) (local.get $len))
        (then
          (if (i32.le_u (i32.add (local.get $i) (i32.const 4)) (local.get $len))
            (then
              (local.set $word
                (i32.load (i32.add (local.get $ptr) (local.get $i))))
              (local.set $k (i32.const 0))
              (loop $quad
                (if (i32.lt_u (local.get $k) (i32.const 4))
                  (then
                    (local.set $crc
                      (i32.xor
                        (local.get $crc)
                        (i32.and (local.get $word) (i32.const 0xff))))
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
                    (local.set $word
                      (i32.shr_u (local.get $word) (i32.const 8)))
                    (local.set $k (i32.add (local.get $k) (i32.const 1)))
                    (br $quad))))
              (local.set $i (i32.add (local.get $i) (i32.const 4)))
              (br $loop))
            (else
              (local.set $crc
                (i32.xor
                  (local.get $crc)
                  (i32.load8_u (i32.add (local.get $ptr) (local.get $i)))))
              (local.set $j (i32.const 0))
              (loop $bits_tail
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
                    (br $bits_tail))))
              (local.set $i (i32.add (local.get $i) (i32.const 1)))
              (br $loop))))))
    (i32.xor (local.get $crc) (i32.const -1)))

  (func (export "adler32_vec") (param $ptr i32) (param $len i32) (result i32)
    (call $adler32_update_vec (i32.const 1) (local.get $ptr) (local.get $len)))

  

  ;; ── Byte-at-a-time CRC-32 update ──
  ;; crc32_update_byte(crc, byte) → crc
  

  ;; ── Byte-at-a-time Adler-32 update ──
  ;; adler32_update_byte(adler, byte) → adler
  
;; deflate-inflate — raw deflate scan & inflate

  ;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid_data,
  ;; 6 limit_exceeded, 7 unsupported.
  ;; Record, little-endian u32:
  ;; 0: consumed
  ;; 4: written or scanned stored payload total
  ;; 8: block_count
  ;; 12: last_block_type
  ;; 16: crc32 over emitted bytes
  ;; 20: adler32 over emitted bytes
  (global $br_src_ptr (mut i32) (i32.const 0))
  (global $br_src_len (mut i32) (i32.const 0))
  (global $br_bitpos (mut i32) (i32.const 0))
  (global $br_bitbuf (mut i32) (i32.const 0))
  (global $br_bitcnt (mut i32) (i32.const 0))

  (func $br_init (param $src_ptr i32) (param $src_len i32)
    (global.set $br_src_ptr (local.get $src_ptr))
    (global.set $br_src_len (local.get $src_len))
    (global.set $br_bitpos (i32.const 0))
    (global.set $br_bitbuf (i32.const 0))
    (global.set $br_bitcnt (i32.const 0)))

  (func $br_read (param $n i32) (result i32)
    (local $byte_off i32)
    (local $byte i32)
    (local $mask i32)
    (local $bits i32)
    (block $ready
      (loop $fill
        (br_if $ready (i32.ge_u (global.get $br_bitcnt) (local.get $n)))
        (local.set $byte_off (i32.shr_u (global.get $br_bitpos) (i32.const 3)))
        (if (i32.ge_u (local.get $byte_off) (global.get $br_src_len))
          (then (return (i32.const -1))))
        (local.set $byte
          (i32.load8_u (i32.add (global.get $br_src_ptr) (local.get $byte_off))))
        (global.set $br_bitbuf
          (i32.or
            (global.get $br_bitbuf)
            (i32.shl (local.get $byte) (global.get $br_bitcnt))))
        (global.set $br_bitcnt (i32.add (global.get $br_bitcnt) (i32.const 8)))
        (global.set $br_bitpos (i32.add (global.get $br_bitpos) (i32.const 8)))
        (br $fill)))
    (local.set $mask (i32.sub (i32.shl (i32.const 1) (local.get $n)) (i32.const 1)))
    (local.set $bits (i32.and (global.get $br_bitbuf) (local.get $mask)))
    (global.set $br_bitbuf (i32.shr_u (global.get $br_bitbuf) (local.get $n)))
    (global.set $br_bitcnt (i32.sub (global.get $br_bitcnt) (local.get $n)))
    (local.get $bits))

  (func $br_align_byte
    (local $used_bits i32)
    (local.set $used_bits (i32.sub (global.get $br_bitpos) (global.get $br_bitcnt)))
    (global.set $br_bitpos
      (i32.and
        (i32.add (local.get $used_bits) (i32.const 7))
        (i32.const -8)))
    (global.set $br_bitbuf (i32.const 0))
    (global.set $br_bitcnt (i32.const 0)))

  (func $br_consumed_bytes (result i32)
    (local $used_bits i32)
    (local.set $used_bits (i32.sub (global.get $br_bitpos) (global.get $br_bitcnt)))
    (i32.shr_u (i32.add (local.get $used_bits) (i32.const 7)) (i32.const 3)))

  (func $reverse_bits (param $v i32) (param $n i32) (result i32)
    (local $r i32)
    (loop $bits
      (if (i32.eqz (local.get $n))
        (then (return (local.get $r))))
      (local.set $r
        (i32.or
          (i32.shl (local.get $r) (i32.const 1))
          (i32.and (local.get $v) (i32.const 1))))
      (local.set $v (i32.shr_u (local.get $v) (i32.const 1)))
      (local.set $n (i32.sub (local.get $n) (i32.const 1)))
      (br $bits))
    (local.get $r))

  (func $fixed_symbol (result i32)
    (local $bits i32)
    (local $next i32)
    (local $rev i32)
    (local.set $bits (call $br_read (i32.const 7)))
    (if (i32.lt_s (local.get $bits) (i32.const 0))
      (then (return (i32.const -1))))
    (local.set $rev (call $reverse_bits (local.get $bits) (i32.const 7)))
    (if (i32.le_u (local.get $rev) (i32.const 23))
      (then (return
        (i32.or (i32.shl (i32.const 7) (i32.const 16))
                (i32.add (i32.const 256) (local.get $rev))))))

    (local.set $next (call $br_read (i32.const 1)))
    (if (i32.lt_s (local.get $next) (i32.const 0))
      (then (return (i32.const -1))))
    (local.set $bits (i32.or (local.get $bits) (i32.shl (local.get $next) (i32.const 7))))
    (local.set $rev (call $reverse_bits (local.get $bits) (i32.const 8)))
    (if (i32.and
          (i32.ge_u (local.get $rev) (i32.const 48))
          (i32.le_u (local.get $rev) (i32.const 191)))
      (then (return
        (i32.or (i32.shl (i32.const 8) (i32.const 16))
                (i32.sub (local.get $rev) (i32.const 48))))))
    (if (i32.and
          (i32.ge_u (local.get $rev) (i32.const 192))
          (i32.le_u (local.get $rev) (i32.const 199)))
      (then (return
        (i32.or (i32.shl (i32.const 8) (i32.const 16))
                (i32.add (i32.const 280) (i32.sub (local.get $rev) (i32.const 192)))))))

    (local.set $next (call $br_read (i32.const 1)))
    (if (i32.lt_s (local.get $next) (i32.const 0))
      (then (return (i32.const -1))))
    (local.set $bits (i32.or (local.get $bits) (i32.shl (local.get $next) (i32.const 8))))
    (local.set $rev (call $reverse_bits (local.get $bits) (i32.const 9)))
    (if (i32.and
          (i32.ge_u (local.get $rev) (i32.const 400))
          (i32.le_u (local.get $rev) (i32.const 511)))
      (then (return
        (i32.or (i32.shl (i32.const 9) (i32.const 16))
                (i32.add (i32.const 144) (i32.sub (local.get $rev) (i32.const 400)))))))
    (i32.const -3))

  (func $length_base (param $sym i32) (result i32)
    (if (i32.le_u (local.get $sym) (i32.const 264))
      (then (return (i32.add (i32.const 3) (i32.sub (local.get $sym) (i32.const 257))))))
    (if (i32.le_u (local.get $sym) (i32.const 268))
      (then (return (i32.add (i32.const 11) (i32.shl (i32.sub (local.get $sym) (i32.const 265)) (i32.const 1))))))
    (if (i32.le_u (local.get $sym) (i32.const 272))
      (then (return (i32.add (i32.const 19) (i32.shl (i32.sub (local.get $sym) (i32.const 269)) (i32.const 2))))))
    (if (i32.le_u (local.get $sym) (i32.const 276))
      (then (return (i32.add (i32.const 35) (i32.shl (i32.sub (local.get $sym) (i32.const 273)) (i32.const 3))))))
    (if (i32.le_u (local.get $sym) (i32.const 280))
      (then (return (i32.add (i32.const 67) (i32.shl (i32.sub (local.get $sym) (i32.const 277)) (i32.const 4))))))
    (if (i32.le_u (local.get $sym) (i32.const 284))
      (then (return (i32.add (i32.const 131) (i32.shl (i32.sub (local.get $sym) (i32.const 281)) (i32.const 5))))))
    (if (i32.eq (local.get $sym) (i32.const 285))
      (then (return (i32.const 258))))
    (i32.const -1))

  (func $length_extra (param $sym i32) (result i32)
    (if (i32.le_u (local.get $sym) (i32.const 264)) (then (return (i32.const 0))))
    (if (i32.le_u (local.get $sym) (i32.const 268)) (then (return (i32.const 1))))
    (if (i32.le_u (local.get $sym) (i32.const 272)) (then (return (i32.const 2))))
    (if (i32.le_u (local.get $sym) (i32.const 276)) (then (return (i32.const 3))))
    (if (i32.le_u (local.get $sym) (i32.const 280)) (then (return (i32.const 4))))
    (if (i32.le_u (local.get $sym) (i32.const 284)) (then (return (i32.const 5))))
    (if (i32.eq (local.get $sym) (i32.const 285)) (then (return (i32.const 0))))
    (i32.const -1))

  (func $dist_base (param $sym i32) (result i32)
    (local $extra i32)
    (local $base i32)
    (if (i32.gt_u (local.get $sym) (i32.const 29))
      (then (return (i32.const -1))))
    (if (i32.lt_u (local.get $sym) (i32.const 4))
      (then (return (i32.add (local.get $sym) (i32.const 1)))))
    (local.set $extra (i32.sub (i32.shr_u (local.get $sym) (i32.const 1)) (i32.const 1)))
    (local.set $base (i32.add (i32.shl (i32.const 1) (i32.add (local.get $extra) (i32.const 1))) (i32.const 1)))
    (if (i32.and (local.get $sym) (i32.const 1))
      (then (local.set $base (i32.add (local.get $base) (i32.shl (i32.const 1) (local.get $extra))))))
    (local.get $base))

  (func $dist_extra (param $sym i32) (result i32)
    (if (i32.gt_u (local.get $sym) (i32.const 29))
      (then (return (i32.const -1))))
    (if (i32.lt_u (local.get $sym) (i32.const 4))
      (then (return (i32.const 0))))
    (i32.sub (i32.shr_u (local.get $sym) (i32.const 1)) (i32.const 1)))

  (func $cl_order (param $idx i32) (result i32)
    (if (i32.eq (local.get $idx) (i32.const 0)) (then (return (i32.const 16))))
    (if (i32.eq (local.get $idx) (i32.const 1)) (then (return (i32.const 17))))
    (if (i32.eq (local.get $idx) (i32.const 2)) (then (return (i32.const 18))))
    (if (i32.eq (local.get $idx) (i32.const 3)) (then (return (i32.const 0))))
    (if (i32.eq (local.get $idx) (i32.const 4)) (then (return (i32.const 8))))
    (if (i32.eq (local.get $idx) (i32.const 5)) (then (return (i32.const 7))))
    (if (i32.eq (local.get $idx) (i32.const 6)) (then (return (i32.const 9))))
    (if (i32.eq (local.get $idx) (i32.const 7)) (then (return (i32.const 6))))
    (if (i32.eq (local.get $idx) (i32.const 8)) (then (return (i32.const 10))))
    (if (i32.eq (local.get $idx) (i32.const 9)) (then (return (i32.const 5))))
    (if (i32.eq (local.get $idx) (i32.const 10)) (then (return (i32.const 11))))
    (if (i32.eq (local.get $idx) (i32.const 11)) (then (return (i32.const 4))))
    (if (i32.eq (local.get $idx) (i32.const 12)) (then (return (i32.const 12))))
    (if (i32.eq (local.get $idx) (i32.const 13)) (then (return (i32.const 3))))
    (if (i32.eq (local.get $idx) (i32.const 14)) (then (return (i32.const 13))))
    (if (i32.eq (local.get $idx) (i32.const 15)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $idx) (i32.const 16)) (then (return (i32.const 14))))
    (if (i32.eq (local.get $idx) (i32.const 17)) (then (return (i32.const 1))))
    (i32.const 15))

  (func $zero_u32_table (param $ptr i32) (param $count i32)
    (local $i i32)
    (loop $zero
      (if (i32.lt_u (local.get $i) (local.get $count))
        (then
          (i32.store (i32.add (local.get $ptr) (i32.shl (local.get $i) (i32.const 2))) (i32.const 0))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $zero)))))

  (func $build_table
    (param $lens_ptr i32)
    (param $symbol_count i32)
    (param $symbols_ptr i32)
    (param $counts_ptr i32)
    (param $offsets_ptr i32)
    (param $maxbits i32)
    (result i32)
    (local $i i32)
    (local $len i32)
    (local $total i32)
    (local $left i32)
    (local $sum i32)
    (local $pos i32)
    (call $zero_u32_table (local.get $counts_ptr) (i32.const 16))
    (call $zero_u32_table (local.get $offsets_ptr) (i32.const 16))
    (local.set $i (i32.const 0))
    (loop $count_lens
      (if (i32.lt_u (local.get $i) (local.get $symbol_count))
        (then
          (local.set $len (i32.load8_u (i32.add (local.get $lens_ptr) (local.get $i))))
          (if (i32.gt_u (local.get $len) (local.get $maxbits)) (then (return (i32.const 3))))
          (if (local.get $len)
            (then
              (i32.store
                (i32.add (local.get $counts_ptr) (i32.shl (local.get $len) (i32.const 2)))
                (i32.add
                  (i32.load (i32.add (local.get $counts_ptr) (i32.shl (local.get $len) (i32.const 2))))
                  (i32.const 1)))
              (local.set $total (i32.add (local.get $total) (i32.const 1)))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $count_lens))))
    (if (i32.eqz (local.get $total)) (then (return (i32.const 3))))
    (local.set $left (i32.const 1))
    (local.set $i (i32.const 1))
    (loop $check_left
      (if (i32.le_u (local.get $i) (local.get $maxbits))
        (then
          (local.set $left
            (i32.sub
              (i32.shl (local.get $left) (i32.const 1))
              (i32.load (i32.add (local.get $counts_ptr) (i32.shl (local.get $i) (i32.const 2))))))
          (if (i32.lt_s (local.get $left) (i32.const 0)) (then (return (i32.const 3))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $check_left))))
    (local.set $sum (i32.const 0))
    (local.set $i (i32.const 1))
    (loop $make_offsets
      (if (i32.le_u (local.get $i) (local.get $maxbits))
        (then
          (i32.store (i32.add (local.get $offsets_ptr) (i32.shl (local.get $i) (i32.const 2))) (local.get $sum))
          (local.set $sum
            (i32.add
              (local.get $sum)
              (i32.load (i32.add (local.get $counts_ptr) (i32.shl (local.get $i) (i32.const 2))))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $make_offsets))))
    (local.set $i (i32.const 0))
    (loop $sort_symbols
      (if (i32.lt_u (local.get $i) (local.get $symbol_count))
        (then
          (local.set $len (i32.load8_u (i32.add (local.get $lens_ptr) (local.get $i))))
          (if (local.get $len)
            (then
              (local.set $pos (i32.load (i32.add (local.get $offsets_ptr) (i32.shl (local.get $len) (i32.const 2)))))
              (i32.store (i32.add (local.get $symbols_ptr) (i32.shl (local.get $pos) (i32.const 2))) (local.get $i))
              (i32.store
                (i32.add (local.get $offsets_ptr) (i32.shl (local.get $len) (i32.const 2)))
                (i32.add (local.get $pos) (i32.const 1)))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $sort_symbols))))
    (i32.const 0))

  (func $decode_symbol
    (param $symbols_ptr i32)
    (param $counts_ptr i32)
    (param $maxbits i32)
    (result i32)
    (local $code i32)
    (local $first i32)
    (local $index i32)
    (local $len i32)
    (local $bit i32)
    (local $count i32)
    (local.set $len (i32.const 1))
    (loop $decode
      (if (i32.le_u (local.get $len) (local.get $maxbits))
        (then
          (local.set $bit (call $br_read (i32.const 1)))
          (if (i32.lt_s (local.get $bit) (i32.const 0)) (then (return (i32.const -1))))
          (local.set $code (i32.or (local.get $code) (local.get $bit)))
          (local.set $count (i32.load (i32.add (local.get $counts_ptr) (i32.shl (local.get $len) (i32.const 2)))))
          (if (i32.lt_u (i32.sub (local.get $code) (local.get $first)) (local.get $count))
            (then
              (return
                (i32.load
                  (i32.add
                    (local.get $symbols_ptr)
                    (i32.shl
                      (i32.add (local.get $index) (i32.sub (local.get $code) (local.get $first)))
                      (i32.const 2)))))))
          (local.set $index (i32.add (local.get $index) (local.get $count)))
          (local.set $first (i32.shl (i32.add (local.get $first) (local.get $count)) (i32.const 1)))
          (local.set $code (i32.shl (local.get $code) (i32.const 1)))
          (local.set $len (i32.add (local.get $len) (i32.const 1)))
          (br $decode))))
    (i32.const -3))

  (func $m66write_record
    (param $out i32)
    (param $consumed i32)
    (param $written i32)
    (param $block_count i32)
    (param $last_block_type i32)
    (param $crc32 i32)
    (param $adler32 i32)
    (i32.store (local.get $out) (local.get $consumed))
    (i32.store (i32.add (local.get $out) (i32.const 4)) (local.get $written))
    (i32.store (i32.add (local.get $out) (i32.const 8)) (local.get $block_count))
    (i32.store (i32.add (local.get $out) (i32.const 12)) (local.get $last_block_type))
    (i32.store (i32.add (local.get $out) (i32.const 16)) (local.get $crc32))
    (i32.store (i32.add (local.get $out) (i32.const 20)) (local.get $adler32)))

  (func (export "deflate_scan_blocks")
    (param $src_ptr i32)
    (param $src_len i32)
    (param $out_record i32)
    (result i32)
    (local $bfinal i32)
    (local $btype i32)
    (local $off i32)
    (local $block_len i32)
    (local $nlen i32)
    (local $payload_end i32)
    (local $block_count i32)
    (local $payload_total i32)
    (call $br_init (local.get $src_ptr) (local.get $src_len))
    (loop $blocks
      (local.set $bfinal (call $br_read (i32.const 1)))
      (if (i32.lt_s (local.get $bfinal) (i32.const 0)) (then (return (i32.const 1))))
      (local.set $btype (call $br_read (i32.const 2)))
      (if (i32.lt_s (local.get $btype) (i32.const 0)) (then (return (i32.const 1))))
      (if (i32.eq (local.get $btype) (i32.const 3)) (then (return (i32.const 3))))
      (if (i32.ne (local.get $btype) (i32.const 0))
        (then
          (call $m66write_record
            (local.get $out_record)
            (call $br_consumed_bytes)
            (local.get $payload_total)
            (local.get $block_count)
            (local.get $btype)
            (i32.const 0)
            (i32.const 1))
          (return (i32.const 7))))
      (call $br_align_byte)
      (local.set $off (i32.shr_u (global.get $br_bitpos) (i32.const 3)))
      (if (i32.gt_u (i32.add (local.get $off) (i32.const 4)) (local.get $src_len))
        (then (return (i32.const 1))))
      (local.set $block_len
        (i32.or
          (i32.load8_u (i32.add (local.get $src_ptr) (local.get $off)))
          (i32.shl
            (i32.load8_u (i32.add (local.get $src_ptr) (i32.add (local.get $off) (i32.const 1))))
            (i32.const 8))))
      (local.set $nlen
        (i32.or
          (i32.load8_u (i32.add (local.get $src_ptr) (i32.add (local.get $off) (i32.const 2))))
          (i32.shl
            (i32.load8_u (i32.add (local.get $src_ptr) (i32.add (local.get $off) (i32.const 3))))
            (i32.const 8))))
      (if (i32.ne (i32.and (i32.xor (local.get $block_len) (local.get $nlen)) (i32.const 65535)) (i32.const 65535))
        (then (return (i32.const 3))))
      (local.set $payload_end (i32.add (i32.add (local.get $off) (i32.const 4)) (local.get $block_len)))
      (if (i32.or
            (i32.lt_u (local.get $payload_end) (local.get $off))
            (i32.gt_u (local.get $payload_end) (local.get $src_len)))
        (then (return (i32.const 1))))
      (local.set $block_count (i32.add (local.get $block_count) (i32.const 1)))
      (local.set $payload_total (i32.add (local.get $payload_total) (local.get $block_len)))
      (global.set $br_bitpos (i32.shl (local.get $payload_end) (i32.const 3)))
      (global.set $br_bitbuf (i32.const 0))
      (global.set $br_bitcnt (i32.const 0))
      (if (local.get $bfinal)
        (then
          (call $m66write_record
            (local.get $out_record)
            (local.get $payload_end)
            (local.get $payload_total)
            (local.get $block_count)
            (i32.const 0)
            (i32.const 0)
            (i32.const 1))
          (return (i32.const 0))))
      (br $blocks))
    (i32.const 1))

  (func (export "deflate_inflate_raw")
    (param $src_ptr i32)
    (param $src_len i32)
    (param $out_ptr i32)
    (param $out_cap i32)
    (param $out_limit i32)
    (param $out_record i32)
    (result i32)
    (local $bfinal i32)
    (local $btype i32)
    (local $off i32)
    (local $block_len i32)
    (local $nlen i32)
    (local $payload_off i32)
    (local $payload_end i32)
    (local $written i32)
    (local $block_count i32)
    (local $crc i32)
    (local $adler i32)
    (local $i i32)
    (local $byte i32)
    (local $packed i32)
    (local $sym i32)
    (local $extra_bits i32)
    (local $extra_val i32)
    (local $length i32)
    (local $dist_sym i32)
    (local $dist i32)
    (local $copy_from i32)
    (local $hlit i32)
    (local $hdist i32)
    (local $hclen i32)
    (local $total_lens i32)
    (local $lens_index i32)
    (local $code i32)
    (local $repeat_count i32)
    (local $prev_len i32)
    (local.set $crc (i32.const -1))
    (local.set $adler (i32.const 1))
    (call $br_init (local.get $src_ptr) (local.get $src_len))
    (loop $blocks
      (local.set $bfinal (call $br_read (i32.const 1)))
      (if (i32.lt_s (local.get $bfinal) (i32.const 0)) (then (return (i32.const 1))))
      (local.set $btype (call $br_read (i32.const 2)))
      (if (i32.lt_s (local.get $btype) (i32.const 0)) (then (return (i32.const 1))))
      (if (i32.eq (local.get $btype) (i32.const 3)) (then (return (i32.const 3))))
      (if (i32.eq (local.get $btype) (i32.const 2))
        (then
          (local.set $hlit (call $br_read (i32.const 5)))
          (if (i32.lt_s (local.get $hlit) (i32.const 0)) (then (return (i32.const 1))))
          (local.set $hlit (i32.add (local.get $hlit) (i32.const 257)))
          (local.set $hdist (call $br_read (i32.const 5)))
          (if (i32.lt_s (local.get $hdist) (i32.const 0)) (then (return (i32.const 1))))
          (local.set $hdist (i32.add (local.get $hdist) (i32.const 1)))
          (local.set $hclen (call $br_read (i32.const 4)))
          (if (i32.lt_s (local.get $hclen) (i32.const 0)) (then (return (i32.const 1))))
          (local.set $hclen (i32.add (local.get $hclen) (i32.const 4)))
          (if (i32.or
                (i32.or (i32.gt_u (local.get $hlit) (i32.const 286)) (i32.gt_u (local.get $hdist) (i32.const 32)))
                (i32.gt_u (local.get $hclen) (i32.const 19)))
            (then (return (i32.const 3))))
          (local.set $i (i32.const 0))
          (loop $zero_cl_lens
            (if (i32.lt_u (local.get $i) (i32.const 19))
              (then
                (i32.store8 (i32.add (i32.const 591724) (local.get $i)) (i32.const 0))
                (local.set $i (i32.add (local.get $i) (i32.const 1)))
                (br $zero_cl_lens))))
          (local.set $i (i32.const 0))
          (loop $read_cl_lens
            (if (i32.lt_u (local.get $i) (local.get $hclen))
              (then
                (local.set $code (call $br_read (i32.const 3)))
                (if (i32.lt_s (local.get $code) (i32.const 0)) (then (return (i32.const 1))))
                (i32.store8
                  (i32.add (i32.const 591724) (call $cl_order (local.get $i)))
                  (local.get $code))
                (local.set $i (i32.add (local.get $i) (i32.const 1)))
                (br $read_cl_lens))))
          (if (call $build_table
                (i32.const 591724)
                (i32.const 19)
                (i32.const 589952)
                (i32.const 589824)
                (i32.const 589888)
                (i32.const 7))
            (then (return (i32.const 3))))
          (local.set $total_lens (i32.add (local.get $hlit) (local.get $hdist)))
          (local.set $i (i32.const 0))
          (loop $zero_dyn_lens
            (if (i32.lt_u (local.get $i) (i32.const 320))
              (then
                (i32.store8 (i32.add (i32.const 591724) (local.get $i)) (i32.const 0))
                (local.set $i (i32.add (local.get $i) (i32.const 1)))
                (br $zero_dyn_lens))))
          (local.set $lens_index (i32.const 0))
          (local.set $prev_len (i32.const 0))
          (loop $read_dyn_lens
            (if (i32.lt_u (local.get $lens_index) (local.get $total_lens))
              (then
                (local.set $code (call $decode_symbol (i32.const 589952) (i32.const 589824) (i32.const 7)))
                (if (i32.lt_s (local.get $code) (i32.const 0))
                  (then
                    (if (i32.eq (local.get $code) (i32.const -1))
                      (then (return (i32.const 1)))
                      (else (return (i32.const 3))))))
                (if (i32.le_u (local.get $code) (i32.const 15))
                  (then
                    (i32.store8 (i32.add (i32.const 591724) (local.get $lens_index)) (local.get $code))
                    (local.set $prev_len (local.get $code))
                    (local.set $lens_index (i32.add (local.get $lens_index) (i32.const 1)))
                    (br $read_dyn_lens)))
                (if (i32.eq (local.get $code) (i32.const 16))
                  (then
                    (if (i32.eqz (local.get $lens_index)) (then (return (i32.const 3))))
                    (local.set $repeat_count (call $br_read (i32.const 2)))
                    (if (i32.lt_s (local.get $repeat_count) (i32.const 0)) (then (return (i32.const 1))))
                    (local.set $repeat_count (i32.add (local.get $repeat_count) (i32.const 3)))
                    (if (i32.gt_u (i32.add (local.get $lens_index) (local.get $repeat_count)) (local.get $total_lens))
                      (then (return (i32.const 3))))
                    (loop $repeat_prev
                      (if (local.get $repeat_count)
                        (then
                          (i32.store8 (i32.add (i32.const 591724) (local.get $lens_index)) (local.get $prev_len))
                          (local.set $lens_index (i32.add (local.get $lens_index) (i32.const 1)))
                          (local.set $repeat_count (i32.sub (local.get $repeat_count) (i32.const 1)))
                          (br $repeat_prev))))
                    (br $read_dyn_lens)))
                (if (i32.eq (local.get $code) (i32.const 17))
                  (then
                    (local.set $repeat_count (call $br_read (i32.const 3)))
                    (if (i32.lt_s (local.get $repeat_count) (i32.const 0)) (then (return (i32.const 1))))
                    (local.set $repeat_count (i32.add (local.get $repeat_count) (i32.const 3)))
                    (if (i32.gt_u (i32.add (local.get $lens_index) (local.get $repeat_count)) (local.get $total_lens))
                      (then (return (i32.const 3))))
                    (local.set $prev_len (i32.const 0))
                    (local.set $lens_index (i32.add (local.get $lens_index) (local.get $repeat_count)))
                    (br $read_dyn_lens)))
                (if (i32.eq (local.get $code) (i32.const 18))
                  (then
                    (local.set $repeat_count (call $br_read (i32.const 7)))
                    (if (i32.lt_s (local.get $repeat_count) (i32.const 0)) (then (return (i32.const 1))))
                    (local.set $repeat_count (i32.add (local.get $repeat_count) (i32.const 11)))
                    (if (i32.gt_u (i32.add (local.get $lens_index) (local.get $repeat_count)) (local.get $total_lens))
                      (then (return (i32.const 3))))
                    (local.set $prev_len (i32.const 0))
                    (local.set $lens_index (i32.add (local.get $lens_index) (local.get $repeat_count)))
                    (br $read_dyn_lens)))
                (return (i32.const 3)))))
          (if (i32.eqz (i32.load8_u (i32.add (i32.const 591724) (i32.const 256))))
            (then (return (i32.const 3))))
          (if (call $build_table
                (i32.const 591724)
                (local.get $hlit)
                (i32.const 590208)
                (i32.const 590080)
                (i32.const 590144)
                (i32.const 15))
            (then (return (i32.const 3))))
          (if (call $build_table
                (i32.add (i32.const 591724) (local.get $hlit))
                (local.get $hdist)
                (i32.const 591552)
                (i32.const 591424)
                (i32.const 591488)
                (i32.const 15))
            (then (return (i32.const 3))))
          (loop $dynamic
            (local.set $sym (call $decode_symbol (i32.const 590208) (i32.const 590080) (i32.const 15)))
            (if (i32.lt_s (local.get $sym) (i32.const 0))
              (then
                (if (i32.eq (local.get $sym) (i32.const -1))
                  (then (return (i32.const 1)))
                  (else (return (i32.const 3))))))
            (if (i32.lt_u (local.get $sym) (i32.const 256))
              (then
                (if (i32.ge_u (local.get $written) (local.get $out_cap)) (then (return (i32.const 2))))
                (if (i32.ge_u (local.get $written) (local.get $out_limit)) (then (return (i32.const 6))))
                (i32.store8 (i32.add (local.get $out_ptr) (local.get $written)) (local.get $sym))
                (local.set $crc (call $crc32_update_byte (local.get $crc) (local.get $sym)))
                (local.set $adler (call $adler32_update_byte (local.get $adler) (local.get $sym)))
                (local.set $written (i32.add (local.get $written) (i32.const 1)))
                (br $dynamic)))
            (if (i32.eq (local.get $sym) (i32.const 256))
              (then
                (local.set $block_count (i32.add (local.get $block_count) (i32.const 1)))
                (if (local.get $bfinal)
                  (then
                    (call $m66write_record
                      (local.get $out_record)
                      (call $br_consumed_bytes)
                      (local.get $written)
                      (local.get $block_count)
                      (i32.const 2)
                      (i32.xor (local.get $crc) (i32.const -1))
                      (local.get $adler))
                    (return (i32.const 0))))
                (br $blocks)))
            (if (i32.gt_u (local.get $sym) (i32.const 285)) (then (return (i32.const 3))))
            (local.set $length (call $length_base (local.get $sym)))
            (local.set $extra_bits (call $length_extra (local.get $sym)))
            (if (i32.lt_s (local.get $length) (i32.const 0)) (then (return (i32.const 3))))
            (if (local.get $extra_bits)
              (then
                (local.set $extra_val (call $br_read (local.get $extra_bits)))
                (if (i32.lt_s (local.get $extra_val) (i32.const 0)) (then (return (i32.const 1))))
                (local.set $length (i32.add (local.get $length) (local.get $extra_val)))))
            (local.set $dist_sym (call $decode_symbol (i32.const 591552) (i32.const 591424) (i32.const 15)))
            (if (i32.lt_s (local.get $dist_sym) (i32.const 0))
              (then
                (if (i32.eq (local.get $dist_sym) (i32.const -1))
                  (then (return (i32.const 1)))
                  (else (return (i32.const 3))))))
            (if (i32.gt_u (local.get $dist_sym) (i32.const 29)) (then (return (i32.const 3))))
            (local.set $dist (call $dist_base (local.get $dist_sym)))
            (local.set $extra_bits (call $dist_extra (local.get $dist_sym)))
            (if (local.get $extra_bits)
              (then
                (local.set $extra_val (call $br_read (local.get $extra_bits)))
                (if (i32.lt_s (local.get $extra_val) (i32.const 0)) (then (return (i32.const 1))))
                (local.set $dist (i32.add (local.get $dist) (local.get $extra_val)))))
            (if (i32.or
                  (i32.gt_u (local.get $dist) (local.get $written))
                  (i32.gt_u (local.get $dist) (i32.const 32768)))
              (then (return (i32.const 3))))
            (if (i32.gt_u (i32.add (local.get $written) (local.get $length)) (local.get $out_cap)) (then (return (i32.const 2))))
            (if (i32.gt_u (i32.add (local.get $written) (local.get $length)) (local.get $out_limit)) (then (return (i32.const 6))))
            (local.set $i (i32.const 0))
            (loop $copy_dyn_match
              (if (i32.lt_u (local.get $i) (local.get $length))
                (then
                  (local.set $copy_from
                    (i32.sub
                      (i32.add (local.get $written) (local.get $i))
                      (local.get $dist)))
                  (local.set $byte (i32.load8_u (i32.add (local.get $out_ptr) (local.get $copy_from))))
                  (i32.store8
                    (i32.add (local.get $out_ptr) (i32.add (local.get $written) (local.get $i)))
                    (local.get $byte))
                  (local.set $crc (call $crc32_update_byte (local.get $crc) (local.get $byte)))
                  (local.set $adler (call $adler32_update_byte (local.get $adler) (local.get $byte)))
                  (local.set $i (i32.add (local.get $i) (i32.const 1)))
                  (br $copy_dyn_match))))
            (local.set $written (i32.add (local.get $written) (local.get $length)))
            (br $dynamic))))
      (if (i32.eq (local.get $btype) (i32.const 1))
        (then
          (loop $fixed
            (local.set $packed (call $fixed_symbol))
            (if (i32.lt_s (local.get $packed) (i32.const 0))
              (then
                (if (i32.eq (local.get $packed) (i32.const -1))
                  (then (return (i32.const 1)))
                  (else (return (i32.const 3))))))
            (local.set $sym (i32.and (local.get $packed) (i32.const 65535)))
            (if (i32.lt_u (local.get $sym) (i32.const 256))
              (then
                (if (i32.ge_u (local.get $written) (local.get $out_cap)) (then (return (i32.const 2))))
                (if (i32.ge_u (local.get $written) (local.get $out_limit)) (then (return (i32.const 6))))
                (i32.store8 (i32.add (local.get $out_ptr) (local.get $written)) (local.get $sym))
                (local.set $crc (call $crc32_update_byte (local.get $crc) (local.get $sym)))
                (local.set $adler (call $adler32_update_byte (local.get $adler) (local.get $sym)))
                (local.set $written (i32.add (local.get $written) (i32.const 1)))
                (br $fixed)))
            (if (i32.eq (local.get $sym) (i32.const 256))
              (then
                (local.set $block_count (i32.add (local.get $block_count) (i32.const 1)))
                (if (local.get $bfinal)
                  (then
                    (call $m66write_record
                      (local.get $out_record)
                      (call $br_consumed_bytes)
                      (local.get $written)
                      (local.get $block_count)
                      (i32.const 1)
                      (i32.xor (local.get $crc) (i32.const -1))
                      (local.get $adler))
                    (return (i32.const 0))))
                (br $blocks)))
            (if (i32.gt_u (local.get $sym) (i32.const 285)) (then (return (i32.const 3))))
            (local.set $length (call $length_base (local.get $sym)))
            (local.set $extra_bits (call $length_extra (local.get $sym)))
            (if (i32.lt_s (local.get $length) (i32.const 0)) (then (return (i32.const 3))))
            (if (local.get $extra_bits)
              (then
                (local.set $extra_val (call $br_read (local.get $extra_bits)))
                (if (i32.lt_s (local.get $extra_val) (i32.const 0)) (then (return (i32.const 1))))
                (local.set $length (i32.add (local.get $length) (local.get $extra_val)))))
            (local.set $dist_sym (call $br_read (i32.const 5)))
            (if (i32.lt_s (local.get $dist_sym) (i32.const 0)) (then (return (i32.const 1))))
            (local.set $dist_sym (call $reverse_bits (local.get $dist_sym) (i32.const 5)))
            (if (i32.gt_u (local.get $dist_sym) (i32.const 29)) (then (return (i32.const 3))))
            (local.set $dist (call $dist_base (local.get $dist_sym)))
            (local.set $extra_bits (call $dist_extra (local.get $dist_sym)))
            (if (local.get $extra_bits)
              (then
                (local.set $extra_val (call $br_read (local.get $extra_bits)))
                (if (i32.lt_s (local.get $extra_val) (i32.const 0)) (then (return (i32.const 1))))
                (local.set $dist (i32.add (local.get $dist) (local.get $extra_val)))))
            (if (i32.or
                  (i32.gt_u (local.get $dist) (local.get $written))
                  (i32.gt_u (local.get $dist) (i32.const 32768)))
              (then (return (i32.const 3))))
            (if (i32.gt_u (i32.add (local.get $written) (local.get $length)) (local.get $out_cap)) (then (return (i32.const 2))))
            (if (i32.gt_u (i32.add (local.get $written) (local.get $length)) (local.get $out_limit)) (then (return (i32.const 6))))
            (local.set $i (i32.const 0))
            (loop $copy_match
              (if (i32.lt_u (local.get $i) (local.get $length))
                (then
                  (local.set $copy_from
                    (i32.sub
                      (i32.add (local.get $written) (local.get $i))
                      (local.get $dist)))
                  (local.set $byte (i32.load8_u (i32.add (local.get $out_ptr) (local.get $copy_from))))
                  (i32.store8
                    (i32.add (local.get $out_ptr) (i32.add (local.get $written) (local.get $i)))
                    (local.get $byte))
                  (local.set $crc (call $crc32_update_byte (local.get $crc) (local.get $byte)))
                  (local.set $adler (call $adler32_update_byte (local.get $adler) (local.get $byte)))
                  (local.set $i (i32.add (local.get $i) (i32.const 1)))
                  (br $copy_match))))
            (local.set $written (i32.add (local.get $written) (local.get $length)))
            (br $fixed))))
      (if (i32.ne (local.get $btype) (i32.const 0))
        (then
          (call $m66write_record
            (local.get $out_record)
            (call $br_consumed_bytes)
            (local.get $written)
            (local.get $block_count)
            (local.get $btype)
            (i32.xor (local.get $crc) (i32.const -1))
            (local.get $adler))
          (return (i32.const 7))))
      (call $br_align_byte)
      (local.set $off (i32.shr_u (global.get $br_bitpos) (i32.const 3)))
      (if (i32.gt_u (i32.add (local.get $off) (i32.const 4)) (local.get $src_len)) (then (return (i32.const 1))))
      (local.set $block_len
        (i32.or
          (i32.load8_u (i32.add (local.get $src_ptr) (local.get $off)))
          (i32.shl
            (i32.load8_u (i32.add (local.get $src_ptr) (i32.add (local.get $off) (i32.const 1))))
            (i32.const 8))))
      (local.set $nlen
        (i32.or
          (i32.load8_u (i32.add (local.get $src_ptr) (i32.add (local.get $off) (i32.const 2))))
          (i32.shl
            (i32.load8_u (i32.add (local.get $src_ptr) (i32.add (local.get $off) (i32.const 3))))
            (i32.const 8))))
      (if (i32.ne (i32.and (i32.xor (local.get $block_len) (local.get $nlen)) (i32.const 65535)) (i32.const 65535))
        (then (return (i32.const 3))))
      (local.set $payload_off (i32.add (local.get $off) (i32.const 4)))
      (local.set $payload_end (i32.add (local.get $payload_off) (local.get $block_len)))
      (if (i32.or
            (i32.lt_u (local.get $payload_end) (local.get $payload_off))
            (i32.gt_u (local.get $payload_end) (local.get $src_len)))
        (then (return (i32.const 1))))
      (if (i32.gt_u (i32.add (local.get $written) (local.get $block_len)) (local.get $out_cap)) (then (return (i32.const 2))))
      (if (i32.gt_u (i32.add (local.get $written) (local.get $block_len)) (local.get $out_limit)) (then (return (i32.const 6))))
      (local.set $i (i32.const 0))
      (loop $copy
        (if (i32.lt_u (local.get $i) (local.get $block_len))
          (then
            (local.set $byte (i32.load8_u (i32.add (local.get $src_ptr) (i32.add (local.get $payload_off) (local.get $i)))))
            (i32.store8 (i32.add (local.get $out_ptr) (i32.add (local.get $written) (local.get $i))) (local.get $byte))
            (local.set $crc (call $crc32_update_byte (local.get $crc) (local.get $byte)))
            (local.set $adler (call $adler32_update_byte (local.get $adler) (local.get $byte)))
            (local.set $i (i32.add (local.get $i) (i32.const 1)))
            (br $copy))))
      (local.set $written (i32.add (local.get $written) (local.get $block_len)))
      (local.set $block_count (i32.add (local.get $block_count) (i32.const 1)))
      (global.set $br_bitpos (i32.shl (local.get $payload_end) (i32.const 3)))
      (global.set $br_bitbuf (i32.const 0))
      (global.set $br_bitcnt (i32.const 0))
      (if (local.get $bfinal)
        (then
          (call $m66write_record
            (local.get $out_record)
            (local.get $payload_end)
            (local.get $written)
            (local.get $block_count)
            (i32.const 0)
            (i32.xor (local.get $crc) (i32.const -1))
            (local.get $adler))
          (return (i32.const 0))))
      (br $blocks))
    (i32.const 1))


;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid_data,
  ;; 7 unsupported. Packed encode return: low32=status, high32=written.

  ;; Output record, little-endian u32:
  ;; 0: block_count
  ;; 4: payload_total
  ;; 8: consumed
  ;; 12: final_seen
  (func $write_scan_record
    (param $out i32)
    (param $block_count i32)
    (param $payload_total i32)
    (param $consumed i32)
    (param $final_seen i32)
    (i32.store (local.get $out) (local.get $block_count))
    (i32.store (i32.add (local.get $out) (i32.const 4)) (local.get $payload_total))
    (i32.store (i32.add (local.get $out) (i32.const 8)) (local.get $consumed))
    (i32.store (i32.add (local.get $out) (i32.const 12)) (local.get $final_seen)))

  (func $m67copy
    (param $src i32)
    (param $dst i32)
    (param $len i32)
    (local $i i32)
    (local.set $i (i32.const 0))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (i32.store8
          (i32.add (local.get $dst) (local.get $i))
          (i32.load8_u (i32.add (local.get $src) (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop))))

  (func (export "deflate_stored_scan")
    (param $ptr i32)
    (param $len i32)
    (param $out i32)
    (result i32)
    (local $off i32)
    (local $header i32)
    (local $bfinal i32)
    (local $btype i32)
    (local $block_len i32)
    (local $nlen i32)
    (local $payload_off i32)
    (local $payload_end i32)
    (local $block_count i32)
    (local $payload_total i32)

    (local.set $off (i32.const 0))
    (local.set $block_count (i32.const 0))
    (local.set $payload_total (i32.const 0))

    (block $done
      (loop $blocks
        (if (i32.ge_u (local.get $off) (local.get $len))
          (then (return (i32.const 1))))

        (local.set $header (i32.load8_u (i32.add (local.get $ptr) (local.get $off))))
        (local.set $bfinal (i32.and (local.get $header) (i32.const 1)))
        (local.set $btype (i32.and (i32.shr_u (local.get $header) (i32.const 1)) (i32.const 3)))

        (if (i32.eq (local.get $btype) (i32.const 3))
          (then (return (i32.const 3))))
        (if (i32.ne (local.get $btype) (i32.const 0))
          (then (return (i32.const 7))))

        (if (i32.gt_u (i32.add (local.get $off) (i32.const 5)) (local.get $len))
          (then (return (i32.const 1))))

        (local.set $block_len
          (i32.or
            (i32.load8_u (i32.add (local.get $ptr) (i32.add (local.get $off) (i32.const 1))))
            (i32.shl
              (i32.load8_u (i32.add (local.get $ptr) (i32.add (local.get $off) (i32.const 2))))
              (i32.const 8))))
        (local.set $nlen
          (i32.or
            (i32.load8_u (i32.add (local.get $ptr) (i32.add (local.get $off) (i32.const 3))))
            (i32.shl
              (i32.load8_u (i32.add (local.get $ptr) (i32.add (local.get $off) (i32.const 4))))
              (i32.const 8))))

        (if (i32.ne (i32.and (i32.xor (local.get $block_len) (local.get $nlen)) (i32.const 65535)) (i32.const 65535))
          (then (return (i32.const 3))))

        (local.set $payload_off (i32.add (local.get $off) (i32.const 5)))
        (local.set $payload_end (i32.add (local.get $payload_off) (local.get $block_len)))
        (if (i32.or
              (i32.lt_u (local.get $payload_end) (local.get $payload_off))
              (i32.gt_u (local.get $payload_end) (local.get $len)))
          (then (return (i32.const 1))))

        (local.set $block_count (i32.add (local.get $block_count) (i32.const 1)))
        (local.set $payload_total (i32.add (local.get $payload_total) (local.get $block_len)))
        (local.set $off (local.get $payload_end))

        (if (local.get $bfinal)
          (then
            (call $write_scan_record
              (local.get $out)
              (local.get $block_count)
              (local.get $payload_total)
              (local.get $off)
              (i32.const 1))
            (return (i32.const 0))))

        (br $blocks)))

    (i32.const 1))

  (func (export "deflate_stored_encode")
    (param $src_ptr i32)
    (param $src_len i32)
    (param $out_ptr i32)
    (param $out_cap i32)
    (result i64)
    (local $src_off i32)
    (local $written i32)
    (local $chunk i32)
    (local $remaining i32)
    (local $is_final i32)
    (local $nlen i32)

    (local.set $src_off (i32.const 0))
    (local.set $written (i32.const 0))

    (block $done
      (loop $blocks
        (local.set $remaining (i32.sub (local.get $src_len) (local.get $src_off)))
        (if (i32.gt_u (local.get $remaining) (i32.const 65535))
          (then
            (local.set $chunk (i32.const 65535))
            (local.set $is_final (i32.const 0)))
          (else
            (local.set $chunk (local.get $remaining))
            (local.set $is_final (i32.const 1))))

        (if (i32.gt_u
              (i32.add (local.get $written) (i32.add (local.get $chunk) (i32.const 5)))
              (local.get $out_cap))
          (then (return (call $pack (i32.const 2) (local.get $written)))))

        (i32.store8
          (i32.add (local.get $out_ptr) (local.get $written))
          (local.get $is_final))
        (i32.store8
          (i32.add (local.get $out_ptr) (i32.add (local.get $written) (i32.const 1)))
          (local.get $chunk))
        (i32.store8
          (i32.add (local.get $out_ptr) (i32.add (local.get $written) (i32.const 2)))
          (i32.shr_u (local.get $chunk) (i32.const 8)))
        (local.set $nlen (i32.xor (local.get $chunk) (i32.const 65535)))
        (i32.store8
          (i32.add (local.get $out_ptr) (i32.add (local.get $written) (i32.const 3)))
          (local.get $nlen))
        (i32.store8
          (i32.add (local.get $out_ptr) (i32.add (local.get $written) (i32.const 4)))
          (i32.shr_u (local.get $nlen) (i32.const 8)))

        (call $m67copy
          (i32.add (local.get $src_ptr) (local.get $src_off))
          (i32.add (local.get $out_ptr) (i32.add (local.get $written) (i32.const 5)))
          (local.get $chunk))

        (local.set $written (i32.add (local.get $written) (i32.add (local.get $chunk) (i32.const 5))))
        (local.set $src_off (i32.add (local.get $src_off) (local.get $chunk)))

        (br_if $done (local.get $is_final))
        (br $blocks)))

    (call $pack (i32.const 0) (local.get $written)))


;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid_data.
  ;; Packed write return: low32=status, high32=written.

  (func $read_le32 (param $ptr i32) (result i32)
    (i32.or
      (i32.or
        (i32.load8_u (local.get $ptr))
        (i32.shl
          (i32.load8_u (i32.add (local.get $ptr) (i32.const 1)))
          (i32.const 8)))
      (i32.or
        (i32.shl
          (i32.load8_u (i32.add (local.get $ptr) (i32.const 2)))
          (i32.const 16))
        (i32.shl
          (i32.load8_u (i32.add (local.get $ptr) (i32.const 3)))
          (i32.const 24)))))

  ;; Output record, little-endian u32:
  ;; 0 deflate_off, 4 deflate_len, 8 flags, 12 mtime, 16 xfl, 20 os,
  ;; 24 expected_crc32, 28 expected_isize, 32 header_len, 36 trailer_off.
  (func $m98write_record
    (param $out i32)
    (param $deflate_off i32)
    (param $deflate_len i32)
    (param $flags i32)
    (param $mtime i32)
    (param $xfl i32)
    (param $os i32)
    (param $expected_crc32 i32)
    (param $expected_isize i32)
    (param $header_len i32)
    (param $trailer_off i32)
    (i32.store (local.get $out) (local.get $deflate_off))
    (i32.store (i32.add (local.get $out) (i32.const 4)) (local.get $deflate_len))
    (i32.store (i32.add (local.get $out) (i32.const 8)) (local.get $flags))
    (i32.store (i32.add (local.get $out) (i32.const 12)) (local.get $mtime))
    (i32.store (i32.add (local.get $out) (i32.const 16)) (local.get $xfl))
    (i32.store (i32.add (local.get $out) (i32.const 20)) (local.get $os))
    (i32.store (i32.add (local.get $out) (i32.const 24)) (local.get $expected_crc32))
    (i32.store (i32.add (local.get $out) (i32.const 28)) (local.get $expected_isize))
    (i32.store (i32.add (local.get $out) (i32.const 32)) (local.get $header_len))
    (i32.store (i32.add (local.get $out) (i32.const 36)) (local.get $trailer_off)))

  ;; Returns the offset immediately after a zero-terminated field, or -1 when
  ;; the terminator is missing before the trailer reservation.
  (func $skip_zstring (param $ptr i32) (param $pos i32) (param $limit i32) (result i32)
    (local $i i32)
    (local.set $i (local.get $pos))
    (block $done
      (loop $scan
        (if (i32.ge_u (local.get $i) (local.get $limit))
          (then (return (i32.const -1))))
        (if (i32.eqz (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
          (then
            (return (i32.add (local.get $i) (i32.const 1)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        br $scan))
    i32.const -1)

  (func (export "gzip_member_scan")
    (param $ptr i32)
    (param $len i32)
    (param $out i32)
    (result i32)
    (local $flags i32)
    (local $mtime i32)
    (local $xfl i32)
    (local $os i32)
    (local $pos i32)
    (local $limit i32)
    (local $xlen i32)
    (local $trailer_off i32)

    (if (i32.lt_u (local.get $len) (i32.const 18))
      (then (return (i32.const 1))))

    (if
      (i32.or
        (i32.or
          (i32.ne (i32.load8_u (local.get $ptr)) (i32.const 31))
          (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))) (i32.const 139)))
        (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 2))) (i32.const 8)))
      (then (return (i32.const 3))))

    (local.set $flags (i32.load8_u (i32.add (local.get $ptr) (i32.const 3))))
    (if (i32.and (local.get $flags) (i32.const 224))
      (then (return (i32.const 3))))

    (local.set $mtime (call $read_le32 (i32.add (local.get $ptr) (i32.const 4))))
    (local.set $xfl (i32.load8_u (i32.add (local.get $ptr) (i32.const 8))))
    (local.set $os (i32.load8_u (i32.add (local.get $ptr) (i32.const 9))))
    (local.set $pos (i32.const 10))
    (local.set $limit (i32.sub (local.get $len) (i32.const 8)))

    ;; FEXTRA: two-byte little-endian XLEN followed by XLEN bytes.
    (if (i32.and (local.get $flags) (i32.const 4))
      (then
        (if (i32.gt_u (i32.add (local.get $pos) (i32.const 2)) (local.get $limit))
          (then (return (i32.const 1))))
        (local.set $xlen
          (i32.or
            (i32.load8_u (i32.add (local.get $ptr) (local.get $pos)))
            (i32.shl
              (i32.load8_u
                (i32.add
                  (local.get $ptr)
                  (i32.add (local.get $pos) (i32.const 1))))
              (i32.const 8))))
        (local.set $pos (i32.add (local.get $pos) (i32.const 2)))
        (if (i32.gt_u (local.get $xlen) (i32.sub (local.get $limit) (local.get $pos)))
          (then (return (i32.const 1))))
        (local.set $pos (i32.add (local.get $pos) (local.get $xlen)))))

    ;; FNAME
    (if (i32.and (local.get $flags) (i32.const 8))
      (then
        (local.set $pos (call $skip_zstring (local.get $ptr) (local.get $pos) (local.get $limit)))
        (if (i32.eq (local.get $pos) (i32.const -1))
          (then (return (i32.const 1))))))

    ;; FCOMMENT
    (if (i32.and (local.get $flags) (i32.const 16))
      (then
        (local.set $pos (call $skip_zstring (local.get $ptr) (local.get $pos) (local.get $limit)))
        (if (i32.eq (local.get $pos) (i32.const -1))
          (then (return (i32.const 1))))))

    ;; FHCRC: current Rust only skips the two header CRC bytes.
    (if (i32.and (local.get $flags) (i32.const 2))
      (then
        (if (i32.gt_u (i32.add (local.get $pos) (i32.const 2)) (local.get $limit))
          (then (return (i32.const 1))))
        (local.set $pos (i32.add (local.get $pos) (i32.const 2)))))

    (local.set $trailer_off (local.get $limit))
    (call $m98write_record
      (local.get $out)
      (local.get $pos)
      (i32.sub (local.get $trailer_off) (local.get $pos))
      (local.get $flags)
      (local.get $mtime)
      (local.get $xfl)
      (local.get $os)
      (call $read_le32 (i32.add (local.get $ptr) (local.get $trailer_off)))
      (call $read_le32 (i32.add (local.get $ptr) (i32.add (local.get $trailer_off) (i32.const 4))))
      (local.get $pos)
      (local.get $trailer_off))
    i32.const 0)

  (func (export "gzip_member_write_header")
    (param $out_ptr i32)
    (param $out_cap i32)
    (result i64)
    (if (i32.lt_u (local.get $out_cap) (i32.const 10))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))
    (i32.store8 (local.get $out_ptr) (i32.const 31))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 1)) (i32.const 139))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 2)) (i32.const 8))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 3)) (i32.const 0))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 4)) (i32.const 0))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 5)) (i32.const 0))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 6)) (i32.const 0))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 7)) (i32.const 0))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 8)) (i32.const 0))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 9)) (i32.const 255))
    (call $pack (i32.const 0) (i32.const 10)))

  (func (export "gzip_member_write_trailer")
    (param $crc32 i32)
    (param $isize i32)
    (param $out_ptr i32)
    (param $out_cap i32)
    (result i64)
    (if (i32.lt_u (local.get $out_cap) (i32.const 8))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))
    (i32.store (local.get $out_ptr) (local.get $crc32))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $isize))
    (call $pack (i32.const 0) (i32.const 8)))


;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid_data, 7 unsupported.
  ;; Packed i64 return: low u32 status, high u32 bytes_written.

  (func $m203read_u32_be (param $ptr i32) (result i32)
    local.get $ptr
    i32.load8_u
    i32.const 24
    i32.shl
    local.get $ptr
    i32.const 1
    i32.add
    i32.load8_u
    i32.const 16
    i32.shl
    i32.or
    local.get $ptr
    i32.const 2
    i32.add
    i32.load8_u
    i32.const 8
    i32.shl
    i32.or
    local.get $ptr
    i32.const 3
    i32.add
    i32.load8_u
    i32.or)

  (func $m203write_record
    (param $out i32)
    (param $deflate_off i32)
    (param $deflate_len i32)
    (param $cmf i32)
    (param $flg i32)
    (param $window_log2 i32)
    (param $flevel i32)
    (param $fdict i32)
    (param $expected_adler32 i32)
    local.get $out
    local.get $deflate_off
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $deflate_len
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $cmf
    i32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $flg
    i32.store
    local.get $out
    i32.const 16
    i32.add
    local.get $window_log2
    i32.store
    local.get $out
    i32.const 20
    i32.add
    local.get $flevel
    i32.store
    local.get $out
    i32.const 24
    i32.add
    local.get $fdict
    i32.store
    local.get $out
    i32.const 28
    i32.add
    local.get $expected_adler32
    i32.store)

  (func (export "zlib_member_scan")
    (param $ptr i32)
    (param $len i32)
    (param $out i32)
    (result i32)
    (local $cmf i32)
    (local $flg i32)
    (local $cinfo i32)
    (local $fdict i32)
    (local $flevel i32)
    (local $adler i32)

    local.get $len
    i32.const 6
    i32.lt_u
    if
      i32.const 1
      return
    end

    local.get $ptr
    i32.load8_u
    local.set $cmf
    local.get $ptr
    i32.const 1
    i32.add
    i32.load8_u
    local.set $flg

    local.get $cmf
    i32.const 15
    i32.and
    i32.const 8
    i32.ne
    if
      i32.const 3
      return
    end

    local.get $cmf
    i32.const 4
    i32.shr_u
    local.tee $cinfo
    i32.const 7
    i32.gt_u
    if
      i32.const 3
      return
    end

    local.get $cmf
    i32.const 256
    i32.mul
    local.get $flg
    i32.add
    i32.const 31
    i32.rem_u
    if
      i32.const 3
      return
    end

    local.get $flg
    i32.const 32
    i32.and
    i32.const 0
    i32.ne
    local.tee $fdict
    if
      i32.const 7
      return
    end

    local.get $flg
    i32.const 6
    i32.shr_u
    local.set $flevel

    local.get $ptr
    local.get $len
    i32.add
    i32.const 4
    i32.sub
    call $m203read_u32_be
    local.set $adler

    local.get $out
    i32.const 2
    local.get $len
    i32.const 6
    i32.sub
    local.get $cmf
    local.get $flg
    local.get $cinfo
    i32.const 8
    i32.add
    local.get $flevel
    local.get $fdict
    local.get $adler
    call $m203write_record

    i32.const 0)

  (func (export "zlib_write_header")
    (param $level i32)
    (param $out_ptr i32)
    (param $out_cap i32)
    (result i64)
    (local $flg i32)

    local.get $out_cap
    i32.const 2
    i32.lt_u
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end

    local.get $level
    i32.eqz
    if
      i32.const 1
      local.set $flg
    else
      local.get $level
      i32.const 1
      i32.eq
      if
        i32.const 94
        local.set $flg
      else
        local.get $level
        i32.const 6
        i32.eq
        if
          i32.const 156
          local.set $flg
        else
          local.get $level
          i32.const 9
          i32.eq
          if
            i32.const 218
            local.set $flg
          else
            i32.const 3
            i32.const 0
            call $pack
            return
          end
        end
      end
    end

    local.get $out_ptr
    i32.const 120
    i32.store8
    local.get $out_ptr
    i32.const 1
    i32.add
    local.get $flg
    i32.store8

    i32.const 0
    i32.const 2
    call $pack)

  (func (export "zlib_write_trailer")
    (param $adler32 i32)
    (param $out_ptr i32)
    (param $out_cap i32)
    (result i64)
    local.get $out_cap
    i32.const 4
    i32.lt_u
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end

    local.get $out_ptr
    local.get $adler32
    i32.const 24
    i32.shr_u
    i32.store8
    local.get $out_ptr
    i32.const 1
    i32.add
    local.get $adler32
    i32.const 16
    i32.shr_u
    i32.store8
    local.get $out_ptr
    i32.const 2
    i32.add
    local.get $adler32
    i32.const 8
    i32.shr_u
    i32.store8
    local.get $out_ptr
    i32.const 3
    i32.add
    local.get $adler32
    i32.store8

    i32.const 0
    i32.const 4
    call $pack)
