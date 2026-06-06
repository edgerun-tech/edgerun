
  ;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid, 4 overflow.
  ;; Parse record, little-endian u32/i32 fields:
  ;; 0 year, 4 month, 8 day, 12 hour, 16 minute, 20 second,
  ;; 24 nanos, 28 signed offset_minutes, 32 unix_seconds_lo, 36 unix_seconds_hi.

  (func $m177digit (param $ptr i32) (param $end i32) (result i32)
    (local $b i32)
    (if (i32.ge_u (local.get $ptr) (local.get $end))
      (then (return (i32.const -1))))
    (local.set $b (i32.load8_u (local.get $ptr)))
    (if (i32.or (i32.lt_u (local.get $b) (i32.const 48)) (i32.gt_u (local.get $b) (i32.const 57)))
      (then (return (i32.const -1))))
    (i32.sub (local.get $b) (i32.const 48)))

  (func $m177two (param $ptr i32) (param $end i32) (result i32)
    (local $a i32)
    (local $b i32)
    (local.set $a (call $m177digit (local.get $ptr) (local.get $end)))
    (if (i32.lt_s (local.get $a) (i32.const 0)) (then (return (i32.const -1))))
    (local.set $b (call $m177digit (i32.add (local.get $ptr) (i32.const 1)) (local.get $end)))
    (if (i32.lt_s (local.get $b) (i32.const 0)) (then (return (i32.const -1))))
    (i32.add (i32.mul (local.get $a) (i32.const 10)) (local.get $b)))

  (func $m177four (param $ptr i32) (param $end i32) (result i32)
    (local $a i32)
    (local $b i32)
    (local.set $a (call $m177two (local.get $ptr) (local.get $end)))
    (if (i32.lt_s (local.get $a) (i32.const 0)) (then (return (i32.const -1))))
    (local.set $b (call $m177two (i32.add (local.get $ptr) (i32.const 2)) (local.get $end)))
    (if (i32.lt_s (local.get $b) (i32.const 0)) (then (return (i32.const -1))))
    (i32.add (i32.mul (local.get $a) (i32.const 100)) (local.get $b)))

  (func $m177is_leap (param $year i32) (result i32)
    (if (i32.ne (i32.rem_u (local.get $year) (i32.const 4)) (i32.const 0))
      (then (return (i32.const 0))))
    (if (i32.ne (i32.rem_u (local.get $year) (i32.const 100)) (i32.const 0))
      (then (return (i32.const 1))))
    (select
      (i32.const 1)
      (i32.const 0)
      (i32.eq (i32.rem_u (local.get $year) (i32.const 400)) (i32.const 0))))

  (func $m177month_days (param $year i32) (param $month i32) (result i32)
    (if (i32.eq (local.get $month) (i32.const 2))
      (then
        (return (select (i32.const 29) (i32.const 28) (call $m177is_leap (local.get $year))))))
    (if
      (i32.or
        (i32.or
          (i32.or
            (i32.eq (local.get $month) (i32.const 1))
            (i32.eq (local.get $month) (i32.const 3)))
          (i32.or
            (i32.eq (local.get $month) (i32.const 5))
            (i32.eq (local.get $month) (i32.const 7))))
        (i32.or
          (i32.or
            (i32.eq (local.get $month) (i32.const 8))
            (i32.eq (local.get $month) (i32.const 10)))
          (i32.eq (local.get $month) (i32.const 12))))
      (then (return (i32.const 31))))
    i32.const 30)

  (func $m177valid_fields
    (param $year i32) (param $month i32) (param $day i32) (param $hour i32) (param $minute i32) (param $second i32)
    (result i32)
    (if (i32.eqz (local.get $year)) (then (return (i32.const 0))))
    (if (i32.or (i32.lt_u (local.get $month) (i32.const 1)) (i32.gt_u (local.get $month) (i32.const 12)))
      (then (return (i32.const 0))))
    (if
      (i32.or
        (i32.lt_u (local.get $day) (i32.const 1))
        (i32.gt_u (local.get $day) (call $m177month_days (local.get $year) (local.get $month))))
      (then (return (i32.const 0))))
    (if (i32.gt_u (local.get $hour) (i32.const 23)) (then (return (i32.const 0))))
    (if (i32.gt_u (local.get $minute) (i32.const 59)) (then (return (i32.const 0))))
    (if (i32.gt_u (local.get $second) (i32.const 59)) (then (return (i32.const 0))))
    i32.const 1)

  (func $m177days_before_year (param $year i32) (result i64)
    (local $y i64)
    (local.set $y (i64.extend_i32_s (i32.sub (local.get $year) (i32.const 1))))
    (i64.add
      (i64.sub
        (i64.add
          (i64.mul (local.get $y) (i64.const 365))
          (i64.div_s (local.get $y) (i64.const 4)))
        (i64.div_s (local.get $y) (i64.const 100)))
      (i64.div_s (local.get $y) (i64.const 400))))

  (func $m177days_since_epoch (param $year i32) (param $month i32) (param $day i32) (result i64)
    (local $m i32)
    (local $days i64)
    (local.set $days
      (i64.sub
        (call $m177days_before_year (local.get $year))
        (i64.const 719162)))
    (local.set $m (i32.const 1))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $m) (local.get $month)))
        (local.set $days
          (i64.add (local.get $days) (i64.extend_i32_u (call $m177month_days (local.get $year) (local.get $m)))))
        (local.set $m (i32.add (local.get $m) (i32.const 1)))
        (br $loop)))
    (i64.add (local.get $days) (i64.extend_i32_u (i32.sub (local.get $day) (i32.const 1)))))

  (func $m177unix_seconds
    (param $year i32) (param $month i32) (param $day i32) (param $hour i32) (param $minute i32) (param $second i32) (param $offset_minutes i32)
    (result i64)
    (i64.sub
      (i64.add
        (i64.add
          (i64.mul (call $m177days_since_epoch (local.get $year) (local.get $month) (local.get $day)) (i64.const 86400))
          (i64.mul (i64.extend_i32_u (local.get $hour)) (i64.const 3600)))
        (i64.add
          (i64.mul (i64.extend_i32_u (local.get $minute)) (i64.const 60))
          (i64.extend_i32_u (local.get $second))))
      (i64.mul (i64.extend_i32_s (local.get $offset_minutes)) (i64.const 60))))

  (func $m177put2 (param $ptr i32) (param $value i32)
    (i32.store8 (local.get $ptr) (i32.add (i32.div_u (local.get $value) (i32.const 10)) (i32.const 48)))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.add (i32.rem_u (local.get $value) (i32.const 10)) (i32.const 48))))

  (func $m177put4 (param $ptr i32) (param $value i32)
    (i32.store8 (local.get $ptr) (i32.add (i32.rem_u (i32.div_u (local.get $value) (i32.const 1000)) (i32.const 10)) (i32.const 48)))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.add (i32.rem_u (i32.div_u (local.get $value) (i32.const 100)) (i32.const 10)) (i32.const 48)))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.add (i32.rem_u (i32.div_u (local.get $value) (i32.const 10)) (i32.const 10)) (i32.const 48)))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 3)) (i32.add (i32.rem_u (local.get $value) (i32.const 10)) (i32.const 48))))

  (func (export "rfc3339_parse") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $end i32)
    (local $year i32)
    (local $month i32)
    (local $day i32)
    (local $hour i32)
    (local $minute i32)
    (local $second i32)
    (local $nanos i32)
    (local $pos i32)
    (local $m177digits i32)
    (local $d i32)
    (local $tz i32)
    (local $sign i32)
    (local $tz_h i32)
    (local $tz_m i32)
    (local $unix i64)
    (if (i32.lt_u (local.get $len) (i32.const 20)) (then (return (i32.const 1))))
    (local.set $end (i32.add (local.get $ptr) (local.get $len)))
    (local.set $year (call $m177four (local.get $ptr) (local.get $end)))
    (local.set $month (call $m177two (i32.add (local.get $ptr) (i32.const 5)) (local.get $end)))
    (local.set $day (call $m177two (i32.add (local.get $ptr) (i32.const 8)) (local.get $end)))
    (local.set $hour (call $m177two (i32.add (local.get $ptr) (i32.const 11)) (local.get $end)))
    (local.set $minute (call $m177two (i32.add (local.get $ptr) (i32.const 14)) (local.get $end)))
    (local.set $second (call $m177two (i32.add (local.get $ptr) (i32.const 17)) (local.get $end)))
    (if
      (i32.or
        (i32.or
          (i32.or
            (i32.or (i32.lt_s (local.get $year) (i32.const 0)) (i32.lt_s (local.get $month) (i32.const 0)))
            (i32.or (i32.lt_s (local.get $day) (i32.const 0)) (i32.lt_s (local.get $hour) (i32.const 0))))
          (i32.or (i32.lt_s (local.get $minute) (i32.const 0)) (i32.lt_s (local.get $second) (i32.const 0))))
        (i32.or
          (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 4))) (i32.const 45))
          (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 7))) (i32.const 45))))
      (then (return (i32.const 3))))
    (if
      (i32.or
        (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 10))) (i32.const 84))
        (i32.or
          (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 13))) (i32.const 58))
          (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 16))) (i32.const 58))))
      (then (return (i32.const 3))))
    (if (i32.eqz (call $m177valid_fields (local.get $year) (local.get $month) (local.get $day) (local.get $hour) (local.get $minute) (local.get $second)))
      (then (return (i32.const 3))))

    (local.set $pos (i32.add (local.get $ptr) (i32.const 19)))
    (if (i32.and (i32.lt_u (local.get $pos) (local.get $end)) (i32.eq (i32.load8_u (local.get $pos)) (i32.const 46)))
      (then
        (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
        (local.set $m177digits (i32.const 0))
        (block $frac_done
          (loop $frac
            (br_if $frac_done (i32.ge_u (local.get $pos) (local.get $end)))
            (local.set $d (call $m177digit (local.get $pos) (local.get $end)))
            (br_if $frac_done (i32.lt_s (local.get $d) (i32.const 0)))
            (if (i32.ge_u (local.get $m177digits) (i32.const 9)) (then (return (i32.const 3))))
            (local.set $nanos (i32.add (i32.mul (local.get $nanos) (i32.const 10)) (local.get $d)))
            (local.set $m177digits (i32.add (local.get $m177digits) (i32.const 1)))
            (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
            (br $frac)))
        (if (i32.eqz (local.get $m177digits)) (then (return (i32.const 3))))
        (block $scale_done
          (loop $scale
            (br_if $scale_done (i32.ge_u (local.get $m177digits) (i32.const 9)))
            (local.set $nanos (i32.mul (local.get $nanos) (i32.const 10)))
            (local.set $m177digits (i32.add (local.get $m177digits) (i32.const 1)))
            (br $scale)))))

    (if (i32.ge_u (local.get $pos) (local.get $end)) (then (return (i32.const 1))))
    (local.set $tz (i32.load8_u (local.get $pos)))
    (if (i32.eq (local.get $tz) (i32.const 90))
      (then
        (if (i32.ne (i32.add (local.get $pos) (i32.const 1)) (local.get $end)) (then (return (i32.const 3))))
        (local.set $tz (i32.const 0)))
      (else
        (if (i32.and (i32.ne (local.get $tz) (i32.const 43)) (i32.ne (local.get $tz) (i32.const 45)))
          (then (return (i32.const 3))))
        (if (i32.ne (i32.add (local.get $pos) (i32.const 6)) (local.get $end)) (then (return (i32.const 3))))
        (local.set $sign (select (i32.const -1) (i32.const 1) (i32.eq (local.get $tz) (i32.const 45))))
        (local.set $tz_h (call $m177two (i32.add (local.get $pos) (i32.const 1)) (local.get $end)))
        (local.set $tz_m (call $m177two (i32.add (local.get $pos) (i32.const 4)) (local.get $end)))
        (if
          (i32.or
            (i32.or (i32.lt_s (local.get $tz_h) (i32.const 0)) (i32.lt_s (local.get $tz_m) (i32.const 0)))
            (i32.or
              (i32.ne (i32.load8_u (i32.add (local.get $pos) (i32.const 3))) (i32.const 58))
              (i32.or (i32.gt_u (local.get $tz_h) (i32.const 23)) (i32.gt_u (local.get $tz_m) (i32.const 59)))))
          (then (return (i32.const 3))))
        (local.set $tz (i32.mul (local.get $sign) (i32.add (i32.mul (local.get $tz_h) (i32.const 60)) (local.get $tz_m))))))

    (local.set $unix
      (call $m177unix_seconds
        (local.get $year) (local.get $month) (local.get $day)
        (local.get $hour) (local.get $minute) (local.get $second)
        (local.get $tz)))
    (i32.store (local.get $out) (local.get $year))
    (i32.store (i32.add (local.get $out) (i32.const 4)) (local.get $month))
    (i32.store (i32.add (local.get $out) (i32.const 8)) (local.get $day))
    (i32.store (i32.add (local.get $out) (i32.const 12)) (local.get $hour))
    (i32.store (i32.add (local.get $out) (i32.const 16)) (local.get $minute))
    (i32.store (i32.add (local.get $out) (i32.const 20)) (local.get $second))
    (i32.store (i32.add (local.get $out) (i32.const 24)) (local.get $nanos))
    (i32.store (i32.add (local.get $out) (i32.const 28)) (local.get $tz))
    (i32.store (i32.add (local.get $out) (i32.const 32)) (i32.wrap_i64 (local.get $unix)))
    (i32.store (i32.add (local.get $out) (i32.const 36)) (i32.wrap_i64 (i64.shr_u (local.get $unix) (i64.const 32))))
    i32.const 0)

  (func (export "rfc3339_emit_utc")
    (param $year i32) (param $month i32) (param $day i32) (param $hour i32) (param $minute i32) (param $second i32) (param $nanos i32) (param $out i32) (param $cap i32)
    (result i64)
    (local $written i32)
    (local $frac i32)
    (local $div i32)
    (local $m177digit i32)
    (local $started i32)
    (if (i32.eqz (call $m177valid_fields (local.get $year) (local.get $month) (local.get $day) (local.get $hour) (local.get $minute) (local.get $second)))
      (then (return (call $pack (i32.const 3) (i32.const 0)))))
    (if (i32.ge_u (local.get $nanos) (i32.const 1000000000))
      (then (return (call $pack (i32.const 3) (i32.const 0)))))
    (local.set $written (select (i32.const 30) (i32.const 20) (i32.ne (local.get $nanos) (i32.const 0))))
    (if (i32.lt_u (local.get $cap) (local.get $written))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))
    (call $m177put4 (local.get $out) (local.get $year))
    (i32.store8 (i32.add (local.get $out) (i32.const 4)) (i32.const 45))
    (call $m177put2 (i32.add (local.get $out) (i32.const 5)) (local.get $month))
    (i32.store8 (i32.add (local.get $out) (i32.const 7)) (i32.const 45))
    (call $m177put2 (i32.add (local.get $out) (i32.const 8)) (local.get $day))
    (i32.store8 (i32.add (local.get $out) (i32.const 10)) (i32.const 84))
    (call $m177put2 (i32.add (local.get $out) (i32.const 11)) (local.get $hour))
    (i32.store8 (i32.add (local.get $out) (i32.const 13)) (i32.const 58))
    (call $m177put2 (i32.add (local.get $out) (i32.const 14)) (local.get $minute))
    (i32.store8 (i32.add (local.get $out) (i32.const 16)) (i32.const 58))
    (call $m177put2 (i32.add (local.get $out) (i32.const 17)) (local.get $second))
    (if (i32.eqz (local.get $nanos))
      (then
        (i32.store8 (i32.add (local.get $out) (i32.const 19)) (i32.const 90))
        (return (call $pack (i32.const 0) (i32.const 20)))))
    (i32.store8 (i32.add (local.get $out) (i32.const 19)) (i32.const 46))
    (local.set $frac (local.get $nanos))
    (local.set $div (i32.const 100000000))
    (local.set $m177digit (i32.const 0))
    (local.set $started (i32.const 0))
    (block $done
      (loop $loop
        (br_if $done (i32.eqz (local.get $div)))
        (local.set $m177digit (i32.div_u (local.get $frac) (local.get $div)))
        (local.set $frac (i32.rem_u (local.get $frac) (local.get $div)))
        (i32.store8 (i32.add (i32.add (local.get $out) (i32.const 20)) (local.get $started)) (i32.add (local.get $m177digit) (i32.const 48)))
        (local.set $started (i32.add (local.get $started) (i32.const 1)))
        (local.set $div (i32.div_u (local.get $div) (i32.const 10)))
        (br $loop)))
    (i32.store8 (i32.add (local.get $out) (i32.const 29)) (i32.const 90))
    (call $pack (i32.const 0) (i32.const 30))))
