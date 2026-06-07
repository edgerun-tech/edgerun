;; Buffer Stage — slot 49
  ;; Addressable state storage with pin/lifetime management.
  ;; A buffer holds typed, addressable data that can be read from or
  ;; written to by multiple ports. Unlike queues, buffers are not
  ;; consumed by reading — they persist until explicitly invalidated.
  ;;
  ;; Config (16 bytes):
  ;;   +0:  elem_size i32  element size in bytes (0 = variable)
  ;;   +4:  capacity  i32  max elements
  ;;   +8:  flags     i32  bit0=pin  bit1=readonly
  ;;   +12: lifetime  i32  0=session 1=flow 2=persistent
  ;;
  ;; State (24 bytes per instance):
  ;;   +0:  tick       i32  (RO, written by pipeline_run)
  ;;   +4:  data_ptr   i32  pointer to pinned data buffer
  ;;   +8:  count      i32  current element count
  ;;   +12: capacity   i32  stored capacity
  ;;   +16: elem_size  i32  stored elem_size
  ;;   +20: flags      i32  stored flags

  (global $BUFFER_FLAG_PIN      i32 (i32.const 1))
  (global $BUFFER_FLAG_READONLY i32 (i32.const 2))
  (global $BUFFER_LIFETIME_SESSION    i32 (i32.const 0))
  (global $BUFFER_LIFETIME_FLOW       i32 (i32.const 1))
  (global $BUFFER_LIFETIME_PERSISTENT i32 (i32.const 2))

  (func $process_buffer (export "process_buffer")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $elem_size i32) (local $capacity i32) (local $flags i32) (local $lifetime i32)
    (local $data_ptr i32) (local $count i32) (local $avail i32) (local $r i32)
    (local $total_bytes i32) (local $write_offset i32)

    (local.set $elem_size (i32.load offset=0 (local.get $cfg)))
    (local.set $capacity (i32.load offset=4 (local.get $cfg)))
    (local.set $flags (i32.load offset=8 (local.get $cfg)))
    (local.set $lifetime (i32.load offset=12 (local.get $cfg)))
    (local.set $data_ptr (i32.load offset=4 (local.get $state)))
    (local.set $count (i32.load offset=8 (local.get $state)))

    ;; First call: allocate buffer memory if pinned or on first use
    (if (i32.eqz (local.get $data_ptr))
      (then
        (if (i32.eqz (local.get $elem_size))
          (then (local.set $elem_size (i32.const 64))))
        (if (i32.eqz (local.get $capacity))
          (then (local.set $capacity (i32.const 16))))
        (local.set $total_bytes (i32.mul (local.get $elem_size) (local.get $capacity)))
        (local.set $data_ptr (call $pipe_alloc (local.get $total_bytes)))
        (if (i32.eq (local.get $data_ptr) (i32.const -1))
          (then (return (i32.sub (i32.const 0) (global.get $STATUS_OVERFLOW)))))
        (i32.store offset=4 (local.get $state) (local.get $data_ptr))
        (i32.store offset=12 (local.get $state) (local.get $capacity))
        (i32.store offset=16 (local.get $state) (local.get $elem_size))
        (i32.store offset=20 (local.get $state) (local.get $flags))
        (return (global.get $STATUS_MORE))))

    ;; Read input data into buffer
    (local.set $avail (call $pipe_available (local.get $input)))
    (if (i32.gt_u (local.get $avail) (i32.const 0))
      (then
        (if (i32.and (local.get $flags) (global.get $BUFFER_FLAG_READONLY))
          (then (return (i32.sub (i32.const 0) (global.get $STATUS_OVERFLOW)))))
        (if (i32.gt_u (local.get $avail) (local.get $scap))
          (then (local.set $avail (local.get $scap))))
        (local.set $write_offset
          (i32.mul (i32.load offset=8 (local.get $state)) (i32.load offset=16 (local.get $state))))
        (if (i32.gt_u (i32.add (local.get $write_offset) (local.get $avail))
                      (i32.mul (i32.load offset=12 (local.get $state)) (i32.load offset=16 (local.get $state))))
          (then (local.set $write_offset (i32.const 0))))
        (local.set $r (call $pipe_read (local.get $input)
          (i32.add (local.get $data_ptr) (local.get $write_offset)) (local.get $avail)))
        (if (i32.gt_s (local.get $r) (i32.const 0))
          (then
            (i32.store offset=8 (local.get $state)
              (i32.add (i32.load offset=8 (local.get $state)) (i32.const 1)))
            (if (i32.ge_u (i32.load offset=8 (local.get $state)) (i32.load offset=12 (local.get $state)))
              (then
                (i32.store offset=8 (local.get $state) (i32.const 0))))))))

    ;; Write buffer contents to output (addressable read)
    (if (i32.gt_u (i32.load offset=8 (local.get $state)) (i32.const 0))
      (then
        (local.set $r (call $pipe_write (local.get $output) (local.get $data_ptr)
          (i32.mul (i32.load offset=8 (local.get $state)) (i32.load offset=16 (local.get $state)))))
        (if (i32.ne (local.get $r) (global.get $STATUS_OK))
          (then (return (global.get $STATUS_MORE))))
        (if (i32.and (local.get $flags) (global.get $BUFFER_FLAG_PIN))
          (then
            (i32.store offset=8 (local.get $state) (i32.const 0))))))

    (if (i32.or (call $pipe_available (local.get $input)) (i32.load offset=8 (local.get $state)))
      (then (return (global.get $STATUS_MORE))))
    (global.get $STATUS_OK))
