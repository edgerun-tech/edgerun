(module
  (import "edgerun-core" "memory" (memory 1))
(func (export "proto_standard_id") (result i32)
    i32.const 300802)

  ;; Status: 0 ok, 1 unsupported kind, 2 short, 3 invalid checksum/path/octal, 4 zero block.
  ;; Header output is eleven little-endian u32 slots:
  ;; kind,size_lo,size_hi,mode,uid,gid,mtime_lo,mtime_hi,path_len,link_len,whiteout_kind.
  ;; kind: regular=0, hardlink=1, symlink=2, char=3, block=4, dir=5, fifo=6, pax=7,
  ;; global_pax=8, gnu_long_name=9, gnu_long_link=10. whiteout: none=0, entry=1, opaque=2.

  (func $field_len (param $ptr i32) (param $max i32) (result i32)
    (local $i i32)
    (local $end i32)
    (local.set $i (i32.const 0))
    (local.set $end (i32.const 0))
    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (local.get $max)))
        (br_if $done (i32.eqz (i32.load8_u (i32.add (local.get $ptr) (local.get $i)))))
        (local.set $end (i32.add (local.get $i) (i32.const 1)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    (block $trim_done
      (loop $trim
        (br_if $trim_done (i32.eqz (local.get $end)))
        (br_if $trim_done
          (i32.ne
            (i32.load8_u (i32.add (local.get $ptr) (i32.sub (local.get $end) (i32.const 1))))
            (i32.const 32)))
        (local.set $end (i32.sub (local.get $end) (i32.const 1)))
        (br $trim)))
    local.get $end)

  (func $is_zero_block (param $ptr i32) (result i32)
    (local $i i32)
    (local.set $i (i32.const 0))
    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (i32.const 512)))
        (if (i32.load8_u (i32.add (local.get $ptr) (local.get $i)))
          (then (return (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    i32.const 1)

  (func $parse_octal64 (param $ptr i32) (param $len i32) (param $out_ptr i32) (result i32)
    (local $i i32)
    (local $b i32)
    (local $saw i32)
    (local $value i64)
    (local.set $i (i32.const 0))
    (local.set $value (i64.const 0))
    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (local.set $b (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
        (if (i32.or (i32.eqz (local.get $b)) (i32.eq (local.get $b) (i32.const 32)))
          (then
            (if (local.get $saw)
              (then (br $done)))))
        (if (i32.and (i32.ge_u (local.get $b) (i32.const 48)) (i32.le_u (local.get $b) (i32.const 55)))
          (then
            (local.set $saw (i32.const 1))
            (local.set $value
              (i64.add
                (i64.shl (local.get $value) (i64.const 3))
                (i64.extend_i32_u (i32.sub (local.get $b) (i32.const 48))))))
          (else
            (if (i32.and (i32.ne (local.get $b) (i32.const 0)) (i32.ne (local.get $b) (i32.const 32)))
              (then (return (i32.const 3))))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    (i32.store (local.get $out_ptr) (i32.wrap_i64 (local.get $value)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 32))))
    i32.const 0)

  (func (export "oci_tar_octal_u64") (param $ptr i32) (param $len i32) (param $out_ptr i32) (result i32)
    (call $parse_octal64 (local.get $ptr) (local.get $len) (local.get $out_ptr)))

  (func $is_component_dots (param $ptr i32) (param $start i32) (param $end i32) (result i32)
    (if (i32.eq (i32.sub (local.get $end) (local.get $start)) (i32.const 2))
      (then
        (return
          (i32.and
            (i32.eq (i32.load8_u (i32.add (local.get $ptr) (local.get $start))) (i32.const 46))
            (i32.eq
              (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $start)) (i32.const 1)))
              (i32.const 46))))))
    i32.const 0)

  (func $path_safe_len (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $component_start i32)
    (local $normalized i32)
    (if (i32.eqz (local.get $len))
      (then (return (i32.const 0))))
    (if (i32.eq (i32.load8_u (local.get $ptr)) (i32.const 47))
      (then (return (i32.const 0))))
    (local.set $i (i32.const 0))
    (local.set $component_start (i32.const 0))
    (block $done
      (loop $scan
        (br_if $done (i32.gt_u (local.get $i) (local.get $len)))
        (if (i32.or
              (i32.eq (local.get $i) (local.get $len))
              (i32.eq (i32.load8_u (i32.add (local.get $ptr) (local.get $i))) (i32.const 47)))
          (then
            (if (i32.gt_u (local.get $i) (local.get $component_start))
              (then
                (if (call $is_component_dots (local.get $ptr) (local.get $component_start) (local.get $i))
                  (then (return (i32.const 0))))
                (if (i32.ne
                      (i32.sub (local.get $i) (local.get $component_start))
                      (i32.const 1))
                  (then (local.set $normalized (i32.const 1)))
                  (else
                    (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (local.get $component_start))) (i32.const 46))
                      (then (local.set $normalized (i32.const 1))))))))
            (local.set $component_start (i32.add (local.get $i) (i32.const 1)))))
        (if (i32.and
              (i32.lt_u (local.get $i) (local.get $len))
              (i32.eqz (i32.load8_u (i32.add (local.get $ptr) (local.get $i)))))
          (then (return (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    local.get $normalized)

  (func (export "oci_tar_path_safe") (param $ptr i32) (param $len i32) (result i32)
    (call $path_safe_len (local.get $ptr) (local.get $len)))

  (func $whiteout_kind_name (param $ptr i32) (param $start i32) (param $end i32) (result i32)
    (local $n i32)
    (local $ok i32)
    (local.set $n (i32.sub (local.get $end) (local.get $start)))
    (if (i32.eq (local.get $n) (i32.const 12))
      (then
        (local.set $ok (i32.const 1))
        (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (local.get $start))) (i32.const 46)) (then (local.set $ok (i32.const 0))))
        (if (i32.ne (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $start)) (i32.const 1))) (i32.const 119)) (then (local.set $ok (i32.const 0))))
        (if (i32.ne (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $start)) (i32.const 2))) (i32.const 104)) (then (local.set $ok (i32.const 0))))
        (if (i32.ne (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $start)) (i32.const 3))) (i32.const 46)) (then (local.set $ok (i32.const 0))))
        (if (i32.ne (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $start)) (i32.const 4))) (i32.const 46)) (then (local.set $ok (i32.const 0))))
        (if (i32.ne (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $start)) (i32.const 5))) (i32.const 119)) (then (local.set $ok (i32.const 0))))
        (if (i32.ne (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $start)) (i32.const 6))) (i32.const 104)) (then (local.set $ok (i32.const 0))))
        (if (i32.ne (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $start)) (i32.const 7))) (i32.const 46)) (then (local.set $ok (i32.const 0))))
        (if (i32.ne (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $start)) (i32.const 8))) (i32.const 46)) (then (local.set $ok (i32.const 0))))
        (if (i32.ne (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $start)) (i32.const 9))) (i32.const 111)) (then (local.set $ok (i32.const 0))))
        (if (i32.ne (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $start)) (i32.const 10))) (i32.const 112)) (then (local.set $ok (i32.const 0))))
        (if (i32.ne (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $start)) (i32.const 11))) (i32.const 113)) (then (local.set $ok (i32.const 0))))
        (if (local.get $ok) (then (return (i32.const 2))))))
    (if (i32.and
          (i32.gt_u (local.get $n) (i32.const 4))
          (i32.and
            (i32.eq (i32.load8_u (i32.add (local.get $ptr) (local.get $start))) (i32.const 46))
            (i32.and
              (i32.eq (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $start)) (i32.const 1))) (i32.const 119))
              (i32.and
                (i32.eq (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $start)) (i32.const 2))) (i32.const 104))
                (i32.eq (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $start)) (i32.const 3))) (i32.const 46))))))
      (then (return (i32.const 1))))
    i32.const 0)

  (func $whiteout_kind (param $name_ptr i32) (param $name_len i32) (result i32)
    (local $i i32)
    (local $last i32)
    (local.set $last (i32.const 0))
    (local.set $i (i32.const 0))
    (block $done2
      (loop $scann
        (br_if $done2 (i32.ge_u (local.get $i) (local.get $name_len)))
        (if (i32.eq (i32.load8_u (i32.add (local.get $name_ptr) (local.get $i))) (i32.const 47))
          (then (local.set $last (i32.add (local.get $i) (i32.const 1)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scann)))
    (call $whiteout_kind_name (local.get $name_ptr) (local.get $last) (local.get $name_len)))

  (func $kind_code (param $byte i32) (result i32)
    (if (i32.or (i32.eqz (local.get $byte)) (i32.eq (local.get $byte) (i32.const 48))) (then (return (i32.const 0))))
    (if (i32.eq (local.get $byte) (i32.const 49)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $byte) (i32.const 50)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $byte) (i32.const 51)) (then (return (i32.const 3))))
    (if (i32.eq (local.get $byte) (i32.const 52)) (then (return (i32.const 4))))
    (if (i32.eq (local.get $byte) (i32.const 53)) (then (return (i32.const 5))))
    (if (i32.eq (local.get $byte) (i32.const 54)) (then (return (i32.const 6))))
    (if (i32.eq (local.get $byte) (i32.const 120)) (then (return (i32.const 7))))
    (if (i32.eq (local.get $byte) (i32.const 103)) (then (return (i32.const 8))))
    (if (i32.eq (local.get $byte) (i32.const 76)) (then (return (i32.const 9))))
    (if (i32.eq (local.get $byte) (i32.const 75)) (then (return (i32.const 10))))
    i32.const 255)

  (func $verify_checksum (param $ptr i32) (result i32)
    (local $i i32)
    (local $sum i64)
    (local $expected i64)
    (if (call $parse_octal64 (i32.add (local.get $ptr) (i32.const 148)) (i32.const 8) (i32.const 0))
      (then (return (i32.const 0))))
    (local.set $expected
      (i64.or
        (i64.extend_i32_u (i32.load (i32.const 0)))
        (i64.shl (i64.extend_i32_u (i32.load (i32.const 4))) (i64.const 32))))
    (local.set $i (i32.const 0))
    (local.set $sum (i64.const 0))
    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (i32.const 512)))
        (local.set $sum
          (i64.add
            (local.get $sum)
            (i64.extend_i32_u
              (select
                (i32.const 32)
                (i32.load8_u (i32.add (local.get $ptr) (local.get $i)))
                (i32.and (i32.ge_u (local.get $i) (i32.const 148)) (i32.lt_u (local.get $i) (i32.const 156)))))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    (i64.eq (local.get $expected) (local.get $sum)))

  (func (export "oci_tar_header_parse") (param $ptr i32) (param $len i32) (param $out_ptr i32) (result i32)
    (local $name_len i32)
    (local $prefix_len i32)
    (local $link_len i32)
    (local $kind i32)
    (local $status i32)
    (if (i32.lt_u (local.get $len) (i32.const 512))
      (then (return (i32.const 2))))
    (if (call $is_zero_block (local.get $ptr))
      (then (return (i32.const 4))))
    (if (i32.eqz (call $verify_checksum (local.get $ptr)))
      (then (return (i32.const 3))))
    (local.set $name_len (call $field_len (local.get $ptr) (i32.const 100)))
    (local.set $prefix_len (call $field_len (i32.add (local.get $ptr) (i32.const 345)) (i32.const 155)))
    (local.set $link_len (call $field_len (i32.add (local.get $ptr) (i32.const 157)) (i32.const 100)))
    (if (i32.eqz (local.get $name_len))
      (then (return (i32.const 3))))
    (if (local.get $prefix_len)
      (then
        (if (i32.eqz (call $path_safe_len (i32.add (local.get $ptr) (i32.const 345)) (local.get $prefix_len)))
          (then (return (i32.const 3))))
        (if (i32.eqz (call $path_safe_len (local.get $ptr) (local.get $name_len)))
          (then (return (i32.const 3)))))
      (else
        (if (i32.eqz (call $path_safe_len (local.get $ptr) (local.get $name_len)))
          (then (return (i32.const 3))))))
    (local.set $kind (call $kind_code (i32.load8_u (i32.add (local.get $ptr) (i32.const 156)))))
    (if (i32.eq (local.get $kind) (i32.const 255))
      (then (return (i32.const 1))))
    (i32.store (local.get $out_ptr) (local.get $kind))
    (local.set $status (call $parse_octal64 (i32.add (local.get $ptr) (i32.const 124)) (i32.const 12) (i32.add (local.get $out_ptr) (i32.const 4))))
    (if (local.get $status) (then (return (local.get $status))))
    (local.set $status (call $parse_octal64 (i32.add (local.get $ptr) (i32.const 100)) (i32.const 8) (i32.const 0)))
    (if (local.get $status) (then (return (local.get $status))))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 12)) (i32.load (i32.const 0)))
    (local.set $status (call $parse_octal64 (i32.add (local.get $ptr) (i32.const 108)) (i32.const 8) (i32.const 0)))
    (if (local.get $status) (then (return (local.get $status))))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 16)) (i32.load (i32.const 0)))
    (local.set $status (call $parse_octal64 (i32.add (local.get $ptr) (i32.const 116)) (i32.const 8) (i32.const 0)))
    (if (local.get $status) (then (return (local.get $status))))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 20)) (i32.load (i32.const 0)))
    (local.set $status (call $parse_octal64 (i32.add (local.get $ptr) (i32.const 136)) (i32.const 12) (i32.add (local.get $out_ptr) (i32.const 24))))
    (if (local.get $status) (then (return (local.get $status))))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 32))
      (i32.add (i32.add (local.get $name_len) (local.get $prefix_len)) (select (i32.const 1) (i32.const 0) (local.get $prefix_len))))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 36)) (local.get $link_len))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 40))
      (call $whiteout_kind
        (local.get $ptr)
        (local.get $name_len)))
    i32.const 0)
)
