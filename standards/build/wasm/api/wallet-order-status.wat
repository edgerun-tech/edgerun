(module
  (type $t0 (func (result i32)))
  (type $t1 (func (param i32) (result i32)))
  (type $t2 (func (param i32 i32) (result i32)))
  (type $t3 (func (param i32 i32 i32) (result i64)))
  (import "edgerun-core" "memory" (memory $memory 1))
  (func $proto_standard_id (export "proto_standard_id") (type $t0) (result i32)
    (i32.const 300085))
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
  (data $d0 (i32.const 4096) "QUOTEDQUOTE_EXPIREDCREATEDAWAITING_DEPOSITDEPOSIT_SEENDEPOSIT_CONFIRMEDEXCHANGINGSENDINGCOMPLETEDACTION_REQUIREDREFUND_REQUIREDREFUNDINGREFUNDEDEXPIREDFAILEDREJECTEDON_HOLDCANCELEDPARTIAL_DEPOSITS"))
