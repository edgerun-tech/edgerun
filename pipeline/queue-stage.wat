;; Queue Stage — slot 48
  ;; Typed queue with 8 configurable policies.
  ;;
  ;; Config (12 bytes):
  ;;   +0:  policy    i32  0=backpressure 1=drop_oldest 2=drop_newest
  ;;                         3=latest 4=coalesce 5=spill 6=fail 7=sample
  ;;   +4:  capacity  i32  max elements
  ;;   +8:  flags     i32  bit0=auto_flush
  ;;
  ;; State (32 bytes per instance):
  ;;   +0:  tick       i32  (RO, written by pipeline_run)
  ;;   +4:  pipe_ptr   i32  internal element pipe (msg format: [len:u32][payload])
  ;;   +8:  count      i32  current element count
  ;;   +12: rd_elem    i32  logical read index (0-based)
  ;;   +16: wr_elem    i32  logical write index
  ;;   +20: spill_ptr  i32  spill pipe for overflow (0 = none)
  ;;   +24: sample_n   i32  sample interval (for policy 7)
  ;;   +28: sample_ct  i32  sample counter

  ;; Queue policy constants
  (global $QUEUE_BACKPRESSURE i32 (i32.const 0))
  (global $QUEUE_DROP_OLDEST  i32 (i32.const 1))
  (global $QUEUE_DROP_NEWEST  i32 (i32.const 2))
  (global $QUEUE_LATEST       i32 (i32.const 3))
  (global $QUEUE_COALESCE     i32 (i32.const 4))
  (global $QUEUE_SPILL        i32 (i32.const 5))
  (global $QUEUE_FAIL         i32 (i32.const 6))
  (global $QUEUE_SAMPLE       i32 (i32.const 7))

  ;; ── Helper: peek message length from pipe without consuming ──
  (func $queue_peek_len (param $pipe i32) (result i32)
  (local $data i32) (local $len i32) (local $q i32)
    (local $fill i32)
    (local.set $fill (call $pipe_fill (local.get $pipe)))
    (if (i32.lt_u (local.get $fill) (i32.const 4))
      (then (return (i32.const 0))))
    (i32.load (i32.add (call $pipe_read_ptr (local.get $pipe) (global.get $HDR)) (i32.const 0)))
  )

  ;; ── Helper: read leading message from pipe ──
  ;; Reads header + payload into scratch. Returns payload bytes or 0.
  (func $queue_read_msg (param $pipe i32) (param $scratch i32) (param $scap i32) (result i32)
    (call $msg_read (local.get $pipe) (local.get $scratch) (local.get $scap))
  )

  ;; ── Helper: write message to pipe ──
  (func $queue_write_msg (param $pipe i32) (param $data i32) (param $len i32) (result i32)
    (call $msg_write (local.get $pipe) (local.get $data) (local.get $len))
  )

  ;; ── Helper: drain one element from internal pipe to output ──
  ;; Returns STATUS_OK if an element was routed, 0 if no elements, negative on error.
  (func $queue_drain_one (param $q i32) (param $output i32) (param $scratch i32) (param $scap i32) (result i32)
    (local $pipe i32) (local $payload_len i32) (local $r i32)
    (local.set $pipe (i32.load offset=4 (local.get $q)))
    (if (i32.eqz (call $pipe_fill (local.get $pipe)))
      (then (return (i32.const 0))))
    (local.set $payload_len (call $msg_read (local.get $pipe) (local.get $scratch) (local.get $scap)))
    (if (i32.le_s (local.get $payload_len) (i32.const 0))
      (then (return (i32.sub (i32.const 0) (global.get $STATUS_OVERFLOW)))))
    (local.set $r (call $msg_write (local.get $output)
      (i32.add (local.get $scratch) (i32.const 4)) (local.get $payload_len)))
    (if (i32.ne (local.get $r) (global.get $STATUS_OK))
      (then (return (local.get $r))))
    (i32.store offset=8 (local.get $q)
      (i32.sub (i32.load offset=8 (local.get $q)) (i32.const 1)))
    (global.get $STATUS_OK)
  )

  ;; ── Helper: store one element into internal pipe ──
  ;; Reads from $input ($avail bytes) and writes to internal pipe as msg.
  (func $queue_store_one (param $input i32) (param $avail i32) (param $q i32) (param $scratch i32) (result i32)
    (local $pipe i32) (local $r i32)
    (local.set $pipe (i32.load offset=4 (local.get $q)))
    (local.set $r (call $pipe_read (local.get $input) (local.get $scratch) (local.get $avail)))
    (if (i32.lt_s (local.get $r) (i32.const 0))
      (then (return (local.get $r))))
    (if (i32.eqz (local.get $r))
      (then (return (global.get $STATUS_OK))))
    (local.set $r (call $msg_write (local.get $pipe) (local.get $scratch) (local.get $r)))
    (if (i32.ne (local.get $r) (global.get $STATUS_OK))
      (then (return (local.get $r))))
    (i32.store offset=8 (local.get $q)
      (i32.add (i32.load offset=8 (local.get $q)) (i32.const 1)))
    (global.get $STATUS_OK)
  )

  ;; ── Helper: drain as many elements as possible to output ──
  (func $queue_drain (param $q i32) (param $output i32) (param $scratch i32) (param $scap i32) (result i32)
    (local $r i32)
    (block $done
      (loop $lp
        (local.set $r (call $queue_drain_one (local.get $q) (local.get $output) (local.get $scratch) (local.get $scap)))
        (if (i32.le_s (local.get $r) (i32.const 0)) (then (br $done)))
        (br $lp)
      )
    )
    (if (i32.lt_s (local.get $r) (i32.const 0)) (then (return (local.get $r))))
    (global.get $STATUS_OK)
  )

  (func $process_queue (export "process_queue")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $policy i32) (local $capacity i32) (local $flags i32)
    (local $pipe i32) (local $count i32) (local $avail i32) (local $r i32)
    (local $sample_n i32) (local $sample_ct i32)

    (local.set $policy (i32.load offset=0 (local.get $cfg)))
    (local.set $capacity (i32.load offset=4 (local.get $cfg)))
    (local.set $flags (i32.load offset=8 (local.get $cfg)))
    (local.set $pipe (i32.load offset=4 (local.get $state)))
    (local.set $count (i32.load offset=8 (local.get $state)))

    ;; First call: init internal pipe
    (if (i32.eqz (local.get $pipe))
      (then
        (local.set $pipe (call $pipe_create
          (i32.mul (local.get $capacity) (i32.const 1024))))
        (if (i32.eq (local.get $pipe) (i32.const -1))
          (then (return (i32.sub (i32.const 0) (global.get $STATUS_OVERFLOW)))))
        (i32.store offset=4 (local.get $state) (local.get $pipe))
        (if (i32.eq (local.get $policy) (global.get $QUEUE_SAMPLE))
          (then
            (local.set $sample_n (i32.load offset=24 (local.get $state)))
            (if (i32.eqz (local.get $sample_n))
              (then
                (i32.store offset=24 (local.get $state) (i32.const 2)))))
        )
        (return (global.get $STATUS_MORE))))

    ;; Read available input
    (local.set $avail (call $pipe_available (local.get $input)))
    (if (i32.gt_u (local.get $avail) (i32.const 0))
      (then
        (block $policy_dispatch
          ;; backpressure
          (if (i32.eq (local.get $policy) (global.get $QUEUE_BACKPRESSURE))
            (then
              (if (i32.ge_u (local.get $count) (local.get $capacity))
                (then (return (global.get $STATUS_MORE))))
              (local.set $r (call $queue_store_one (local.get $input) (local.get $avail) (local.get $state) (local.get $scratch)))
              (br $policy_dispatch)))

          ;; drop_oldest
          (if (i32.eq (local.get $policy) (global.get $QUEUE_DROP_OLDEST))
            (then
              (if (i32.ge_u (local.get $count) (local.get $capacity))
                (then
                  (drop (call $queue_read_msg (local.get $pipe) (local.get $scratch) (local.get $scap)))
                  (i32.store offset=8 (local.get $state)
                    (i32.sub (i32.load offset=8 (local.get $state)) (i32.const 1)))))
              (local.set $r (call $queue_store_one (local.get $input) (local.get $avail) (local.get $state) (local.get $scratch)))
              (br $policy_dispatch)))

          ;; drop_newest
          (if (i32.eq (local.get $policy) (global.get $QUEUE_DROP_NEWEST))
            (then
              (if (i32.lt_u (local.get $count) (local.get $capacity))
                (then
                  (local.set $r (call $queue_store_one (local.get $input) (local.get $avail) (local.get $state) (local.get $scratch)))))
              (br $policy_dispatch)))

          ;; latest
          (if (i32.eq (local.get $policy) (global.get $QUEUE_LATEST))
            (then
              (if (i32.gt_u (local.get $count) (i32.const 0))
                (then
                  (drop (call $queue_read_msg (local.get $pipe) (local.get $scratch) (local.get $scap)))
                  (i32.store offset=8 (local.get $state) (i32.const 0))))
              (local.set $r (call $queue_store_one (local.get $input) (local.get $avail) (local.get $state) (local.get $scratch)))
              (br $policy_dispatch)))

          ;; fail
          (if (i32.eq (local.get $policy) (global.get $QUEUE_FAIL))
            (then
              (if (i32.ge_u (local.get $count) (local.get $capacity))
                (then (return (i32.sub (i32.const 0) (global.get $STATUS_OVERFLOW)))))
              (local.set $r (call $queue_store_one (local.get $input) (local.get $avail) (local.get $state) (local.get $scratch)))
              (br $policy_dispatch)))

          ;; sample
          (if (i32.eq (local.get $policy) (global.get $QUEUE_SAMPLE))
            (then
              (local.set $sample_ct (i32.load offset=28 (local.get $state)))
              (local.set $sample_n (i32.load offset=24 (local.get $state)))
              (if (i32.eqz (local.get $sample_n))
                (then (local.set $sample_n (i32.const 2))))
              (local.set $sample_ct (i32.add (local.get $sample_ct) (i32.const 1)))
              (local.set $sample_ct (i32.rem_u (local.get $sample_ct) (local.get $sample_n)))
              (i32.store offset=28 (local.get $state) (local.get $sample_ct))
              (if (i32.eqz (local.get $sample_ct))
                (then
                  (local.set $r (call $queue_store_one (local.get $input) (local.get $avail) (local.get $state) (local.get $scratch)))))
              (br $policy_dispatch)))

          ;; coalesce — pass through for now (needs merge function)
          (if (i32.eq (local.get $policy) (global.get $QUEUE_COALESCE))
            (then
              (drop (call $pipe_read (local.get $input) (local.get $scratch) (local.get $avail)))
              (drop (call $pipe_write (local.get $output) (local.get $scratch) (local.get $avail)))
              (return (global.get $STATUS_OK))))

          ;; spill
          (if (i32.eq (local.get $policy) (global.get $QUEUE_SPILL))
            (then
              (if (i32.ge_u (local.get $count) (local.get $capacity))
                (then
                  (local.set $r (i32.load offset=20 (local.get $state)))
                  (if (i32.eqz (local.get $r))
                    (then
                      (local.set $r (call $pipe_create (i32.mul (local.get $capacity) (i32.const 1024))))
                      (i32.store offset=20 (local.get $state) (local.get $r))))
                  (if (i32.gt_u (local.get $avail) (local.get $scap))
                    (then (local.set $avail (local.get $scap))))
                  (drop (call $pipe_read (local.get $input) (local.get $scratch) (local.get $avail)))
                  (drop (call $msg_write (local.get $r) (local.get $scratch) (local.get $avail))))
                (else
                  (local.set $r (call $queue_store_one (local.get $input) (local.get $avail) (local.get $state) (local.get $scratch)))))
              (br $policy_dispatch)))
        )))

    ;; Drain to output
    (local.set $r (call $queue_drain (local.get $state) (local.get $output) (local.get $scratch) (local.get $scap)))
    (if (i32.lt_s (local.get $r) (i32.const 0))
      (then (return (local.get $r))))

    ;; Return MORE if data remains upstream or internally
    (if (i32.or
          (call $pipe_available (local.get $input))
          (i32.load offset=8 (local.get $state)))
      (then (return (global.get $STATUS_MORE))))
    (global.get $STATUS_OK))
