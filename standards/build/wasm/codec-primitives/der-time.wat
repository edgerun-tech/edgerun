(module
  (memory (export "memory") 1)

  (func (export "proto_abi_version") (result i32)
    i32.const 2)

  (func (export "proto_standard_id") (result i32)
    i32.const 300029)

  ;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid, 4 overflow.
  ;; Record:
  ;; 0 kind: 1 UTCTime, 2 GeneralizedTime
  ;; 4 year, 8 month, 12 day, 16 hour, 20 minute, 24 second
  ;; 28 unix_seconds_lo, 32 unix_seconds_hi, 36 header_len, 40 total_len

  (func $digit (param $ptr i32) (param $end i32) (result i32)
    (local $b i32)
    (if (i32.ge_u (local.get $ptr) (local.get $end))
      (then (return (i32.const -1))))
    (local.set $b (i32.load8_u (local.get $ptr)))
    (if (i32.or (i32.lt_u (local.get $b) (i32.const 48)) (i32.gt_u (local.get $b) (i32.const 57)))
      (then (return (i32.const -1))))
    (i32.sub (local.get $b) (i32.const 48)))

  (func $two (param $ptr i32) (param $end i32) (result i32)
    (local $a i32)
    (local $b i32)
    (local.set $a (call $digit (local.get $ptr) (local.get $end)))
    (if (i32.lt_s (local.get $a) (i32.const 0)) (then (return (i32.const -1))))
    (local.set $b (call $digit (i32.add (local.get $ptr) (i32.const 1)) (local.get $end)))
    (if (i32.lt_s (local.get $b) (i32.const 0)) (then (return (i32.const -1))))
    (i32.add (i32.mul (local.get $a) (i32.const 10)) (local.get $b)))

  (func $four (param $ptr i32) (param $end i32) (result i32)
    (local $a i32)
    (local $b i32)
    (local.set $a (call $two (local.get $ptr) (local.get $end)))
    (if (i32.lt_s (local.get $a) (i32.const 0)) (then (return (i32.const -1))))
    (local.set $b (call $two (i32.add (local.get $ptr) (i32.const 2)) (local.get $end)))
    (if (i32.lt_s (local.get $b) (i32.const 0)) (then (return (i32.const -1))))
    (i32.add (i32.mul (local.get $a) (i32.const 100)) (local.get $b)))

  (func $is_leap (param $year i32) (result i32)
    (if (i32.ne (i32.rem_u (local.get $year) (i32.const 4)) (i32.const 0))
      (then (return (i32.const 0))))
    (if (i32.ne (i32.rem_u (local.get $year) (i32.const 100)) (i32.const 0))
      (then (return (i32.const 1))))
    (select
      (i32.const 1)
      (i32.const 0)
      (i32.eq (i32.rem_u (local.get $year) (i32.const 400)) (i32.const 0))))

  (func $month_days (param $year i32) (param $month i32) (result i32)
    (if (i32.eq (local.get $month) (i32.const 2))
      (then
        (return
          (select (i32.const 29) (i32.const 28) (call $is_leap (local.get $year))))))
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

  (func $days_before_year (param $year i32) (result i64)
    (local $y i64)
    (local.set $y (i64.extend_i32_s (i32.sub (local.get $year) (i32.const 1))))
    (i64.sub
      (i64.sub
        (i64.add
          (i64.mul (local.get $y) (i64.const 365))
          (i64.div_s (local.get $y) (i64.const 4)))
        (i64.div_s (local.get $y) (i64.const 100)))
      (i64.mul (i64.div_s (local.get $y) (i64.const 400)) (i64.const -1))))

  (func $days_since_epoch (param $year i32) (param $month i32) (param $day i32) (result i64)
    (local $m i32)
    (local $days i64)
    (local.set $days
      (i64.sub
        (call $days_before_year (local.get $year))
        (i64.const 719162)))
    (local.set $m (i32.const 1))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $m) (local.get $month)))
        (local.set $days
          (i64.add (local.get $days) (i64.extend_i32_u (call $month_days (local.get $year) (local.get $m)))))
        (local.set $m (i32.add (local.get $m) (i32.const 1)))
        (br $loop)))
    (i64.add (local.get $days) (i64.extend_i32_u (i32.sub (local.get $day) (i32.const 1)))))

  (func $unix_seconds
    (param $year i32) (param $month i32) (param $day i32) (param $hour i32) (param $minute i32) (param $second i32)
    (result i64)
    (i64.add
      (i64.add
        (i64.mul (call $days_since_epoch (local.get $year) (local.get $month) (local.get $day)) (i64.const 86400))
        (i64.mul (i64.extend_i32_u (local.get $hour)) (i64.const 3600)))
      (i64.add
        (i64.mul (i64.extend_i32_u (local.get $minute)) (i64.const 60))
        (i64.extend_i32_u (local.get $second)))))

  (func $valid_fields
    (param $year i32) (param $month i32) (param $day i32) (param $hour i32) (param $minute i32) (param $second i32)
    (result i32)
    (if (i32.or (i32.lt_u (local.get $month) (i32.const 1)) (i32.gt_u (local.get $month) (i32.const 12)))
      (then (return (i32.const 0))))
    (if (i32.or
          (i32.lt_u (local.get $day) (i32.const 1))
          (i32.gt_u (local.get $day) (call $month_days (local.get $year) (local.get $month))))
      (then (return (i32.const 0))))
    (if (i32.gt_u (local.get $hour) (i32.const 23)) (then (return (i32.const 0))))
    (if (i32.gt_u (local.get $minute) (i32.const 59)) (then (return (i32.const 0))))
    (if (i32.gt_u (local.get $second) (i32.const 59)) (then (return (i32.const 0))))
    i32.const 1)

  (func $parse_len (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $first i32)
    (local $value i32)
    (if (i32.lt_u (local.get $len) (i32.const 2))
      (then (return (i32.const 1))))
    (local.set $first (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))))
    (if (i32.lt_u (local.get $first) (i32.const 128))
      (then
        (i32.store (local.get $out) (i32.const 2))
        (i32.store (i32.add (local.get $out) (i32.const 4)) (local.get $first))
        (return (i32.const 0))))
    (if (i32.eq (local.get $first) (i32.const 129))
      (then
        (if (i32.lt_u (local.get $len) (i32.const 3)) (then (return (i32.const 1))))
        (local.set $value (i32.load8_u (i32.add (local.get $ptr) (i32.const 2))))
        (if (i32.lt_u (local.get $value) (i32.const 128)) (then (return (i32.const 3))))
        (i32.store (local.get $out) (i32.const 3))
        (i32.store (i32.add (local.get $out) (i32.const 4)) (local.get $value))
        (return (i32.const 0))))
    (return (i32.const 3)))

  (func (export "der_time_decode") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $tag i32)
    (local $status i32)
    (local $hdr i32)
    (local $vlen i32)
    (local $body i32)
    (local $end i32)
    (local $kind i32)
    (local $year i32)
    (local $month i32)
    (local $day i32)
    (local $hour i32)
    (local $minute i32)
    (local $second i32)
    (local $unix i64)
    (local.set $status (call $parse_len (local.get $ptr) (local.get $len) (local.get $out)))
    (if (local.get $status) (then (return (local.get $status))))
    (local.set $hdr (i32.load (local.get $out)))
    (local.set $vlen (i32.load (i32.add (local.get $out) (i32.const 4))))
    (if (i32.lt_u (local.get $len) (i32.add (local.get $hdr) (local.get $vlen)))
      (then (return (i32.const 1))))
    (if (i32.ne (local.get $len) (i32.add (local.get $hdr) (local.get $vlen)))
      (then (return (i32.const 3))))
    (local.set $tag (i32.load8_u (local.get $ptr)))
    (local.set $body (i32.add (local.get $ptr) (local.get $hdr)))
    (local.set $end (i32.add (local.get $body) (local.get $vlen)))
    local.get $tag
    i32.const 23
    i32.eq
    if
      (if (i32.ne (local.get $vlen) (i32.const 13)) (then (return (i32.const 3))))
      (local.set $kind (i32.const 1))
      (local.set $year (call $two (local.get $body) (local.get $end)))
      (if (i32.lt_s (local.get $year) (i32.const 0)) (then (return (i32.const 3))))
      (local.set $year
        (select
          (i32.add (local.get $year) (i32.const 1900))
          (i32.add (local.get $year) (i32.const 2000))
          (i32.ge_u (local.get $year) (i32.const 50))))
      (local.set $month (call $two (i32.add (local.get $body) (i32.const 2)) (local.get $end)))
      (local.set $day (call $two (i32.add (local.get $body) (i32.const 4)) (local.get $end)))
      (local.set $hour (call $two (i32.add (local.get $body) (i32.const 6)) (local.get $end)))
      (local.set $minute (call $two (i32.add (local.get $body) (i32.const 8)) (local.get $end)))
      (local.set $second (call $two (i32.add (local.get $body) (i32.const 10)) (local.get $end)))
      (if (i32.ne (i32.load8_u (i32.add (local.get $body) (i32.const 12))) (i32.const 90))
        (then (return (i32.const 3))))
    else
      (if (i32.ne (local.get $tag) (i32.const 24)) (then (return (i32.const 3))))
      (if (i32.ne (local.get $vlen) (i32.const 15)) (then (return (i32.const 3))))
      (local.set $kind (i32.const 2))
      (local.set $year (call $four (local.get $body) (local.get $end)))
      (local.set $month (call $two (i32.add (local.get $body) (i32.const 4)) (local.get $end)))
      (local.set $day (call $two (i32.add (local.get $body) (i32.const 6)) (local.get $end)))
      (local.set $hour (call $two (i32.add (local.get $body) (i32.const 8)) (local.get $end)))
      (local.set $minute (call $two (i32.add (local.get $body) (i32.const 10)) (local.get $end)))
      (local.set $second (call $two (i32.add (local.get $body) (i32.const 12)) (local.get $end)))
      (if (i32.ne (i32.load8_u (i32.add (local.get $body) (i32.const 14))) (i32.const 90))
        (then (return (i32.const 3))))
    end
    (if
      (i32.or
        (i32.or
          (i32.or (i32.lt_s (local.get $month) (i32.const 0)) (i32.lt_s (local.get $day) (i32.const 0)))
          (i32.or (i32.lt_s (local.get $hour) (i32.const 0)) (i32.lt_s (local.get $minute) (i32.const 0))))
        (i32.lt_s (local.get $second) (i32.const 0)))
      (then (return (i32.const 3))))
    (if (i32.eqz (call $valid_fields (local.get $year) (local.get $month) (local.get $day) (local.get $hour) (local.get $minute) (local.get $second)))
      (then (return (i32.const 3))))
    (local.set $unix
      (call $unix_seconds
        (local.get $year) (local.get $month) (local.get $day)
        (local.get $hour) (local.get $minute) (local.get $second)))
    (i32.store (local.get $out) (local.get $kind))
    (i32.store (i32.add (local.get $out) (i32.const 4)) (local.get $year))
    (i32.store (i32.add (local.get $out) (i32.const 8)) (local.get $month))
    (i32.store (i32.add (local.get $out) (i32.const 12)) (local.get $day))
    (i32.store (i32.add (local.get $out) (i32.const 16)) (local.get $hour))
    (i32.store (i32.add (local.get $out) (i32.const 20)) (local.get $minute))
    (i32.store (i32.add (local.get $out) (i32.const 24)) (local.get $second))
    (i32.store (i32.add (local.get $out) (i32.const 28)) (i32.wrap_i64 (local.get $unix)))
    (i32.store (i32.add (local.get $out) (i32.const 32)) (i32.wrap_i64 (i64.shr_u (local.get $unix) (i64.const 32))))
    (i32.store (i32.add (local.get $out) (i32.const 36)) (local.get $hdr))
    (i32.store (i32.add (local.get $out) (i32.const 40)) (local.get $len))
    i32.const 0)
)
