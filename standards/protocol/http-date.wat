
;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid, 4 overflow.
  ;; Parse output record, little-endian u32 fields:
  ;; 0 unix_secs_lo, 4 unix_secs_hi, 8 year, 12 month, 16 day, 20 hour,
  ;; 24 minute, 28 second, 32 weekday (Mon=1..Sun=7), 36 format_kind
  ;; format_kind: 1 IMF-fixdate, 2 RFC850 obsolete date, 3 asctime obsolete date.

  (func $m108b (param $p i32) (result i32)
    (i32.load8_u (local.get $p)))

  (func $eq3 (param $p i32) (param $a i32) (param $m108b i32) (param $c i32) (result i32)
    (i32.and
      (i32.and
        (i32.eq (call $m108b (local.get $p)) (local.get $a))
        (i32.eq (call $m108b (i32.add (local.get $p) (i32.const 1))) (local.get $m108b)))
      (i32.eq (call $m108b (i32.add (local.get $p) (i32.const 2))) (local.get $c))))

  (func $m108digit (param $p i32) (result i32)
    (local $x i32)
    (local.set $x (i32.sub (call $m108b (local.get $p)) (i32.const 48)))
    (if (i32.gt_u (local.get $x) (i32.const 9))
      (then (return (i32.const -1))))
    local.get $x)

  (func $one_or_two_day (param $p i32) (result i32)
    (local $a i32)
    (local $m108b i32)
    (if (i32.eq (call $m108b (local.get $p)) (i32.const 32))
      (then
        (local.set $m108b (call $m108digit (i32.add (local.get $p) (i32.const 1))))
        (if (i32.lt_s (local.get $m108b) (i32.const 0)) (then (return (i32.const -1))))
        (return (local.get $m108b))))
    (local.set $a (call $m108digit (local.get $p)))
    (local.set $m108b (call $m108digit (i32.add (local.get $p) (i32.const 1))))
    (if (i32.or (i32.lt_s (local.get $a) (i32.const 0)) (i32.lt_s (local.get $m108b) (i32.const 0)))
      (then (return (i32.const -1))))
    (i32.add (i32.mul (local.get $a) (i32.const 10)) (local.get $m108b)))

  (func $m108two (param $p i32) (result i32)
    (local $a i32)
    (local $m108b i32)
    (local.set $a (call $m108digit (local.get $p)))
    (local.set $m108b (call $m108digit (i32.add (local.get $p) (i32.const 1))))
    (if (i32.or (i32.lt_s (local.get $a) (i32.const 0)) (i32.lt_s (local.get $m108b) (i32.const 0)))
      (then (return (i32.const -1))))
    (i32.add (i32.mul (local.get $a) (i32.const 10)) (local.get $m108b)))

  (func $m108four (param $p i32) (result i32)
    (local $a i32)
    (local $m108b i32)
    (local.set $a (call $m108two (local.get $p)))
    (local.set $m108b (call $m108two (i32.add (local.get $p) (i32.const 2))))
    (if (i32.or (i32.lt_s (local.get $a) (i32.const 0)) (i32.lt_s (local.get $m108b) (i32.const 0)))
      (then (return (i32.const -1))))
    (i32.add (i32.mul (local.get $a) (i32.const 100)) (local.get $m108b)))

  (func $m108is_leap (param $year i32) (result i32)
    (if (i32.ne (i32.rem_u (local.get $year) (i32.const 4)) (i32.const 0))
      (then (return (i32.const 0))))
    (if (i32.ne (i32.rem_u (local.get $year) (i32.const 100)) (i32.const 0))
      (then (return (i32.const 1))))
    (i32.eq (i32.rem_u (local.get $year) (i32.const 400)) (i32.const 0)))

  (func $m108month_days (param $year i32) (param $month i32) (result i32)
    (if (i32.eq (local.get $month) (i32.const 2))
      (then (return (select (i32.const 29) (i32.const 28) (call $m108is_leap (local.get $year))))))
    (if (i32.or
          (i32.or (i32.eq (local.get $month) (i32.const 4)) (i32.eq (local.get $month) (i32.const 6)))
          (i32.or (i32.eq (local.get $month) (i32.const 9)) (i32.eq (local.get $month) (i32.const 11))))
      (then (return (i32.const 30))))
    i32.const 31)

  (func $month3 (param $p i32) (result i32)
    (if (call $eq3 (local.get $p) (i32.const 74) (i32.const 97) (i32.const 110)) (then (return (i32.const 1))))
    (if (call $eq3 (local.get $p) (i32.const 70) (i32.const 101) (i32.const 98)) (then (return (i32.const 2))))
    (if (call $eq3 (local.get $p) (i32.const 77) (i32.const 97) (i32.const 114)) (then (return (i32.const 3))))
    (if (call $eq3 (local.get $p) (i32.const 65) (i32.const 112) (i32.const 114)) (then (return (i32.const 4))))
    (if (call $eq3 (local.get $p) (i32.const 77) (i32.const 97) (i32.const 121)) (then (return (i32.const 5))))
    (if (call $eq3 (local.get $p) (i32.const 74) (i32.const 117) (i32.const 110)) (then (return (i32.const 6))))
    (if (call $eq3 (local.get $p) (i32.const 74) (i32.const 117) (i32.const 108)) (then (return (i32.const 7))))
    (if (call $eq3 (local.get $p) (i32.const 65) (i32.const 117) (i32.const 103)) (then (return (i32.const 8))))
    (if (call $eq3 (local.get $p) (i32.const 83) (i32.const 101) (i32.const 112)) (then (return (i32.const 9))))
    (if (call $eq3 (local.get $p) (i32.const 79) (i32.const 99) (i32.const 116)) (then (return (i32.const 10))))
    (if (call $eq3 (local.get $p) (i32.const 78) (i32.const 111) (i32.const 118)) (then (return (i32.const 11))))
    (if (call $eq3 (local.get $p) (i32.const 68) (i32.const 101) (i32.const 99)) (then (return (i32.const 12))))
    i32.const 0)

  (func $wday3 (param $p i32) (result i32)
    (if (call $eq3 (local.get $p) (i32.const 77) (i32.const 111) (i32.const 110)) (then (return (i32.const 1))))
    (if (call $eq3 (local.get $p) (i32.const 84) (i32.const 117) (i32.const 101)) (then (return (i32.const 2))))
    (if (call $eq3 (local.get $p) (i32.const 87) (i32.const 101) (i32.const 100)) (then (return (i32.const 3))))
    (if (call $eq3 (local.get $p) (i32.const 84) (i32.const 104) (i32.const 117)) (then (return (i32.const 4))))
    (if (call $eq3 (local.get $p) (i32.const 70) (i32.const 114) (i32.const 105)) (then (return (i32.const 5))))
    (if (call $eq3 (local.get $p) (i32.const 83) (i32.const 97) (i32.const 116)) (then (return (i32.const 6))))
    (if (call $eq3 (local.get $p) (i32.const 83) (i32.const 117) (i32.const 110)) (then (return (i32.const 7))))
    i32.const 0)

  (func $match (param $p i32) (param $len i32) (param $off i32) (param $s i32) (param $slen i32) (result i32)
    (local $i i32)
    (if (i32.gt_u (i32.add (local.get $off) (local.get $slen)) (local.get $len))
      (then (return (i32.const 0))))
    (loop $loop
      (if (i32.eq (local.get $i) (local.get $slen)) (then (return (i32.const 1))))
      (if (i32.ne
            (i32.load8_u (i32.add (local.get $p) (i32.add (local.get $off) (local.get $i))))
            (i32.load8_u (i32.add (local.get $s) (local.get $i))))
        (then (return (i32.const 0))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      br $loop)
    i32.const 1)

  (func $wday_long (param $p i32) (param $len i32) (result i64)
    ;; packed as low32=wday, high32=prefix length.
    (if (call $match (local.get $p) (local.get $len) (i32.const 0) (i32.const 1200) (i32.const 8)) (then (return (i64.const 34359738369)))) ;; Monday,
    (if (call $match (local.get $p) (local.get $len) (i32.const 0) (i32.const 1208) (i32.const 9)) (then (return (i64.const 38654705666)))) ;; Tuesday,
    (if (call $match (local.get $p) (local.get $len) (i32.const 0) (i32.const 1217) (i32.const 11)) (then (return (i64.const 47244640259)))) ;; Wednesday,
    (if (call $match (local.get $p) (local.get $len) (i32.const 0) (i32.const 1228) (i32.const 10)) (then (return (i64.const 42949672964)))) ;; Thursday,
    (if (call $match (local.get $p) (local.get $len) (i32.const 0) (i32.const 1238) (i32.const 8)) (then (return (i64.const 34359738373)))) ;; Friday,
    (if (call $match (local.get $p) (local.get $len) (i32.const 0) (i32.const 1246) (i32.const 10)) (then (return (i64.const 42949672966)))) ;; Saturday,
    (if (call $match (local.get $p) (local.get $len) (i32.const 0) (i32.const 1256) (i32.const 8)) (then (return (i64.const 34359738375)))) ;; Sunday,
    i64.const 0)

  (func $m108days_before_year (param $year i32) (result i32)
    (local $y i32)
    (local $days i32)
    (local.set $y (i32.const 1970))
    (loop $loop
      (if (i32.ge_u (local.get $y) (local.get $year)) (then (return (local.get $days))))
      (local.set $days (i32.add (local.get $days) (select (i32.const 366) (i32.const 365) (call $m108is_leap (local.get $y)))))
      (local.set $y (i32.add (local.get $y) (i32.const 1)))
      br $loop)
    local.get $days)

  (func $days_before_month (param $year i32) (param $month i32) (result i32)
    (local $m i32)
    (local $days i32)
    (local.set $m (i32.const 1))
    (loop $loop
      (if (i32.ge_u (local.get $m) (local.get $month)) (then (return (local.get $days))))
      (local.set $days (i32.add (local.get $days) (call $m108month_days (local.get $year) (local.get $m))))
      (local.set $m (i32.add (local.get $m) (i32.const 1)))
      br $loop)
    local.get $days)

  (func $parts_to_unix (param $year i32) (param $month i32) (param $day i32) (param $hour i32) (param $minute i32) (param $second i32) (result i64)
    (local $days i32)
    (local.set $days
      (i32.add
        (i32.add (call $m108days_before_year (local.get $year)) (call $days_before_month (local.get $year) (local.get $month)))
        (i32.sub (local.get $day) (i32.const 1))))
    (i64.add
      (i64.mul (i64.extend_i32_u (local.get $days)) (i64.const 86400))
      (i64.extend_i32_u
        (i32.add
          (i32.add (i32.mul (local.get $hour) (i32.const 3600)) (i32.mul (local.get $minute) (i32.const 60)))
          (local.get $second)))))

  (func $http_date_validate_parts (export "http_date_validate_parts")
    (param $year i32) (param $month i32) (param $day i32)
    (param $hour i32) (param $minute i32) (param $second i32) (param $weekday i32)
    (result i32)
    (local $days i32)
    (if (i32.or (i32.lt_u (local.get $year) (i32.const 1970)) (i32.gt_u (local.get $year) (i32.const 9999))) (then (return (i32.const 3))))
    (if (i32.or (i32.lt_u (local.get $month) (i32.const 1)) (i32.gt_u (local.get $month) (i32.const 12))) (then (return (i32.const 3))))
    (if (i32.or (i32.lt_u (local.get $day) (i32.const 1)) (i32.gt_u (local.get $day) (call $m108month_days (local.get $year) (local.get $month)))) (then (return (i32.const 3))))
    (if (i32.or (i32.ge_u (local.get $hour) (i32.const 24)) (i32.or (i32.ge_u (local.get $minute) (i32.const 60)) (i32.ge_u (local.get $second) (i32.const 60)))) (then (return (i32.const 3))))
    (if (i32.or (i32.lt_u (local.get $weekday) (i32.const 1)) (i32.gt_u (local.get $weekday) (i32.const 7))) (then (return (i32.const 3))))
    (local.set $days
      (i32.add
        (i32.add (call $m108days_before_year (local.get $year)) (call $days_before_month (local.get $year) (local.get $month)))
        (i32.sub (local.get $day) (i32.const 1))))
    (if (i32.ne (local.get $weekday) (i32.add (i32.rem_u (i32.add (local.get $days) (i32.const 3)) (i32.const 7)) (i32.const 1)))
      (then (return (i32.const 3))))
    i32.const 0)

  (func $m108write_record (param $out i32) (param $unix i64)
    (param $year i32) (param $month i32) (param $day i32)
    (param $hour i32) (param $minute i32) (param $second i32)
    (param $weekday i32) (param $kind i32)
    (i32.store (local.get $out) (i32.wrap_i64 (local.get $unix)))
    (i32.store (i32.add (local.get $out) (i32.const 4)) (i32.wrap_i64 (i64.shr_u (local.get $unix) (i64.const 32))))
    (i32.store (i32.add (local.get $out) (i32.const 8)) (local.get $year))
    (i32.store (i32.add (local.get $out) (i32.const 12)) (local.get $month))
    (i32.store (i32.add (local.get $out) (i32.const 16)) (local.get $day))
    (i32.store (i32.add (local.get $out) (i32.const 20)) (local.get $hour))
    (i32.store (i32.add (local.get $out) (i32.const 24)) (local.get $minute))
    (i32.store (i32.add (local.get $out) (i32.const 28)) (local.get $second))
    (i32.store (i32.add (local.get $out) (i32.const 32)) (local.get $weekday))
    (i32.store (i32.add (local.get $out) (i32.const 36)) (local.get $kind)))

  (func $finish_parse (param $out i32)
    (param $year i32) (param $month i32) (param $day i32)
    (param $hour i32) (param $minute i32) (param $second i32)
    (param $weekday i32) (param $kind i32) (result i32)
    (local $unix i64)
    (if (call $http_date_validate_parts
          (local.get $year) (local.get $month) (local.get $day)
          (local.get $hour) (local.get $minute) (local.get $second) (local.get $weekday))
      (then (return (i32.const 3))))
    (local.set $unix (call $parts_to_unix (local.get $year) (local.get $month) (local.get $day) (local.get $hour) (local.get $minute) (local.get $second)))
    (call $m108write_record (local.get $out) (local.get $unix)
      (local.get $year) (local.get $month) (local.get $day)
      (local.get $hour) (local.get $minute) (local.get $second)
      (local.get $weekday) (local.get $kind))
    i32.const 0)

  (func (export "http_date_parse") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $year i32) (local $month i32) (local $day i32)
    (local $hour i32) (local $minute i32) (local $second i32)
    (local $weekday i32) (local $x i64) (local $off i32)
    (if (i32.lt_u (local.get $len) (i32.const 24)) (then (return (i32.const 1))))

    ;; IMF-fixdate: Sun, 06 Nov 1994 08:49:37 GMT
    (if (i32.eq (local.get $len) (i32.const 29))
      (then
        (if (i32.and
              (i32.and
                (i32.and (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.const 3))) (i32.const 44)) (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.const 4))) (i32.const 32)))
                (i32.and (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.const 7))) (i32.const 32)) (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.const 11))) (i32.const 32))))
              (i32.and
                (i32.and (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.const 16))) (i32.const 32)) (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.const 19))) (i32.const 58)))
                (i32.and (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.const 22))) (i32.const 58)) (call $match (local.get $ptr) (local.get $len) (i32.const 25) (i32.const 1264) (i32.const 4)))))
          (then
            (local.set $weekday (call $wday3 (local.get $ptr)))
            (local.set $day (call $m108two (i32.add (local.get $ptr) (i32.const 5))))
            (local.set $month (call $month3 (i32.add (local.get $ptr) (i32.const 8))))
            (local.set $year (call $m108four (i32.add (local.get $ptr) (i32.const 12))))
            (local.set $hour (call $m108two (i32.add (local.get $ptr) (i32.const 17))))
            (local.set $minute (call $m108two (i32.add (local.get $ptr) (i32.const 20))))
            (local.set $second (call $m108two (i32.add (local.get $ptr) (i32.const 23))))
            (if (i32.and
                  (i32.and (i32.gt_u (local.get $weekday) (i32.const 0)) (i32.gt_u (local.get $month) (i32.const 0)))
                  (i32.and
                    (i32.and (i32.ge_s (local.get $day) (i32.const 0)) (i32.ge_s (local.get $year) (i32.const 0)))
                    (i32.and (i32.ge_s (local.get $hour) (i32.const 0)) (i32.and (i32.ge_s (local.get $minute) (i32.const 0)) (i32.ge_s (local.get $second) (i32.const 0))))))
              (then
                (return (call $finish_parse (local.get $out)
                  (local.get $year) (local.get $month) (local.get $day)
                  (local.get $hour) (local.get $minute) (local.get $second)
                  (local.get $weekday) (i32.const 1)))))))))

    ;; RFC850 obsolete: Sunday, 06-Nov-94 08:49:37 GMT
    (local.set $x (call $wday_long (local.get $ptr) (local.get $len)))
    (if (i64.ne (local.get $x) (i64.const 0))
      (then
        (local.set $weekday (i32.wrap_i64 (local.get $x)))
        (local.set $off (i32.wrap_i64 (i64.shr_u (local.get $x) (i64.const 32))))
        (if (i32.eq (local.get $len) (i32.add (local.get $off) (i32.const 22)))
          (then
            (if (i32.and
                  (i32.and (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.add (local.get $off) (i32.const 2)))) (i32.const 45)) (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.add (local.get $off) (i32.const 6)))) (i32.const 45)))
                  (i32.and
                    (i32.and (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.add (local.get $off) (i32.const 9)))) (i32.const 32)) (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.add (local.get $off) (i32.const 12)))) (i32.const 58)))
                    (i32.and (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.add (local.get $off) (i32.const 15)))) (i32.const 58)) (call $match (local.get $ptr) (local.get $len) (i32.add (local.get $off) (i32.const 18)) (i32.const 1264) (i32.const 4)))))
              (then
                (local.set $day (call $m108two (i32.add (local.get $ptr) (local.get $off))))
                (local.set $month (call $month3 (i32.add (local.get $ptr) (i32.add (local.get $off) (i32.const 3)))))
                (local.set $year (call $m108two (i32.add (local.get $ptr) (i32.add (local.get $off) (i32.const 7)))))
                (if (i32.ge_s (local.get $year) (i32.const 0))
                  (then
                    (if (i32.lt_u (local.get $year) (i32.const 70))
                      (then (local.set $year (i32.add (local.get $year) (i32.const 2000))))
                      (else (local.set $year (i32.add (local.get $year) (i32.const 1900)))))))
                (local.set $hour (call $m108two (i32.add (local.get $ptr) (i32.add (local.get $off) (i32.const 10)))))
                (local.set $minute (call $m108two (i32.add (local.get $ptr) (i32.add (local.get $off) (i32.const 13)))))
                (local.set $second (call $m108two (i32.add (local.get $ptr) (i32.add (local.get $off) (i32.const 16)))))
                (if (i32.and
                      (i32.gt_u (local.get $month) (i32.const 0))
                      (i32.and
                        (i32.and (i32.ge_s (local.get $day) (i32.const 0)) (i32.ge_s (local.get $year) (i32.const 0)))
                        (i32.and (i32.ge_s (local.get $hour) (i32.const 0)) (i32.and (i32.ge_s (local.get $minute) (i32.const 0)) (i32.ge_s (local.get $second) (i32.const 0))))))
                  (then
                    (return (call $finish_parse (local.get $out)
                      (local.get $year) (local.get $month) (local.get $day)
                      (local.get $hour) (local.get $minute) (local.get $second)
                      (local.get $weekday) (i32.const 2)))))))))))

    ;; asctime obsolete: Sun Nov  6 08:49:37 1994
    (if (i32.eq (local.get $len) (i32.const 24))
      (then
        (if (i32.and
              (i32.and (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.const 3))) (i32.const 32)) (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.const 7))) (i32.const 32)))
              (i32.and
                (i32.and (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.const 10))) (i32.const 32)) (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.const 13))) (i32.const 58)))
                (i32.and (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.const 16))) (i32.const 58)) (i32.eq (call $m108b (i32.add (local.get $ptr) (i32.const 19))) (i32.const 32)))))
          (then
            (local.set $weekday (call $wday3 (local.get $ptr)))
            (local.set $month (call $month3 (i32.add (local.get $ptr) (i32.const 4))))
            (local.set $day (call $one_or_two_day (i32.add (local.get $ptr) (i32.const 8))))
            (local.set $hour (call $m108two (i32.add (local.get $ptr) (i32.const 11))))
            (local.set $minute (call $m108two (i32.add (local.get $ptr) (i32.const 14))))
            (local.set $second (call $m108two (i32.add (local.get $ptr) (i32.const 17))))
            (local.set $year (call $m108four (i32.add (local.get $ptr) (i32.const 20))))
            (if (i32.and
                  (i32.and (i32.gt_u (local.get $weekday) (i32.const 0)) (i32.gt_u (local.get $month) (i32.const 0)))
                  (i32.and
                    (i32.and (i32.ge_s (local.get $day) (i32.const 0)) (i32.ge_s (local.get $year) (i32.const 0)))
                    (i32.and (i32.ge_s (local.get $hour) (i32.const 0)) (i32.and (i32.ge_s (local.get $minute) (i32.const 0)) (i32.ge_s (local.get $second) (i32.const 0))))))
              (then
                (return (call $finish_parse (local.get $out)
                  (local.get $year) (local.get $month) (local.get $day)
                  (local.get $hour) (local.get $minute) (local.get $second)
                  (local.get $weekday) (i32.const 3)))))))))

    i32.const 3)

  (func $m108put2 (param $p i32) (param $v i32)
    (i32.store8 (local.get $p) (i32.add (i32.const 48) (i32.div_u (local.get $v) (i32.const 10))))
    (i32.store8 (i32.add (local.get $p) (i32.const 1)) (i32.add (i32.const 48) (i32.rem_u (local.get $v) (i32.const 10)))))

  (func $m108put4 (param $p i32) (param $v i32)
    (i32.store8 (local.get $p) (i32.add (i32.const 48) (i32.div_u (local.get $v) (i32.const 1000))))
    (i32.store8 (i32.add (local.get $p) (i32.const 1)) (i32.add (i32.const 48) (i32.rem_u (i32.div_u (local.get $v) (i32.const 100)) (i32.const 10))))
    (i32.store8 (i32.add (local.get $p) (i32.const 2)) (i32.add (i32.const 48) (i32.rem_u (i32.div_u (local.get $v) (i32.const 10)) (i32.const 10))))
    (i32.store8 (i32.add (local.get $p) (i32.const 3)) (i32.add (i32.const 48) (i32.rem_u (local.get $v) (i32.const 10)))))

  (func $copy3 (param $dst i32) (param $src i32)
    (i32.store8 (local.get $dst) (i32.load8_u (local.get $src)))
    (i32.store8 (i32.add (local.get $dst) (i32.const 1)) (i32.load8_u (i32.add (local.get $src) (i32.const 1))))
    (i32.store8 (i32.add (local.get $dst) (i32.const 2)) (i32.load8_u (i32.add (local.get $src) (i32.const 2)))))

  (func $wday_name_ptr (param $weekday i32) (result i32)
    (i32.add (i32.const 1280) (i32.mul (i32.sub (local.get $weekday) (i32.const 1)) (i32.const 3))))

  (func $month_name_ptr (param $month i32) (result i32)
    (i32.add (i32.const 1304) (i32.mul (i32.sub (local.get $month) (i32.const 1)) (i32.const 3))))

  (func (export "http_date_format") (param $lo i32) (param $hi i32) (param $out i32) (param $cap i32) (result i64)
    (local $secs i64) (local $days i64) (local $sod i64)
    (local $year i32) (local $month i32) (local $mdays i32)
    (local $day i32) (local $hour i32) (local $minute i32) (local $second i32) (local $weekday i32)
    (if (i32.lt_u (local.get $cap) (i32.const 29)) (then (return (call $pack (i32.const 2) (i32.const 0)))))
    (local.set $secs (i64.or (i64.extend_i32_u (local.get $lo)) (i64.shl (i64.extend_i32_u (local.get $hi)) (i64.const 32))))
    (if (i64.ge_u (local.get $secs) (i64.const 253402300800)) (then (return (call $pack (i32.const 4) (i32.const 0)))))
    (local.set $days (i64.div_u (local.get $secs) (i64.const 86400)))
    (local.set $sod (i64.rem_u (local.get $secs) (i64.const 86400)))
    (local.set $weekday (i32.add (i32.rem_u (i32.add (i32.wrap_i64 (local.get $days)) (i32.const 3)) (i32.const 7)) (i32.const 1)))
    (local.set $hour (i32.wrap_i64 (i64.div_u (local.get $sod) (i64.const 3600))))
    (local.set $minute (i32.wrap_i64 (i64.div_u (i64.rem_u (local.get $sod) (i64.const 3600)) (i64.const 60))))
    (local.set $second (i32.wrap_i64 (i64.rem_u (local.get $sod) (i64.const 60))))
    (local.set $year (i32.const 1970))
    (loop $year_loop
      (local.set $mdays (select (i32.const 366) (i32.const 365) (call $m108is_leap (local.get $year))))
      (if (i64.lt_u (local.get $days) (i64.extend_i32_u (local.get $mdays))) (then))
      (if (i64.ge_u (local.get $days) (i64.extend_i32_u (local.get $mdays)))
        (then
          (local.set $days (i64.sub (local.get $days) (i64.extend_i32_u (local.get $mdays)))
          )
          (local.set $year (i32.add (local.get $year) (i32.const 1)))
          br $year_loop)))
    (local.set $month (i32.const 1))
    (loop $month_loop
      (local.set $mdays (call $m108month_days (local.get $year) (local.get $month)))
      (if (i64.lt_u (local.get $days) (i64.extend_i32_u (local.get $mdays))) (then))
      (if (i64.ge_u (local.get $days) (i64.extend_i32_u (local.get $mdays)))
        (then
          (local.set $days (i64.sub (local.get $days) (i64.extend_i32_u (local.get $mdays))))
          (local.set $month (i32.add (local.get $month) (i32.const 1)))
          br $month_loop)))
    (local.set $day (i32.add (i32.wrap_i64 (local.get $days)) (i32.const 1)))

    (call $copy3 (local.get $out) (call $wday_name_ptr (local.get $weekday)))
    (i32.store8 (i32.add (local.get $out) (i32.const 3)) (i32.const 44))
    (i32.store8 (i32.add (local.get $out) (i32.const 4)) (i32.const 32))
    (call $m108put2 (i32.add (local.get $out) (i32.const 5)) (local.get $day))
    (i32.store8 (i32.add (local.get $out) (i32.const 7)) (i32.const 32))
    (call $copy3 (i32.add (local.get $out) (i32.const 8)) (call $month_name_ptr (local.get $month)))
    (i32.store8 (i32.add (local.get $out) (i32.const 11)) (i32.const 32))
    (call $m108put4 (i32.add (local.get $out) (i32.const 12)) (local.get $year))
    (i32.store8 (i32.add (local.get $out) (i32.const 16)) (i32.const 32))
    (call $m108put2 (i32.add (local.get $out) (i32.const 17)) (local.get $hour))
    (i32.store8 (i32.add (local.get $out) (i32.const 19)) (i32.const 58))
    (call $m108put2 (i32.add (local.get $out) (i32.const 20)) (local.get $minute))
    (i32.store8 (i32.add (local.get $out) (i32.const 22)) (i32.const 58))
    (call $m108put2 (i32.add (local.get $out) (i32.const 23)) (local.get $second))
    (i32.store8 (i32.add (local.get $out) (i32.const 25)) (i32.const 32))
    (i32.store8 (i32.add (local.get $out) (i32.const 26)) (i32.const 71))
    (i32.store8 (i32.add (local.get $out) (i32.const 27)) (i32.const 77))
    (i32.store8 (i32.add (local.get $out) (i32.const 28)) (i32.const 84))
    (call $pack (i32.const 0) (i32.const 29)))

  ;; Static strings used by parser/formatter.
  (data (i32.const 1200) "Monday, Tuesday, Wednesday, Thursday, Friday, Saturday, Sunday,  GMT")
  (data (i32.const 1280) "MonTueWedThuFriSatSun")
  (data (i32.const 1304) "JanFebMarAprMayJunJulAugSepOctNovDec")
