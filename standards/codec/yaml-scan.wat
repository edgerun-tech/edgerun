
  ;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid, 4 overflow, 5 unexpected_end.
  ;; Line kinds: 0 blank, 1 comment, 2 mapping, 3 sequence, 4 scalar, 5 doc_start, 6 doc_end.
  ;; Scalar kinds: 0 empty, 1 null, 2 bool, 3 int, 4 float, 5 quoted, 6 inline_sequence,
  ;; 7 inline_mapping, 8 bare_string.

  (func (export "proto_standard_id") (result i32)
    i32.const 300042)

  (func $m202byte (param $ptr i32) (param $off i32) (result i32)
    (i32.load8_u (i32.add (local.get $ptr) (local.get $off))))

  (func $m202is_ws (param $c i32) (result i32)
    (i32.or (i32.eq (local.get $c) (i32.const 32)) (i32.eq (local.get $c) (i32.const 9))))



  (func $m202skip_ws (param $ptr i32) (param $len i32) (param $p i32) (result i32)
    (loop $again
      (if (i32.and (i32.lt_u (local.get $p) (local.get $len)) (call $m202is_ws (call $m202byte (local.get $ptr) (local.get $p))))
        (then
          (local.set $p (i32.add (local.get $p) (i32.const 1)))
          (br $again))))
    (local.get $p))

  (func $m202trim_right (param $ptr i32) (param $start i32) (param $end i32) (result i32)
    (loop $again
      (if (i32.and
            (i32.gt_u (local.get $end) (local.get $start))
            (call $m202is_ws (call $m202byte (local.get $ptr) (i32.sub (local.get $end) (i32.const 1)))))
        (then
          (local.set $end (i32.sub (local.get $end) (i32.const 1)))
          (br $again))))
    (local.get $end))

  (func $m202line_end (param $ptr i32) (param $len i32) (param $offset i32) (result i32)
    (local $p i32)
    (local.set $p (local.get $offset))
    (loop $again
      (if (i32.ge_u (local.get $p) (local.get $len)) (then (return (local.get $len))))
      (if (i32.or (i32.eq (call $m202byte (local.get $ptr) (local.get $p)) (i32.const 10)) (i32.eq (call $m202byte (local.get $ptr) (local.get $p)) (i32.const 13)))
        (then (return (local.get $p))))
      (local.set $p (i32.add (local.get $p) (i32.const 1)))
      (br $again))
    (local.get $len))

  (func $m202next_line (param $ptr i32) (param $len i32) (param $end i32) (result i32)
    (local $p i32)
    (local.set $p (local.get $end))
    (if (i32.and (i32.lt_u (local.get $p) (local.get $len)) (i32.eq (call $m202byte (local.get $ptr) (local.get $p)) (i32.const 13)))
      (then (local.set $p (i32.add (local.get $p) (i32.const 1)))))
    (if (i32.and (i32.lt_u (local.get $p) (local.get $len)) (i32.eq (call $m202byte (local.get $ptr) (local.get $p)) (i32.const 10)))
      (then (local.set $p (i32.add (local.get $p) (i32.const 1)))))
    (local.get $p))

  (func $m202write4 (param $out i32) (param $a i32) (param $b i32) (param $c i32) (param $d i32)
    (i32.store (local.get $out) (local.get $a))
    (i32.store (i32.add (local.get $out) (i32.const 4)) (local.get $b))
    (i32.store (i32.add (local.get $out) (i32.const 8)) (local.get $c))
    (i32.store (i32.add (local.get $out) (i32.const 12)) (local.get $d)))

  (func $write8
    (param $out i32) (param $a i32) (param $b i32) (param $c i32) (param $d i32)
    (param $e i32) (param $f i32) (param $g i32) (param $h i32)
    (call $m202write4 (local.get $out) (local.get $a) (local.get $b) (local.get $c) (local.get $d))
    (call $m202write4 (i32.add (local.get $out) (i32.const 16)) (local.get $e) (local.get $f) (local.get $g) (local.get $h)))

  (func $m202match_lit (param $ptr i32) (param $start i32) (param $end i32) (param $lit i32) (param $lit_len i32) (result i32)
    (local $i i32)
    (if (i32.ne (i32.sub (local.get $end) (local.get $start)) (local.get $lit_len)) (then (return (i32.const 0))))
    (loop $again
      (if (i32.ge_u (local.get $i) (local.get $lit_len)) (then (return (i32.const 1))))
      (if (i32.ne (call $m202byte (local.get $ptr) (i32.add (local.get $start) (local.get $i))) (call $m202byte (i32.const 0) (i32.add (local.get $lit) (local.get $i))))
        (then (return (i32.const 0))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $again))
    (i32.const 1))

  (data (i32.const 0) "null~trueyesonfalsenooff---...")

  (func $m202scan_quoted (param $ptr i32) (param $len i32) (param $p i32) (param $quote i32) (result i32)
    (local $c i32)
    (local.set $p (i32.add (local.get $p) (i32.const 1)))
    (loop $again
      (if (i32.ge_u (local.get $p) (local.get $len)) (then (return (i32.const -1))))
      (local.set $c (call $m202byte (local.get $ptr) (local.get $p)))
      (if (i32.eq (local.get $c) (local.get $quote))
        (then (return (i32.add (local.get $p) (i32.const 1)))))
      (if (i32.and (i32.eq (local.get $quote) (i32.const 34)) (i32.eq (local.get $c) (i32.const 92)))
        (then
          (local.set $p (i32.add (local.get $p) (i32.const 1)))
          (if (i32.ge_u (local.get $p) (local.get $len)) (then (return (i32.const -1))))))
      (if (i32.or (i32.eq (local.get $c) (i32.const 10)) (i32.eq (local.get $c) (i32.const 13)))
        (then (return (i32.const -1))))
      (local.set $p (i32.add (local.get $p) (i32.const 1)))
      (br $again))
    (i32.const -1))

  (func $scan_balanced (param $ptr i32) (param $len i32) (param $p i32) (param $open i32) (param $close i32) (result i32)
    (local $depth i32)
    (local $c i32)
    (local.set $depth (i32.const 1))
    (local.set $p (i32.add (local.get $p) (i32.const 1)))
    (loop $again
      (if (i32.ge_u (local.get $p) (local.get $len)) (then (return (i32.const -1))))
      (local.set $c (call $m202byte (local.get $ptr) (local.get $p)))
      (if (i32.eq (local.get $c) (i32.const 34))
        (then
          (local.set $p (call $m202scan_quoted (local.get $ptr) (local.get $len) (local.get $p) (i32.const 34)))
          (if (i32.lt_s (local.get $p) (i32.const 0)) (then (return (i32.const -1))))
          (br $again)))
      (if (i32.eq (local.get $c) (i32.const 39))
        (then
          (local.set $p (call $m202scan_quoted (local.get $ptr) (local.get $len) (local.get $p) (i32.const 39)))
          (if (i32.lt_s (local.get $p) (i32.const 0)) (then (return (i32.const -1))))
          (br $again)))
      (if (i32.eq (local.get $c) (local.get $open))
        (then (local.set $depth (i32.add (local.get $depth) (i32.const 1)))))
      (if (i32.eq (local.get $c) (local.get $close))
        (then
          (local.set $depth (i32.sub (local.get $depth) (i32.const 1)))
          (if (i32.eqz (local.get $depth)) (then (return (i32.add (local.get $p) (i32.const 1)))))))
      (local.set $p (i32.add (local.get $p) (i32.const 1)))
      (br $again))
    (i32.const -1))

  (func $content_end (param $ptr i32) (param $start i32) (param $end i32) (result i32)
    (local $p i32)
    (local $c i32)
    (local $in_single i32)
    (local $in_double i32)
    (local $escaped i32)
    (local.set $p (local.get $start))
    (loop $again
      (if (i32.ge_u (local.get $p) (local.get $end)) (then (return (call $m202trim_right (local.get $ptr) (local.get $start) (local.get $end)))))
      (local.set $c (call $m202byte (local.get $ptr) (local.get $p)))
      (if (local.get $escaped)
        (then
          (local.set $escaped (i32.const 0))
          (local.set $p (i32.add (local.get $p) (i32.const 1)))
          (br $again)))
      (if (i32.and (local.get $in_double) (i32.eq (local.get $c) (i32.const 92)))
        (then
          (local.set $escaped (i32.const 1))
          (local.set $p (i32.add (local.get $p) (i32.const 1)))
          (br $again)))
      (if (i32.and (i32.eqz (local.get $in_double)) (i32.eq (local.get $c) (i32.const 39)))
        (then (local.set $in_single (i32.xor (local.get $in_single) (i32.const 1)))))
      (if (i32.and (i32.eqz (local.get $in_single)) (i32.eq (local.get $c) (i32.const 34)))
        (then (local.set $in_double (i32.xor (local.get $in_double) (i32.const 1)))))
      (if (i32.and
            (i32.and (i32.eqz (local.get $in_single)) (i32.eqz (local.get $in_double)))
            (i32.eq (local.get $c) (i32.const 35)))
        (then
          (if (i32.or (i32.eq (local.get $p) (local.get $start)) (call $m202is_ws (call $m202byte (local.get $ptr) (i32.sub (local.get $p) (i32.const 1)))))
            (then (return (call $m202trim_right (local.get $ptr) (local.get $start) (local.get $p)))))))
      (local.set $p (i32.add (local.get $p) (i32.const 1)))
      (br $again))
    (local.get $end))

  (func $find_colon (param $ptr i32) (param $start i32) (param $end i32) (result i32)
    (local $p i32)
    (local $c i32)
    (local $in_single i32)
    (local $in_double i32)
    (local $escaped i32)
    (local.set $p (local.get $start))
    (loop $again
      (if (i32.ge_u (local.get $p) (local.get $end)) (then (return (i32.const -1))))
      (local.set $c (call $m202byte (local.get $ptr) (local.get $p)))
      (if (local.get $escaped)
        (then
          (local.set $escaped (i32.const 0))
          (local.set $p (i32.add (local.get $p) (i32.const 1)))
          (br $again)))
      (if (i32.and (local.get $in_double) (i32.eq (local.get $c) (i32.const 92)))
        (then
          (local.set $escaped (i32.const 1))
          (local.set $p (i32.add (local.get $p) (i32.const 1)))
          (br $again)))
      (if (i32.and (i32.eqz (local.get $in_double)) (i32.eq (local.get $c) (i32.const 39)))
        (then (local.set $in_single (i32.xor (local.get $in_single) (i32.const 1)))))
      (if (i32.and (i32.eqz (local.get $in_single)) (i32.eq (local.get $c) (i32.const 34)))
        (then (local.set $in_double (i32.xor (local.get $in_double) (i32.const 1)))))
      (if (i32.and (i32.and (i32.eqz (local.get $in_single)) (i32.eqz (local.get $in_double))) (i32.eq (local.get $c) (i32.const 58)))
        (then (return (local.get $p))))
      (local.set $p (i32.add (local.get $p) (i32.const 1)))
      (br $again))
    (i32.const -1))

  (func $m202scan_numberish (param $ptr i32) (param $start i32) (param $end i32) (result i32)
    (local $p i32)
    (local $kind i32)
    (local.set $p (local.get $start))
    (local.set $kind (i32.const 3))
    (if (i32.and (i32.lt_u (local.get $p) (local.get $end)) (i32.or (i32.eq (call $m202byte (local.get $ptr) (local.get $p)) (i32.const 45)) (i32.eq (call $m202byte (local.get $ptr) (local.get $p)) (i32.const 43))))
      (then (local.set $p (i32.add (local.get $p) (i32.const 1)))))
    (if (i32.or (i32.ge_u (local.get $p) (local.get $end)) (i32.eqz (call $is_digit (call $m202byte (local.get $ptr) (local.get $p)))))
      (then (return (i32.const 0))))
    (loop $digits
      (local.set $p (i32.add (local.get $p) (i32.const 1)))
      (br_if $digits (i32.and (i32.lt_u (local.get $p) (local.get $end)) (call $is_digit (call $m202byte (local.get $ptr) (local.get $p))))))
    (if (i32.and (i32.lt_u (local.get $p) (local.get $end)) (i32.eq (call $m202byte (local.get $ptr) (local.get $p)) (i32.const 46)))
      (then
        (local.set $kind (i32.const 4))
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (if (i32.or (i32.ge_u (local.get $p) (local.get $end)) (i32.eqz (call $is_digit (call $m202byte (local.get $ptr) (local.get $p)))))
          (then (return (i32.const 0))))
        (loop $frac
          (local.set $p (i32.add (local.get $p) (i32.const 1)))
          (br_if $frac (i32.and (i32.lt_u (local.get $p) (local.get $end)) (call $is_digit (call $m202byte (local.get $ptr) (local.get $p))))))))
    (if (i32.and
          (i32.lt_u (local.get $p) (local.get $end))
          (i32.or (i32.eq (call $m202byte (local.get $ptr) (local.get $p)) (i32.const 101)) (i32.eq (call $m202byte (local.get $ptr) (local.get $p)) (i32.const 69))))
      (then
        (local.set $kind (i32.const 4))
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (if (i32.and
              (i32.lt_u (local.get $p) (local.get $end))
              (i32.or (i32.eq (call $m202byte (local.get $ptr) (local.get $p)) (i32.const 45)) (i32.eq (call $m202byte (local.get $ptr) (local.get $p)) (i32.const 43))))
          (then (local.set $p (i32.add (local.get $p) (i32.const 1)))))
        (if (i32.or (i32.ge_u (local.get $p) (local.get $end)) (i32.eqz (call $is_digit (call $m202byte (local.get $ptr) (local.get $p)))))
          (then (return (i32.const 0))))
        (loop $exp
          (local.set $p (i32.add (local.get $p) (i32.const 1)))
          (br_if $exp (i32.and (i32.lt_u (local.get $p) (local.get $end)) (call $is_digit (call $m202byte (local.get $ptr) (local.get $p))))))))
    (if (i32.eq (local.get $p) (local.get $end)) (then (return (local.get $kind))))
    (i32.const 0))

  (func $scalar_kind (param $ptr i32) (param $start i32) (param $end i32) (param $out i32) (result i32)
    (local $c i32)
    (local $p i32)
    (if (i32.ge_u (local.get $start) (local.get $end))
      (then
        (call $m202write4 (local.get $out) (i32.const 0) (local.get $start) (i32.const 0) (i32.const 0))
        (return (i32.const 0))))
    (local.set $c (call $m202byte (local.get $ptr) (local.get $start)))
    (if (i32.or (i32.eq (local.get $c) (i32.const 34)) (i32.eq (local.get $c) (i32.const 39)))
      (then
        (local.set $p (call $m202scan_quoted (local.get $ptr) (local.get $end) (local.get $start) (local.get $c)))
        (if (i32.lt_s (local.get $p) (i32.const 0)) (then (return (i32.const 5))))
        (if (i32.ne (local.get $p) (local.get $end)) (then (return (i32.const 3))))
        (call $m202write4
          (local.get $out)
          (i32.const 5)
          (i32.add (local.get $start) (i32.const 1))
          (i32.sub (i32.sub (local.get $end) (local.get $start)) (i32.const 2))
          (select (i32.const 1) (i32.const 2) (i32.eq (local.get $c) (i32.const 34))))
        (return (i32.const 0))))
    (if (i32.or
          (call $m202match_lit (local.get $ptr) (local.get $start) (local.get $end) (i32.const 0) (i32.const 4))
          (call $m202match_lit (local.get $ptr) (local.get $start) (local.get $end) (i32.const 4) (i32.const 1)))
      (then
        (call $m202write4 (local.get $out) (i32.const 1) (local.get $start) (i32.sub (local.get $end) (local.get $start)) (i32.const 0))
        (return (i32.const 0))))
    (if (i32.or
          (i32.or
            (call $m202match_lit (local.get $ptr) (local.get $start) (local.get $end) (i32.const 5) (i32.const 4))
            (call $m202match_lit (local.get $ptr) (local.get $start) (local.get $end) (i32.const 9) (i32.const 3)))
          (call $m202match_lit (local.get $ptr) (local.get $start) (local.get $end) (i32.const 12) (i32.const 2)))
      (then
        (call $m202write4 (local.get $out) (i32.const 2) (local.get $start) (i32.sub (local.get $end) (local.get $start)) (i32.const 1))
        (return (i32.const 0))))
    (if (i32.or
          (i32.or
            (call $m202match_lit (local.get $ptr) (local.get $start) (local.get $end) (i32.const 14) (i32.const 5))
            (call $m202match_lit (local.get $ptr) (local.get $start) (local.get $end) (i32.const 19) (i32.const 2)))
          (call $m202match_lit (local.get $ptr) (local.get $start) (local.get $end) (i32.const 21) (i32.const 3)))
      (then
        (call $m202write4 (local.get $out) (i32.const 2) (local.get $start) (i32.sub (local.get $end) (local.get $start)) (i32.const 0))
        (return (i32.const 0))))
    (local.set $p (call $m202scan_numberish (local.get $ptr) (local.get $start) (local.get $end)))
    (if (local.get $p)
      (then
        (call $m202write4 (local.get $out) (local.get $p) (local.get $start) (i32.sub (local.get $end) (local.get $start)) (i32.const 0))
        (return (i32.const 0))))
    (if (i32.eq (local.get $c) (i32.const 91))
      (then
        (local.set $p (call $scan_balanced (local.get $ptr) (local.get $end) (local.get $start) (i32.const 91) (i32.const 93)))
        (if (i32.lt_s (local.get $p) (i32.const 0)) (then (return (i32.const 5))))
        (if (i32.ne (local.get $p) (local.get $end)) (then (return (i32.const 3))))
        (call $m202write4 (local.get $out) (i32.const 6) (local.get $start) (i32.sub (local.get $end) (local.get $start)) (i32.const 0))
        (return (i32.const 0))))
    (if (i32.eq (local.get $c) (i32.const 123))
      (then
        (local.set $p (call $scan_balanced (local.get $ptr) (local.get $end) (local.get $start) (i32.const 123) (i32.const 125)))
        (if (i32.lt_s (local.get $p) (i32.const 0)) (then (return (i32.const 5))))
        (if (i32.ne (local.get $p) (local.get $end)) (then (return (i32.const 3))))
        (call $m202write4 (local.get $out) (i32.const 7) (local.get $start) (i32.sub (local.get $end) (local.get $start)) (i32.const 0))
        (return (i32.const 0))))
    (call $m202write4 (local.get $out) (i32.const 8) (local.get $start) (i32.sub (local.get $end) (local.get $start)) (i32.const 0))
    (i32.const 0))

  (func (export "yaml_scan_scalar") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $start i32)
    (local $end i32)
    (local.set $start (call $m202skip_ws (local.get $ptr) (local.get $len) (i32.const 0)))
    (local.set $end (call $m202trim_right (local.get $ptr) (local.get $start) (local.get $len)))
    (call $scalar_kind (local.get $ptr) (local.get $start) (local.get $end) (local.get $out)))

  (func $yaml_scan_line (export "yaml_scan_line") (param $ptr i32) (param $len i32) (param $offset i32) (param $out i32) (result i32)
    (local $line_start i32)
    (local $m202line_end i32)
    (local $content_start i32)
    (local $content_end i32)
    (local $indent i32)
    (local $colon i32)
    (local $value_start i32)
    (if (i32.gt_u (local.get $offset) (local.get $len)) (then (return (i32.const 1))))
    (local.set $line_start (local.get $offset))
    (local.set $m202line_end (call $m202line_end (local.get $ptr) (local.get $len) (local.get $offset)))
    (local.set $content_start (local.get $line_start))
    (loop $indent_loop
      (if (i32.and (i32.lt_u (local.get $content_start) (local.get $m202line_end)) (i32.eq (call $m202byte (local.get $ptr) (local.get $content_start)) (i32.const 32)))
        (then
          (local.set $content_start (i32.add (local.get $content_start) (i32.const 1)))
          (br $indent_loop))))
    (if (i32.and (i32.lt_u (local.get $content_start) (local.get $m202line_end)) (i32.eq (call $m202byte (local.get $ptr) (local.get $content_start)) (i32.const 9)))
      (then (return (i32.const 3))))
    (local.set $indent (i32.sub (local.get $content_start) (local.get $line_start)))
    (local.set $content_end (call $content_end (local.get $ptr) (local.get $content_start) (local.get $m202line_end)))
    (if (i32.ge_u (local.get $content_start) (local.get $content_end))
      (then
        (if (i32.and (i32.lt_u (local.get $content_start) (local.get $m202line_end)) (i32.eq (call $m202byte (local.get $ptr) (local.get $content_start)) (i32.const 35)))
          (then
            (call $write8 (local.get $out) (i32.const 1) (local.get $indent) (local.get $line_start) (i32.sub (local.get $m202line_end) (local.get $line_start)) (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0))
            (return (i32.const 0))))
        (call $write8 (local.get $out) (i32.const 0) (local.get $indent) (local.get $line_start) (i32.sub (local.get $m202line_end) (local.get $line_start)) (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0))
        (return (i32.const 0))))
    (if (call $m202match_lit (local.get $ptr) (local.get $content_start) (local.get $content_end) (i32.const 24) (i32.const 3))
      (then
        (call $write8 (local.get $out) (i32.const 5) (local.get $indent) (local.get $line_start) (i32.sub (local.get $m202line_end) (local.get $line_start)) (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0))
        (return (i32.const 0))))
    (if (call $m202match_lit (local.get $ptr) (local.get $content_start) (local.get $content_end) (i32.const 27) (i32.const 3))
      (then
        (call $write8 (local.get $out) (i32.const 6) (local.get $indent) (local.get $line_start) (i32.sub (local.get $m202line_end) (local.get $line_start)) (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0))
        (return (i32.const 0))))
    (if (i32.eq (call $m202byte (local.get $ptr) (local.get $content_start)) (i32.const 45))
      (then
        (if (i32.or
              (i32.eq (i32.add (local.get $content_start) (i32.const 1)) (local.get $content_end))
              (call $m202is_ws (call $m202byte (local.get $ptr) (i32.add (local.get $content_start) (i32.const 1)))))
          (then
            (local.set $value_start (call $m202skip_ws (local.get $ptr) (local.get $content_end) (i32.add (local.get $content_start) (i32.const 1))))
            (call $write8
              (local.get $out)
              (i32.const 3)
              (local.get $indent)
              (local.get $line_start)
              (i32.sub (local.get $m202line_end) (local.get $line_start))
              (local.get $content_start)
              (i32.const 1)
              (local.get $value_start)
              (i32.sub (local.get $content_end) (local.get $value_start)))
            (return (i32.const 0))))))
    (local.set $colon (call $find_colon (local.get $ptr) (local.get $content_start) (local.get $content_end)))
    (if (i32.ge_s (local.get $colon) (i32.const 0))
      (then
        (local.set $value_start (call $m202skip_ws (local.get $ptr) (local.get $content_end) (i32.add (local.get $colon) (i32.const 1))))
        (call $write8
          (local.get $out)
          (i32.const 2)
          (local.get $indent)
          (local.get $line_start)
          (i32.sub (local.get $m202line_end) (local.get $line_start))
          (local.get $content_start)
          (i32.sub (call $m202trim_right (local.get $ptr) (local.get $content_start) (local.get $colon)) (local.get $content_start))
          (local.get $value_start)
          (i32.sub (local.get $content_end) (local.get $value_start)))
        (return (i32.const 0))))
    (call $write8
      (local.get $out)
      (i32.const 4)
      (local.get $indent)
      (local.get $line_start)
      (i32.sub (local.get $m202line_end) (local.get $line_start))
      (local.get $content_start)
      (i32.sub (local.get $content_end) (local.get $content_start))
      (local.get $content_start)
      (i32.sub (local.get $content_end) (local.get $content_start)))
    (i32.const 0))

  (func (export "yaml_scan_document") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $p i32)
    (local $status i32)
    (local $kind i32)
    (local $indent i32)
    (local $line_count i32)
    (local $nonempty_count i32)
    (local $doc_count i32)
    (local $max_indent i32)
    (local $mapping_count i32)
    (local $sequence_count i32)
    (local $comment_count i32)
    (block $done
      (loop $again
      (if (i32.ge_u (local.get $p) (local.get $len)) (then (br $done)))
      (local.set $status (call $yaml_scan_line (local.get $ptr) (local.get $len) (local.get $p) (i32.add (local.get $out) (i32.const 64))))
      (if (local.get $status) (then (return (local.get $status))))
      (local.set $kind (i32.load (i32.add (local.get $out) (i32.const 64))))
      (local.set $indent (i32.load (i32.add (local.get $out) (i32.const 68))))
      (local.set $line_count (i32.add (local.get $line_count) (i32.const 1)))
      (if (i32.gt_u (local.get $indent) (local.get $max_indent)) (then (local.set $max_indent (local.get $indent))))
      (if (i32.ne (local.get $kind) (i32.const 0)) (then (local.set $nonempty_count (i32.add (local.get $nonempty_count) (i32.const 1)))))
      (if (i32.eq (local.get $kind) (i32.const 1)) (then (local.set $comment_count (i32.add (local.get $comment_count) (i32.const 1)))))
      (if (i32.eq (local.get $kind) (i32.const 2)) (then (local.set $mapping_count (i32.add (local.get $mapping_count) (i32.const 1)))))
      (if (i32.eq (local.get $kind) (i32.const 3)) (then (local.set $sequence_count (i32.add (local.get $sequence_count) (i32.const 1)))))
      (if (i32.eq (local.get $kind) (i32.const 5)) (then (local.set $doc_count (i32.add (local.get $doc_count) (i32.const 1)))))
      (local.set $p (call $m202next_line (local.get $ptr) (local.get $len) (call $m202line_end (local.get $ptr) (local.get $len) (local.get $p))))
      (br $again)))
    (call $write8
      (local.get $out)
      (local.get $line_count)
      (local.get $nonempty_count)
      (local.get $doc_count)
      (local.get $max_indent)
      (local.get $mapping_count)
      (local.get $sequence_count)
      (local.get $comment_count)
      (local.get $p))
    (i32.const 0))
