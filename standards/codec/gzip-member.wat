
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
