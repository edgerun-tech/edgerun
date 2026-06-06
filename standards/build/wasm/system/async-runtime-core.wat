;; Async/runtime utility semantics captured from local futures-channel,
  ;; futures-core/task, futures-executor/task, tokio-shaped macros, once-cell,
  ;; parking, thread_local, and pin-project-lite compatibility crates.
  ;;
  ;; Poll/result states: pending=0 ready-ok=1 ready-none=2 full=3 disconnected=4
  ;;                    canceled=5 shutdown=6 entered=7 enter-error=8
  ;; AtomicWaker bits: waiting=0 registering=1 waking=2 registering|waking=3
  ;; Parker states: empty=0 parked=1 notified=2
  ;; Once states: empty=0 initializing=1 initialized=2 poisoned=3

  (func $m32bool (param $x i32) (result i32)
    local.get $x
    i32.const 0
    i32.ne)

  (func $m32min (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.lt_s
    if (result i32)
      local.get $a
    else
      local.get $b
    end)

  (func $m32max0 (param $n i32) (result i32)
    local.get $n
    i32.const 0
    i32.gt_s
    if (result i32)
      local.get $n
    else
      i32.const 0
    end)

  (export "mpsc_capacity" (func $mpsc_capacity))
  (func $mpsc_capacity
    (param $buffer i32)
    (param $sender_count i32)
    (result i32)
    ;; Bounded mpsc gives every sender one guaranteed slot plus shared buffer.
    local.get $buffer
    local.get $sender_count
    i32.add)

  (export "mpsc_send_decision" (func $mpsc_send_decision))
  (func $mpsc_send_decision
    (param $bounded i32)
    (param $open i32)
    (param $queued i32)
    (param $buffer i32)
    (param $sender_parked i32)
    (result i32)
    ;; ready-ok=accepted, full=backpressure, disconnected=closed.
    local.get $open
    call $m32bool
    i32.eqz
    if (result i32)
      i32.const 4
    else
      local.get $bounded
      call $m32bool
      local.get $sender_parked
      call $m32bool
      i32.and
      if (result i32)
        i32.const 3
      else
        i32.const 1
      end
    end)

  (export "mpsc_sender_parks_after_send" (func $mpsc_sender_parks_after_send))
  (func $mpsc_sender_parks_after_send
    (param $bounded i32)
    (param $new_queued i32)
    (param $buffer i32)
    (result i32)
    local.get $bounded
    call $m32bool
    local.get $new_queued
    local.get $buffer
    i32.gt_s
    i32.and)

  (export "mpsc_recv_decision" (func $mpsc_recv_decision))
  (func $mpsc_recv_decision
    (param $queued i32)
    (param $open i32)
    (param $receiver_alive i32)
    (result i32)
    local.get $receiver_alive
    call $m32bool
    i32.eqz
    if (result i32)
      i32.const 2
    else
      local.get $queued
      i32.const 0
      i32.gt_s
      if (result i32)
        i32.const 1
      else
        local.get $open
        call $m32bool
        if (result i32)
          i32.const 0
        else
          i32.const 2
        end
      end
    end)

  (export "mpsc_try_recv_error" (func $mpsc_try_recv_error))
  (func $mpsc_try_recv_error
    (param $queued i32)
    (param $open i32)
    (result i32)
    local.get $queued
    i32.const 0
    i32.gt_s
    if (result i32)
      i32.const 1
    else
      local.get $open
      call $m32bool
      if (result i32)
        i32.const 0
      else
        i32.const 4
      end
    end)

  (export "mpsc_close_action" (func $mpsc_close_action))
  (func $mpsc_close_action
    (param $queued i32)
    (param $parked_senders i32)
    (result i32)
    ;; Low 16: drainable queued messages. High 16: senders to unpark.
    local.get $parked_senders
    i32.const 16
    i32.shl
    local.get $queued
    i32.const 0xffff
    i32.and
    i32.or)

  (export "mpsc_drop_sender_closes" (func $mpsc_drop_sender_closes))
  (func $mpsc_drop_sender_closes
    (param $previous_sender_count i32)
    (result i32)
    local.get $previous_sender_count
    i32.const 1
    i32.eq)

  (export "oneshot_send_decision" (func $oneshot_send_decision))
  (func $oneshot_send_decision
    (param $complete i32)
    (param $data_lock_acquired i32)
    (param $receiver_closed_after_lock i32)
    (param $receiver_took_data i32)
    (result i32)
    local.get $complete
    call $m32bool
    local.get $data_lock_acquired
    call $m32bool
    i32.eqz
    i32.or
    if (result i32)
      i32.const 4
    else
      local.get $receiver_closed_after_lock
      call $m32bool
      local.get $receiver_took_data
      call $m32bool
      i32.eqz
      i32.and
      if (result i32)
        i32.const 4
      else
        i32.const 1
      end
    end)

  (export "oneshot_recv_decision" (func $oneshot_recv_decision))
  (func $oneshot_recv_decision
    (param $complete i32)
    (param $data_present i32)
    (param $rx_task_lock_failed i32)
    (result i32)
    local.get $complete
    call $m32bool
    local.get $rx_task_lock_failed
    call $m32bool
    i32.or
    if (result i32)
      local.get $data_present
      call $m32bool
      if (result i32)
        i32.const 1
      else
        i32.const 5
      end
    else
      i32.const 0
    end)

  (export "oneshot_is_terminated" (func $oneshot_is_terminated))
  (func $oneshot_is_terminated
    (param $complete i32)
    (param $data_present i32)
    (result i32)
    local.get $complete
    call $m32bool
    local.get $data_present
    call $m32bool
    i32.eqz
    i32.and)

  (export "atomic_waker_register_result" (func $atomic_waker_register_result))
  (func $atomic_waker_register_result
    (param $state i32)
    (param $same_waker i32)
    (result i32)
    ;; 0 stored, 1 unchanged same task, 2 self-wake, 3 dropped racing register.
    local.get $state
    i32.const 0
    i32.eq
    if (result i32)
      local.get $same_waker
      call $m32bool
      if (result i32)
        i32.const 1
      else
        i32.const 0
      end
    else
      local.get $state
      i32.const 2
      i32.eq
      if (result i32)
        i32.const 2
      else
        i32.const 3
      end
    end)

  (export "atomic_waker_take_result" (func $atomic_waker_take_result))
  (func $atomic_waker_take_result
    (param $state i32)
    (param $has_waker i32)
    (result i32)
    ;; 1 means take+wake is possible; 0 means wake bit records contention/no-op.
    local.get $state
    i32.const 0
    i32.eq
    local.get $has_waker
    call $m32bool
    i32.and)

  (export "once_set_result" (func $once_set_result))
  (func $once_set_result
    (param $state i32)
    (result i32)
    local.get $state
    i32.const 0
    i32.eq
    if (result i32)
      i32.const 1
    else
      i32.const 4
    end)

  (export "once_get_result" (func $once_get_result))
  (func $once_get_result
    (param $state i32)
    (result i32)
    local.get $state
    i32.const 2
    i32.eq)

  (export "once_get_or_init_transition" (func $once_get_or_init_transition))
  (func $once_get_or_init_transition
    (param $state i32)
    (param $init_ok i32)
    (result i32)
    local.get $state
    i32.const 2
    i32.eq
    if (result i32)
      i32.const 2
    else
      local.get $state
      i32.const 3
      i32.eq
      if (result i32)
        i32.const 3
      else
        local.get $init_ok
        call $m32bool
        if (result i32)
          i32.const 2
        else
          i32.const 0
        end
      end
    end)

  (export "lazy_force_result" (func $lazy_force_result))
  (func $lazy_force_result
    (param $cell_state i32)
    (param $init_present i32)
    (result i32)
    local.get $cell_state
    i32.const 2
    i32.eq
    if (result i32)
      i32.const 1
    else
      local.get $init_present
      call $m32bool
      if (result i32)
        i32.const 1
      else
        i32.const 3
      end
    end)

  (export "executor_enter_result" (func $executor_enter_result))
  (func $executor_enter_result
    (param $already_entered i32)
    (result i32)
    local.get $already_entered
    call $m32bool
    if (result i32)
      i32.const 8
    else
      i32.const 7
    end)

  (export "local_pool_step_result" (func $local_pool_step_result))
  (func $local_pool_step_result
    (param $main_ready i32)
    (param $task_completed i32)
    (param $incoming_count i32)
    (param $woken i32)
    (result i32)
    ;; ready-ok means a target/task completed, pending means keep polling,
    ;; ready-none means no progress is available.
    local.get $main_ready
    local.get $task_completed
    i32.or
    call $m32bool
    if (result i32)
      i32.const 1
    else
      local.get $incoming_count
      i32.const 0
      i32.gt_s
      local.get $woken
      call $m32bool
      i32.or
      if (result i32)
        i32.const 0
      else
        i32.const 2
      end
    end)

  (export "spawn_result" (func $spawn_result))
  (func $spawn_result
    (param $executor_alive i32)
    (result i32)
    local.get $executor_alive
    call $m32bool
    if (result i32)
      i32.const 1
    else
      i32.const 6
    end)

  (export "parker_park_result" (func $parker_park_result))
  (func $parker_park_result
    (param $state i32)
    (param $timeout_zero i32)
    (param $notified_during_wait i32)
    (result i32)
    local.get $state
    i32.const 2
    i32.eq
    if (result i32)
      i32.const 1
    else
      local.get $timeout_zero
      call $m32bool
      if (result i32)
        i32.const 0
      else
        local.get $notified_during_wait
        call $m32bool
      end
    end)

  (export "parker_unpark_result" (func $parker_unpark_result))
  (func $parker_unpark_result
    (param $state i32)
    (result i32)
    ;; returns whether this call created a new notification.
    local.get $state
    i32.const 2
    i32.ne)

  (export "thread_local_bucket_index" (func $thread_local_bucket_index))
  (func $thread_local_bucket_index
    (param $thread_id i32)
    (result i32)
    ;; Bucket n stores 2^n entries. bucket=floor(log2(id+1)).
    (local $x i32)
    (local $bucket i32)
    local.get $thread_id
    i32.const 1
    i32.add
    local.set $x
    i32.const 0
    local.set $bucket
    loop $again
      local.get $x
      i32.const 1
      i32.gt_u
      if
        local.get $x
        i32.const 1
        i32.shr_u
        local.set $x
        local.get $bucket
        i32.const 1
        i32.add
        local.set $bucket
        br $again
      end
    end
    local.get $bucket)

  (export "thread_local_insert_result" (func $thread_local_insert_result))
  (func $thread_local_insert_result
    (param $present i32)
    (param $create_ok i32)
    (result i32)
    local.get $present
    call $m32bool
    if (result i32)
      i32.const 1
    else
      local.get $create_ok
      call $m32bool
      if (result i32)
        i32.const 1
      else
        i32.const 4
      end
    end)

  (export "tokio_join_poll_count" (func $tokio_join_poll_count))
  (func $tokio_join_poll_count
    (param $future_count i32)
    (result i32)
    local.get $future_count
    call $m32max0)

  (export "tokio_select_winner" (func $tokio_select_winner))
  (func $tokio_select_winner
    (param $first_ready i32)
    (param $second_ready i32)
    (param $third_ready i32)
    (param $first_enabled i32)
    (param $second_enabled i32)
    (param $third_enabled i32)
    (result i32)
    ;; local select is biased: first ready enabled branch wins. none=0.
    local.get $first_ready
    local.get $first_enabled
    i32.and
    call $m32bool
    if (result i32)
      i32.const 1
    else
      local.get $second_ready
      local.get $second_enabled
      i32.and
      call $m32bool
      if (result i32)
        i32.const 2
      else
        local.get $third_ready
        local.get $third_enabled
        i32.and
        call $m32bool
        if (result i32)
          i32.const 3
        else
          i32.const 0
        end
      end
    end)

  (export "pin_project_valid" (func $pin_project_valid))
  (func $pin_project_valid
    (param $repr_packed i32)
    (param $has_pinned_drop i32)
    (param $manual_drop_impl i32)
    (param $project_unpin_forced_off i32)
    (result i32)
    ;; Pin projection rejects packed layout and blocks ordinary Drop movement.
    local.get $repr_packed
    call $m32bool
    local.get $manual_drop_impl
    call $m32bool
    local.get $has_pinned_drop
    call $m32bool
    i32.eqz
    i32.and
    i32.or
    i32.eqz)

  (export "pin_project_field_kind" (func $pin_project_field_kind))
  (func $pin_project_field_kind
    (param $field_is_pinned i32)
    (param $by_ref i32)
    (param $replace i32)
    (result i32)
    ;; 1 Pin<&mut T>, 2 &mut T, 3 Pin<&T>, 4 &T, 5 replacement projection.
    local.get $replace
    call $m32bool
    if (result i32)
      i32.const 5
    else
      local.get $field_is_pinned
      call $m32bool
      if (result i32)
        local.get $by_ref
        call $m32bool
        if (result i32)
          i32.const 3
        else
          i32.const 1
        end
      else
        local.get $by_ref
        call $m32bool
        if (result i32)
          i32.const 4
        else
          i32.const 2
        end
      end
    end)