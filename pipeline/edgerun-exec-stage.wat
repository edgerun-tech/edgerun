;; Edgerun Exec Stage — slot 47
  ;; Stage type: batch (state=0 initially)
  ;; Input:  data to process via input pipe
  ;; Output: processed data via output pipe
  ;; Config: node info (name_off, name_len, kind, placement) — 16 bytes
  ;;
  ;; Dispatches to node-specific handlers based on config kind.
  ;; Kinds handled by other stages (FLOW, QUEUE, BUFFER, MUX, DEMUX, CDC, SPLIT)
  ;; return -1 to signal they should be routed to their dedicated stage.

  (func $process_edgerun_exec (export "process_edgerun_exec")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $len_slot i32) (local $read i32) (local $kind i32)

    (local.set $len_slot (i32.sub (i32.add (local.get $scratch) (local.get $scap)) (i32.const 4)))
    (drop (call $pipe_read_ptr (local.get $input) (local.get $len_slot)))
    (local.set $read (i32.load (local.get $len_slot)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))

    ;; Read config kind from offset 8 (name_off:4, name_len:4, kind:4, placement:4)
    (if (i32.and (local.get $cfg) (i32.ge_u (local.get $clen) (i32.const 16)))
      (then
        (local.set $kind (i32.load offset=8 (local.get $cfg)))
      )
    )

    ;; ── Dispatch by node kind ──
    ;; CLOCK is a control signal — don't drain data
    (if (i32.eq (local.get $kind) (global.get $ER_NODE_KIND_CLOCK))
      (then (return (i32.const 0)))
    )

    ;; Pass-through kinds drain input → output: NODE, ACTOR, JOIN, KERNEL, GATE,
    ;; BATCH, REDUCE, MAP, ARBITER, FEEDBACK.
    ;; These form two ranges: 1-2 (NODE, ACTOR) and 8-16 (JOIN..FEEDBACK).
    (block $pass_through
      (if (i32.and (i32.ge_u (local.get $kind) (global.get $ER_NODE_KIND_NODE))
                   (i32.le_u (local.get $kind) (global.get $ER_NODE_KIND_ACTOR)))
        (then (br $pass_through))
      )
      (if (i32.and (i32.ge_u (local.get $kind) (global.get $ER_NODE_KIND_JOIN))
                   (i32.le_u (local.get $kind) (global.get $ER_NODE_KIND_FEEDBACK)))
        (then (br $pass_through))
      )
      ;; Not a pass-through kind — route to another stage
      (return (i32.const -1))
    )
    (return (call $pipe_drain (local.get $input) (local.get $output) (local.get $scratch) (local.get $scap)))
  )
