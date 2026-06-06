(module
  (import "edgerun-core" "memory" (memory 1))
;; Captures the useful portable semantics from edgerun-js-sys:
  ;; ECMAScript global families, Temporal option/unit spellings, and futures glue state.
  (func $m122lower (param $c i32) (result i32)
    (if (result i32)
      (i32.and (i32.ge_u (local.get $c) (i32.const 65)) (i32.le_u (local.get $c) (i32.const 90)))
      (then (i32.add (local.get $c) (i32.const 32)))
      (else (local.get $c))))

  (func $m122fnv1a_lower (param $ptr i32) (param $len i32) (result i32)
    (local $end i32) (local $h i32)
    (local.set $end (i32.add (local.get $ptr) (local.get $len)))
    (local.set $h (i32.const 0x811c9dc5))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $ptr) (local.get $end)))
        (local.set $h
          (i32.mul
            (i32.xor (local.get $h) (call $m122lower (i32.load8_u (local.get $ptr))))
            (i32.const 0x01000193)))
        (local.set $ptr (i32.add (local.get $ptr) (i32.const 1)))
        (br $loop)))
    (local.get $h))

  (func $m122is (param $h i32) (param $want i32) (result i32)
    (i32.eq (local.get $h) (local.get $want)))

  (func (export "proto_standard_id") (result i32) (i32.const 300134))

  ;; Global kind codes:
  ;; 1 object, 2 function, 3 array, 4 promise, 5 map, 6 set, 7 weakmap, 8 weakset,
  ;; 9 reflect, 10 json, 11 math, 12 symbol, 13 bigint, 14 date, 15 regexp,
  ;; 16 error, 17 arraybuffer, 18 dataview, 19 typedarray, 20 temporal, 21 atomics.
  (func (export "js_sys_global_kind") (param $ptr i32) (param $len i32) (result i32)
    (local $h i32)
    (local.set $h (call $m122fnv1a_lower (local.get $ptr) (local.get $len)))
    (if (call $m122is (local.get $h) (i32.const 0xb8c60cba)) (then (return (i32.const 1))))
    (if (call $m122is (local.get $h) (i32.const 0x9ed64249)) (then (return (i32.const 2))))
    (if (call $m122is (local.get $h) (i32.const 0x8a58ad26)) (then (return (i32.const 3))))
    (if (call $m122is (local.get $h) (i32.const 0x2217e942)) (then (return (i32.const 4))))
    (if (call $m122is (local.get $h) (i32.const 0xdfa2efb1)) (then (return (i32.const 5))))
    (if (call $m122is (local.get $h) (i32.const 0xc6270703)) (then (return (i32.const 6))))
    (if (call $m122is (local.get $h) (i32.const 0xd06836c5)) (then (return (i32.const 7))))
    (if (call $m122is (local.get $h) (i32.const 0x2393219f)) (then (return (i32.const 8))))
    (if (call $m122is (local.get $h) (i32.const 0x92c778aa)) (then (return (i32.const 9))))
    (if (call $m122is (local.get $h) (i32.const 0x36a1a243)) (then (return (i32.const 10))))
    (if (call $m122is (local.get $h) (i32.const 0xee88998f)) (then (return (i32.const 11))))
    (if (call $m122is (local.get $h) (i32.const 0xf3fb51d1)) (then (return (i32.const 12))))
    (if (call $m122is (local.get $h) (i32.const 0x8a67a5ca)) (then (return (i32.const 13))))
    (if (call $m122is (local.get $h) (i32.const 0xd472dc59)) (then (return (i32.const 14))))
    (if (call $m122is (local.get $h) (i32.const 0xf7863c98)) (then (return (i32.const 15))))
    (if (call $m122is (local.get $h) (i32.const 0x21918751)) (then (return (i32.const 16))))
    (if (call $m122is (local.get $h) (i32.const 0x4062a559)) (then (return (i32.const 16))))
    (if (call $m122is (local.get $h) (i32.const 0x94dea414)) (then (return (i32.const 16))))
    (if (call $m122is (local.get $h) (i32.const 0xbf228dd0)) (then (return (i32.const 17))))
    (if (call $m122is (local.get $h) (i32.const 0x3f2e3dd8)) (then (return (i32.const 18))))
    (if (call $m122is (local.get $h) (i32.const 0x9e602273)) (then (return (i32.const 19))))
    (if (call $m122is (local.get $h) (i32.const 0x66ce0730)) (then (return (i32.const 19))))
    (if (call $m122is (local.get $h) (i32.const 0xcd2507c0)) (then (return (i32.const 19))))
    (if (call $m122is (local.get $h) (i32.const 0x2686acda)) (then (return (i32.const 19))))
    (if (call $m122is (local.get $h) (i32.const 0xcaa70a53)) (then (return (i32.const 19))))
    (if (call $m122is (local.get $h) (i32.const 0x5d69243c)) (then (return (i32.const 19))))
    (if (call $m122is (local.get $h) (i32.const 0x42dfc421)) (then (return (i32.const 19))))
    (if (call $m122is (local.get $h) (i32.const 0x7f5f9d4d)) (then (return (i32.const 19))))
    (if (call $m122is (local.get $h) (i32.const 0x9c4e1504)) (then (return (i32.const 19))))
    (if (call $m122is (local.get $h) (i32.const 0x4f49ef7d)) (then (return (i32.const 19))))
    (if (call $m122is (local.get $h) (i32.const 0xc15ef4fc)) (then (return (i32.const 19))))
    (if (call $m122is (local.get $h) (i32.const 0x3a1353b3)) (then (return (i32.const 20))))
    (if (call $m122is (local.get $h) (i32.const 0xc032d849)) (then (return (i32.const 21))))
    (i32.const 0))

  ;; Family codes: 1 constructor/object, 2 collection, 3 namespace, 4 numeric,
  ;; 5 error, 6 buffer/view, 7 typed-array, 8 temporal, 9 concurrency.
  (func (export "js_sys_binding_family") (param $kind i32) (result i32)
    (if (i32.or (i32.or (i32.eq (local.get $kind) (i32.const 5)) (i32.eq (local.get $kind) (i32.const 6)))
                (i32.or (i32.eq (local.get $kind) (i32.const 7)) (i32.eq (local.get $kind) (i32.const 8))))
      (then (return (i32.const 2))))
    (if (i32.or (i32.or (i32.eq (local.get $kind) (i32.const 9)) (i32.eq (local.get $kind) (i32.const 10)))
                (i32.eq (local.get $kind) (i32.const 11)))
      (then (return (i32.const 3))))
    (if (i32.or (i32.eq (local.get $kind) (i32.const 12)) (i32.eq (local.get $kind) (i32.const 13)))
      (then (return (i32.const 4))))
    (if (i32.eq (local.get $kind) (i32.const 16)) (then (return (i32.const 5))))
    (if (i32.or (i32.eq (local.get $kind) (i32.const 17)) (i32.eq (local.get $kind) (i32.const 18)))
      (then (return (i32.const 6))))
    (if (i32.eq (local.get $kind) (i32.const 19)) (then (return (i32.const 7))))
    (if (i32.eq (local.get $kind) (i32.const 20)) (then (return (i32.const 8))))
    (if (i32.eq (local.get $kind) (i32.const 21)) (then (return (i32.const 9))))
    (if (i32.gt_u (local.get $kind) (i32.const 0)) (then (return (i32.const 1))))
    (i32.const 0))

  ;; Temporal option codes:
  ;; 1 constrain, 2 reject, 3 balance, 4 compatible, 5 earlier, 6 later,
  ;; 7 use, 8 prefer, 9 ignore, 10 auto, 11 always, 12 never, 13 critical,
  ;; 14 next, 15 previous.
  (func (export "js_sys_temporal_option_code") (param $ptr i32) (param $len i32) (result i32)
    (local $h i32)
    (local.set $h (call $m122fnv1a_lower (local.get $ptr) (local.get $len)))
    (if (call $m122is (local.get $h) (i32.const 0x5216f40a)) (then (return (i32.const 1))))
    (if (call $m122is (local.get $h) (i32.const 0x8534cac6)) (then (return (i32.const 2))))
    (if (call $m122is (local.get $h) (i32.const 0x989a92eb)) (then (return (i32.const 3))))
    (if (call $m122is (local.get $h) (i32.const 0xd697fc33)) (then (return (i32.const 4))))
    (if (call $m122is (local.get $h) (i32.const 0x3bf19b1b)) (then (return (i32.const 5))))
    (if (call $m122is (local.get $h) (i32.const 0x523ab9a9)) (then (return (i32.const 6))))
    (if (call $m122is (local.get $h) (i32.const 0x5791c4f4)) (then (return (i32.const 7))))
    (if (call $m122is (local.get $h) (i32.const 0x0a3bda77)) (then (return (i32.const 8))))
    (if (call $m122is (local.get $h) (i32.const 0x7e3f1843)) (then (return (i32.const 9))))
    (if (call $m122is (local.get $h) (i32.const 0x923fa396)) (then (return (i32.const 10))))
    (if (call $m122is (local.get $h) (i32.const 0x6736afe4)) (then (return (i32.const 11))))
    (if (call $m122is (local.get $h) (i32.const 0x0ac95089)) (then (return (i32.const 12))))
    (if (call $m122is (local.get $h) (i32.const 0x51540388)) (then (return (i32.const 13))))
    (if (call $m122is (local.get $h) (i32.const 0x5cb68de8)) (then (return (i32.const 14))))
    (if (call $m122is (local.get $h) (i32.const 0xf45be528)) (then (return (i32.const 15))))
    (i32.const 0))

  ;; Unit codes: 1 year, 2 month, 3 week, 4 day, 5 hour, 6 minute,
  ;; 7 second, 8 millisecond, 9 microsecond, 10 nanosecond. Plural accepted.
  (func (export "js_sys_temporal_unit_code") (param $ptr i32) (param $len i32) (result i32)
    (local $h i32)
    (local.set $h (call $m122fnv1a_lower (local.get $ptr) (local.get $len)))
    (if (i32.or (call $m122is (local.get $h) (i32.const 0xae7f4d1c)) (call $m122is (local.get $h) (i32.const 0x2166e5bd))) (then (return (i32.const 1))))
    (if (i32.or (call $m122is (local.get $h) (i32.const 0xd67a0605)) (call $m122is (local.get $h) (i32.const 0x18182bc2))) (then (return (i32.const 2))))
    (if (i32.or (call $m122is (local.get $h) (i32.const 0xd5c6965d)) (call $m122is (local.get $h) (i32.const 0xb59e6a6a))) (then (return (i32.const 3))))
    (if (i32.or (call $m122is (local.get $h) (i32.const 0xe44f21fd)) (call $m122is (local.get $h) (i32.const 0xf691d28a))) (then (return (i32.const 4))))
    (if (i32.or (call $m122is (local.get $h) (i32.const 0xb6032c0f)) (call $m122is (local.get $h) (i32.const 0x02ff0734))) (then (return (i32.const 5))))
    (if (i32.or (call $m122is (local.get $h) (i32.const 0x38e70f69)) (call $m122is (local.get $h) (i32.const 0xadbcc5ee))) (then (return (i32.const 6))))
    (if (i32.or (call $m122is (local.get $h) (i32.const 0xabf8d4dd)) (call $m122is (local.get $h) (i32.const 0x66b6cdea))) (then (return (i32.const 7))))
    (if (i32.or (call $m122is (local.get $h) (i32.const 0x02fc34ea)) (call $m122is (local.get $h) (i32.const 0x4c06ccdb))) (then (return (i32.const 8))))
    (if (i32.or (call $m122is (local.get $h) (i32.const 0xb59808c3)) (call $m122is (local.get $h) (i32.const 0x8e55ad10))) (then (return (i32.const 9))))
    (if (i32.or (call $m122is (local.get $h) (i32.const 0x24da3897)) (call $m122is (local.get $h) (i32.const 0xe7878eec))) (then (return (i32.const 10))))
    (i32.const 0))

  ;; "auto" => 10, ASCII digits 0..9 => that digit, otherwise invalid -1.
  (func (export "js_sys_fractional_second_digits") (param $ptr i32) (param $len i32) (result i32)
    (if (call $m122is (call $m122fnv1a_lower (local.get $ptr) (local.get $len)) (i32.const 0x923fa396))
      (then (return (i32.const 10))))
    (if (i32.eq (local.get $len) (i32.const 1))
      (then
        (if (i32.and (i32.ge_u (i32.load8_u (local.get $ptr)) (i32.const 48))
                     (i32.le_u (i32.load8_u (local.get $ptr)) (i32.const 57)))
          (then (return (i32.sub (i32.load8_u (local.get $ptr)) (i32.const 48)))))))
    (i32.const -1))

  ;; Queue scheduling: already scheduled => no-op; otherwise queueMicrotask if
  ;; present, Promise.then fallback if not.
  (func (export "js_sys_queue_schedule_state") (param $m122is_scheduled i32) (param $has_queue_microtask i32) (result i32)
    (if (local.get $m122is_scheduled) (then (return (i32.const 0))))
    (if (local.get $has_queue_microtask) (then (return (i32.const 1))))
    (i32.const 2))

  ;; A tick runs only the tasks present at tick start; appended tasks wait.
  (func (export "js_sys_queue_tick_run_count") (param $initial_len i32) (result i32)
    (local.get $initial_len))

  ;; Stream poll result codes: 0 pending, 1 ready-none/done, 2 ready-item,
  ;; 3 ready-error/done. If no cached next future, iterator.next() is called.
  (func (export "js_sys_stream_poll")
    (param $done i32) (param $has_next i32) (param $next_iterator_ok i32)
    (param $future_ready i32) (param $future_ok i32) (param $iterator_done i32)
    (result i32)
    (if (local.get $done) (then (return (i32.const 1))))
    (if (i32.and (i32.eqz (local.get $has_next)) (i32.eqz (local.get $next_iterator_ok)))
      (then (return (i32.const 3))))
    (if (i32.eqz (local.get $future_ready)) (then (return (i32.const 0))))
    (if (i32.eqz (local.get $future_ok)) (then (return (i32.const 3))))
    (if (local.get $iterator_done) (then (return (i32.const 1))))
    (i32.const 2))

  ;; waitAsync strategy: native Atomics.waitAsync when available, worker fallback otherwise.
  (func (export "js_sys_wait_async_strategy") (param $has_native_wait_async i32) (result i32)
    (if (local.get $has_native_wait_async) (then (return (i32.const 1))))
    (i32.const 2))
)
