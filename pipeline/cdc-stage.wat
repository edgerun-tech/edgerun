;; CDC Stage — slot 50
  ;; Clock domain crossing — wraps a queue with clock metadata tracking.
  ;; Records source/target clock ticks for each element crossing.
  ;;
  ;; Config (16 bytes):
  ;;   +0:  src_clock i32  source clock ID
  ;;   +4:  dst_clock i32  target clock ID
  ;;   +8:  capacity  i32  max elements in crossing buffer
  ;;   +12: policy    i32  queue policy (backpressure/drop_oldest/etc)
  ;;
  ;; State (24 bytes per instance):
  ;;   +0:  tick       i32  (RO, written by pipeline_run)
  ;;   +4:  pipe_ptr   i32  internal element pipe
  ;;   +8:  count      i32  current element count
  ;;   +12: last_src   i32  last source clock tick
  ;;   +16: last_dst   i32  last target clock tick
  ;;   +20: crossed    i32  total elements crossed

  (func $process_cdc (export "process_cdc")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $capacity i32) (local $policy i32) (local $pipe i32)
    (local $count i32) (local $avail i32) (local $r i32) (local $payload_len i32)

    (local.set $capacity (i32.load offset=8 (local.get $cfg)))
    (local.set $policy (i32.load offset=12 (local.get $cfg)))
    (local.set $pipe (i32.load offset=4 (local.get $state)))
    (local.set $count (i32.load offset=8 (local.get $state)))

    ;; First call: init internal pipe
    (if (i32.eqz (local.get $pipe))
      (then
        (if (i32.eqz (local.get $capacity))
          (then (local.set $capacity (i32.const 16))))
        (local.set $pipe (call $pipe_create
          (i32.mul (local.get $capacity) (i32.const 1024))))
        (if (i32.eq (local.get $pipe) (i32.const -1))
          (then (return (i32.sub (i32.const 0) (global.get $STATUS_OVERFLOW)))))
        (i32.store offset=4 (local.get $state) (local.get $pipe))
        (return (global.get $STATUS_MORE))))

    ;; Read input into internal buffer
    (local.set $avail (call $pipe_available (local.get $input)))
    (if (i32.gt_u (local.get $avail) (i32.const 0))
      (then
        (if (i32.ge_u (local.get $count) (local.get $capacity))
          (then
            (if (i32.eq (local.get $policy) (global.get $QUEUE_BACKPRESSURE))
              (then (return (global.get $STATUS_MORE))))
            (if (i32.eq (local.get $policy) (global.get $QUEUE_DROP_OLDEST))
              (then
                (drop (call $msg_read (local.get $pipe) (local.get $scratch) (local.get $scap)))
                (i32.store offset=8 (local.get $state)
                  (i32.sub (local.get $count) (i32.const 1)))
                (local.set $count (i32.sub (local.get $count) (i32.const 1)))))))
        (if (i32.gt_u (local.get $avail) (local.get $scap))
          (then (local.set $avail (local.get $scap))))
        (local.set $r (call $pipe_read (local.get $input) (local.get $scratch) (local.get $avail)))
        (if (i32.gt_s (local.get $r) (i32.const 0))
          (then
            (local.set $r (call $msg_write (local.get $pipe) (local.get $scratch) (local.get $r)))
            (if (i32.eq (local.get $r) (global.get $STATUS_OK))
              (then
                (i32.store offset=8 (local.get $state)
                  (i32.add (i32.load offset=8 (local.get $state)) (i32.const 1)))
                (i32.store offset=12 (local.get $state) (i32.load offset=0 (local.get $state)))))))))

    ;; Drain to output
    (local.set $count (i32.load offset=8 (local.get $state)))
    (block $drain_done
      (loop $drain_lp
        (br_if $drain_done (i32.eqz (local.get $count)))
        (local.set $payload_len (call $msg_read (local.get $pipe) (local.get $scratch) (local.get $scap)))
        (if (i32.le_s (local.get $payload_len) (i32.const 0))
          (then (br $drain_done)))
        (local.set $r (call $msg_write (local.get $output)
          (i32.add (local.get $scratch) (i32.const 4)) (local.get $payload_len)))
        (if (i32.ne (local.get $r) (global.get $STATUS_OK))
          (then
            ;; Push back — can't write to output yet
            (drop (call $queue_write_msg (local.get $pipe)
              (i32.add (local.get $scratch) (i32.const 4)) (local.get $payload_len)))
            (br $drain_done)))
        (i32.store offset=8 (local.get $state)
          (i32.sub (i32.load offset=8 (local.get $state)) (i32.const 1)))
        (i32.store offset=16 (local.get $state) (i32.load offset=0 (local.get $state)))
        (i32.store offset=20 (local.get $state)
          (i32.add (i32.load offset=20 (local.get $state)) (i32.const 1)))
        (local.set $count (i32.load offset=8 (local.get $state)))
        (br $drain_lp)
      )
    )

    (if (i32.or (call $pipe_available (local.get $input)) (i32.load offset=8 (local.get $state)))
      (then (return (global.get $STATUS_MORE))))
    (global.get $STATUS_OK))
