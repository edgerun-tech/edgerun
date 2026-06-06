  ;; Pipeline Core — pipeline_run + dispatch table

    ;; Standard ID removed — merged into single module

  ;; ── Stage dispatch table ──
  (table (export "stage_table") 64 funcref)

  ;; ── Stage type constants (dispatch table indices) ──
  (func (export "STAGE_PASSTHROUGH") (result i32) i32.const 0)
  (func (export "STAGE_HEX_ENCODE")  (result i32) i32.const 1)
  (func (export "STAGE_HEX_DECODE")  (result i32) i32.const 2)
  (func (export "STAGE_B64_ENCODE")  (result i32) i32.const 3)
  (func (export "STAGE_B64_DECODE")  (result i32) i32.const 4)
  (func (export "STAGE_TRANSPORT")   (result i32) i32.const 5)
  (func (export "STAGE_MUX_STATIC")  (result i32) i32.const 6)
  (func (export "STAGE_DEMUX_STATIC") (result i32) i32.const 7)
  (func (export "STAGE_MUX_DYNAMIC") (result i32) i32.const 8)
  (func (export "STAGE_DEMUX_DYNAMIC") (result i32) i32.const 9)
  (func (export "STAGE_WS_FRAME")     (result i32) i32.const 10)
  (func (export "STAGE_WS_ENCODE")    (result i32) i32.const 11)
  (func (export "STAGE_WS_DECODE")    (result i32) i32.const 12)
  (func (export "STAGE_EXEC")         (result i32) i32.const 13)
  (func (export "STAGE_FRAME_PACER")  (result i32) i32.const 15)
  (func (export "STAGE_SHA256")       (result i32) i32.const 16)
  (func (export "STAGE_HMAC_SHA256")  (result i32) i32.const 17)

  ;; ── Stage function type ──
  ;; (input_pipe, output_pipe, config_ptr, config_len, scratch, scap, state_ptr) -> result
  (type $stage_fn (func (param i32 i32 i32 i32 i32 i32 i32) (result i32)))

  ;; ── Pipeline descriptor layout ──
  ;; +0:  magic      i32
  ;; +4:  version    i32
  ;; +8:  pipe_cap   i32  — intermediate pipe capacity
  ;; +12: stage_count i32
  ;; +16: tick       i32  — incremented per pipeline_run call
  ;; +20: stages[] — each:
  ;;   +0:  stage_type i32
  ;;   +4:  config     i32
  ;;   +8:  config_len i32
  ;;   +12: state_ptr  i32
  ;;   total: 16 bytes
  (func (export "PIPELINE_MAGIC")   (result i32) i32.const 0x50495045)
  (func (export "PIPELINE_VERSION") (result i32) i32.const 1)

  (global $PD_MAGIC    i32 (i32.const 0))
  (global $PD_VERSION  i32 (i32.const 4))
  (global $PD_PIPE_CAP i32 (i32.const 8))
  (global $PD_COUNT    i32 (i32.const 12))
  (global $PD_TICK     i32 (i32.const 16))
  (global $PD_FRAME    i32 (i32.const 20))
  (global $PD_STAGES   i32 (i32.const 24))
  (global $PS_TYPE     i32 (i32.const 0))
  (global $PS_CONFIG   i32 (i32.const 4))
  (global $PS_CLEN     i32 (i32.const 8))
  (global $PS_STATE    i32 (i32.const 12))
  (global $PS_SIZE     i32 (i32.const 16))

  ;; pipeline_create(pipe_cap, stage_count) → desc_ptr | -1
  (func (export "pipeline_create") (param $pcap i32) (param $count i32) (result i32)
    (local $desc i32) (local $sz i32)
    (local.set $sz (i32.add (global.get $PD_STAGES)
      (i32.mul (local.get $count) (global.get $PS_SIZE))))
    (local.set $desc (call $pipe_alloc (local.get $sz)))
    (if (i32.eq (local.get $desc) (i32.const -1))
      (then (return (i32.const -1))))
    (i32.store offset=0 (local.get $desc) (i32.const 0x50495045))
    (i32.store offset=4 (local.get $desc) (i32.const 1))
    (i32.store offset=8 (local.get $desc) (local.get $pcap))
    (i32.store offset=12 (local.get $desc) (local.get $count))
    (i32.store offset=16 (local.get $desc) (i32.const 0))
    (i32.store offset=20 (local.get $desc) (i32.const 0))
    local.get $desc)

  ;; pipeline_set_stage(desc, index, stage_type, config, config_len)
  (func (export "pipeline_set_stage")
    (param $desc i32) (param $idx i32) (param $stype i32)
    (param $cfg i32) (param $clen i32)
    (local $slot i32)
    (local.set $slot
      (i32.add (global.get $PD_STAGES)
        (i32.mul (local.get $idx) (global.get $PS_SIZE))))
    (i32.store (i32.add (local.get $desc) (local.get $slot)) (local.get $stype))
    (i32.store offset=4 (i32.add (local.get $desc) (local.get $slot)) (local.get $cfg))
    (i32.store offset=8 (i32.add (local.get $desc) (local.get $slot)) (local.get $clen)))

  ;; pipeline_set_stage_state(desc, index, state_ptr)
  (func (export "pipeline_set_stage_state")
    (param $desc i32) (param $idx i32) (param $state i32)
    (local $slot i32)
    (local.set $slot
      (i32.add (global.get $PD_STAGES)
        (i32.mul (local.get $idx) (global.get $PS_SIZE))))
    (i32.store offset=12 (i32.add (local.get $desc) (local.get $slot)) (local.get $state)))

  (func (export "pipeline_get_stage_type") (param $desc i32) (param $idx i32) (result i32)
    (local $slot i32)
    (local.set $slot
      (i32.add (global.get $PD_STAGES)
        (i32.mul (local.get $idx) (global.get $PS_SIZE))))
    (i32.load (i32.add (local.get $desc) (local.get $slot))))

  (func (export "pipeline_get_tick") (param $desc i32) (result i32)
    (i32.load offset=16 (local.get $desc)))

  (func (export "pipeline_set_frame_size") (param $desc i32) (param $frame i32)
    (i32.store offset=20 (local.get $desc) (local.get $frame)))

  (func (export "pipeline_get_frame_size") (param $desc i32) (result i32)
    (i32.load offset=20 (local.get $desc)))

  ;; pipeline_run(desc, input_pipe, output_pipe, scratch, scap) → OK | MORE | error
  ;; Fuses consecutive batch stages (state_ptr==0) by reusing a single intermediate pipe.
  ;; When frame_size > 0, intermediate pipes use aligned capacity for zero-copy SIMD.
  (func $pipeline_run (export "pipeline_run")
    (param $desc i32) (param $input i32) (param $output i32)
    (param $scratch i32) (param $scap i32) (result i32)
    (local $count i32) (local $pcap i32) (local $frame i32)
    (local $i i32) (local $stype i32)
    (local $out i32) (local $prev i32) (local $result i32)
    (local $stages i32) (local $slot i32)
    (local $cfg i32) (local $clen i32) (local $state i32)
    (local $snapshot i32) (local $last_i i32)
    (local $reusable i32) (local $in_batch i32)

    (local.set $count (i32.load offset=12 (local.get $desc)))
    (local.set $last_i (i32.sub (local.get $count) (i32.const 1)))
    (local.set $pcap (i32.load offset=8 (local.get $desc)))
    (local.set $frame (i32.load offset=20 (local.get $desc)))
    (local.set $stages (i32.add (local.get $desc) (global.get $PD_STAGES)))
    (local.set $prev (local.get $input))

    ;; Increment tick for this run
    (i32.store offset=16 (local.get $desc)
      (i32.add (i32.load offset=16 (local.get $desc)) (i32.const 1)))

    ;; Save heap snapshot before allocating intermediate pipes
    (local.set $snapshot (call $pipe_snapshot))

    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $count)))
        (local.set $stype (i32.load (i32.add (local.get $stages) (local.get $slot))))
        (local.set $cfg (i32.load offset=4 (i32.add (local.get $stages) (local.get $slot))))
        (local.set $clen (i32.load offset=8 (i32.add (local.get $stages) (local.get $slot))))
        (local.set $state (i32.load offset=12 (i32.add (local.get $stages) (local.get $slot))))

        ;; Write current tick to state[0] if stage has state
        (if (local.get $state)
          (then (i32.store (local.get $state) (i32.load offset=16 (local.get $desc)))))

        ;; Determine output pipe.
        ;; Optimisation: consecutive batch stages reuse a single intermediate pipe.
        (if (i32.eq (local.get $i) (local.get $last_i))
          (then
            (local.set $out (local.get $output))
            (local.set $in_batch (i32.const 0)))
          (else
            (if (i32.and (local.get $in_batch) (i32.eqz (local.get $state)))
              (then
                ;; Consecutive batch stage: reuse the same pipe as prev and out.
                ;; The stage reads from prev (draining it, auto-reset) then writes to out.
                (local.set $out (local.get $prev)))
              (else
                (if (local.get $frame)
                  (then
                    (local.set $out (call $pipe_create_aligned (local.get $pcap) (local.get $frame)))
                    (if (i32.eq (local.get $out) (i32.const -1))
                      (then (local.set $result (i32.const -1)) (br $done))))
                  (else
                    (local.set $out (call $pipe_create (local.get $pcap)))
                    (if (i32.eq (local.get $out) (i32.const -1))
                      (then (local.set $result (i32.const -1)) (br $done)))))
                (if (i32.eqz (local.get $state))
                  (then
                    (local.set $reusable (local.get $out))
                    (local.set $in_batch (i32.const 1))))))))

        ;; Call stage via dispatch table
        (local.set $result
          (call_indirect (type $stage_fn)
            (local.get $prev) (local.get $out) (local.get $cfg) (local.get $clen)
            (local.get $scratch) (local.get $scap) (local.get $state)
            (local.get $stype)))

        ;; Break on error or yield
        (if (i32.or
              (i32.lt_s (local.get $result) (i32.const 0))
              (i32.eq (local.get $result) (global.get $STATUS_MORE)))
          (then (br $done)))

        ;; Stage completed — reset result to OK for pipeline return value
        (local.set $result (global.get $STATUS_OK))

        ;; Close previous intermediate if it was a distinct pipe (not fused/reused)
        (if (i32.and (local.get $i) (i32.ne (local.get $prev) (local.get $out)))
          (then (call $pipe_close (local.get $prev))))

        ;; End batch run if current stage has state (streaming)
        (if (local.get $state)
          (then (local.set $in_batch (i32.const 0))))

        (local.set $prev (local.get $out))
        (local.set $slot (i32.add (local.get $slot) (global.get $PS_SIZE)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))

    ;; On full success, restore heap to free intermediate pipes.
    ;; On MORE (yield), intermediate pipes must persist for next call.
    (if (i32.eq (local.get $result) (global.get $STATUS_OK))
      (then (call $pipe_restore (local.get $snapshot))))
    local.get $result)

  ;; ── Built-in passthrough stage (table index 0) ──
  (func $stage_passthrough
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (call $pipe_drain (local.get $input) (local.get $output) (local.get $scratch) (local.get $scap)))

  (elem (i32.const 0) $stage_passthrough)
