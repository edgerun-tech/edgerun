;; Edgerun Compile Stage — slot 145
  ;; Stage type: batch (state=0)
  ;; Input:  .edgerun source text via input pipe
  ;; Output: IR graph offset (i32) serialized as 4 bytes via output pipe
  ;;
  ;; Compiles .edgerun source → IR graph via $er_parse,
  ;; resolves placements via $er_resolve,
  ;; lowers to pipeline descriptor via $er_lower.
  ;; The caller can then pipeline_run the descriptor.

  (func $process_edgerun_compile (export "process_edgerun_compile")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $len_slot i32) (local $g i32)
    (local $plan i32) (local $desc i32) (local $pcap i32) (local $read i32)

    (local.set $len_slot (i32.sub (i32.add (local.get $scratch) (local.get $scap)) (i32.const 4)))
    (drop (call $pipe_read_ptr (local.get $input) (local.get $len_slot)))
    (local.set $read (i32.load (local.get $len_slot)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))

    ;; Copy source to scratch buffer
    (drop (call $pipe_read (local.get $input) (local.get $scratch) (local.get $read)))

    ;; Parse .edgerun source → IR graph
    (local.set $g (call $er_parse (local.get $scratch) (local.get $read)))
    (if (i32.eq (local.get $g) (i32.const -1)) (then (return (i32.const -1))))

    ;; Resolve placements
    (local.set $plan (call $er_resolve (local.get $g)))
    (if (i32.eqz (local.get $plan)) (then (return (i32.const -1))))

    ;; Read pcap from config (or default to 4096)
    (local.set $pcap (i32.const 4096))
    (if (i32.and (local.get $cfg) (i32.ge_u (local.get $clen) (i32.const 4)))
      (then
        (local.set $pcap (i32.load (local.get $cfg)))
      )
    )

    ;; Lower IR graph → pipeline descriptor
    (local.set $desc (call $er_lower (local.get $g) (local.get $pcap)))
    (if (i32.eq (local.get $desc) (i32.const -1)) (then (return (i32.const -1))))

    ;; Write descriptor offset to output pipe
    (i32.store (local.get $scratch) (local.get $desc))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (i32.const 4)))
    (i32.const 4)
  )
