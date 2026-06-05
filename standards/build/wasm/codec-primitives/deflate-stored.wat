(module
  (memory (export "memory") 4)

  (func (export "proto_abi_version") (result i32)
    i32.const 2)

  (func (export "proto_standard_id") (result i32)
    i32.const 300066)

  ;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid_data,
  ;; 7 unsupported. Packed encode return: low32=status, high32=written.
  (func $pack (param $status i32) (param $written i32) (result i64)
    (i64.or
      (i64.extend_i32_u (local.get $status))
      (i64.shl (i64.extend_i32_u (local.get $written)) (i64.const 32))))

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

  (func $copy
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

        (call $copy
          (i32.add (local.get $src_ptr) (local.get $src_off))
          (i32.add (local.get $out_ptr) (i32.add (local.get $written) (i32.const 5)))
          (local.get $chunk))

        (local.set $written (i32.add (local.get $written) (i32.add (local.get $chunk) (i32.const 5))))
        (local.set $src_off (i32.add (local.get $src_off) (local.get $chunk)))

        (br_if $done (local.get $is_final))
        (br $blocks)))

    (call $pack (i32.const 0) (local.get $written)))
)
