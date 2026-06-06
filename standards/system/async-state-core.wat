;; Poll code: 0 pending, 1 ready value/ok/some, 2 ready none/end, 3 ready err.
  ;; Return code: 0 pending, 1 left/value ready, 2 right ready, 3 end, 4 left err, 5 right err.
  (func (export "futures_select_poll") (param $left i32) (param $right i32) (result i32)
    (if (i32.eq (local.get $left) (i32.const 1)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $right) (i32.const 1)) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "futures_try_select_poll") (param $left i32) (param $right i32) (result i32)
    (if (i32.eq (local.get $left) (i32.const 3)) (then (return (i32.const 4))))
    (if (i32.eq (local.get $left) (i32.const 1)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $right) (i32.const 3)) (then (return (i32.const 5))))
    (if (i32.eq (local.get $right) (i32.const 1)) (then (return (i32.const 2))))
    i32.const 0)

  ;; MaybeDone state: 0 future, 1 done output retained, 2 gone output taken.
  ;; Encodes new_state * 10 + poll_result. poll_result: 0 pending, 1 ready unit, 9 panic.
  (func (export "futures_maybe_done_poll") (param $state i32) (param $poll i32) (result i32)
    (if (i32.eq (local.get $state) (i32.const 2)) (then (return (i32.const 29))))
    (if (i32.eq (local.get $state) (i32.const 1)) (then (return (i32.const 11))))
    (if (i32.eq (local.get $poll) (i32.const 1)) (then (return (i32.const 11))))
    i32.const 0)

  (func (export "futures_maybe_done_take_output") (param $state i32) (result i32)
    (if (i32.eq (local.get $state) (i32.const 1)) (then (return (i32.const 21))))
    (i32.mul (local.get $state) (i32.const 10)))

  ;; Fuse keeps completed futures pending forever, and completed streams yielding None forever.
  (func (export "futures_future_fuse_poll") (param $active i32) (param $poll i32) (result i32)
    (if (i32.eqz (local.get $active)) (then (return (i32.const 0))))
    (if (i32.eq (local.get $poll) (i32.const 1)) (then (return (i32.const 11))))
    i32.const 10)

  (func (export "futures_stream_fuse_poll") (param $done i32) (param $poll i32) (result i32)
    (if (local.get $done) (then (return (i32.const 12))))
    (if (i32.eq (local.get $poll) (i32.const 2)) (then (return (i32.const 12))))
    (if (i32.eq (local.get $poll) (i32.const 1)) (then (return (i32.const 1))))
    i32.const 0)

  (func $full_mask (param $arity i32) (result i32)
    (i32.sub (i32.shl (i32.const 1) (local.get $arity)) (i32.const 1)))

  (func $futures_join_poll (export "futures_join_poll") (param $done_mask i32) (param $ready_mask i32) (param $arity i32) (result i32)
    (if (i32.eq (i32.or (local.get $done_mask) (local.get $ready_mask)) (call $full_mask (local.get $arity)))
      (then (return (i32.const 1))))
    i32.const 0)

  ;; err_index is 1-based. Any error wins immediately; otherwise all oks must be ready.
  (func (export "futures_try_join_poll") (param $done_mask i32) (param $ready_mask i32) (param $arity i32) (param $err_index i32) (result i32)
    (if (i32.gt_u (local.get $err_index) (i32.const 0)) (then (return (i32.add (i32.const 10) (local.get $err_index)))))
    (call $futures_join_poll (local.get $done_mask) (local.get $ready_mask) (local.get $arity)))

  ;; SelectWithStrategy state: 0 start, 1 left finished, 2 right finished, 3 both finished.
  ;; side: 0 left first, 1 right first. left/right poll use Poll code above.
  ;; Encodes new_state * 10 + result code.
  (func $finish_state (param $state i32) (param $side i32) (result i32)
    (if (i32.eq (local.get $state) (i32.const 0))
      (then
        (if (i32.eqz (local.get $side)) (then (return (i32.const 1))))
        (return (i32.const 2))))
    (if (i32.or
        (i32.and (i32.eq (local.get $state) (i32.const 1)) (i32.eq (local.get $side) (i32.const 1)))
        (i32.and (i32.eq (local.get $state) (i32.const 2)) (i32.eqz (local.get $side))))
      (then (return (i32.const 3))))
    local.get $state)

  (func (export "futures_stream_select_poll") (param $state i32) (param $side i32) (param $left i32) (param $right i32) (result i32)
    (local $first i32)
    (local $other i32)
    (local $new_state i32)
    (if (i32.eq (local.get $state) (i32.const 3)) (then (return (i32.const 33))))
    (if (i32.eq (local.get $state) (i32.const 1))
      (then
        (if (i32.eq (local.get $right) (i32.const 2)) (then (return (i32.const 33))))
        (if (i32.eq (local.get $right) (i32.const 1)) (then (return (i32.const 21))))
        (return (i32.const 10))))
    (if (i32.eq (local.get $state) (i32.const 2))
      (then
        (if (i32.eq (local.get $left) (i32.const 2)) (then (return (i32.const 33))))
        (if (i32.eq (local.get $left) (i32.const 1)) (then (return (i32.const 11))))
        (return (i32.const 20))))
    (local.set $first (select (local.get $right) (local.get $left) (local.get $side)))
    (local.set $other (select (local.get $left) (local.get $right) (local.get $side)))
    (if (i32.eq (local.get $first) (i32.const 1))
      (then (return (select (i32.const 2) (i32.const 1) (local.get $side)))))
    (local.set $new_state (local.get $state))
    (if (i32.eq (local.get $first) (i32.const 2))
      (then (local.set $new_state (call $finish_state (local.get $new_state) (local.get $side)))))
    (if (i32.eq (local.get $other) (i32.const 1))
      (then
        (return (i32.add
          (i32.mul (local.get $new_state) (i32.const 10))
          (select (i32.const 1) (i32.const 2) (local.get $side))))))
    (if (i32.eq (local.get $other) (i32.const 2))
      (then
        (local.set $new_state (call $finish_state (local.get $new_state) (i32.xor (local.get $side) (i32.const 1))))
        (if (i32.eq (local.get $first) (i32.const 2))
          (then (return (i32.add (i32.mul (local.get $new_state) (i32.const 10)) (i32.const 3)))))
        (return (i32.mul (local.get $new_state) (i32.const 10)))))
    (i32.mul (local.get $new_state) (i32.const 10)))

  ;; ReadyChunks: count buffered ready items until capacity, pending flushes non-empty chunk.
  ;; Returns 0 pending, 1 ready chunk, 2 end empty, 3 continue collecting.
  (func (export "futures_ready_chunks_poll") (param $items i32) (param $capacity i32) (param $poll i32) (result i32)
    (if (i32.eq (local.get $poll) (i32.const 0))
      (then
        (if (i32.eqz (local.get $items)) (then (return (i32.const 0))))
        (return (i32.const 1))))
    (if (i32.eq (local.get $poll) (i32.const 2))
      (then
        (if (i32.eqz (local.get $items)) (then (return (i32.const 2))))
        (return (i32.const 1))))
    (if (i32.ge_u (i32.add (local.get $items) (i32.const 1)) (local.get $capacity)) (then (return (i32.const 1))))
    i32.const 3)

  ;; Sink buffer: capacity 0 delegates; otherwise queued items must drain before readiness.
  (func (export "futures_sink_buffer_ready") (param $capacity i32) (param $queued i32) (param $inner_ready i32) (result i32)
    (if (i32.eqz (local.get $capacity)) (then (return (local.get $inner_ready))))
    (if (i32.and (i32.gt_u (local.get $queued) (i32.const 0)) (i32.eqz (local.get $inner_ready))) (then (return (i32.const 0))))
    (if (i32.ge_u (local.get $queued) (local.get $capacity)) (then (return (i32.const 0))))
    i32.const 1)

  (func (export "futures_sink_fanout_ready") (param $left_ready i32) (param $right_ready i32) (result i32)
    (i32.and (local.get $left_ready) (local.get $right_ready)))
