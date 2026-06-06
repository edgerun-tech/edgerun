(module
  (import "edgerun-core" "memory" (memory 1))
;; Captures edgerun-opentelemetry-upstream API semantics: W3C baggage,
  ;; tracestate/traceparent validation, metric builders, span status/kind, and log severity.
  (func $m145is_hex (param $c i32) (result i32)
    (i32.or
      (i32.or (i32.and (i32.ge_u (local.get $c) (i32.const 48)) (i32.le_u (local.get $c) (i32.const 57)))
              (i32.and (i32.ge_u (local.get $c) (i32.const 97)) (i32.le_u (local.get $c) (i32.const 102))))
      (i32.and (i32.ge_u (local.get $c) (i32.const 65)) (i32.le_u (local.get $c) (i32.const 70)))))

  (func $is_lower_or_digit (param $c i32) (result i32)
    (i32.or
      (i32.and (i32.ge_u (local.get $c) (i32.const 97)) (i32.le_u (local.get $c) (i32.const 122)))
      (i32.and (i32.ge_u (local.get $c) (i32.const 48)) (i32.le_u (local.get $c) (i32.const 57)))))

  (func $is_tracestate_special (param $c i32) (result i32)
    (i32.or
      (i32.or (i32.eq (local.get $c) (i32.const 95)) (i32.eq (local.get $c) (i32.const 45)))
      (i32.or (i32.eq (local.get $c) (i32.const 42)) (i32.eq (local.get $c) (i32.const 47)))))

  (func $baggage_bad_key_char (param $c i32) (result i32)
    (i32.or
      (i32.or
        (i32.or (i32.eq (local.get $c) (i32.const 40)) (i32.eq (local.get $c) (i32.const 41)))
        (i32.or (i32.eq (local.get $c) (i32.const 44)) (i32.eq (local.get $c) (i32.const 47))))
      (i32.or
        (i32.or
          (i32.or (i32.eq (local.get $c) (i32.const 58)) (i32.eq (local.get $c) (i32.const 59)))
          (i32.or (i32.eq (local.get $c) (i32.const 60)) (i32.eq (local.get $c) (i32.const 61))))
        (i32.or
          (i32.or
            (i32.or (i32.eq (local.get $c) (i32.const 62)) (i32.eq (local.get $c) (i32.const 63)))
            (i32.or (i32.eq (local.get $c) (i32.const 64)) (i32.eq (local.get $c) (i32.const 91))))
          (i32.or
            (i32.or (i32.eq (local.get $c) (i32.const 92)) (i32.eq (local.get $c) (i32.const 93)))
            (i32.or
              (i32.or (i32.eq (local.get $c) (i32.const 123)) (i32.eq (local.get $c) (i32.const 125)))
              (i32.eq (local.get $c) (i32.const 34))))))))

  (func $all_hex_nonzero (param $ptr i32) (param $len i32) (result i32)
    (local $end i32) (local $nonzero i32) (local $c i32)
    (local.set $end (i32.add (local.get $ptr) (local.get $len)))
    (block $bad
      (loop $loop
        (br_if $bad (i32.ge_u (local.get $ptr) (local.get $end)))
        (local.set $c (i32.load8_u (local.get $ptr)))
        (if (i32.eqz (call $m145is_hex (local.get $c))) (then (return (i32.const 0))))
        (if (i32.ne (local.get $c) (i32.const 48)) (then (local.set $nonzero (i32.const 1))))
        (local.set $ptr (i32.add (local.get $ptr) (i32.const 1)))
        (br $loop)))
    (local.get $nonzero))

  (func (export "proto_standard_id") (result i32) (i32.const 300135))

  (func (export "otel_baggage_key_valid") (param $ptr i32) (param $len i32) (result i32)
    (local $end i32) (local $c i32)
    (if (i32.eqz (local.get $len)) (then (return (i32.const 0))))
    (local.set $end (i32.add (local.get $ptr) (local.get $len)))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $ptr) (local.get $end)))
        (local.set $c (i32.load8_u (local.get $ptr)))
        (if (i32.or
              (i32.or (i32.lt_u (local.get $c) (i32.const 33)) (i32.gt_u (local.get $c) (i32.const 126)))
              (call $baggage_bad_key_char (local.get $c)))
          (then (return (i32.const 0))))
        (local.set $ptr (i32.add (local.get $ptr) (i32.const 1)))
        (br $loop)))
    (i32.const 1))

  ;; Insert result: 0 ok-new, 1 ok-replace, 2 invalid-key, 3 too-many-pairs, 4 too-long.
  (func (export "otel_baggage_insert_result")
    (param $entries i32) (param $current_len i32) (param $key_len i32) (param $value_len i32)
    (param $meta_len i32) (param $existed i32) (param $prev_entry_len i32) (param $key_valid i32)
    (result i32)
    (local $new_len i32)
    (if (i32.eqz (local.get $key_valid)) (then (return (i32.const 2))))
    (if (i32.and (i32.eqz (local.get $existed)) (i32.ge_u (local.get $entries) (i32.const 64)))
      (then (return (i32.const 3))))
    (local.set $new_len
      (i32.add
        (i32.sub (local.get $current_len) (select (local.get $prev_entry_len) (i32.const 0) (local.get $existed)))
        (i32.add (local.get $key_len) (i32.add (local.get $value_len) (local.get $meta_len)))))
    (if (i32.gt_u (local.get $new_len) (i32.const 8192)) (then (return (i32.const 4))))
    (select (i32.const 1) (i32.const 0) (local.get $existed)))

  (func (export "otel_tracestate_key_valid") (param $ptr i32) (param $len i32) (result i32)
    (local $end i32) (local $i i32) (local $c i32) (local $vendor i32)
    (if (i32.or (i32.eqz (local.get $len)) (i32.gt_u (local.get $len) (i32.const 256)))
      (then (return (i32.const 0))))
    (local.set $end (i32.add (local.get $ptr) (local.get $len)))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $ptr) (local.get $end)))
        (local.set $c (i32.load8_u (local.get $ptr)))
        (if (i32.eqz (i32.or (i32.or (call $is_lower_or_digit (local.get $c)) (call $is_tracestate_special (local.get $c)))
                             (i32.eq (local.get $c) (i32.const 64))))
          (then (return (i32.const 0))))
        (if (i32.and (i32.eqz (local.get $i)) (i32.eqz (call $is_lower_or_digit (local.get $c))))
          (then (return (i32.const 0))))
        (if (i32.eq (local.get $c) (i32.const 64))
          (then
            (if (i32.or (local.get $vendor) (i32.lt_u (i32.add (local.get $i) (i32.const 14)) (local.get $len)))
              (then (return (i32.const 0))))
            (local.set $vendor (i32.add (local.get $i) (i32.const 1))))
          (else
            (if (i32.and
                  (i32.and (local.get $vendor) (i32.eq (local.get $i) (local.get $vendor)))
                  (i32.eqz (call $is_lower_or_digit (local.get $c))))
              (then (return (i32.const 0))))))
        (local.set $ptr (i32.add (local.get $ptr) (i32.const 1)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (i32.const 1))

  (func (export "otel_tracestate_value_valid") (param $ptr i32) (param $len i32) (result i32)
    (local $end i32) (local $c i32)
    (if (i32.gt_u (local.get $len) (i32.const 256)) (then (return (i32.const 0))))
    (local.set $end (i32.add (local.get $ptr) (local.get $len)))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $ptr) (local.get $end)))
        (local.set $c (i32.load8_u (local.get $ptr)))
        (if (i32.or (i32.eq (local.get $c) (i32.const 44)) (i32.eq (local.get $c) (i32.const 61)))
          (then (return (i32.const 0))))
        (local.set $ptr (i32.add (local.get $ptr) (i32.const 1)))
        (br $loop)))
    (i32.const 1))

  (func (export "otel_trace_flags_with_sampled") (param $flags i32) (param $sampled i32) (result i32)
    (if (local.get $sampled)
      (then (return (i32.or (local.get $flags) (i32.const 1)))))
    (i32.and (local.get $flags) (i32.const 254)))

  (func (export "otel_trace_flags_is_sampled") (param $flags i32) (result i32)
    (i32.and (local.get $flags) (i32.const 1)))

  (func (export "otel_span_context_valid") (param $trace_nonzero i32) (param $span_nonzero i32) (result i32)
    (i32.and (local.get $trace_nonzero) (local.get $span_nonzero)))

  ;; traceparent: version(2)-trace_id(32)-span_id(16)-flags(2), nonzero ids.
  (func (export "otel_traceparent_valid") (param $ptr i32) (param $len i32) (result i32)
    (if (i32.ne (local.get $len) (i32.const 55)) (then (return (i32.const 0))))
    (if (i32.or (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 2))) (i32.const 45))
                (i32.or (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 35))) (i32.const 45))
                        (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 52))) (i32.const 45))))
      (then (return (i32.const 0))))
    (if (i32.eqz (call $m145is_hex (i32.load8_u (local.get $ptr)))) (then (return (i32.const 0))))
    (if (i32.eqz (call $m145is_hex (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))))) (then (return (i32.const 0))))
    (if (i32.eqz (call $all_hex_nonzero (i32.add (local.get $ptr) (i32.const 3)) (i32.const 32))) (then (return (i32.const 0))))
    (if (i32.eqz (call $all_hex_nonzero (i32.add (local.get $ptr) (i32.const 36)) (i32.const 16))) (then (return (i32.const 0))))
    (if (i32.eqz (call $m145is_hex (i32.load8_u (i32.add (local.get $ptr) (i32.const 53))))) (then (return (i32.const 0))))
    (call $m145is_hex (i32.load8_u (i32.add (local.get $ptr) (i32.const 54)))))

  ;; Status rank is Unset=0, Error=1, Ok=2. Ok is final via max rank.
  (func (export "otel_status_apply") (param $current i32) (param $next i32) (result i32)
    (if (i32.gt_u (local.get $current) (local.get $next)) (then (return (local.get $current))))
    (local.get $next))

  ;; Span kind bitmask: 1 sync, 2 async, 4 remote incoming, 8 remote outgoing.
  ;; 1 client, 2 server, 3 producer, 4 consumer, 5 internal.
  (func (export "otel_span_kind_relation") (param $kind i32) (result i32)
    (if (i32.eq (local.get $kind) (i32.const 1)) (then (return (i32.const 9))))
    (if (i32.eq (local.get $kind) (i32.const 2)) (then (return (i32.const 5))))
    (if (i32.eq (local.get $kind) (i32.const 3)) (then (return (i32.const 10))))
    (if (i32.eq (local.get $kind) (i32.const 4)) (then (return (i32.const 6))))
    (i32.const 0))

  (func (export "otel_metric_unit_valid") (param $ptr i32) (param $len i32) (result i32)
    (local $end i32) (local $c i32)
    (if (i32.gt_u (local.get $len) (i32.const 63)) (then (return (i32.const 0))))
    (local.set $end (i32.add (local.get $ptr) (local.get $len)))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $ptr) (local.get $end)))
        (local.set $c (i32.load8_u (local.get $ptr)))
        (if (i32.or (i32.lt_u (local.get $c) (i32.const 1)) (i32.gt_u (local.get $c) (i32.const 127)))
          (then (return (i32.const 0))))
        (local.set $ptr (i32.add (local.get $ptr) (i32.const 1)))
        (br $loop)))
    (i32.const 1))

  (func (export "otel_histogram_boundaries_valid") (param $ptr i32) (param $count i32) (result i32)
    (local $i i32) (local $prev f64) (local $cur f64)
    (if (i32.eqz (local.get $count)) (then (return (i32.const 1))))
    (local.set $cur (f64.load (local.get $ptr)))
    (if (f64.ne (local.get $cur) (local.get $cur)) (then (return (i32.const 0))))
    (if (f64.gt (f64.abs (local.get $cur)) (f64.const 1.7976931348623157e308)) (then (return (i32.const 0))))
    (local.set $prev (local.get $cur))
    (local.set $i (i32.const 1))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $count)))
        (local.set $cur (f64.load (i32.add (local.get $ptr) (i32.mul (local.get $i) (i32.const 8)))))
        (if (f64.ne (local.get $cur) (local.get $cur)) (then (return (i32.const 0))))
        (if (f64.gt (f64.abs (local.get $cur)) (f64.const 1.7976931348623157e308)) (then (return (i32.const 0))))
        (if (f64.le (local.get $cur) (local.get $prev)) (then (return (i32.const 0))))
        (local.set $prev (local.get $cur))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (i32.const 1))

  ;; Severity band: 1 trace, 2 debug, 3 info, 4 warn, 5 error, 6 fatal.
  (func (export "otel_log_severity_band") (param $severity i32) (result i32)
    (if (i32.or (i32.lt_s (local.get $severity) (i32.const 1)) (i32.gt_s (local.get $severity) (i32.const 24)))
      (then (return (i32.const 0))))
    (i32.add (i32.div_u (i32.sub (local.get $severity) (i32.const 1)) (i32.const 4)) (i32.const 1)))

  ;; Composite propagator inject/extract runs in constructor order; zero is no-op.
  (func (export "otel_composite_steps") (param $propagator_count i32) (result i32)
    (local.get $propagator_count))
)
