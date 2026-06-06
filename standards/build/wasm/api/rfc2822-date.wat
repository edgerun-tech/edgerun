(module
  (import "edgerun-core" "memory" (memory 1))
  (import "edgerun-core" "pack" (func $pack (param i32 i32) (result i64)))

  (func (export "proto_standard_id") (result i32)
    i32.const 300054)

  ;; Status values: 0 ok, 2 output_short, 4 overflow.
  ;; Return bits: low32=status, high32=written.


  (func $m157put2 (param $ptr i32) (param $value i32)
    (i32.store8 (local.get $ptr) (i32.add (i32.div_u (local.get $value) (i32.const 10)) (i32.const 48)))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.add (i32.rem_u (local.get $value) (i32.const 10)) (i32.const 48))))

  (func $m157put4 (param $ptr i32) (param $value i32)
    (i32.store8 (local.get $ptr) (i32.add (i32.rem_u (i32.div_u (local.get $value) (i32.const 1000)) (i32.const 10)) (i32.const 48)))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.add (i32.rem_u (i32.div_u (local.get $value) (i32.const 100)) (i32.const 10)) (i32.const 48)))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.add (i32.rem_u (i32.div_u (local.get $value) (i32.const 10)) (i32.const 10)) (i32.const 48)))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 3)) (i32.add (i32.rem_u (local.get $value) (i32.const 10)) (i32.const 48))))

  (func $put_day_name (param $ptr i32) (param $dow i32)
    (if (i32.eq (local.get $dow) (i32.const 0)) (then
      (i32.store8 (local.get $ptr) (i32.const 77)) (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 111)) (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 110)) (return)))
    (if (i32.eq (local.get $dow) (i32.const 1)) (then
      (i32.store8 (local.get $ptr) (i32.const 84)) (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 117)) (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 101)) (return)))
    (if (i32.eq (local.get $dow) (i32.const 2)) (then
      (i32.store8 (local.get $ptr) (i32.const 87)) (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 101)) (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 100)) (return)))
    (if (i32.eq (local.get $dow) (i32.const 3)) (then
      (i32.store8 (local.get $ptr) (i32.const 84)) (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 104)) (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 117)) (return)))
    (if (i32.eq (local.get $dow) (i32.const 4)) (then
      (i32.store8 (local.get $ptr) (i32.const 70)) (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 114)) (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 105)) (return)))
    (if (i32.eq (local.get $dow) (i32.const 5)) (then
      (i32.store8 (local.get $ptr) (i32.const 83)) (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 97)) (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 116)) (return)))
    (i32.store8 (local.get $ptr) (i32.const 83))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 117))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 110)))

  (func $put_month_name (param $ptr i32) (param $month i32)
    (if (i32.eq (local.get $month) (i32.const 1)) (then
      (i32.store8 (local.get $ptr) (i32.const 74)) (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 97)) (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 110)) (return)))
    (if (i32.eq (local.get $month) (i32.const 2)) (then
      (i32.store8 (local.get $ptr) (i32.const 70)) (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 101)) (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 98)) (return)))
    (if (i32.eq (local.get $month) (i32.const 3)) (then
      (i32.store8 (local.get $ptr) (i32.const 77)) (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 97)) (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 114)) (return)))
    (if (i32.eq (local.get $month) (i32.const 4)) (then
      (i32.store8 (local.get $ptr) (i32.const 65)) (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 112)) (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 114)) (return)))
    (if (i32.eq (local.get $month) (i32.const 5)) (then
      (i32.store8 (local.get $ptr) (i32.const 77)) (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 97)) (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 121)) (return)))
    (if (i32.eq (local.get $month) (i32.const 6)) (then
      (i32.store8 (local.get $ptr) (i32.const 74)) (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 117)) (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 110)) (return)))
    (if (i32.eq (local.get $month) (i32.const 7)) (then
      (i32.store8 (local.get $ptr) (i32.const 74)) (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 117)) (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 108)) (return)))
    (if (i32.eq (local.get $month) (i32.const 8)) (then
      (i32.store8 (local.get $ptr) (i32.const 65)) (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 117)) (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 103)) (return)))
    (if (i32.eq (local.get $month) (i32.const 9)) (then
      (i32.store8 (local.get $ptr) (i32.const 83)) (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 101)) (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 112)) (return)))
    (if (i32.eq (local.get $month) (i32.const 10)) (then
      (i32.store8 (local.get $ptr) (i32.const 79)) (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 99)) (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 116)) (return)))
    (if (i32.eq (local.get $month) (i32.const 11)) (then
      (i32.store8 (local.get $ptr) (i32.const 78)) (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 111)) (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 118)) (return)))
    (i32.store8 (local.get $ptr) (i32.const 68))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 101))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 99)))

  (func (export "rfc2822_format_utc")
    (param $unix_low i32) (param $unix_high i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $unix i64)
    (local $days i64)
    (local $rem i64)
    (local $hour i32)
    (local $minute i32)
    (local $second i32)
    (local $dow i32)
    (local $z i64)
    (local $era i64)
    (local $doe i64)
    (local $yoe i64)
    (local $y i64)
    (local $doy i64)
    (local $mp i64)
    (local $day i32)
    (local $month i32)
    (local $year i32)
    (if (i32.lt_u (local.get $out_cap) (i32.const 31))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))
    (local.set $unix
      (i64.or
        (i64.extend_i32_u (local.get $unix_low))
        (i64.shl (i64.extend_i32_u (local.get $unix_high)) (i64.const 32))))
    (local.set $days (i64.div_u (local.get $unix) (i64.const 86400)))
    (local.set $rem (i64.rem_u (local.get $unix) (i64.const 86400)))
    (local.set $hour (i32.wrap_i64 (i64.div_u (local.get $rem) (i64.const 3600))))
    (local.set $rem (i64.rem_u (local.get $rem) (i64.const 3600)))
    (local.set $minute (i32.wrap_i64 (i64.div_u (local.get $rem) (i64.const 60))))
    (local.set $second (i32.wrap_i64 (i64.rem_u (local.get $rem) (i64.const 60))))
    (local.set $dow (i32.wrap_i64 (i64.rem_u (i64.add (local.get $days) (i64.const 3)) (i64.const 7))))

    (local.set $z (i64.add (local.get $days) (i64.const 719468)))
    (local.set $era (i64.div_u (local.get $z) (i64.const 146097)))
    (local.set $doe (i64.sub (local.get $z) (i64.mul (local.get $era) (i64.const 146097))))
    (local.set $yoe
      (i64.div_u
        (i64.add
          (i64.sub
            (i64.sub (local.get $doe) (i64.div_u (local.get $doe) (i64.const 1460)))
            (i64.div_u (local.get $doe) (i64.const 36524)))
          (i64.div_u (local.get $doe) (i64.const 146096)))
        (i64.const 365)))
    (local.set $y (i64.add (local.get $yoe) (i64.mul (local.get $era) (i64.const 400))))
    (local.set $doy
      (i64.sub
        (local.get $doe)
        (i64.add
          (i64.sub (i64.mul (i64.const 365) (local.get $yoe)) (i64.div_u (local.get $yoe) (i64.const 100)))
          (i64.div_u (local.get $yoe) (i64.const 4)))))
    (local.set $mp (i64.div_u (i64.add (i64.mul (i64.const 5) (local.get $doy)) (i64.const 2)) (i64.const 153)))
    (local.set $day
      (i32.wrap_i64
        (i64.add
          (i64.sub
            (local.get $doy)
            (i64.div_u (i64.add (i64.mul (i64.const 153) (local.get $mp)) (i64.const 2)) (i64.const 5)))
          (i64.const 1))))
    (local.set $month
      (i32.wrap_i64
        (i64.add
          (local.get $mp)
          (select (i64.const 3) (i64.const -9) (i64.lt_u (local.get $mp) (i64.const 10))))))
    (local.set $y
      (i64.add
        (local.get $y)
        (select (i64.const 1) (i64.const 0) (i32.le_u (local.get $month) (i32.const 2)))))
    (if (i64.gt_u (local.get $y) (i64.const 9999))
      (then (return (call $pack (i32.const 4) (i32.const 0)))))
    (local.set $year (i32.wrap_i64 (local.get $y)))

    (call $put_day_name (local.get $out_ptr) (local.get $dow))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 3)) (i32.const 44))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 4)) (i32.const 32))
    (call $m157put2 (i32.add (local.get $out_ptr) (i32.const 5)) (local.get $day))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 7)) (i32.const 32))
    (call $put_month_name (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $month))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 11)) (i32.const 32))
    (call $m157put4 (i32.add (local.get $out_ptr) (i32.const 12)) (local.get $year))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 16)) (i32.const 32))
    (call $m157put2 (i32.add (local.get $out_ptr) (i32.const 17)) (local.get $hour))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 19)) (i32.const 58))
    (call $m157put2 (i32.add (local.get $out_ptr) (i32.const 20)) (local.get $minute))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 22)) (i32.const 58))
    (call $m157put2 (i32.add (local.get $out_ptr) (i32.const 23)) (local.get $second))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 25)) (i32.const 32))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 26)) (i32.const 43))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 27)) (i32.const 48))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 28)) (i32.const 48))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 29)) (i32.const 48))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 30)) (i32.const 48))
    (call $pack (i32.const 0) (i32.const 31)))
)