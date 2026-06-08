;; Frame Pacer Stage — tick-driven accumulation to frame_size chunks
  ;; Slots into any pipeline to provide frame-aligned data to downstream stages.
  ;;
  ;; State (80 bytes):
  ;;   +0:  tick          i32  — pipeline tick (RO, written by pipeline_run)
  ;;   +4:  frame_size    i32  — stride (from config[0])
  ;;   +8:  buf_len       i32  — bytes buffered so far
  ;;   +12: last_flush    i32  — tick when last frame was emitted
  ;;   +16: buf[64]             — internal buffer
  ;;
  ;; Config (4 bytes):
  ;;   +0: timeout_ticks i32  — flush partial frame after N idle ticks (0=never)

  (func $process_frame_pacer (export "process_frame_pacer")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $tick i32) (local $frame_size i32) (local $buf_len i32) (local $last_flush i32)
    (local $timeout i32) (local $avail i32) (local $space i32) (local $read i32)
    (local $buf i32)

    (local.set $tick (i32.load offset=0 (local.get $state)))
    (local.set $frame_size (i32.load offset=4 (local.get $state)))
    (local.set $buf_len (i32.load offset=8 (local.get $state)))
    (local.set $last_flush (i32.load offset=12 (local.get $state)))
    (local.set $buf (i32.add (local.get $state) (i32.const 16)))
    (local.set $timeout (i32.load offset=0 (local.get $cfg)))

    ;; First call: init frame_size from config, yield
    (if (i32.eqz (local.get $frame_size))
      (then
        (local.set $frame_size (i32.load offset=0 (local.get $cfg)))
        (i32.store offset=4 (local.get $state) (local.get $frame_size))
        (return (global.get $STATUS_MORE))))

    ;; Accumulate input data into buffer
    (local.set $avail (call $pipe_available (local.get $input)))
    (if (i32.gt_u (local.get $avail) (i32.const 0))
      (then
        (local.set $space (i32.sub (local.get $frame_size) (local.get $buf_len)))
        (if (i32.gt_u (local.get $space) (i32.const 0))
          (then
            (local.set $read
              (call $pipe_read (local.get $input)
                (i32.add (local.get $buf) (local.get $buf_len))
                (call $min_u (local.get $space) (local.get $avail))))
            (local.set $buf_len (i32.add (local.get $buf_len) (local.get $read)))
            (i32.store offset=8 (local.get $state) (local.get $buf_len))))))

    ;; Full frame: write, shift, yield if more input
    (if (i32.ge_u (local.get $buf_len) (local.get $frame_size))
      (then
        (drop (call $pipe_write (local.get $output) (local.get $buf) (local.get $frame_size)))
        (local.set $buf_len (i32.sub (local.get $buf_len) (local.get $frame_size)))
        (if (i32.gt_u (local.get $buf_len) (i32.const 0))
          (then
            (call $memcpy_off
              (local.get $buf) (i32.const 0)
              (local.get $buf) (local.get $frame_size)
              (local.get $buf_len))))
        (i32.store offset=8 (local.get $state) (local.get $buf_len))
        (i32.store offset=12 (local.get $state) (local.get $tick))
        (if (call $pipe_available (local.get $input))
          (then (return (global.get $STATUS_MORE))))
        (return (global.get $STATUS_OK))))

    ;; Partial frame: check timeout
    (if (i32.gt_u (local.get $buf_len) (i32.const 0))
      (then
        (if (i32.and (local.get $timeout)
              (i32.ge_u
                (i32.sub (local.get $tick) (local.get $last_flush))
                (local.get $timeout)))
          (then
            (drop (call $pipe_write (local.get $output) (local.get $buf) (local.get $buf_len)))
            (i32.store offset=8 (local.get $state) (i32.const 0))
            (i32.store offset=12 (local.get $state) (local.get $tick))
            (return (global.get $STATUS_OK))))
        (return (global.get $STATUS_MORE))))

    (global.get $STATUS_OK))
