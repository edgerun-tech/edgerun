(module
  (import "edgerun-core" "memory" (memory 1))
(func (export "proto_standard_id") (result i32)
    i32.const 300021)

  ;; status: 0 ok, 1 input_short, 2 output_short, 3 invalid, 4 overflow, 5 done.
  ;; Decode the compact first+second OID root octet used by local const_oid.
  ;; out record: first_arc:u32, second_arc:u32, next_offset:u32.
  (func $der_oid_root_decode (export "der_oid_root_decode") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $octet i32)
    (if (i32.eqz (local.get $len))
      (then (return (i32.const 1))))
    (local.set $octet (i32.load8_u (local.get $ptr)))
    (if (i32.gt_u (local.get $octet) (i32.const 119))
      (then (return (i32.const 3))))
    (i32.store (local.get $out) (i32.div_u (local.get $octet) (i32.const 40)))
    (i32.store (i32.add (local.get $out) (i32.const 4))
      (i32.rem_u (local.get $octet) (i32.const 40)))
    (i32.store (i32.add (local.get $out) (i32.const 8)) (i32.const 1))
    i32.const 0)

  ;; Iterate one body arc after the root octet.
  ;; out record: arc:u32, next_offset:u32, encoded_len:u32.
  (func $der_oid_next_arc (export "der_oid_next_arc") (param $ptr i32) (param $len i32) (param $offset i32) (param $out i32) (result i32)
    (local $cursor i32)
    (local $byte i32)
    (local $count i32)
    (local $arc i64)
    (local $start i32)
    (if (i32.ge_u (local.get $offset) (local.get $len))
      (then (return (i32.const 5))))
    (if (i32.eqz (local.get $offset))
      (then (return (i32.const 3))))
    (local.set $cursor (local.get $offset))
    (local.set $start (local.get $offset))
    (local.set $arc (i64.const 0))
    (local.set $count (i32.const 0))
    (block $done
      (loop $loop
        (if (i32.ge_u (local.get $cursor) (local.get $len))
          (then (return (i32.const 1))))
        (local.set $byte (i32.load8_u (i32.add (local.get $ptr) (local.get $cursor))))
        (if (i32.and
              (i32.and
                (i32.eqz (local.get $count))
                (i32.eq (local.get $byte) (i32.const 128)))
              (i32.lt_u (i32.add (local.get $cursor) (i32.const 1)) (local.get $len)))
          (then (return (i32.const 3))))
        (local.set $count (i32.add (local.get $count) (i32.const 1)))
        (if (i32.gt_u (local.get $count) (i32.const 5))
          (then (return (i32.const 4))))
        (local.set $arc
          (i64.or
            (i64.shl (local.get $arc) (i64.const 7))
            (i64.extend_i32_u (i32.and (local.get $byte) (i32.const 127)))))
        (if (i64.gt_u (local.get $arc) (i64.const 4294967295))
          (then (return (i32.const 4))))
        (local.set $cursor (i32.add (local.get $cursor) (i32.const 1)))
        (br_if $done (i32.eqz (i32.and (local.get $byte) (i32.const 128))))
        (br $loop)))
    (i32.store (local.get $out) (i32.wrap_i64 (local.get $arc)))
    (i32.store (i32.add (local.get $out) (i32.const 4)) (local.get $cursor))
    (i32.store (i32.add (local.get $out) (i32.const 8))
      (i32.sub (local.get $cursor) (local.get $start)))
    i32.const 0)

  (func (export "der_oid_value_validate") (param $ptr i32) (param $len i32) (param $scratch i32) (result i32)
    (local $status i32)
    (local $offset i32)
    (local.set $status (call $der_oid_root_decode (local.get $ptr) (local.get $len) (local.get $scratch)))
    (if (local.get $status) (then (return (local.get $status))))
    (local.set $offset (i32.load (i32.add (local.get $scratch) (i32.const 8))))
    (block $done
      (loop $loop
        (local.set $status
          (call $der_oid_next_arc (local.get $ptr) (local.get $len) (local.get $offset) (local.get $scratch)))
        (if (i32.eq (local.get $status) (i32.const 5))
          (then (br $done)))
        (if (local.get $status)
          (then (return (local.get $status))))
        (local.set $offset (i32.load (i32.add (local.get $scratch) (i32.const 4))))
        (br $loop)))
    i32.const 0)

  ;; Return bits: low32=status, high32=written.
  (func (export "der_oid_root_encode") (param $first i32) (param $second i32) (param $out i32) (param $cap i32) (result i64)
    (if (i32.or (i32.gt_u (local.get $first) (i32.const 2)) (i32.gt_u (local.get $second) (i32.const 39)))
      (then (return (i64.const 3))))
    (if (i32.lt_u (local.get $cap) (i32.const 1))
      (then (return (i64.const 2))))
    (i32.store8 (local.get $out)
      (i32.add (i32.mul (local.get $first) (i32.const 40)) (local.get $second)))
    i64.const 4294967296)

  ;; Encode one u32 body arc as base-128.
  ;; Return bits: low32=status, high32=written.
  (func (export "der_oid_arc_encode") (param $arc i32) (param $out i32) (param $cap i32) (result i64)
    (local $needed i32)
    (local $i i32)
    (local $shift i32)
    (local $byte i32)
    (local.set $needed
      (select
        (i32.const 1)
        (select
          (i32.const 2)
          (select
            (i32.const 3)
            (select
              (i32.const 4)
              (i32.const 5)
              (i32.le_u (local.get $arc) (i32.const 268435455)))
            (i32.le_u (local.get $arc) (i32.const 2097151)))
          (i32.le_u (local.get $arc) (i32.const 16383)))
        (i32.le_u (local.get $arc) (i32.const 127))))
    (if (i32.lt_u (local.get $cap) (local.get $needed))
      (then (return (i64.const 2))))
    (local.set $i (i32.const 0))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $needed)))
        (local.set $shift
          (i32.mul
            (i32.sub (i32.sub (local.get $needed) (local.get $i)) (i32.const 1))
            (i32.const 7)))
        (local.set $byte
          (i32.and (i32.shr_u (local.get $arc) (local.get $shift)) (i32.const 127)))
        (if (i32.lt_u (local.get $i) (i32.sub (local.get $needed) (i32.const 1)))
          (then (local.set $byte (i32.or (local.get $byte) (i32.const 128)))))
        (i32.store8 (i32.add (local.get $out) (local.get $i)) (local.get $byte))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (i64.shl (i64.extend_i32_u (local.get $needed)) (i64.const 32)))
)
