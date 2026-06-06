  ;; Pipe Core — byte pipes + bump allocators

    ;; Standard ID removed — merged into single module

  ;; ── Pipe struct layout (16 byte header + data[cap]) ──
  ;; +0:  rd    read cursor from data start
  ;; +4:  wr    write cursor from data start
  ;; +8:  cap   total data capacity
  ;; +12: state 0=open 1=closed
  ;; +16: data[cap]
  (global $PIPE_RD     i32 (i32.const 0))
  (global $PIPE_WR     i32 (i32.const 4))
  (global $PIPE_CAP    i32 (i32.const 8))
  (global $PIPE_STATE  i32 (i32.const 12))
  (global $PIPE_HEADER i32 (i32.const 16))

  ;; ── Bump allocator: 0x40000 – 0x80000 ──
  (global $HEAP_START i32 (i32.const 0x40000))
  (global $HEAP_END   i32 (i32.const 0x80000))
  (global $heap_ptr (mut i32) (i32.const 0x40000))

  (func $pipe_alloc (export "pipe_alloc") (param $size i32) (result i32)
    (local $ptr i32)
    (local.set $ptr (global.get $heap_ptr))
    (if (i32.gt_u (i32.add (local.get $ptr) (local.get $size)) (global.get $HEAP_END))
      (then (return (i32.const -1))))
    (global.set $heap_ptr (i32.add (global.get $heap_ptr) (local.get $size)))
    local.get $ptr)

  ;; ── Pipe creation ──

  (func $pipe_create (export "pipe_create") (param $cap i32) (result i32)
    (local $p i32)
    (if (i32.eqz (local.get $cap)) (then (return (i32.const -1))))
    (local.set $p (call $pipe_alloc
      (i32.add (global.get $PIPE_HEADER) (local.get $cap))))
    (if (i32.eq (local.get $p) (i32.const -1)) (then (return (i32.const -1))))
    (i32.store offset=0 (local.get $p) (i32.const 0))
    (i32.store offset=4 (local.get $p) (i32.const 0))
    (i32.store offset=8 (local.get $p) (local.get $cap))
    (i32.store offset=12 (local.get $p) (i32.const 0))
    local.get $p)

  ;; ── Write ──

  (func $pipe_write (export "pipe_write")
    (param $p i32) (param $src i32) (param $len i32) (result i32)
    (local $wr i32) (local $cap i32)
    (if (i32.eqz (local.get $len)) (then (return (global.get $STATUS_OK))))
    (local.set $wr (i32.load offset=4 (local.get $p)))
    (local.set $cap (i32.load offset=8 (local.get $p)))
    (if (i32.gt_u (i32.add (local.get $wr) (local.get $len)) (local.get $cap))
      (then (return (global.get $STATUS_OVERFLOW))))
    (call $memcpy_off
      (i32.add (local.get $p) (global.get $PIPE_HEADER))
      (local.get $wr)
      (local.get $src)
      (i32.const 0)
      (local.get $len))
    (i32.store offset=4 (local.get $p)
      (i32.add (i32.load offset=4 (local.get $p)) (local.get $len)))
    global.get $STATUS_OK)

  ;; ── Read ──

  (func $pipe_read (export "pipe_read")
    (param $p i32) (param $dst i32) (param $max i32) (result i32)
    (local $avail i32) (local $rd i32)
    (local.set $avail
      (i32.sub (i32.load offset=4 (local.get $p))
               (i32.load offset=0 (local.get $p))))
    (if (i32.eqz (local.get $avail)) (then (return (i32.const 0))))
    (if (i32.gt_u (local.get $avail) (local.get $max))
      (then (local.set $avail (local.get $max))))
    (local.set $rd (i32.load offset=0 (local.get $p)))
    (call $memcpy_off
      (local.get $dst) (i32.const 0)
      (i32.add (local.get $p) (global.get $PIPE_HEADER))
      (local.get $rd)
      (local.get $avail))
    (i32.store offset=0 (local.get $p)
      (i32.add (local.get $rd) (local.get $avail)))
    (if (i32.eq (i32.load offset=0 (local.get $p))
                (i32.load offset=4 (local.get $p)))
      (then
        (i32.store offset=0 (local.get $p) (i32.const 0))
        (i32.store offset=4 (local.get $p) (i32.const 0))))
    local.get $avail)

  ;; ── Query ──

  (func $pipe_available (export "pipe_available") (param $p i32) (result i32)
    (i32.sub (i32.load offset=4 (local.get $p))
             (i32.load offset=0 (local.get $p))))

  (func (export "pipe_space") (param $p i32) (result i32)
    (i32.sub (i32.load offset=8 (local.get $p))
             (i32.load offset=4 (local.get $p))))

  (func (export "pipe_state") (param $p i32) (result i32)
    (i32.load offset=12 (local.get $p)))

  ;; ── Close / Reset ──

  (func $pipe_close (export "pipe_close") (param $p i32)
    (i32.store offset=12 (local.get $p) (i32.const 1)))

  ;; ── Heap lifecycle management ──
  ;; Snapshot saves current heap pointer; Restore rolls back to snapshot.
  ;; Intermediate allocations (pipes, nodes) above the snapshot are freed.
  ;; Persistent allocations below the snapshot are preserved.
  (func $pipe_snapshot (export "pipe_snapshot") (result i32)
    (global.get $heap_ptr))

  (func $pipe_restore (export "pipe_restore") (param $snap i32)
    (global.set $heap_ptr (local.get $snap)))

  (func (export "pipe_reset_heap")
    (global.set $heap_ptr (global.get $HEAP_START)))

  (func (export "pipe_reset") (param $p i32)
    (i32.store offset=0 (local.get $p) (i32.const 0))
    (i32.store offset=4 (local.get $p) (i32.const 0))
    (i32.store offset=12 (local.get $p) (i32.const 0)))

  ;; ── Pipe drain ──

  (func $pipe_drain (export "pipe_drain")
    (param $src i32) (param $dst i32) (param $tmp i32) (param $tcap i32) (result i32)
    (local $n i32)
    (local.set $n (call $pipe_read (local.get $src) (local.get $tmp) (local.get $tcap)))
    (if (i32.le_s (local.get $n) (i32.const 0))
      (then (return (local.get $n))))
    (call $pipe_write (local.get $dst) (local.get $tmp) (local.get $n))
    (return (local.get $n)))

  ;; memcpy imported from edgerun-core as $memcpy_off(dst, doff, src, soff, len)
