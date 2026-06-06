  (import "edgerun" "to_lower" (func $m80lower_ascii (param i32) (result i32)))
  (import "dns" "is_label_byte" (func $is_label_byte (param i32) (result i32)))

(func (export "proto_standard_id") (result i32)
    i32.const 300025)

  ;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid, 6 too_long.

  ;; Output record, little-endian:
  ;; 0:u32 consumed wire bytes
  ;; 4:u32 label count
  ;; 8:u32 normalized dotted-name length, excluding root dot
  ;; 12..: repeated u32 label_offset_from_input, u32 label_len
  ;;
  ;; This scanner accepts uncompressed RFC1035 wire names only. Compression
  ;; pointers and reserved top-bit label forms are detected and rejected.
  (func $dns_name_scan (export "dns_name_scan")
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
                (local.set $b (call $m80lower_ascii (i32.load8_u (i32.add (local.get $in_ptr) (i32.add (local.get $off) (local.get $j))))))
                (i32.store8 (i32.add (local.get $out_ptr) (local.get $written)) (local.get $b))
                (local.set $written (i32.add (local.get $written) (i32.const 1)))
                (local.set $j (i32.add (local.get $j) (i32.const 1)))
                (br $copy_label))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $labels_loop))))
    (call $pack (i32.const 0) (local.get $written)))
