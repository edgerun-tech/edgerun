   ;; Pipe Core — byte pipes + bump allocators
   ;; $min_u is defined in runtime/math-utils.wat (include before this fragment)

    ;; Standard ID removed — merged into single module

   ;; ── Pipe struct layout — PIPE_* offset globals defined in runtime/memory-map.wat ──
   ;; Include memory-map.wat before this fragment.

   ;; ── Bump allocator (HEAP_START/HEAP_END from memory-map.wat) ──
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

  ;; pipe_create_aligned(cap, align) — capacity rounded up to align multiple
  (func $pipe_create_aligned (export "pipe_create_aligned") (param $cap i32) (param $align i32) (result i32)
    (local $aligned i32)
    (if (i32.eqz (local.get $cap)) (then (return (i32.const -1))))
    (if (i32.le_u (local.get $align) (i32.const 1))
      (then (return (call $pipe_create (local.get $cap)))))
    (local.set $aligned
      (i32.mul
        (i32.div_u
          (i32.add (local.get $cap) (i32.sub (local.get $align) (i32.const 1)))
          (local.get $align))
        (local.get $align)))
    (call $pipe_create (local.get $aligned)))

  (func (export "pipe_set_mode") (param $p i32) (param $mode i32)
    (i32.store offset=12 (local.get $p)
      (i32.or (i32.load offset=12 (local.get $p)) (local.get $mode))))

  ;; ── Helper: available bytes (handles circular wrap) ──
  (func $pipe_fill (param $p i32) (result i32)
    (local $rd i32) (local $wr i32) (local $cap i32)
    (local.set $rd (i32.load offset=0 (local.get $p)))
    (local.set $wr (i32.load offset=4 (local.get $p)))
    (if (result i32) (i32.and (i32.load offset=12 (local.get $p)) (global.get $PIPE_MODE_CIRCULAR))
      (then
        (local.set $cap (i32.load offset=8 (local.get $p)))
        (if (result i32) (i32.ge_u (local.get $wr) (local.get $rd))
          (then (i32.sub (local.get $wr) (local.get $rd)))
          (else (i32.sub (i32.add (local.get $cap) (local.get $wr)) (local.get $rd)))))
      (else (i32.sub (local.get $wr) (local.get $rd)))))

  ;; ── Helper: space available (cap - fill) ──
  (func $pipe_room (param $p i32) (result i32)
    (i32.sub (i32.load offset=8 (local.get $p)) (call $pipe_fill (local.get $p))))

  ;; ── Write ──

  (func $pipe_write (export "pipe_write")
    (param $p i32) (param $src i32) (param $len i32) (result i32)
    (local $wr i32) (local $cap i32)
    (if (i32.eqz (local.get $len)) (then (return (global.get $STATUS_OK))))
    (if (i32.and (i32.load offset=12 (local.get $p)) (global.get $PIPE_MODE_CIRCULAR))
      (then (return (call $pipe_write_circ (local.get $p) (local.get $src) (local.get $len)))))
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
      (i32.add (local.get $wr) (local.get $len)))
    (global.get $STATUS_OK))

  (func $pipe_write_circ (param $p i32) (param $src i32) (param $len i32) (result i32)
    (local $wr i32) (local $rd i32) (local $cap i32) (local $fill i32)
    (local $off i32) (local $remain i32) (local $base i32)
    (local.set $rd (i32.load offset=0 (local.get $p)))
    (local.set $wr (i32.load offset=4 (local.get $p)))
    (local.set $cap (i32.load offset=8 (local.get $p)))
    (if (i32.ge_u (local.get $wr) (local.get $rd))
      (then (local.set $fill (i32.sub (local.get $wr) (local.get $rd))))
      (else (local.set $fill (i32.sub (i32.add (local.get $cap) (local.get $wr)) (local.get $rd)))))
    (if (i32.gt_u (i32.add (local.get $fill) (local.get $len)) (local.get $cap))
      (then (return (global.get $STATUS_OVERFLOW))))
    (local.set $off (local.get $wr))
    (local.set $remain (i32.sub (local.get $cap) (local.get $off)))
    (local.set $base (i32.add (local.get $p) (global.get $PIPE_HEADER)))
    (if (i32.le_u (local.get $len) (local.get $remain))
      (then
        (call $memcpy_off (local.get $base) (local.get $off) (local.get $src) (i32.const 0) (local.get $len))
        (i32.store offset=4 (local.get $p) (i32.add (local.get $off) (local.get $len))))
      (else
        (call $memcpy_off (local.get $base) (local.get $off) (local.get $src) (i32.const 0) (local.get $remain))
        (call $memcpy_off (local.get $base) (i32.const 0) (local.get $src) (local.get $remain) (i32.sub (local.get $len) (local.get $remain)))
        (i32.store offset=4 (local.get $p) (i32.sub (local.get $len) (local.get $remain)))))
    global.get $STATUS_OK)

  ;; ── Read ──

  (func $pipe_read (export "pipe_read")
    (param $p i32) (param $dst i32) (param $max i32) (result i32)
    (local $avail i32) (local $rd i32) (local $n i32)
    (local.set $avail (call $pipe_fill (local.get $p)))
    (if (i32.eqz (local.get $avail)) (then (return (i32.const 0))))
    (if (i32.gt_u (local.get $avail) (local.get $max))
      (then (local.set $avail (local.get $max))))
    (if (i32.and (i32.load offset=12 (local.get $p)) (global.get $PIPE_MODE_CIRCULAR))
      (then (return (call $pipe_read_circ (local.get $p) (local.get $dst) (local.get $avail)))))
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
  (func $pipe_read_circ (param $p i32) (param $dst i32) (param $max i32) (result i32)
    (local $rd i32) (local $cap i32) (local $base i32)
    (local $remain i32) (local $n i32)
    (local.set $rd (i32.load offset=0 (local.get $p)))
    (local.set $cap (i32.load offset=8 (local.get $p)))
    (local.set $base (i32.add (local.get $p) (global.get $PIPE_HEADER)))
    (local.set $remain (i32.sub (local.get $cap) (local.get $rd)))
    (if (i32.le_u (local.get $max) (local.get $remain))
      (then
        (call $memcpy_off (local.get $dst) (i32.const 0) (local.get $base) (local.get $rd) (local.get $max))
        (i32.store offset=0 (local.get $p) (i32.add (local.get $rd) (local.get $max))))
      (else
        (call $memcpy_off (local.get $dst) (i32.const 0) (local.get $base) (local.get $rd) (local.get $remain))
        (local.set $n (i32.sub (local.get $max) (local.get $remain)))
        (call $memcpy_off (local.get $dst) (local.get $remain) (local.get $base) (i32.const 0) (local.get $n))
        (i32.store offset=0 (local.get $p) (local.get $n))))
    local.get $max)

  ;; ── Zero-copy read ──
  ;; Returns pointer to readable data (first contiguous segment).
  ;; For linear pipes: direct pointer to pipe data buffer + rd offset.
  ;; For circular pipes: only returns the segment from rd to end-of-buffer;
  ;;   caller must handle wrap-around or use pipe_read for full copy.
  ;; Writes available byte count to [len_ptr].
  (func $pipe_read_ptr (export "pipe_read_ptr") (param $p i32) (param $len_ptr i32) (result i32)
    (local $avail i32)
    (local.set $avail (call $pipe_fill (local.get $p)))
    (if (i32.eqz (local.get $avail))
      (then
        (i32.store (local.get $len_ptr) (i32.const 0))
        (return (i32.const 0))))
    (if (i32.and (i32.load offset=12 (local.get $p)) (global.get $PIPE_MODE_CIRCULAR))
      (then
        ;; Cap at first contiguous segment (rd → cap)
        (local.set $avail
          (call $min_u (local.get $avail)
            (i32.sub (i32.load offset=8 (local.get $p))
                     (i32.load offset=0 (local.get $p)))))))
    (i32.store (local.get $len_ptr) (local.get $avail))
    (i32.add (i32.add (local.get $p) (global.get $PIPE_HEADER))
             (i32.load offset=0 (local.get $p))))

  ;; Advance read cursor by n bytes after zero-copy read.
  (func $pipe_advance (export "pipe_advance") (param $p i32) (param $n i32)
    (local $rd i32) (local $cap i32)
    (if (i32.eqz (local.get $n)) (then (return)))
    (local.set $rd (i32.load offset=0 (local.get $p)))
    (if (i32.and (i32.load offset=12 (local.get $p)) (global.get $PIPE_MODE_CIRCULAR))
      (then
        (local.set $cap (i32.load offset=8 (local.get $p)))
        (local.set $rd (i32.add (local.get $rd) (local.get $n)))
        (if (i32.ge_u (local.get $rd) (local.get $cap))
          (then (local.set $rd (i32.sub (local.get $rd) (local.get $cap)))))
        (i32.store offset=0 (local.get $p) (local.get $rd)))
      (else
        (i32.store offset=0 (local.get $p)
          (i32.add (local.get $rd) (local.get $n)))
        (if (i32.eq (i32.load offset=0 (local.get $p))
                    (i32.load offset=4 (local.get $p)))
          (then
            (i32.store offset=0 (local.get $p) (i32.const 0))
            (i32.store offset=4 (local.get $p) (i32.const 0)))))))

  ;; ── Query ──

  (func $pipe_available (export "pipe_available") (param $p i32) (result i32)
    (call $pipe_fill (local.get $p)))

  (func (export "pipe_space") (param $p i32) (result i32)
    (call $pipe_room (local.get $p)))

  (func (export "pipe_state") (param $p i32) (result i32)
    (i32.load offset=12 (local.get $p)))

  ;; ── Close / Reset ──

  (func $pipe_close (export "pipe_close") (param $p i32)
    (i32.store offset=12 (local.get $p)
      (i32.or (global.get $PIPE_CLOSED)
              (i32.and (i32.load offset=12 (local.get $p)) (global.get $PIPE_MODE_CIRCULAR)))))

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
