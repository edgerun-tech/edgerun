(module
  ;; Captures log facade, valuable structured values, and tracing-opentelemetry layer semantics.
  ;; Level/filter codes: off=0,error=1,warn=2,info=3,debug=4,trace=5.
  (func (export "proto_abi_version") (result i32) (i32.const 2))
  (func (export "proto_standard_id") (result i32) (i32.const 300140))

  (func $telemetry_log_level_enabled (export "telemetry_log_level_enabled") (param $level i32) (param $filter i32) (result i32)
    (i32.and
      (i32.and (i32.gt_s (local.get $level) (i32.const 0)) (i32.gt_s (local.get $filter) (i32.const 0)))
      (i32.le_s (local.get $level) (local.get $filter))))

  (func (export "telemetry_log_set_logger_result") (param $already_set i32) (result i32)
    ;; 0 ok, 1 SetLoggerError
    (select (i32.const 1) (i32.const 0) (local.get $already_set)))

  (func (export "telemetry_log_macro_evaluates")
    (param $level i32) (param $static_filter i32) (param $runtime_filter i32) (param $logger_enabled i32)
    (result i32)
    (i32.and
      (call $telemetry_log_level_enabled (local.get $level) (local.get $static_filter))
      (i32.and
        (call $telemetry_log_level_enabled (local.get $level) (local.get $runtime_filter))
        (local.get $logger_enabled))))

  (func (export "telemetry_log_dispatch_action") (param $logger_registered i32) (param $record_enabled i32) (result i32)
    ;; 0 noop, 1 call logger.log
    (i32.and (local.get $logger_registered) (local.get $record_enabled)))

  ;; Key-value value kind codes:
  ;; 0 null,1 u64,2 i64,3 u128,4 i128,5 f64,6 bool,7 str,8 char,9 error,
  ;; 10 debug,11 display,12 sval/structured.
  (func (export "telemetry_kv_visit_method") (param $value_kind i32) (result i32)
    (if (i32.eq (local.get $value_kind) (i32.const 0)) (then (return (i32.const 0))))
    (if (i32.and (i32.ge_u (local.get $value_kind) (i32.const 1)) (i32.le_u (local.get $value_kind) (i32.const 12)))
      (then (return (local.get $value_kind))))
    (i32.const -1))

  (func (export "telemetry_kv_option_kind") (param $is_some i32) (param $inner_kind i32) (result i32)
    (select (local.get $inner_kind) (i32.const 0) (local.get $is_some)))

  (func (export "telemetry_kv_source_get_steps") (param $pair_count i32) (param $match_index i32) (result i32)
    ;; get() visits pairs until match; match_index < pair_count means found at index.
    (if (i32.lt_u (local.get $match_index) (local.get $pair_count))
      (then (return (i32.add (local.get $match_index) (i32.const 1)))))
    (local.get $pair_count))

  ;; Valuable Value categories: primitive=1, struct=2, enum=3, list=4, map=5,
  ;; tuple=6, unit=7.
  (func (export "telemetry_valuable_visit_route") (param $value_kind i32) (result i32)
    (if (i32.and (i32.ge_u (local.get $value_kind) (i32.const 1)) (i32.le_u (local.get $value_kind) (i32.const 16)))
      (then (return (i32.const 1))))
    (if (i32.eq (local.get $value_kind) (i32.const 20)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $value_kind) (i32.const 21)) (then (return (i32.const 3))))
    (if (i32.eq (local.get $value_kind) (i32.const 22)) (then (return (i32.const 4))))
    (if (i32.eq (local.get $value_kind) (i32.const 23)) (then (return (i32.const 5))))
    (if (i32.eq (local.get $value_kind) (i32.const 24)) (then (return (i32.const 6))))
    (if (i32.eq (local.get $value_kind) (i32.const 0)) (then (return (i32.const 7))))
    (i32.const 0))

  (func (export "telemetry_valuable_named_iter_count") (param $field_count i32) (result i32)
    (local.get $field_count))

  (func (export "telemetry_valuable_map_visit_calls") (param $entry_count i32) (result i32)
    ;; Each map entry calls visit_entry(key,value) once.
    (local.get $entry_count))

  ;; OTel span kind codes: server=1, client=2, producer=3, consumer=4, internal=5.
  (func (export "telemetry_otel_span_kind_valid") (param $kind_code i32) (result i32)
    (i32.and (i32.ge_u (local.get $kind_code) (i32.const 1)) (i32.le_u (local.get $kind_code) (i32.const 5))))

  ;; Status codes: unset=0,error=1,ok=2.
  (func (export "telemetry_otel_status_code") (param $field_code i32) (result i32)
    (if (i32.eq (local.get $field_code) (i32.const 1)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $field_code) (i32.const 2)) (then (return (i32.const 1))))
    (i32.const 0))

  ;; Span event visitor field action:
  ;; 1 set event name from message, 2 skip log.* metadata, 3 update span builder,
  ;; 4 add attribute.
  (func (export "telemetry_otel_event_field_action")
    (param $is_message i32) (param $is_log_metadata i32) (param $is_otel_control i32)
    (result i32)
    (if (local.get $is_message) (then (return (i32.const 1))))
    (if (local.get $is_log_metadata) (then (return (i32.const 2))))
    (if (local.get $is_otel_control) (then (return (i32.const 3))))
    (i32.const 4))

  ;; Metric prefix kind: monotonic_counter=1,counter=2,histogram=3,gauge=4,attribute=0.
  ;; Value type: u64=1,i64=2,f64=3,bool/str/debug attr otherwise.
  ;; Returns instrument type:
  ;; 1 CounterU64,2 CounterF64,3 UpDownCounterI64,4 UpDownCounterF64,
  ;; 5 HistogramU64,6 HistogramF64,7 GaugeU64,8 GaugeI64,9 GaugeF64,0 attr,-1 ignored.
  (func (export "telemetry_metric_instrument_kind")
    (param $prefix_kind i32) (param $value_type i32) (param $u64_fits_i64 i32)
    (result i32)
    (if (i32.eq (local.get $prefix_kind) (i32.const 1))
      (then
        (if (i32.eq (local.get $value_type) (i32.const 1)) (then (return (i32.const 1))))
        (if (i32.eq (local.get $value_type) (i32.const 2)) (then (return (i32.const 1))))
        (if (i32.eq (local.get $value_type) (i32.const 3)) (then (return (i32.const 2))))))
    (if (i32.eq (local.get $prefix_kind) (i32.const 2))
      (then
        (if (i32.eq (local.get $value_type) (i32.const 1))
          (then (return (select (i32.const 3) (i32.const -1) (local.get $u64_fits_i64)))))
        (if (i32.eq (local.get $value_type) (i32.const 2)) (then (return (i32.const 3))))
        (if (i32.eq (local.get $value_type) (i32.const 3)) (then (return (i32.const 4))))))
    (if (i32.eq (local.get $prefix_kind) (i32.const 3))
      (then
        (if (i32.eq (local.get $value_type) (i32.const 1)) (then (return (i32.const 5))))
        (if (i32.eq (local.get $value_type) (i32.const 3)) (then (return (i32.const 6))))))
    (if (i32.eq (local.get $prefix_kind) (i32.const 4))
      (then
        (if (i32.eq (local.get $value_type) (i32.const 1)) (then (return (i32.const 7))))
        (if (i32.eq (local.get $value_type) (i32.const 2)) (then (return (i32.const 8))))
        (if (i32.eq (local.get $value_type) (i32.const 3)) (then (return (i32.const 9))))))
    (i32.const 0))

  ;; update_or_insert first reads, then writes/inserts only when absent.
  ;; 1 update existing, 2 insert then update.
  (func (export "telemetry_metric_update_path") (param $already_exists i32) (result i32)
    (select (i32.const 1) (i32.const 2) (local.get $already_exists)))

  ;; Layer data state: builder present until activation consumes it into OTel context/span.
  ;; 0 missing,1 builder,2 active context.
  (func (export "telemetry_otel_activation_result") (param $state i32) (result i32)
    (if (i32.eq (local.get $state) (i32.const 1)) (then (return (i32.const 2))))
    (local.get $state))

  (memory (export "memory") 1)
)
