;; Captures edgerun-runtime core state machines: executor queue, timers,
  ;; channels, semaphore, select, cancellation, ring buffer, and host net poll gates.
  (func (export "proto_abi_version") (result i32) (i32.const 2))
  (func (export "proto_standard_id") (result i32) (i32.const 300137))

  ;; spawn enqueues one task and increments pending.
  (func (export "rt_spawn_pending_after") (param $pending i32) (result i32)
    (i32.add (local.get $pending) (i32.const 1)))

  ;; run_queue polls a snapshot. Ready tasks decrement pending; pending tasks are requeued.
  ;; packed result: low16=new_pending_count, high16=requeued_count.
  (func (export "rt_run_queue_result")
    (param $pending_count i32) (param $snapshot_len i32) (param $ready_count i32)
    (result i32)
    (local $requeued i32)
    (local.set $requeued (i32.sub (local.get $snapshot_len) (local.get $ready_count)))
    (i32.or
      (i32.and (i32.sub (local.get $pending_count) (local.get $ready_count)) (i32.const 0xffff))
      (i32.shl (i32.and (local.get $requeued) (i32.const 0xffff)) (i32.const 16))))

  ;; Builder clamps worker/blocking thread counts to at least one.
  (func (export "rt_builder_threads") (param $requested i32) (result i32)
    (if (i32.lt_u (local.get $requested) (i32.const 1)) (then (return (i32.const 1))))
    (local.get $requested))

  (func (export "rt_elapsed_since") (param $now i64) (param $start i64) (result i64)
    (if (i64.lt_u (local.get $now) (local.get $start)) (then (return (i64.const 0))))
    (i64.sub (local.get $now) (local.get $start)))

  (func (export "rt_us_to_ticks") (param $us i64) (result i64)
    (i64.mul (local.get $us) (i64.const 10)))

  (func (export "rt_ms_to_ticks") (param $ms i64) (result i64)
    (i64.mul (local.get $ms) (i64.const 10000)))

  ;; TimerWheel poll: expired entries fire; periodic expired entries remain.
  ;; packed result: low16=fired, high16=remaining.
  (func (export "rt_timer_poll_result")
    (param $entry_count i32) (param $expired_count i32) (param $expired_periodic_count i32)
    (result i32)
    (i32.or
      (i32.and (local.get $expired_count) (i32.const 0xffff))
      (i32.shl
        (i32.and
          (i32.add (i32.sub (local.get $entry_count) (local.get $expired_count)) (local.get $expired_periodic_count))
          (i32.const 0xffff))
        (i32.const 16))))

  ;; Sleep/timeout poll codes: 0 pending, 1 ready-ok, 2 elapsed.
  (func (export "rt_sleep_poll") (param $now i64) (param $deadline i64) (result i32)
    (select (i32.const 1) (i32.const 0) (i64.ge_u (local.get $now) (local.get $deadline))))

  (func (export "rt_timeout_poll") (param $now i64) (param $deadline i64) (param $inner_ready i32) (result i32)
    (if (i64.ge_u (local.get $now) (local.get $deadline)) (then (return (i32.const 2))))
    (select (i32.const 1) (i32.const 0) (local.get $inner_ready)))

  ;; One-shot channel: 0 pending/register, 1 ready value, 2 closed/empty, 3 send rejected.
  (func (export "rt_oneshot_recv_state") (param $sent i32) (param $has_value i32) (result i32)
    (if (i32.eqz (local.get $sent)) (then (return (i32.const 0))))
    (select (i32.const 1) (i32.const 2) (local.get $has_value)))

  (func (export "rt_oneshot_send_state") (param $sent i32) (result i32)
    (select (i32.const 3) (i32.const 1) (local.get $sent)))

  ;; MPSC try_send: 0 ok, 1 full, 2 closed. cap=0 is unbounded.
  (func (export "rt_mpsc_try_send")
    (param $cap i32) (param $len i32) (param $closed i32) (result i32)
    (if (local.get $closed) (then (return (i32.const 2))))
    (if (i32.and (i32.gt_u (local.get $cap) (i32.const 0)) (i32.ge_u (local.get $len) (local.get $cap)))
      (then (return (i32.const 1))))
    (i32.const 0))

  ;; MPSC try_recv: 0 value, 1 empty, 2 disconnected.
  (func (export "rt_mpsc_try_recv") (param $len i32) (param $closed i32) (result i32)
    (if (i32.gt_u (local.get $len) (i32.const 0)) (then (return (i32.const 0))))
    (select (i32.const 2) (i32.const 1) (local.get $closed)))

  ;; Semaphore try_acquire: returns new permit count or -1 when unavailable.
  (func (export "rt_semaphore_try_acquire") (param $permits i32) (result i32)
    (if (i32.eqz (local.get $permits)) (then (return (i32.const -1))))
    (i32.sub (local.get $permits) (i32.const 1)))

  ;; Dropping a permit increments permits and wakes one waiter if present.
  ;; packed: low16=new_permits, high16=wake_count.
  (func (export "rt_semaphore_release") (param $permits i32) (param $waiters i32) (result i32)
    (i32.or
      (i32.and (i32.add (local.get $permits) (i32.const 1)) (i32.const 0xffff))
      (i32.shl (select (i32.const 1) (i32.const 0) (i32.gt_u (local.get $waiters) (i32.const 0))) (i32.const 16))))

  ;; Select is left-biased by polling order.
  ;; Input bitmask has ready futures in bits 0..3. Return ready index, or -1.
  (func (export "rt_select_ready_index") (param $ready_mask i32) (param $future_count i32) (result i32)
    (local $i i32)
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $future_count)))
        (if (i32.and (local.get $ready_mask) (i32.shl (i32.const 1) (local.get $i)))
          (then (return (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (i32.const -1))

  ;; Interval poll updates next = now + duration when ready.
  (func (export "rt_interval_next_after_poll") (param $now i64) (param $next i64) (param $duration i64) (result i64)
    (if (i64.ge_u (local.get $now) (local.get $next))
      (then (return (i64.add (local.get $now) (local.get $duration)))))
    (local.get $next))

  ;; Cancellation poll: 1 ready; 0 pending and register once.
  (func (export "rt_cancel_poll") (param $cancelled_before i32) (param $registered i32) (param $cancelled_after_register i32) (result i32)
    (if (local.get $cancelled_before) (then (return (i32.const 1))))
    (select (i32.const 1) (i32.const 0) (local.get $cancelled_after_register)))

  ;; Ring capacity is next power of two, minimum 2. This accepts already rounded input too.
  (func (export "rt_ring_capacity") (param $requested i32) (result i32)
    (local $cap i32)
    (local.set $cap (i32.const 2))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $cap) (local.get $requested)))
        (if (i32.ge_u (local.get $cap) (i32.const 0x40000000)) (then (return (i32.const 0x80000000))))
        (local.set $cap (i32.shl (local.get $cap) (i32.const 1)))
        (br $loop)))
    (local.get $cap))

  (func (export "rt_ring_len") (param $head i32) (param $tail i32) (param $capacity i32) (result i32)
    (i32.and (i32.sub (local.get $tail) (local.get $head)) (i32.sub (local.get $capacity) (i32.const 1))))

  (func (export "rt_ring_push_count") (param $len i32) (param $capacity i32) (param $input_len i32) (result i32)
    (local $free i32)
    (local.set $free (i32.sub (i32.sub (local.get $capacity) (i32.const 1)) (local.get $len)))
    (select (local.get $input_len) (local.get $free) (i32.lt_u (local.get $input_len) (local.get $free))))

  ;; Host net poll: 0 ready-ok, 1 pending-would-block-and-wake, 2 ready-error.
  (func (export "rt_host_io_poll_state") (param $ok i32) (param $would_block i32) (result i32)
    (if (local.get $ok) (then (return (i32.const 0))))
    (select (i32.const 1) (i32.const 2) (local.get $would_block)))

  ;; Socket binding authority stays behind node-core/capability gates.
  (func (export "rt_socket_bind_allowed") (param $node_core_feature i32) (param $capability_granted i32) (result i32)
    (i32.and (local.get $node_core_feature) (local.get $capability_granted)))

  (memory (export "memory") 1)