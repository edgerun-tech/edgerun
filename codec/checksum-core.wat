;; ── Canonical checksum functions ──
;; Extracted from codec/base64.wat and codec/compress.wat duplicate definitions.

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

  (func $adler32_update_vec (export "adler32_update_vec")
    (param $initial i32) (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $a i32)
    (local $b i32)
    (local $a0 i32)
    (local $sum i32)
    (local $wsum i32)
    (local $v v128)
    (local $b0 i32)
    (local $b1 i32)
    (local $b2 i32)
    (local $b3 i32)
    (local $b4 i32)
    (local $b5 i32)
    (local $b6 i32)
    (local $b7 i32)
    (local $b8 i32)
    (local $b9 i32)
    (local $b10 i32)
    (local $b11 i32)
    (local $b12 i32)
    (local $b13 i32)
    (local $b14 i32)
    (local $b15 i32)
    (local.set $a
      (i32.and (local.get $initial) (i32.const 65535)))
    (local.set $b
      (i32.and
        (i32.shr_u (local.get $initial) (i32.const 16))
        (i32.const 65535)))
    (loop $loop
      (if (i32.le_u (i32.add (local.get $i) (i32.const 16)) (local.get $len))
        (then
          (local.set $v
            (v128.load (i32.add (local.get $ptr) (local.get $i))))
          (local.set $a0 (local.get $a))
          (local.set $b0
            (i8x16.extract_lane_u 0 (local.get $v)))
          (local.set $b1
            (i8x16.extract_lane_u 1 (local.get $v)))
          (local.set $b2
            (i8x16.extract_lane_u 2 (local.get $v)))
          (local.set $b3
            (i8x16.extract_lane_u 3 (local.get $v)))
          (local.set $b4
            (i8x16.extract_lane_u 4 (local.get $v)))
          (local.set $b5
            (i8x16.extract_lane_u 5 (local.get $v)))
          (local.set $b6
            (i8x16.extract_lane_u 6 (local.get $v)))
          (local.set $b7
            (i8x16.extract_lane_u 7 (local.get $v)))
          (local.set $b8
            (i8x16.extract_lane_u 8 (local.get $v)))
          (local.set $b9
            (i8x16.extract_lane_u 9 (local.get $v)))
          (local.set $b10
            (i8x16.extract_lane_u 10 (local.get $v)))
          (local.set $b11
            (i8x16.extract_lane_u 11 (local.get $v)))
          (local.set $b12
            (i8x16.extract_lane_u 12 (local.get $v)))
          (local.set $b13
            (i8x16.extract_lane_u 13 (local.get $v)))
          (local.set $b14
            (i8x16.extract_lane_u 14 (local.get $v)))
          (local.set $b15
            (i8x16.extract_lane_u 15 (local.get $v)))
          (local.set $sum
            (i32.add
              (i32.add
                (i32.add
                  (i32.add
                    (i32.add
                      (i32.add
                        (i32.add
                          (i32.add
                            (i32.add
                              (i32.add
                                (i32.add
                                  (i32.add
                                    (i32.add
                                      (i32.add
                                        (i32.add
                                          (local.get $b0)
                                          (local.get $b1))
                                        (local.get $b2))
                                      (local.get $b3))
                                    (local.get $b4))
                                  (local.get $b5))
                                (local.get $b6))
                              (local.get $b7))
                            (local.get $b8))
                          (local.get $b9))
                        (local.get $b10))
                      (local.get $b11))
                    (local.get $b12))
                  (local.get $b13))
                (local.get $b14))
              (local.get $b15)))
          (local.set $wsum
            (i32.add
              (i32.add
                (i32.add
                  (i32.add
                    (i32.add
                      (i32.add
                        (i32.add
                          (i32.add
                            (i32.add
                              (i32.add
                                (i32.add
                                  (i32.add
                                    (i32.add
                                      (i32.add
                                        (i32.add
                                          (i32.mul (i32.const 16) (local.get $b0))
                                          (i32.mul (i32.const 15) (local.get $b1)))
                                        (i32.mul (i32.const 14) (local.get $b2)))
                                      (i32.mul (i32.const 13) (local.get $b3)))
                                    (i32.mul (i32.const 12) (local.get $b4)))
                                  (i32.mul (i32.const 11) (local.get $b5)))
                                (i32.mul (i32.const 10) (local.get $b6)))
                              (i32.mul (i32.const 9) (local.get $b7)))
                            (i32.mul (i32.const 8) (local.get $b8)))
                          (i32.mul (i32.const 7) (local.get $b9)))
                        (i32.mul (i32.const 6) (local.get $b10)))
                      (i32.mul (i32.const 5) (local.get $b11)))
                    (i32.mul (i32.const 4) (local.get $b12)))
                  (i32.mul (i32.const 3) (local.get $b13)))
                (i32.mul (i32.const 2) (local.get $b14)))
              (i32.mul (i32.const 1) (local.get $b15))))
          (local.set $a
            (i32.rem_u
              (i32.add (local.get $a) (local.get $sum))
              (i32.const 65521)))
          (local.set $b
            (i32.rem_u
              (i32.add
                (i32.add
                  (local.get $b)
                  (i32.mul (i32.const 16) (local.get $a0)))
                (local.get $wsum))
              (i32.const 65521)))
          (local.set $i (i32.add (local.get $i) (i32.const 16)))
          (br $loop))
        (else
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
              (br $loop))))))
    (i32.or
      (i32.shl (local.get $b) (i32.const 16))
      (local.get $a)))

  (func $crc32_update_byte (export "crc32_update_byte") (param $crc i32) (param $byte i32) (result i32)
    (local $c i32) (local $i i32)
    (local.set $c (i32.xor (local.get $crc) (local.get $byte)))
    (local.set $i (i32.const 0))
    (block $done
      (loop $bits
        (br_if $done (i32.eq (local.get $i) (i32.const 8)))
        (if (i32.and (local.get $c) (i32.const 1))
          (then
            (local.set $c
              (i32.xor (i32.shr_u (local.get $c) (i32.const 1)) (i32.const 0xedb88320))))
          (else
            (local.set $c (i32.shr_u (local.get $c) (i32.const 1)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $bits)))
    (local.get $c))

  (func $adler32_update_byte (export "adler32_update_byte") (param $adler i32) (param $byte i32) (result i32)
    (local $s1 i32) (local $s2 i32)
    (local.set $s1 (i32.and (local.get $adler) (i32.const 65535)))
    (local.set $s2 (i32.shr_u (local.get $adler) (i32.const 16)))
    (local.set $s1 (i32.rem_u (i32.add (local.get $s1) (local.get $byte)) (i32.const 65521)))
    (local.set $s2 (i32.rem_u (i32.add (local.get $s2) (local.get $s1)) (i32.const 65521)))
    (i32.or (local.get $s1) (i32.shl (local.get $s2) (i32.const 16))))
