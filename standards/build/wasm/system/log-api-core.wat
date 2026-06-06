(module
  (import "edgerun-core" "memory" (memory 1))
;; Captures remaining tracing/tracing-core/tracing-log/tracing-attributes semantics.
  ;; Level codes: off=0, error=1, warn=2, info=3, debug=4, trace=5.
  ;; Interest codes: never=0, sometimes=1, always=2.

  (func $m186lower (param $c i32) (result i32)
    (if (result i32)
      (i32.and (i32.ge_u (local.get $c) (i32.const 65)) (i32.le_u (local.get $c) (i32.const 90)))
      (then (i32.add (local.get $c) (i32.const 32)))
      (else (local.get $c))))

  (func $m186fnv1a_lower (param $ptr i32) (param $len i32) (result i32)
    (local $end i32) (local $h i32)
    (local.set $end (i32.add (local.get $ptr) (local.get $len)))
    (local.set $h (i32.const 0x811c9dc5))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $ptr) (local.get $end)))
        (local.set $h
          (i32.mul
            (i32.xor (local.get $h) (call $m186lower (i32.load8_u (local.get $ptr))))
            (i32.const 0x01000193)))
        (local.set $ptr (i32.add (local.get $ptr) (i32.const 1)))
        (br $loop)))
    (local.get $h))

  (func (export "proto_standard_id") (result i32) (i32.const 300136))

  (func (export "tracing_level_code") (param $ptr i32) (param $len i32) (result i32)
    (local $h i32)
    (local.set $h (call $m186fnv1a_lower (local.get $ptr) (local.get $len)))
    (if (i32.eq (local.get $h) (i32.const 0xab3a8a0a)) (then (return (i32.const 0))))
    (if (i32.eq (local.get $h) (i32.const 0x21918751)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $h) (i32.const 0x84fa6af1)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $h) (i32.const 0x0fb40705)) (then (return (i32.const 3))))
    (if (i32.eq (local.get $h) (i32.const 0x5864ed98)) (then (return (i32.const 4))))
    (if (i32.eq (local.get $h) (i32.const 0x813d75ae)) (then (return (i32.const 5))))
    (i32.const -1))

  ;; Tracing enables a level when level <= filter and the filter is not off.
  (func $tracing_level_enabled (export "tracing_level_enabled") (param $level i32) (param $filter i32) (result i32)
    (i32.and
      (i32.and (i32.gt_s (local.get $level) (i32.const 0)) (i32.gt_s (local.get $filter) (i32.const 0)))
      (i32.le_s (local.get $level) (local.get $filter))))

  ;; Feature precedence: release feature wins in release builds, debug feature
  ;; wins otherwise, unset defaults to TRACE.
  (func (export "tracing_static_max_level")
    (param $is_release i32) (param $release_feature i32) (param $debug_feature i32)
    (result i32)
    (if (i32.and (local.get $is_release) (i32.ge_s (local.get $release_feature) (i32.const 0)))
      (then (return (local.get $release_feature))))
    (if (i32.ge_s (local.get $debug_feature) (i32.const 0))
      (then (return (local.get $debug_feature))))
    (i32.const 5))

  ;; Combine cached interests across active subscribers. Dynamic interest wins,
  ;; then any always, otherwise never.
  (func (export "tracing_interest_join") (param $a i32) (param $b i32) (result i32)
    (if (i32.or (i32.eq (local.get $a) (i32.const 1)) (i32.eq (local.get $b) (i32.const 1)))
      (then (return (i32.const 1))))
    (if (i32.or (i32.eq (local.get $a) (i32.const 2)) (i32.eq (local.get $b) (i32.const 2)))
      (then (return (i32.const 2))))
    (i32.const 0))

  (func $tracing_cached_interest_enabled (export "tracing_cached_interest_enabled") (param $interest i32) (param $dynamic_enabled i32) (result i32)
    (if (i32.eq (local.get $interest) (i32.const 2)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $interest) (i32.const 1)) (then (return (local.get $dynamic_enabled))))
    (i32.const 0))

  ;; Register_callsite default: enabled => always, disabled => never.
  (func (export "tracing_register_callsite_default") (param $enabled i32) (result i32)
    (select (i32.const 2) (i32.const 0) (local.get $enabled)))

  ;; Span IDs are NonZeroU64.
  (func (export "tracing_span_id_valid") (param $id i64) (result i32)
    (i64.ne (local.get $id) (i64.const 0)))

  ;; Parent/current codes: 0 current/contextual, 1 root, 2 explicit.
  (func (export "tracing_parent_code") (param $has_explicit i32) (param $root i32) (result i32)
    (if (local.get $has_explicit) (then (return (i32.const 2))))
    (if (local.get $root) (then (return (i32.const 1))))
    (i32.const 0))

  ;; Current span state: 0 unknown, 1 known none, 2 current span.
  (func (export "tracing_current_state") (param $has_current i32) (param $known_none i32) (result i32)
    (if (local.get $has_current) (then (return (i32.const 2))))
    (if (local.get $known_none) (then (return (i32.const 1))))
    (i32.const 0))

  (func (export "tracing_field_index_valid") (param $index i32) (param $field_count i32) (result i32)
    (i32.lt_u (local.get $index) (local.get $field_count)))

  (func (export "tracing_value_present_count") (param $mask i32) (param $field_count i32) (result i32)
    (local $i i32) (local $n i32)
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $field_count)))
        (if (i32.and (local.get $mask) (i32.shl (i32.const 1) (local.get $i)))
          (then (local.set $n (i32.add (local.get $n) (i32.const 1)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (local.get $n))

  ;; Value kind codes from tracing field visitor dispatch.
  ;; i64=1,u64=2,i128=3,u128=4,f64=5,bool=6,str=7,debug/display=8,valuable=9.
  (func (export "tracing_value_kind_supported") (param $kind i32) (result i32)
    (i32.and (i32.ge_u (local.get $kind) (i32.const 1)) (i32.le_u (local.get $kind) (i32.const 9))))

  ;; Metadata kind names: span=1, event=2.
  (func (export "tracing_metadata_kind_code") (param $ptr i32) (param $len i32) (result i32)
    (local $h i32)
    (local.set $h (call $m186fnv1a_lower (local.get $ptr) (local.get $len)))
    (if (i32.eq (local.get $h) (i32.const 0x290182c1)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $h) (i32.const 0xfe30d09f)) (then (return (i32.const 2))))
    (i32.const 0))

  ;; Macro construction gate: static filter, cached interest, and dispatcher enabled all allow emission.
  (func (export "tracing_macro_emit_allowed")
    (param $level i32) (param $static_filter i32) (param $interest i32) (param $dynamic_enabled i32)
    (result i32)
    (i32.and
      (call $tracing_level_enabled (local.get $level) (local.get $static_filter))
      (call $tracing_cached_interest_enabled (local.get $interest) (local.get $dynamic_enabled))))

  ;; LogTracer enable gate: compare log level to current tracing max, then ignore target prefixes,
  ;; then ask dispatcher/cache.
  (func (export "tracing_log_enabled")
    (param $log_level i32) (param $current_tracing_filter i32) (param $ignored_target i32)
    (param $dispatcher_enabled i32)
    (result i32)
    (if (i32.eqz (call $tracing_level_enabled (local.get $log_level) (local.get $current_tracing_filter)))
      (then (return (i32.const 0))))
    (if (local.get $ignored_target) (then (return (i32.const 0))))
    (local.get $dispatcher_enabled))

  ;; Log record dispatch only happens when enabled.
  (func (export "tracing_log_dispatch_action") (param $enabled i32) (result i32)
    (select (i32.const 1) (i32.const 0) (local.get $enabled)))

  ;; #[instrument] macro shape: creates span, records args, enters before function body,
  ;; awaits async body inside instrumented future when async.
  ;; return bitmask: 1 span, 2 args, 4 enter, 8 async-future.
  (func (export "tracing_instrument_shape") (param $records_args i32) (param $is_async i32) (result i32)
    (i32.or
      (i32.or (i32.const 1) (i32.const 4))
      (i32.or (select (i32.const 2) (i32.const 0) (local.get $records_args))
              (select (i32.const 8) (i32.const 0) (local.get $is_async)))))
)
