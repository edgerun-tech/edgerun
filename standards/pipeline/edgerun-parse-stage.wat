;; Edgerun Parse Stage — slot 46
  ;; Stage type: batch (state=0)
  ;; Input:  .edgerun source text via input pipe
  ;; Output: IR graph offset (i32) serialized as 4 bytes via output pipe
  ;;
  ;; Calls er_parse from lang/edgerun-parse.wat (merged in module scope).

  (func $process_edgerun_parse (export "process_edgerun_parse")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $len_slot i32) (local $read i32) (local $g i32)

    (local.set $len_slot (i32.sub (i32.add (local.get $scratch) (local.get $scap)) (i32.const 4)))
    (drop (call $pipe_read_ptr (local.get $input) (local.get $len_slot)))
    (local.set $read (i32.load (local.get $len_slot)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))

    ;; Copy source to scratch buffer for parsing
    (drop (call $pipe_read (local.get $input) (local.get $scratch) (local.get $read)))

    ;; Parse the .edgerun source
    (local.set $g (call $er_parse (local.get $scratch) (local.get $read)))
    (if (i32.eq (local.get $g) (i32.const -1)) (then (return (i32.const -1))))

    ;; Write graph offset to output pipe as 4-byte i32
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (i32.const 4)))
    (i32.store (local.get $scratch) (local.get $g))
    (i32.const 4)
  )
