(module
  (import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))
  (import "edgerun" "lo" (func $lo (param i64) (result i32)))
  (import "edgerun" "hi" (func $hi (param i64) (result i32)))
  (import "edgerun" "memcpy" (func $memcpy (param i32 i32 i32) (result i32)))
  (import "edgerun" "memset" (func $memset (param i32 i32 i32) (result i32)))
  (import "edgerun" "load8_u" (func $load8_u (param i32) (result i32)))
  (import "math" "min" (func $min (param i32 i32) (result i32)))
  (import "math" "max" (func $max (param i32 i32) (result i32)))
  (import "edgerun" "is_digit" (func $is_digit (param i32) (result i32)))
  (import "edgerun" "is_hex" (func $is_hex (param i32) (result i32)))
  (import "edgerun" "string_eq" (func $string_eq (param i32 i32 i32 i32) (result i32)))
  (import "edgerun" "strlen" (func $strlen (param i32 i32) (result i32)))
  (memory (export "memory") 1)
  (type $t0 (func (result i32)))
  (type $t1 (func (param i32) (result i32)))
  (type $t2 (func (param i32 i32) (result i32)))
  (type $t3 (func (param i32 i32 i32) (result i64)))
  (func $wallet_status_valid (export "wallet_status_valid") (type $t1) (param $p0 i32) (result i32)
    (i32.and
      (i32.ge_u
        (local.get $p0)
        (i32.const 1))
      (i32.le_u
        (local.get $p0)
        (i32.const 19))))
  (func $wallet_status_terminal (export "wallet_status_terminal") (type $t1) (param $p0 i32) (result i32)
    (if $I0
      (i32.eq
        (local.get $p0)
        (i32.const 9))
      (then
        (return
          (i32.const 1))))
    (if $I1
      (i32.eq
        (local.get $p0)
        (i32.const 13))
      (then
        (return
          (i32.const 1))))
    (if $I2
      (i32.eq
        (local.get $p0)
        (i32.const 14))
      (then
        (return
          (i32.const 1))))
    (if $I3
      (i32.eq
        (local.get $p0)
        (i32.const 15))
      (then
        (return
          (i32.const 1))))
    (if $I4
      (i32.eq
        (local.get $p0)
        (i32.const 16))
      (then
        (return
          (i32.const 1))))
    (if $I5
      (i32.eq
        (local.get $p0)
        (i32.const 18))
      (then
        (return
          (i32.const 1))))
    (i32.const 0))
  (func $f4 (type $t2) (param $p0 i32) (param $p1 i32) (result i32)
    (i32.or
      (i32.shl
        (local.get $p0)
        (i32.const 8))
      (local.get $p1)))
  (func $wallet_status_can_transition (export "wallet_status_can_transition") (type $t2) (param $p0 i32) (param $p1 i32) (result i32)
    (local $l2 i32)
    (if $I0
      (i32.eqz
        (call $wallet_status_valid
          (local.get $p0)))
      (then
        (return
          (i32.const 0))))
    (if $I1
      (i32.eqz
        (call $wallet_status_valid
          (local.get $p1)))
      (then
        (return
          (i32.const 0))))
    (if $I2
      (i32.eq
        (local.get $p0)
        (local.get $p1))
      (then
        (return
          (i32.const 1))))
    (if $I3
      (call $wallet_status_terminal
        (local.get $p0))
      (then
        (return
          (i32.const 0))))
    (local.set $l2
      (call $f4
        (local.get $p0)
        (local.get $p1)))
    (if $I4
      (i32.eq
        (local.get $l2)
        (i32.const 258))
      (then
        (return
          (i32.const 1))))
    (if $I5
      (i32.eq
        (local.get $l2)
        (i32.const 259))
      (then
        (return
          (i32.const 1))))
    (if $I6
      (i32.eq
        (local.get $l2)
        (i32.const 270))
      (then
        (return
          (i32.const 1))))
    (if $I7
      (i32.eq
        (local.get $l2)
        (i32.const 274))
      (then
        (return
          (i32.const 1))))
    (if $I8
      (i32.eq
        (local.get $l2)
        (i32.const 772))
      (then
        (return
          (i32.const 1))))
    (if $I9
      (i32.eq
        (local.get $l2)
        (i32.const 782))
      (then
        (return
          (i32.const 1))))
    (if $I10
      (i32.eq
        (local.get $l2)
        (i32.const 786))
      (then
        (return
          (i32.const 1))))
    (if $I11
      (i32.eq
        (local.get $l2)
        (i32.const 784))
      (then
        (return
          (i32.const 1))))
    (if $I12
      (i32.eq
        (local.get $l2)
        (i32.const 1029))
      (then
        (return
          (i32.const 1))))
    (if $I13
      (i32.eq
        (local.get $l2)
        (i32.const 1031))
      (then
        (return
          (i32.const 1))))
    (if $I14
      (i32.eq
        (local.get $l2)
        (i32.const 1038))
      (then
        (return
          (i32.const 1))))
    (if $I15
      (i32.eq
        (local.get $l2)
        (i32.const 1042))
      (then
        (return
          (i32.const 1))))
    (if $I16
      (i32.eq
        (local.get $l2)
        (i32.const 1035))
      (then
        (return
          (i32.const 1))))
    (if $I17
      (i32.eq
        (local.get $l2)
        (i32.const 1286))
      (then
        (return
          (i32.const 1))))
    (if $I18
      (i32.eq
        (local.get $l2)
        (i32.const 1287))
      (then
        (return
          (i32.const 1))))
    (if $I19
      (i32.eq
        (local.get $l2)
        (i32.const 1294))
      (then
        (return
          (i32.const 1))))
    (if $I20
      (i32.eq
        (local.get $l2)
        (i32.const 1543))
      (then
        (return
          (i32.const 1))))
    (if $I21
      (i32.eq
        (local.get $l2)
        (i32.const 1550))
      (then
        (return
          (i32.const 1))))
    (if $I22
      (i32.eq
        (local.get $l2)
        (i32.const 1800))
      (then
        (return
          (i32.const 1))))
    (if $I23
      (i32.eq
        (local.get $l2)
        (i32.const 1801))
      (then
        (return
          (i32.const 1))))
    (if $I24
      (i32.eq
        (local.get $l2)
        (i32.const 1809))
      (then
        (return
          (i32.const 1))))
    (if $I25
      (i32.eq
        (local.get $l2)
        (i32.const 1815))
      (then
        (return
          (i32.const 1))))
    (if $I26
      (i32.eq
        (local.get $l2)
        (i32.const 2057))
      (then
        (return
          (i32.const 1))))
    (if $I27
      (i32.eq
        (local.get $l2)
        (i32.const 2065))
      (then
        (return
          (i32.const 1))))
    (if $I28
      (i32.eq
        (local.get $l2)
        (i32.const 2063))
      (then
        (return
          (i32.const 1))))
    (if $I29
      (i32.eq
        (local.get $l2)
        (i32.const 2567))
      (then
        (return
          (i32.const 1))))
    (if $I30
      (i32.eq
        (local.get $l2)
        (i32.const 2568))
      (then
        (return
          (i32.const 1))))
    (if $I31
      (i32.eq
        (local.get $l2)
        (i32.const 2569))
      (then
        (return
          (i32.const 1))))
    (if $I32
      (i32.eq
        (local.get $l2)
        (i32.const 2578))
      (then
        (return
          (i32.const 1))))
    (if $I33
      (i32.eq
        (local.get $l2)
        (i32.const 2828))
      (then
        (return
          (i32.const 1))))
    (if $I34
      (i32.eq
        (local.get $l2)
        (i32.const 2829))
      (then
        (return
          (i32.const 1))))
    (if $I35
      (i32.eq
        (local.get $l2)
        (i32.const 3085))
      (then
        (return
          (i32.const 1))))
    (if $I36
      (i32.eq
        (local.get $l2)
        (i32.const 3087))
      (then
        (return
          (i32.const 1))))
    (if $I37
      (i32.eq
        (local.get $l2)
        (i32.const 4359))
      (then
        (return
          (i32.const 1))))
    (if $I38
      (i32.eq
        (local.get $l2)
        (i32.const 4360))
      (then
        (return
          (i32.const 1))))
    (if $I39
      (i32.eq
        (local.get $l2)
        (i32.const 4361))
      (then
        (return
          (i32.const 1))))
    (if $I40
      (i32.eq
        (local.get $l2)
        (i32.const 4367))
      (then
        (return
          (i32.const 1))))
    (if $I41
      (i32.eq
        (local.get $l2)
        (i32.const 4370))
      (then
        (return
          (i32.const 1))))
    (if $I42
      (i32.eq
        (local.get $l2)
        (i32.const 4869))
      (then
        (return
          (i32.const 1))))
    (if $I43
      (i32.eq
        (local.get $l2)
        (i32.const 4871))
      (then
        (return
          (i32.const 1))))
    (if $I44
      (i32.eq
        (local.get $l2)
        (i32.const 4875))
      (then
        (return
          (i32.const 1))))
    (i32.const 0))
  (func $f6 (type $t1) (param $p0 i32) (result i32)
    (if $I0
      (i32.eq
        (local.get $p0)
        (i32.const 1))
      (then
        (return
          (i32.const 4096))))
    (if $I1
      (i32.eq
        (local.get $p0)
        (i32.const 2))
      (then
        (return
          (i32.const 4102))))
    (if $I2
      (i32.eq
        (local.get $p0)
        (i32.const 3))
      (then
        (return
          (i32.const 4115))))
    (if $I3
      (i32.eq
        (local.get $p0)
        (i32.const 4))
      (then
        (return
          (i32.const 4122))))
    (if $I4
      (i32.eq
        (local.get $p0)
        (i32.const 5))
      (then
        (return
          (i32.const 4138))))
    (if $I5
      (i32.eq
        (local.get $p0)
        (i32.const 6))
      (then
        (return
          (i32.const 4150))))
    (if $I6
      (i32.eq
        (local.get $p0)
        (i32.const 7))
      (then
        (return
          (i32.const 4167))))
    (if $I7
      (i32.eq
        (local.get $p0)
        (i32.const 8))
      (then
        (return
          (i32.const 4177))))
    (if $I8
      (i32.eq
        (local.get $p0)
        (i32.const 9))
      (then
        (return
          (i32.const 4184))))
    (if $I9
      (i32.eq
        (local.get $p0)
        (i32.const 10))
      (then
        (return
          (i32.const 4193))))
    (if $I10
      (i32.eq
        (local.get $p0)
        (i32.const 11))
      (then
        (return
          (i32.const 4208))))
    (if $I11
      (i32.eq
        (local.get $p0)
        (i32.const 12))
      (then
        (return
          (i32.const 4223))))
    (if $I12
      (i32.eq
        (local.get $p0)
        (i32.const 13))
      (then
        (return
          (i32.const 4232))))
    (if $I13
      (i32.eq
        (local.get $p0)
        (i32.const 14))
      (then
        (return
          (i32.const 4240))))
    (if $I14
      (i32.eq
        (local.get $p0)
        (i32.const 15))
      (then
        (return
          (i32.const 4247))))
    (if $I15
      (i32.eq
        (local.get $p0)
        (i32.const 16))
      (then
        (return
          (i32.const 4253))))
    (if $I16
      (i32.eq
        (local.get $p0)
        (i32.const 17))
      (then
        (return
          (i32.const 4261))))
    (if $I17
      (i32.eq
        (local.get $p0)
        (i32.const 18))
      (then
        (return
          (i32.const 4268))))
    (if $I18
      (i32.eq
        (local.get $p0)
        (i32.const 19))
      (then
        (return
          (i32.const 4276))))
    (i32.const 0))
  (func $f7 (type $t1) (param $p0 i32) (result i32)
    (if $I0
      (i32.eq
        (local.get $p0)
        (i32.const 1))
      (then
        (return
          (i32.const 6))))
    (if $I1
      (i32.eq
        (local.get $p0)
        (i32.const 2))
      (then
        (return
          (i32.const 13))))
    (if $I2
      (i32.eq
        (local.get $p0)
        (i32.const 3))
      (then
        (return
          (i32.const 7))))
    (if $I3
      (i32.eq
        (local.get $p0)
        (i32.const 4))
      (then
        (return
          (i32.const 16))))
    (if $I4
      (i32.eq
        (local.get $p0)
        (i32.const 5))
      (then
        (return
          (i32.const 12))))
    (if $I5
      (i32.eq
        (local.get $p0)
        (i32.const 6))
      (then
        (return
          (i32.const 17))))
    (if $I6
      (i32.eq
        (local.get $p0)
        (i32.const 7))
      (then
        (return
          (i32.const 10))))
    (if $I7
      (i32.eq
        (local.get $p0)
        (i32.const 8))
      (then
        (return
          (i32.const 7))))
    (if $I8
      (i32.eq
        (local.get $p0)
        (i32.const 9))
      (then
        (return
          (i32.const 9))))
    (if $I9
      (i32.eq
        (local.get $p0)
        (i32.const 10))
      (then
        (return
          (i32.const 15))))
    (if $I10
      (i32.eq
        (local.get $p0)
        (i32.const 11))
      (then
        (return
          (i32.const 15))))
    (if $I11
      (i32.eq
        (local.get $p0)
        (i32.const 12))
      (then
        (return
          (i32.const 9))))
    (if $I12
      (i32.eq
        (local.get $p0)
        (i32.const 13))
      (then
        (return
          (i32.const 8))))
    (if $I13
      (i32.eq
        (local.get $p0)
        (i32.const 14))
      (then
        (return
          (i32.const 7))))
    (if $I14
      (i32.eq
        (local.get $p0)
        (i32.const 15))
      (then
        (return
          (i32.const 6))))
    (if $I15
      (i32.eq
        (local.get $p0)
        (i32.const 16))
      (then
        (return
          (i32.const 8))))
    (if $I16
      (i32.eq
        (local.get $p0)
        (i32.const 17))
      (then
        (return
          (i32.const 7))))
    (if $I17
      (i32.eq
        (local.get $p0)
        (i32.const 18))
      (then
        (return
          (i32.const 8))))
    (if $I18
      (i32.eq
        (local.get $p0)
        (i32.const 19))
      (then
        (return
          (i32.const 16))))
    (i32.const 0))
  (func $wallet_status_label (export "wallet_status_label") (type $t3) (param $p0 i32) (param $p1 i32) (param $p2 i32) (result i64)
    (local $l3 i32) (local $l4 i32) (local $l5 i32)
    (if $I0
      (i32.eqz
        (call $wallet_status_valid
          (local.get $p0)))
      (then
        (return
          (i64.const 3))))
    (local.set $l3
      (call $f6
        (local.get $p0)))
    (local.set $l4
      (call $f7
        (local.get $p0)))
    (if $I1
      (i32.lt_u
        (local.get $p2)
        (local.get $l4))
      (then
        (return
          (i64.const 2))))
    (local.set $l5
      (i32.const 0))
    (block $B2
      (loop $L3
        (br_if $B2
          (i32.ge_u
            (local.get $l5)
            (local.get $l4)))
        (i32.store8
          (i32.add
            (local.get $p1)
            (local.get $l5))
          (i32.load8_u
            (i32.add
              (local.get $l3)
              (local.get $l5))))
        (local.set $l5
          (i32.add
            (local.get $l5)
            (i32.const 1)))
        (br $L3)))
    (i64.shl
      (i64.extend_i32_u
        (local.get $l4))
      (i64.const 32)))
  (type $t10 (func (result i32)))
  (type $t11 (func (param i32 i64 i64 i32)))
  (type $t12 (func (param i64 i64 i32 i32) (result i32)))
  (type $t13 (func (param i64 i64 i32 i32 i32) (result i32)))
  (type $t14 (func (param i64 i64 i32)))
  (type $t15 (func (param i32 i32 i32) (result i32)))
  (type $t16 (func (param i64 i64 i32 i64 i64 i32 i32) (result i32)))
  (type $t17 (func (param i64 i64 i32 i64 i64 i32) (result i32)))
  (type $t18 (func (param i64 i64) (result i32)))
  (type $t19 (func (param i64 i64 i32 i32 i32) (result i64)))
  (func $f2 (type $t11) (param $p0 i32) (param $p1 i64) (param $p2 i64) (param $p3 i32)
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
  (func $f3 (type $t12) (param $p0 i64) (param $p1 i64) (param $p2 i32) (param $p3 i32) (result i32)
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
  (func $f4 (type $t13) (param $p0 i64) (param $p1 i64) (param $p2 i32) (param $p3 i32) (param $p4 i32) (result i32)
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
  (func $f5 (type $t14) (param $p0 i64) (param $p1 i64) (param $p2 i32)
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
  (func $wallet_decimal_parse (export "wallet_decimal_parse") (type $t15) (param $p0 i32) (param $p1 i32) (param $p2 i32) (result i32)
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
  (func $wallet_decimal_normalize (export "wallet_decimal_normalize") (type $t13) (param $p0 i64) (param $p1 i64) (param $p2 i32) (param $p3 i32) (param $p4 i32) (result i32)
    (call $f4
      (local.get $p0)
      (local.get $p1)
      (local.get $p2)
      (local.get $p3)
      (local.get $p4)))
  (func $wallet_decimal_add (export "wallet_decimal_add") (type $t16) (param $p0 i64) (param $p1 i64) (param $p2 i32) (param $p3 i64) (param $p4 i64) (param $p5 i32) (param $p6 i32) (result i32)
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
  (func $wallet_decimal_sub (export "wallet_decimal_sub") (type $t16) (param $p0 i64) (param $p1 i64) (param $p2 i32) (param $p3 i64) (param $p4 i64) (param $p5 i32) (param $p6 i32) (result i32)
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
  (func $wallet_decimal_compare (export "wallet_decimal_compare") (type $t17) (param $p0 i64) (param $p1 i64) (param $p2 i32) (param $p3 i64) (param $p4 i64) (param $p5 i32) (result i32)
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
  (func $f11 (type $t18) (param $p0 i64) (param $p1 i64) (result i32)
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
  (func $wallet_decimal_format (export "wallet_decimal_format") (type $t19) (param $p0 i64) (param $p1 i64) (param $p2 i32) (param $p3 i32) (param $p4 i32) (result i64)
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
  (global $g1 i32 (i32.const 65064))
;; Wallet settlement and exec-runner semantics plundered from edgerun-wallet and edgerun-exec-runner.

  (func (export "wallet_exec_abi_version") (result i32) i32.const 1)
  (func (export "wallet_core_abi_version") (result i32) i32.const 1)
  (func (export "exec_runner_version") (result i32) i32.const 1)
  (func (export "exec_runner_magic") (result i32) i32.const 0x52585245) ;; ERXR little-endian word
  (func (export "exec_max_program_size") (result i64) i64.const 536870912)
  (func (export "exec_max_blob_size") (result i64) i64.const 536870912)

  (func (export "wallet_chain_family_valid") (param $family i32) (result i32)
    ;; EdgeRun, BitcoinLike, EvmLike, SolanaLike, TronLike, Other.
    (if (i32.or (i32.and (i32.ge_u (local.get $family) (i32.const 1)) (i32.le_u (local.get $family) (i32.const 5))) (i32.eq (local.get $family) (i32.const 255))) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "wallet_is_edgerun_chain") (param $family i32) (result i32)
    (if (i32.eq (local.get $family) (i32.const 1)) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "wallet_amount_is_zero") (param $mantissa_hi i64) (param $mantissa_lo i64) (result i32)
    (if (i64.eqz (i64.or (local.get $mantissa_hi) (local.get $mantissa_lo))) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "canonical_asset_id_parts") (param $has_contract i32) (result i32)
    ;; 2 parts: SYMBOL:network, 3 with contract.
    (if (local.get $has_contract) (then (return (i32.const 3))))
    i32.const 2)

  (func (export "wallet_event_apply_result")
    (param $event_kind i32) (param $amount_zero i32) (param $duplicate_claim i32)
    (param $duplicate_observation i32) (param $has_account i32) (param $sufficient_balance i32)
    (result i32)
    ;; event: 1 intent, 2 external observed, 3 emission claimed, 4 emission finalized, 5 burn claimed, 6 rejected.
    ;; 0 ok, 1 duplicate claim, 2 duplicate observation, 3 unknown account, 4 insufficient balance, 5 zero amount.
    (if (i32.eq (local.get $event_kind) (i32.const 2))
      (then
        (if (local.get $duplicate_observation) (then (return (i32.const 2))))
        (return (i32.const 0))))
    (if (i32.eq (local.get $event_kind) (i32.const 4))
      (then
        (if (local.get $amount_zero) (then (return (i32.const 5))))
        (if (local.get $duplicate_claim) (then (return (i32.const 1))))
        (return (i32.const 0))))
    (if (i32.eq (local.get $event_kind) (i32.const 5))
      (then
        (if (local.get $amount_zero) (then (return (i32.const 5))))
        (if (i32.eqz (local.get $has_account)) (then (return (i32.const 3))))
        (if (i32.eqz (local.get $sufficient_balance)) (then (return (i32.const 4))))
        (return (i32.const 0))))
    i32.const 0)

  (func (export "wallet_credit_result") (param $amount_zero i32) (param $overflow i32) (result i32)
    ;; 0 ok, 5 zero amount, 6 overflow.
    (if (local.get $amount_zero) (then (return (i32.const 5))))
    (if (local.get $overflow) (then (return (i32.const 6))))
    i32.const 0)

  (func (export "wallet_debit_result") (param $amount_zero i32) (param $has_account i32) (param $sufficient_balance i32) (result i32)
    ;; 0 ok, 3 unknown account, 4 insufficient balance, 5 zero amount.
    (if (local.get $amount_zero) (then (return (i32.const 5))))
    (if (i32.eqz (local.get $has_account)) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $sufficient_balance)) (then (return (i32.const 4))))
    i32.const 0)

  (func (export "wallet_emission_evidence_result")
    (param $amount_zero i32) (param $admission_ok i32) (param $receipt_ok i32) (param $notary_ok i32)
    (param $claim_hashes_match i32) (param $receipt_binds_admission i32) (param $notary_binds_admission i32)
    (result i32)
    ;; 0 ok, 1 invalid admission, 2 invalid receipt, 3 invalid notary, 4 proof mismatch, 5 zero amount.
    (if (local.get $amount_zero) (then (return (i32.const 5))))
    (if (i32.eqz (local.get $admission_ok)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $receipt_ok)) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $notary_ok)) (then (return (i32.const 3))))
    (if (i32.eqz (i32.and (i32.and (local.get $claim_hashes_match) (local.get $receipt_binds_admission)) (local.get $notary_binds_admission))) (then (return (i32.const 4))))
    i32.const 0)

  (func (export "wallet_preimage_domain_code") (param $kind i32) (result i32)
    ;; 1 chain, 2 asset, 3 account, 4 address, 5 transfer, 6 observation, 7 emission claim.
    (if (i32.and (i32.ge_u (local.get $kind) (i32.const 1)) (i32.le_u (local.get $kind) (i32.const 7))) (then (return (local.get $kind))))
    i32.const 0)

  (func (export "exec_msg_valid") (param $msg i32) (result i32)
    ;; common 1,10..14, legacy exec 2..4, CAS 20..25.
    (if (i32.eq (local.get $msg) (i32.const 1)) (then (return (i32.const 1))))
    (if (i32.and (i32.ge_u (local.get $msg) (i32.const 2)) (i32.le_u (local.get $msg) (i32.const 4))) (then (return (i32.const 1))))
    (if (i32.and (i32.ge_u (local.get $msg) (i32.const 10)) (i32.le_u (local.get $msg) (i32.const 14))) (then (return (i32.const 1))))
    (if (i32.and (i32.ge_u (local.get $msg) (i32.const 20)) (i32.le_u (local.get $msg) (i32.const 25))) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "exec_read_frame_result") (param $magic_ok i32) (param $version i32) (result i32)
    ;; 0 ok, 1 bad magic, 2 bad version.
    (if (i32.eqz (local.get $magic_ok)) (then (return (i32.const 1))))
    (if (i32.ne (local.get $version) (i32.const 1)) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "exec_send_frame_result") (param $payload_len_hi i32) (result i32)
    ;; payload must fit u32.
    (if (i32.ne (local.get $payload_len_hi) (i32.const 0)) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "exec_begin_result") (param $payload_len i32) (param $size i64) (param $argc i32) (result i32)
    ;; 0 ok, 1 truncated fixed fields, 2 program too large. Empty argv defaults to ./program.
    (if (i32.lt_u (local.get $payload_len) (i32.const 84)) (then (return (i32.const 1))))
    (if (i64.gt_u (local.get $size) (i64.const 536870912)) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "exec_argv_count_after_parse") (param $argc i32) (result i32)
    (if (i32.eqz (local.get $argc)) (then (return (i32.const 1))))
    local.get $argc)

  (func (export "exec_job_chunk_result") (param $msg_type i32) (param $chunk_len i64) (param $remaining i64) (result i32)
    ;; 0 ok, 1 expected chunk, 2 chunk exceeds declared size.
    (if (i32.ne (local.get $msg_type) (i32.const 3)) (then (return (i32.const 1))))
    (if (i64.gt_u (local.get $chunk_len) (local.get $remaining)) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "exec_job_end_result") (param $msg_type i32) (result i32)
    ;; 0 ok, 1 expected EXEC_END.
    (if (i32.ne (local.get $msg_type) (i32.const 4)) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "exec_exit_code") (param $status_has_code i32) (param $code i32) (param $signal i32) (param $timed_out i32) (result i32)
    (if (local.get $timed_out) (then (return (i32.const 124))))
    (if (local.get $status_has_code) (then (return (local.get $code))))
    (i32.add (i32.const 128) (local.get $signal)))

  (func (export "exec_pipe_msg_type") (param $is_stderr i32) (result i32)
    (if (local.get $is_stderr) (then (return (i32.const 11))))
    i32.const 10)

  (func (export "cas_root_kind") (param $kind i32) (result i32)
    ;; 1 blobs/sha256, 2 jobs.
    (if (i32.and (i32.ge_u (local.get $kind) (i32.const 1)) (i32.le_u (local.get $kind) (i32.const 2))) (then (return (local.get $kind))))
    i32.const 0)

  (func (export "cas_put_begin_result") (param $payload_len i32) (param $size i64) (result i32)
    ;; 0 ok, 1 truncated fixed fields, 2 blob too large.
    (if (i32.lt_u (local.get $payload_len) (i32.const 44)) (then (return (i32.const 1))))
    (if (i64.gt_u (local.get $size) (i64.const 536870912)) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "cas_put_result") (param $blob_exists i32) (param $hash_matches i32) (result i32)
    ;; 23 PUT_OK, 25 BLOB_EXISTS, 13 ERROR.
    (if (local.get $blob_exists) (then (return (i32.const 25))))
    (if (i32.eqz (local.get $hash_matches)) (then (return (i32.const 13))))
    i32.const 23)

  (func (export "cas_put_chunk_result") (param $msg_type i32) (param $chunk_len i64) (param $remaining i64) (result i32)
    ;; 0 ok, 1 expected PUT_CHUNK, 2 chunk exceeds declared size.
    (if (i32.ne (local.get $msg_type) (i32.const 21)) (then (return (i32.const 1))))
    (if (i64.gt_u (local.get $chunk_len) (local.get $remaining)) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "cas_put_end_result") (param $msg_type i32) (result i32)
    (if (i32.ne (local.get $msg_type) (i32.const 22)) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "cas_exec_result") (param $blob_exists i32) (result i32)
    ;; 0 execute, 13 send error.
    (if (i32.eqz (local.get $blob_exists)) (then (return (i32.const 13))))
    i32.const 0)

  (func (export "hex32_result") (param $len i32) (param $all_hex i32) (result i32)
    ;; 0 ok, 1 wrong length, 2 invalid hex.
    (if (i32.ne (local.get $len) (i32.const 64)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $all_hex)) (then (return (i32.const 2))))
    i32.const 0)

  (data $d0 (i32.const 4096) "QUOTEDQUOTE_EXPIREDCREATEDAWAITING_DEPOSITDEPOSIT_SEENDEPOSIT_CONFIRMEDEXCHANGINGSENDINGCOMPLETEDACTION_REQUIREDREFUND_REQUIREDREFUNDINGREFUNDEDEXPIREDFAILEDREJECTEDON_HOLDCANCELEDPARTIAL_DEPOSITS")
)
