(module
  (type $t0 (func (result i32)))
  (type $t1 (func (param i32 i64 i64 i32)))
  (type $t2 (func (param i64 i64 i32 i32) (result i32)))
  (type $t3 (func (param i64 i64 i32 i32 i32) (result i32)))
  (type $t4 (func (param i64 i64 i32)))
  (type $t5 (func (param i32 i32 i32) (result i32)))
  (type $t6 (func (param i64 i64 i32 i64 i64 i32 i32) (result i32)))
  (type $t7 (func (param i64 i64 i32 i64 i64 i32) (result i32)))
  (type $t8 (func (param i64 i64) (result i32)))
  (type $t9 (func (param i64 i64 i32 i32 i32) (result i64)))
  (import "edgerun-core" "memory" (memory $memory 1))
  (func $proto_standard_id (export "proto_standard_id") (type $t0) (result i32)
    (i32.const 300160))
  (func $f2 (type $t1) (param $p0 i32) (param $p1 i64) (param $p2 i64) (param $p3 i32)
    (i64.store
      (local.get $p0)
      (local.get $p1))
    (i64.store
      (i32.add
        (local.get $p0)
        (i32.const 8))
      (local.get $p2))
    (i32.store
      (i32.add
        (local.get $p0)
        (i32.const 16))
      (local.get $p3)))
  (func $f3 (type $t2) (param $p0 i64) (param $p1 i64) (param $p2 i32) (param $p3 i32) (result i32)
    (local $l4 i64) (local $l5 i64) (local $l6 i64) (local $l7 i64) (local $l8 i64) (local $l9 i64) (local $l10 i64)
    (local.set $l4
      (i64.const 4294967295))
    (local.set $l5
      (i64.add
        (i64.mul
          (i64.and
            (local.get $p0)
            (local.get $l4))
          (i64.const 10))
        (i64.extend_i32_u
          (local.get $p2))))
    (local.set $l6
      (i64.add
        (i64.mul
          (i64.shr_u
            (local.get $p0)
            (i64.const 32))
          (i64.const 10))
        (i64.shr_u
          (local.get $l5)
          (i64.const 32))))
    (local.set $l7
      (i64.or
        (i64.and
          (local.get $l5)
          (local.get $l4))
        (i64.shl
          (i64.and
            (local.get $l6)
            (local.get $l4))
          (i64.const 32))))
    (local.set $l8
      (i64.shr_u
        (local.get $l6)
        (i64.const 32)))
    (local.set $l9
      (i64.add
        (i64.mul
          (i64.and
            (local.get $p1)
            (local.get $l4))
          (i64.const 10))
        (local.get $l8)))
    (local.set $l10
      (i64.add
        (i64.mul
          (i64.shr_u
            (local.get $p1)
            (i64.const 32))
          (i64.const 10))
        (i64.shr_u
          (local.get $l9)
          (i64.const 32))))
    (if $I0
      (i64.ne
        (i64.shr_u
          (local.get $l10)
          (i64.const 32))
        (i64.const 0))
      (then
        (return
          (i32.const 1))))
    (i64.store
      (local.get $p3)
      (local.get $l7))
    (i64.store
      (i32.add
        (local.get $p3)
        (i32.const 8))
      (i64.or
        (i64.and
          (local.get $l9)
          (local.get $l4))
        (i64.shl
          (i64.and
            (local.get $l10)
            (local.get $l4))
          (i64.const 32))))
    (i32.const 0))
  (func $f4 (type $t3) (param $p0 i64) (param $p1 i64) (param $p2 i32) (param $p3 i32) (param $p4 i32) (result i32)
    (local $l5 i32) (local $l6 i32)
    (if $I0
      (i32.gt_u
        (local.get $p3)
        (i32.const 38))
      (then
        (return
          (i32.const 3))))
    (if $I1
      (i32.gt_u
        (local.get $p2)
        (i32.const 38))
      (then
        (return
          (i32.const 3))))
    (if $I2
      (i32.eq
        (local.get $p2)
        (local.get $p3))
      (then
        (call $f2
          (local.get $p4)
          (local.get $p0)
          (local.get $p1)
          (local.get $p3))
        (return
          (i32.const 0))))
    (if $I3
      (i32.lt_u
        (local.get $p2)
        (local.get $p3))
      (then
        (local.set $l6
          (i32.sub
            (local.get $p3)
            (local.get $p2)))
        (block $B4
          (loop $L5
            (br_if $B4
              (i32.eqz
                (local.get $l6)))
            (local.set $l5
              (call $f3
                (local.get $p0)
                (local.get $p1)
                (i32.const 0)
                (local.get $p4)))
            (if $I6
              (local.get $l5)
              (then
                (return
                  (local.get $l5))))
            (local.set $p0
              (i64.load
                (local.get $p4)))
            (local.set $p1
              (i64.load
                (i32.add
                  (local.get $p4)
                  (i32.const 8))))
            (local.set $l6
              (i32.sub
                (local.get $l6)
                (i32.const 1)))
            (br $L5)))
        (call $f2
          (local.get $p4)
          (local.get $p0)
          (local.get $p1)
          (local.get $p3))
        (return
          (i32.const 0))))
    (local.set $l6
      (i32.sub
        (local.get $p2)
        (local.get $p3)))
    (block $B7
      (loop $L8
        (br_if $B7
          (i32.eqz
            (local.get $l6)))
        (call $f5
          (local.get $p0)
          (local.get $p1)
          (local.get $p4))
        (local.set $p0
          (i64.load
            (local.get $p4)))
        (local.set $p1
          (i64.load
            (i32.add
              (local.get $p4)
              (i32.const 8))))
        (local.set $l6
          (i32.sub
            (local.get $l6)
            (i32.const 1)))
        (br $L8)))
    (call $f2
      (local.get $p4)
      (local.get $p0)
      (local.get $p1)
      (local.get $p3))
    (i32.const 0))
  (func $f5 (type $t4) (param $p0 i64) (param $p1 i64) (param $p2 i32)
    (local $l3 i64) (local $l4 i64) (local $l5 i64) (local $l6 i32) (local $l7 i64)
    (local.set $l6
      (i32.const 127))
    (block $B0
      (loop $L1
        (if $I2
          (i32.ge_u
            (local.get $l6)
            (i32.const 64))
          (then
            (local.set $l7
              (i64.and
                (i64.shr_u
                  (local.get $p1)
                  (i64.extend_i32_u
                    (i32.sub
                      (local.get $l6)
                      (i32.const 64))))
                (i64.const 1))))
          (else
            (local.set $l7
              (i64.and
                (i64.shr_u
                  (local.get $p0)
                  (i64.extend_i32_u
                    (local.get $l6)))
                (i64.const 1)))))
        (local.set $l5
          (i64.or
            (i64.shl
              (local.get $l5)
              (i64.const 1))
            (local.get $l7)))
        (if $I3
          (i64.ge_u
            (local.get $l5)
            (i64.const 10))
          (then
            (local.set $l5
              (i64.sub
                (local.get $l5)
                (i64.const 10)))
            (if $I4
              (i32.ge_u
                (local.get $l6)
                (i32.const 64))
              (then
                (local.set $l4
                  (i64.or
                    (local.get $l4)
                    (i64.shl
                      (i64.const 1)
                      (i64.extend_i32_u
                        (i32.sub
                          (local.get $l6)
                          (i32.const 64)))))))
              (else
                (local.set $l3
                  (i64.or
                    (local.get $l3)
                    (i64.shl
                      (i64.const 1)
                      (i64.extend_i32_u
                        (local.get $l6)))))))))
        (br_if $B0
          (i32.eqz
            (local.get $l6)))
        (local.set $l6
          (i32.sub
            (local.get $l6)
            (i32.const 1)))
        (br $L1)))
    (i64.store
      (local.get $p2)
      (local.get $l3))
    (i64.store
      (i32.add
        (local.get $p2)
        (i32.const 8))
      (local.get $l4)))
  (func $wallet_decimal_parse (export "wallet_decimal_parse") (type $t5) (param $p0 i32) (param $p1 i32) (param $p2 i32) (result i32)
    (local $l3 i32) (local $l4 i32) (local $l5 i32) (local $l6 i32) (local $l7 i32) (local $l8 i64) (local $l9 i64) (local $l10 i32)
    (if $I0
      (i32.eqz
        (local.get $p1))
      (then
        (return
          (i32.const 3))))
    (block $B1
      (loop $L2
        (br_if $B1
          (i32.ge_u
            (local.get $l3)
            (local.get $p1)))
        (local.set $l4
          (i32.load8_u
            (i32.add
              (local.get $p0)
              (local.get $l3))))
        (if $I3
          (i32.eq
            (local.get $l4)
            (i32.const 46))
          (then
            (if $I4
              (local.get $l5)
              (then
                (return
                  (i32.const 3))))
            (local.set $l5
              (i32.const 1)))
          (else
            (if $I5
              (i32.or
                (i32.lt_u
                  (local.get $l4)
                  (i32.const 48))
                (i32.gt_u
                  (local.get $l4)
                  (i32.const 57)))
              (then
                (return
                  (i32.const 3))))
            (local.set $l6
              (i32.const 1))
            (if $I6
              (local.get $l5)
              (then
                (local.set $l7
                  (i32.add
                    (local.get $l7)
                    (i32.const 1)))
                (if $I7
                  (i32.gt_u
                    (local.get $l7)
                    (i32.const 38))
                  (then
                    (return
                      (i32.const 3))))))
            (local.set $l10
              (call $f3
                (local.get $l8)
                (local.get $l9)
                (i32.sub
                  (local.get $l4)
                  (i32.const 48))
                (local.get $p2)))
            (if $I8
              (local.get $l10)
              (then
                (return
                  (local.get $l10))))
            (local.set $l8
              (i64.load
                (local.get $p2)))
            (local.set $l9
              (i64.load
                (i32.add
                  (local.get $p2)
                  (i32.const 8))))))
        (local.set $l3
          (i32.add
            (local.get $l3)
            (i32.const 1)))
        (br $L2)))
    (if $I9
      (i32.eqz
        (local.get $l6))
      (then
        (return
          (i32.const 3))))
    (call $f2
      (local.get $p2)
      (local.get $l8)
      (local.get $l9)
      (local.get $l7))
    (i32.const 0))
  (func $wallet_decimal_normalize (export "wallet_decimal_normalize") (type $t3) (param $p0 i64) (param $p1 i64) (param $p2 i32) (param $p3 i32) (param $p4 i32) (result i32)
    (call $f4
      (local.get $p0)
      (local.get $p1)
      (local.get $p2)
      (local.get $p3)
      (local.get $p4)))
  (func $wallet_decimal_add (export "wallet_decimal_add") (type $t6) (param $p0 i64) (param $p1 i64) (param $p2 i32) (param $p3 i64) (param $p4 i64) (param $p5 i32) (param $p6 i32) (result i32)
    (local $l7 i32) (local $l8 i32) (local $l9 i64) (local $l10 i64) (local $l11 i64) (local $l12 i64) (local $l13 i64) (local $l14 i64)
    (local.set $l7
      (select
        (local.get $p2)
        (local.get $p5)
        (i32.gt_u
          (local.get $p2)
          (local.get $p5))))
    (local.set $l8
      (call $f4
        (local.get $p0)
        (local.get $p1)
        (local.get $p2)
        (local.get $l7)
        (local.get $p6)))
    (if $I0
      (local.get $l8)
      (then
        (return
          (local.get $l8))))
    (local.set $l9
      (i64.load
        (local.get $p6)))
    (local.set $l10
      (i64.load
        (i32.add
          (local.get $p6)
          (i32.const 8))))
    (local.set $l8
      (call $f4
        (local.get $p3)
        (local.get $p4)
        (local.get $p5)
        (local.get $l7)
        (local.get $p6)))
    (if $I1
      (local.get $l8)
      (then
        (return
          (local.get $l8))))
    (local.set $l11
      (i64.load
        (local.get $p6)))
    (local.set $l12
      (i64.load
        (i32.add
          (local.get $p6)
          (i32.const 8))))
    (local.set $l13
      (i64.add
        (local.get $l9)
        (local.get $l11)))
    (local.set $l14
      (i64.add
        (i64.add
          (local.get $l10)
          (local.get $l12))
        (i64.extend_i32_u
          (i64.lt_u
            (local.get $l13)
            (local.get $l9)))))
    (if $I2
      (i32.or
        (i64.lt_u
          (local.get $l14)
          (local.get $l10))
        (i32.and
          (i64.eq
            (local.get $l14)
            (local.get $l10))
          (i64.gt_u
            (local.get $l12)
            (i64.const 0))))
      (then
        (return
          (i32.const 1))))
    (call $f2
      (local.get $p6)
      (local.get $l13)
      (local.get $l14)
      (local.get $l7))
    (i32.const 0))
  (func $wallet_decimal_sub (export "wallet_decimal_sub") (type $t6) (param $p0 i64) (param $p1 i64) (param $p2 i32) (param $p3 i64) (param $p4 i64) (param $p5 i32) (param $p6 i32) (result i32)
    (local $l7 i32) (local $l8 i32) (local $l9 i64) (local $l10 i64) (local $l11 i64) (local $l12 i64) (local $l13 i64)
    (local.set $l7
      (select
        (local.get $p2)
        (local.get $p5)
        (i32.gt_u
          (local.get $p2)
          (local.get $p5))))
    (local.set $l8
      (call $f4
        (local.get $p0)
        (local.get $p1)
        (local.get $p2)
        (local.get $l7)
        (local.get $p6)))
    (if $I0
      (local.get $l8)
      (then
        (return
          (local.get $l8))))
    (local.set $l9
      (i64.load
        (local.get $p6)))
    (local.set $l10
      (i64.load
        (i32.add
          (local.get $p6)
          (i32.const 8))))
    (local.set $l8
      (call $f4
        (local.get $p3)
        (local.get $p4)
        (local.get $p5)
        (local.get $l7)
        (local.get $p6)))
    (if $I1
      (local.get $l8)
      (then
        (return
          (local.get $l8))))
    (local.set $l11
      (i64.load
        (local.get $p6)))
    (local.set $l12
      (i64.load
        (i32.add
          (local.get $p6)
          (i32.const 8))))
    (if $I2
      (i32.or
        (i64.gt_u
          (local.get $l12)
          (local.get $l10))
        (i32.and
          (i64.eq
            (local.get $l12)
            (local.get $l10))
          (i64.gt_u
            (local.get $l11)
            (local.get $l9))))
      (then
        (return
          (i32.const 3))))
    (local.set $l13
      (i64.extend_i32_u
        (i64.lt_u
          (local.get $l9)
          (local.get $l11))))
    (call $f2
      (local.get $p6)
      (i64.sub
        (local.get $l9)
        (local.get $l11))
      (i64.sub
        (i64.sub
          (local.get $l10)
          (local.get $l12))
        (local.get $l13))
      (local.get $l7))
    (i32.const 0))
  (func $wallet_decimal_compare (export "wallet_decimal_compare") (type $t7) (param $p0 i64) (param $p1 i64) (param $p2 i32) (param $p3 i64) (param $p4 i64) (param $p5 i32) (result i32)
    (local $l6 i32) (local $l7 i32) (local $l8 i64) (local $l9 i64) (local $l10 i64) (local $l11 i64)
    (local.set $l6
      (select
        (local.get $p2)
        (local.get $p5)
        (i32.gt_u
          (local.get $p2)
          (local.get $p5))))
    (local.set $l7
      (call $f4
        (local.get $p0)
        (local.get $p1)
        (local.get $p2)
        (local.get $l6)
        (i32.const 64000)))
    (if $I0
      (local.get $l7)
      (then
        (return
          (i32.const 0))))
    (local.set $l8
      (i64.load
        (i32.const 64000)))
    (local.set $l9
      (i64.load
        (i32.const 64008)))
    (local.set $l7
      (call $f4
        (local.get $p3)
        (local.get $p4)
        (local.get $p5)
        (local.get $l6)
        (i32.const 64024)))
    (if $I1
      (local.get $l7)
      (then
        (return
          (i32.const 0))))
    (local.set $l10
      (i64.load
        (i32.const 64024)))
    (local.set $l11
      (i64.load
        (i32.const 64032)))
    (if $I2
      (i64.gt_u
        (local.get $l9)
        (local.get $l11))
      (then
        (return
          (i32.const 1))))
    (if $I3
      (i64.lt_u
        (local.get $l9)
        (local.get $l11))
      (then
        (return
          (i32.const -1))))
    (if $I4
      (i64.gt_u
        (local.get $l8)
        (local.get $l10))
      (then
        (return
          (i32.const 1))))
    (if $I5
      (i64.lt_u
        (local.get $l8)
        (local.get $l10))
      (then
        (return
          (i32.const -1))))
    (i32.const 0))
  (func $f11 (type $t8) (param $p0 i64) (param $p1 i64) (result i32)
    (local $l2 i32) (local $l3 i64) (local $l4 i64) (local $l5 i64) (local $l6 i32) (local $l7 i64)
    (local.set $l2
      (i32.const 64))
    (if $I0
      (i32.and
        (i64.eqz
          (local.get $p0))
        (i64.eqz
          (local.get $p1)))
      (then
        (i32.store8
          (i32.add
            (global.get $g0)
            (i32.const 63))
          (i32.const 48))
        (return
          (i32.const 1))))
    (block $B1
      (loop $L2
        (br_if $B1
          (i32.and
            (i64.eqz
              (local.get $p0))
            (i64.eqz
              (local.get $p1))))
        (local.set $l3
          (i64.const 0))
        (local.set $l4
          (i64.const 0))
        (local.set $l5
          (i64.const 0))
        (local.set $l6
          (i32.const 127))
        (block $B3
          (loop $L4
            (if $I5
              (i32.ge_u
                (local.get $l6)
                (i32.const 64))
              (then
                (local.set $l7
                  (i64.and
                    (i64.shr_u
                      (local.get $p1)
                      (i64.extend_i32_u
                        (i32.sub
                          (local.get $l6)
                          (i32.const 64))))
                    (i64.const 1))))
              (else
                (local.set $l7
                  (i64.and
                    (i64.shr_u
                      (local.get $p0)
                      (i64.extend_i32_u
                        (local.get $l6)))
                    (i64.const 1)))))
            (local.set $l5
              (i64.or
                (i64.shl
                  (local.get $l5)
                  (i64.const 1))
                (local.get $l7)))
            (if $I6
              (i64.ge_u
                (local.get $l5)
                (i64.const 10))
              (then
                (local.set $l5
                  (i64.sub
                    (local.get $l5)
                    (i64.const 10)))
                (if $I7
                  (i32.ge_u
                    (local.get $l6)
                    (i32.const 64))
                  (then
                    (local.set $l4
                      (i64.or
                        (local.get $l4)
                        (i64.shl
                          (i64.const 1)
                          (i64.extend_i32_u
                            (i32.sub
                              (local.get $l6)
                              (i32.const 64)))))))
                  (else
                    (local.set $l3
                      (i64.or
                        (local.get $l3)
                        (i64.shl
                          (i64.const 1)
                          (i64.extend_i32_u
                            (local.get $l6)))))))))
            (br_if $B3
              (i32.eqz
                (local.get $l6)))
            (local.set $l6
              (i32.sub
                (local.get $l6)
                (i32.const 1)))
            (br $L4)))
        (local.set $l2
          (i32.sub
            (local.get $l2)
            (i32.const 1)))
        (i32.store8
          (i32.add
            (global.get $g0)
            (local.get $l2))
          (i32.add
            (i32.wrap_i64
              (local.get $l5))
            (i32.const 48)))
        (local.set $p0
          (local.get $l3))
        (local.set $p1
          (local.get $l4))
        (br $L2)))
    (i32.sub
      (i32.const 64)
      (local.get $l2)))
  (func $wallet_decimal_format (export "wallet_decimal_format") (type $t9) (param $p0 i64) (param $p1 i64) (param $p2 i32) (param $p3 i32) (param $p4 i32) (result i64)
    (local $l5 i32) (local $l6 i32) (local $l7 i32) (local $l8 i32) (local $l9 i32)
    (if $I0
      (i32.gt_u
        (local.get $p2)
        (i32.const 38))
      (then
        (return
          (i64.const 3))))
    (local.set $l5
      (call $f11
        (local.get $p0)
        (local.get $p1)))
    (if $I1
      (i32.eqz
        (local.get $p2))
      (then
        (if $I2
          (i32.lt_u
            (local.get $p4)
            (local.get $l5))
          (then
            (return
              (i64.const 2))))
        (local.set $l8
          (i32.const 0))
        (block $B3
          (loop $L4
            (br_if $B3
              (i32.ge_u
                (local.get $l8)
                (local.get $l5)))
            (i32.store8
              (i32.add
                (local.get $p3)
                (local.get $l8))
              (i32.load8_u
                (i32.add
                  (i32.sub
                    (global.get $g1)
                    (local.get $l5))
                  (local.get $l8))))
            (local.set $l8
              (i32.add
                (local.get $l8)
                (i32.const 1)))
            (br $L4)))
        (return
          (i64.shl
            (i64.extend_i32_u
              (local.get $l5))
            (i64.const 32)))))
    (if $I5
      (i32.gt_u
        (local.get $l5)
        (local.get $p2))
      (then
        (local.set $l6
          (i32.sub
            (local.get $l5)
            (local.get $p2)))
        (local.set $l7
          (i32.add
            (i32.add
              (local.get $l6)
              (i32.const 1))
            (local.get $p2)))
        (if $I6
          (i32.lt_u
            (local.get $p4)
            (local.get $l7))
          (then
            (return
              (i64.const 2))))
        (local.set $l8
          (i32.const 0))
        (block $B7
          (loop $L8
            (br_if $B7
              (i32.ge_u
                (local.get $l8)
                (local.get $l6)))
            (i32.store8
              (i32.add
                (local.get $p3)
                (local.get $l8))
              (i32.load8_u
                (i32.add
                  (i32.sub
                    (global.get $g1)
                    (local.get $l5))
                  (local.get $l8))))
            (local.set $l8
              (i32.add
                (local.get $l8)
                (i32.const 1)))
            (br $L8)))
        (i32.store8
          (i32.add
            (local.get $p3)
            (local.get $l6))
          (i32.const 46))
        (local.set $l8
          (i32.const 0))
        (block $B9
          (loop $L10
            (br_if $B9
              (i32.ge_u
                (local.get $l8)
                (local.get $p2)))
            (i32.store8
              (i32.add
                (i32.add
                  (i32.add
                    (local.get $p3)
                    (local.get $l6))
                  (i32.const 1))
                (local.get $l8))
              (i32.load8_u
                (i32.add
                  (i32.add
                    (i32.sub
                      (global.get $g1)
                      (local.get $l5))
                    (local.get $l6))
                  (local.get $l8))))
            (local.set $l8
              (i32.add
                (local.get $l8)
                (i32.const 1)))
            (br $L10)))
        (return
          (i64.shl
            (i64.extend_i32_u
              (local.get $l7))
            (i64.const 32)))))
    (local.set $l9
      (i32.sub
        (local.get $p2)
        (local.get $l5)))
    (local.set $l7
      (i32.add
        (i32.add
          (i32.const 2)
          (local.get $l9))
        (local.get $l5)))
    (if $I11
      (i32.lt_u
        (local.get $p4)
        (local.get $l7))
      (then
        (return
          (i64.const 2))))
    (i32.store8
      (local.get $p3)
      (i32.const 48))
    (i32.store8
      (i32.add
        (local.get $p3)
        (i32.const 1))
      (i32.const 46))
    (local.set $l8
      (i32.const 0))
    (block $B12
      (loop $L13
        (br_if $B12
          (i32.ge_u
            (local.get $l8)
            (local.get $l9)))
        (i32.store8
          (i32.add
            (i32.add
              (local.get $p3)
              (i32.const 2))
            (local.get $l8))
          (i32.const 48))
        (local.set $l8
          (i32.add
            (local.get $l8)
            (i32.const 1)))
        (br $L13)))
    (local.set $l8
      (i32.const 0))
    (block $B14
      (loop $L15
        (br_if $B14
          (i32.ge_u
            (local.get $l8)
            (local.get $l5)))
        (i32.store8
          (i32.add
            (i32.add
              (i32.add
                (local.get $p3)
                (i32.const 2))
              (local.get $l9))
            (local.get $l8))
          (i32.load8_u
            (i32.add
              (i32.sub
                (global.get $g1)
                (local.get $l5))
              (local.get $l8))))
        (local.set $l8
          (i32.add
            (local.get $l8)
            (i32.const 1)))
        (br $L15)))
    (i64.shl
      (i64.extend_i32_u
        (local.get $l7))
      (i64.const 32)))
  (global $g0 i32 (i32.const 65000))
  (global $g1 i32 (i32.const 65064)))
